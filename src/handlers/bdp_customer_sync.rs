/* [Fase 7.1+7.2] Handlers para sync bidireccional de clientes Glory ↔ BDP.
 * POST /api/bdp/customers/import     — Importar clientes desde BDP a Glory (ExportCustomers)
 * POST /api/clientes/:id/bdp-sync    — Push de un cliente Glory a BDP (CreateCustomer)
 *
 * Patrón: sigue el mismo flujo que bdp_article_map::importar_catalogo.
 * Los campos BDP del cliente se mapean así:
 *   BDP.Customer      → clientes.bdp_customer_code
 *   BDP.FiscalName    → clientes.nombre + apellidos
 *   BDP.MobilePhone   → clientes.telefono
 *   BDP.EMail         → clientes.email
 *
 * Gotchas:
 *   - ExportCustomers puede devolver muchos registros (~43k) → import batch con progreso.
 *   - Matching Glory↔BDP: por teléfono (primario) o email (secundario).
 *   - CreateCustomer requiere `code` (entero) → siempre lo proporciona explícitamente el usuario. */

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use tracing::warn;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::CrearClienteRequest;
use crate::repositories::ClienteRepository;
use crate::services::{
    BdpCreateCustomerRequest, BdpExportCustomersRequest, BdpWeblinkClient, ClienteService,
    ConfiguracionService,
};
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bdp/customers/import", post(importar_clientes_bdp))
        .route("/clientes/:id/bdp-sync", post(sincronizar_cliente_bdp))
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct BdpCustomerSyncRequest {
    /// Código reservado explícitamente para este cliente. Nunca se calcula.
    pub bdp_customer_code: i32,
    /// Debe identificar exactamente el cliente y código que se crearán.
    pub confirmacion: String,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct BdpCustomerImportRequest {
    /// `false` solo calcula el impacto; `true` aplica cambios locales en Glory.
    #[serde(default)]
    pub aplicar: bool,
    /// Al aplicar debe ser exactamente `IMPORTAR CLIENTES BDP`.
    pub confirmacion: Option<String>,
}

/** Circuit breaker: máximo de errores consecutivos antes de abortar batch. */
const MAX_CONSECUTIVE_ERRORS: u32 = 10;

fn customer_code(value: &serde_json::Value) -> Option<i32> {
    value
        .get("Customer")
        .or_else(|| value.get("Code"))
        .and_then(|code| {
            code.as_i64()
                .or_else(|| code.as_str()?.trim().parse::<i64>().ok())
        })
        .and_then(|code| i32::try_from(code).ok())
        .filter(|code| *code > 0)
}

/* [Fase 7.1] Importar clientes desde BDP a Glory.
 * Llama a ExportCustomers, matchea por teléfono/email con clientes existentes,
 * y crea nuevos clientes en Glory si no existen. */
#[utoipa::path(
    post,
    path = "/api/bdp/customers/import",
    tag = "BDP Clientes",
    request_body = BdpCustomerImportRequest,
    responses(
        (status = 200, description = "Importación completada", body = serde_json::Value),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
#[allow(clippy::too_many_lines)]
pub async fn importar_clientes_bdp(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BdpCustomerImportRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if req.aplicar && req.confirmacion.as_deref() != Some("IMPORTAR CLIENTES BDP") {
        return Err(AppError::Validation(
            "Aplicación bloqueada: escriba exactamente IMPORTAR CLIENTES BDP. No se modificó Glory ni BDP."
                .into(),
        ));
    }
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "BDP no está configurado. Configura URL y credenciales primero.".into(),
        ));
    }

    let client = BdpWeblinkClient::new(&config);

    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    let customers_json = client
        .export_customers(&BdpExportCustomersRequest::default())
        .await
        .map_err(|e| AppError::Internal(format!("Error exportando clientes BDP: {e}")))?;

    /* Parsear array de clientes — BDP devuelve {"Customers": [...]} */
    let customers = customers_json
        .get("Customers")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Internal("Respuesta BDP no contiene array 'Customers'.".into()))?;

    let error_msg = customers_json
        .get("ErrorMessage")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !error_msg.is_empty() {
        return Err(AppError::Internal(format!(
            "BDP devolvió error: {error_msg}"
        )));
    }

    let mut importados: u32 = 0;
    let mut actualizados: u32 = 0;
    let mut sin_cambios: u32 = 0;
    let mut errores: u32 = 0;
    let mut conflictos: u32 = 0;
    let mut codigos_vistos = std::collections::HashSet::new();
    let mut muestra_conflictos = Vec::new();
    let mut consecutive_errors: u32 = 0;

    for cust in customers {
        #[allow(clippy::cast_possible_truncation)]
        let bdp_code = customer_code(cust).unwrap_or(0);
        let fiscal_name = cust
            .get("FiscalName")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let commercial_name = cust
            .get("CommercialName")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let mobile_phone = cust
            .get("MobilePhone")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let email = cust.get("EMail").and_then(|v| v.as_str()).unwrap_or("");
        let address = cust.get("Address").and_then(|v| v.as_str()).unwrap_or("");

        if bdp_code == 0
            || fiscal_name.trim().is_empty()
            || (mobile_phone.trim().is_empty() && email.trim().is_empty())
            || !codigos_vistos.insert(bdp_code)
        {
            errores += 1;
            consecutive_errors += 1;
            if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                break;
            }
            continue;
        }
        consecutive_errors = 0;

        /* Buscar cliente existente por teléfono o email */
        let existing = match ClienteRepository::find_by_telefono_o_email(
            &state.pool,
            auth.user_id,
            mobile_phone,
            email,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("[AUDIT-N5] Error DB buscando cliente: {e}");
                errores += 1;
                consecutive_errors += 1;
                if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                    break;
                }
                continue;
            }
        };

        if let Some(cliente) = existing {
            /* Nunca reemplazar un vínculo BDP diferente basándonos solo en una
             * coincidencia heurística de teléfono/email. */
            if cliente.bdp_customer_code.is_none() {
                if req.aplicar {
                    match ClienteRepository::update_bdp_sync(
                        &state.pool,
                        cliente.id,
                        Some(bdp_code),
                        true,
                        None,
                    )
                    .await
                    {
                        Ok(()) => {
                            actualizados += 1;
                            consecutive_errors = 0;
                        }
                        Err(e) => {
                            warn!("[AUDIT-N5] Error actualizando sync BDP: {e}");
                            errores += 1;
                            consecutive_errors += 1;
                            if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                                break;
                            }
                        }
                    }
                } else {
                    actualizados += 1;
                }
            } else if cliente.bdp_customer_code == Some(bdp_code) {
                sin_cambios += 1;
            } else {
                conflictos += 1;
                if muestra_conflictos.len() < 20 {
                    muestra_conflictos.push(serde_json::json!({
                        "cliente_id": cliente.id,
                        "codigo_local": cliente.bdp_customer_code,
                        "codigo_bdp": bdp_code,
                        "motivo": "identidad coincide pero ya está vinculada a otro código"
                    }));
                }
            }
        } else {
            /* Cliente no existe en Glory → crearlo */
            /* Dividir fiscal_name en nombre + apellidos (heurística: primera palabra = nombre) */
            let (nombre, apellidos) = split_name(fiscal_name);

            let nuevo = CrearClienteRequest {
                nombre: nombre.to_string(),
                apellidos: Some(apellidos.to_string()),
                telefono: Some(mobile_phone.to_string()),
                prefijo_telefono: None,
                email: Some(email.to_string()),
                empresa: Some(commercial_name.to_string()),
                notas: Some(format!("Importado de BDP (código {bdp_code}). {address}")),
                foto_url: None,
                consentimiento_comercial_email: None,
                consentimiento_comercial_sms: None,
                enviar_encuestas: None,
                alergias: None,
                preferencias_bebida: None,
                preferencias_ubicacion: None,
            };

            if req.aplicar {
                /* Crear localmente y vincular. El índice único por usuario y
                 * código BDP evita que una carrera duplique la identidad remota. */
                match ClienteService::create_bdp_import(&state.pool, auth.user_id, nuevo, bdp_code)
                    .await
                {
                    Ok(_) => {
                        importados += 1;
                        consecutive_errors = 0;
                    }
                    Err(e) => {
                        warn!("[AUDIT-N5] Error creando cliente importado: {e}");
                        errores += 1;
                        consecutive_errors += 1;
                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                            break;
                        }
                    }
                }
            } else {
                importados += 1;
            }
        }
    }

    Ok(Json(serde_json::json!({
        "imported": importados,
        "updated": actualizados,
        "unchanged": sin_cambios,
        "errors": errores,
        "conflicts": conflictos,
        "conflict_sample": muestra_conflictos,
        "total": customers.len(),
        "applied": req.aplicar,
        "writes_to_bdp": false,
    })))
} /* [Fase 7.2] Push controlado de un cliente Glory a BDP (CreateCustomer).
   * Exige código explícito, verifica colisión y siempre usa Overwrite=false.
   *
   * [AUDIT-1.3] Reconciliación: si BDP crea el cliente pero Glory no recibe
   * respuesta (timeout/crash), la auditoría queda "ambiguo". La próxima
   * tentativa será bloqueada por ensure_no_unresolved(). El operador debe:
   *   1. Consultar BDP directamente para verificar si el cliente fue creado.
   *   2. Si fue creado, usar la importación (POST /bdp/customers/import)
   *      para vincular el cliente Glory al código BDP existente.
   *   3. Si no fue creado, limpiar la auditoría ambigua manualmente.
   * No hay MarketplaceOrderId para clientes (solo para comandas). */
#[utoipa::path(
    post,
    path = "/api/clientes/{id}/bdp-sync",
    tag = "BDP Clientes",
    params(("id" = Uuid, Path, description = "ID del cliente")),
    request_body = BdpCustomerSyncRequest,
    responses(
        (status = 200, description = "Cliente sincronizado con BDP", body = serde_json::Value),
        (status = 400, description = "BDP no configurado", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 404, description = "Cliente no encontrado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
/* [187A-1] La secuencia de seguridad se mantiene lineal para que confirmación,
 * preflight, autorización, llamada única y cierre de auditoría sean auditables
 * en un solo flujo y no pueda omitirse accidentalmente una guarda. */
#[allow(clippy::too_many_lines)]
pub async fn sincronizar_cliente_bdp(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(sync_req): Json<BdpCustomerSyncRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let cliente = ClienteRepository::find_by_id(&state.pool, id, auth.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Error buscando cliente: {e}")))?
        .ok_or_else(|| AppError::NotFound("Cliente no encontrado".into()))?;

    let expected_confirmation = format!(
        "CREAR CLIENTE {} {} {}",
        cliente.nombre, cliente.apellidos, sync_req.bdp_customer_code
    );
    if sync_req.confirmacion.trim() != expected_confirmation {
        return Err(AppError::Validation(format!(
            "Confirmación inválida. Escriba exactamente: {expected_confirmation}"
        )));
    }
    let config = ConfiguracionService::obtener(&state.pool, auth.user_id).await?;

    if config.bdp_base_url.is_empty() || config.bdp_login.is_empty() {
        return Err(AppError::Validation(
            "BDP no está configurado. Configura URL y credenciales primero.".into(),
        ));
    }

    if config.bdp_sync_mode != "unidirectional" {
        return Err(AppError::Validation(
            "BDP está en modo solo lectura; no se ejecutó ninguna escritura.".into(),
        ));
    }
    if !config.bdp_auto_backup_before_write {
        return Err(AppError::Validation(
            "Escritura BDP bloqueada: auto-backup pre-write desactivado.".into(),
        ));
    }

    if sync_req.bdp_customer_code <= 0 {
        return Err(AppError::Validation(
            "Debe indicar un bdp_customer_code explícito mayor que cero.".into(),
        ));
    }

    /* Nunca sobrescribir ni generar códigos. */
    if cliente.bdp_customer_code.is_some() {
        return Err(AppError::Validation(
            "Cliente BDP existente: Overwrite está bloqueado y requiere una autorización específica no implementada."
                .into(),
        ));
    }
    let bdp_code = sync_req.bdp_customer_code;

    let client = BdpWeblinkClient::new(&config);
    let _session = client
        .login()
        .await
        .map_err(|e| AppError::Internal(format!("Error login BDP: {e}")))?;

    /* Preflight obligatorio de identidad/código. Overwrite=false protege la
     * carrera final, pero primero evitamos intentar una colisión conocida. */
    let exported = client
        .export_customers(&BdpExportCustomersRequest::default())
        .await
        .map_err(|e| AppError::Internal(format!("No se pudo verificar el código BDP: {e}")))?;
    let remote = exported
        .get("Customers")
        .and_then(serde_json::Value::as_array)
        .and_then(|customers| {
            customers
                .iter()
                .find(|customer| customer_code(customer) == Some(bdp_code))
        });
    if let Some(remote) = remote {
        let remote_phone = remote
            .get("MobilePhone")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim();
        let remote_email = remote
            .get("EMail")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim();
        let same_identity = (!cliente.telefono.trim().is_empty()
            && cliente.telefono.trim() == remote_phone)
            || (!cliente.email.trim().is_empty()
                && cliente.email.trim().eq_ignore_ascii_case(remote_email));
        if same_identity {
            ClienteRepository::update_bdp_sync(&state.pool, cliente.id, Some(bdp_code), true, None)
                .await
                .map_err(|e| AppError::Internal(format!("Error vinculando cliente BDP: {e}")))?;
            return Ok(Json(serde_json::json!({
                "cliente_id": cliente.id,
                "bdp_customer_code": bdp_code,
                "bdp_synced": true,
                "linked_existing": true
            })));
        }
        return Err(AppError::Conflict(format!(
            "El código BDP {bdp_code} ya pertenece a otro cliente; no se escribió nada."
        )));
    }

    /* Construir nombre completo: apellidos + nombre */
    let fiscal_name = if cliente.apellidos.is_empty() {
        cliente.nombre.clone()
    } else {
        format!("{} {}", cliente.apellidos, cliente.nombre)
    };

    let req = BdpCreateCustomerRequest {
        code: bdp_code,
        fiscal_name,
        commercial_name: cliente.nombre.clone(),
        mobile_phone: cliente.telefono.clone(),
        email: cliente.email.clone(),
        overwrite: false,
    };

    let datos_cliente = serde_json::json!({
        "cliente_id": cliente.id,
        "bdp_customer_code": bdp_code,
        "overwrite": false
    });
    crate::services::BdpWriteGuard::ensure_no_unresolved(
        &state.pool,
        auth.user_id,
        "cliente_id",
        cliente.id,
        &["create_customer"],
    )
    .await
    .map_err(AppError::Validation)?;
    let snapshot_pre_id = crate::services::BdpBackupService::preparar_snapshot_escritura(
        &state.pool,
        auth.user_id,
        "create_customer",
        &config,
        None,
    )
    .await
    .map_err(|e| {
        AppError::Validation(format!(
            "Pre-write audit BDP falló; creación de cliente bloqueada: {e}"
        ))
    })?;

    let audit_id = crate::services::BdpWriteGuard::authorize(
        &state.pool,
        auth.user_id,
        &config,
        "create_customer",
        "cliente",
        cliente.id,
        "cliente_id",
        &datos_cliente,
        snapshot_pre_id,
        None,
    )
    .await
    .map_err(AppError::Validation)?;

    let resp = match client.create_customer(&req).await {
        Ok(resp) => resp,
        Err(e) => {
            let msg = format!("Error creando cliente en BDP: {e}");
            let resultado = if matches!(
                e,
                crate::services::bdp_weblink::BdpWeblinkError::Http(_)
                    | crate::services::bdp_weblink::BdpWeblinkError::Api { .. }
            ) {
                "ambiguo"
            } else {
                "error"
            };
            crate::services::BdpBackupService::actualizar_resultado(
                &state.pool,
                audit_id,
                resultado,
                None,
                Some(&msg),
            )
            .await
            .map_err(|audit_error| {
                AppError::Internal(format!(
                    "{msg}; además falló el cierre de auditoría: {audit_error}"
                ))
            })?;
            return Err(AppError::Internal(msg));
        }
    };

    /* Verificar ErrorMessage de BDP */
    let error_msg = resp
        .get("ErrorMessage")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !error_msg.is_empty() {
        crate::services::BdpBackupService::actualizar_resultado(
            &state.pool,
            audit_id,
            "error",
            Some(&resp),
            Some(error_msg),
        )
        .await
        .map_err(|audit_error| {
            AppError::Internal(format!(
                "BDP rechazó el cliente y falló el cierre de auditoría: {audit_error}"
            ))
        })?;
        /* Guardar error en el cliente */
        let _ = ClienteRepository::update_bdp_sync(
            &state.pool,
            cliente.id,
            None,
            false,
            Some(error_msg),
        )
        .await;
        return Err(AppError::Validation(format!(
            "BDP devolvió error: {error_msg}"
        )));
    }

    /* [AUDIT-N1] Envolver marca local + auditoría en transacción atómica
     * para que, si el proceso muere después del HTTP, no quede
     * bdp_synced=true sin auditoría cerrada (o viceversa). */
    let commit_result = async {
        let mut tx = state
            .pool
            .begin()
            .await
            .map_err(|e| format!("Error iniciando tx post-create_customer: {e}"))?;

        sqlx::query(
            "UPDATE clientes SET bdp_customer_code = $2, bdp_synced = true, bdp_synced_at = NOW(), bdp_sync_error = NULL WHERE id = $1",
        )
        .bind(cliente.id)
        .bind(bdp_code)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!(
            "BDP confirmó el cliente {bdp_code}, pero no se pudo persistir el vínculo local: {e}"
        ))?;

        sqlx::query(
            r"UPDATE bdp_audit_log
            SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
            WHERE id = $1",
        )
        .bind(audit_id)
        .bind(Some(&resp))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Cliente confirmado, pero falló el cierre de auditoría: {e}"))?;

        tx.commit()
            .await
            .map_err(|e| format!("Error confirmando tx post-create_customer: {e}"))
    }
    .await;

    if let Err(e) = commit_result {
        /* La tx falló pero BDP ya creó el cliente → auditoría ambigua. */
        let _ = crate::services::BdpBackupService::actualizar_resultado(
            &state.pool,
            audit_id,
            "ambiguo",
            Some(&resp),
            Some(&e),
        )
        .await;
        return Err(AppError::Internal(e));
    }

    Ok(Json(serde_json::json!({
        "cliente_id": cliente.id,
        "bdp_customer_code": bdp_code,
        "bdp_synced": true,
        "bdp_synced_at": chrono::Utc::now().to_rfc3339(),
    })))
}

/// Heurística para dividir un nombre completo BDP en nombre + apellidos.
/// BDP envía "APELLIDOS NOMBRE" o "NOMBRE APELLIDOS" — asumimos que la
/// primera palabra es el nombre si el string tiene pocas palabras,
/// o que las últimas 2+ palabras son apellidos.
fn split_name(full_name: &str) -> (&str, &str) {
    let parts: Vec<&str> = full_name.split_whitespace().collect();
    match parts.len() {
        0 => ("Sin nombre", ""),
        1 => (parts[0], ""),
        2 => (parts[0], parts[1]),
        _ => {
            /* 3+ palabras: primera = nombre, resto = apellidos */
            let nombre = parts[0];
            let apellidos_start = full_name.find(parts[1]).unwrap_or(0);
            (nombre, &full_name[apellidos_start..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn customer_code_accepts_number_or_string_and_rejects_invalid() {
        assert_eq!(
            customer_code(&serde_json::json!({"Customer": 42})),
            Some(42)
        );
        assert_eq!(customer_code(&serde_json::json!({"Code": "43"})), Some(43));
        assert_eq!(customer_code(&serde_json::json!({"Customer": 0})), None);
        assert_eq!(
            customer_code(&serde_json::json!({"Customer": "otro"})),
            None
        );
    }

    #[test]
    fn split_name_never_panics_on_empty_or_single_name() {
        assert_eq!(split_name(""), ("Sin nombre", ""));
        assert_eq!(split_name("Ana"), ("Ana", ""));
        assert_eq!(split_name("Ana Pérez"), ("Ana", "Pérez"));
    }
}
