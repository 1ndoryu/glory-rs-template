/* [BKP-003] Motor de snapshots y auditoría BDP.
 * Gestiona snapshots (puntos de restauración) y audit log (traza inmutable).
 * Pre-write snapshots: selectivos, máximo 1 llamada adicional a BDP.
 * Restauración: solo datos locales de Glory (BDP no permite delete/update via API).
 * Todas las queries usan runtime sqlx (no macros compile-time) para compatibilidad SQLX_OFFLINE. */

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::{response_error_message, BdpWeblinkClient};
use crate::services::bdp_weblink_catalog::{
    BdpExportArticlesRequest, BdpExportCustomersRequest, BdpExportDepartmentsRequest,
    BdpGetEmployeesRequest, BdpGetOrderRequest, BdpGetRoomsTablesRequest, BdpOrderIdentifier,
};

/// Snapshot almacenado en la base de datos.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema, sqlx::FromRow)]
pub struct BdpSnapshot {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tipo: String,
    pub direccion: String,
    pub trigger_tipo: String,
    pub datos: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
    pub target_base_url: Option<String>,
    pub connection_fingerprint: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub notas: Option<String>,
}

/// Entrada del audit log.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema, sqlx::FromRow)]
pub struct BdpAuditEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub operacion: String,
    pub direccion: String,
    pub snapshot_pre_id: Option<Uuid>,
    pub datos_enviados: Option<serde_json::Value>,
    pub resultado: String,
    pub datos_respuesta: Option<serde_json::Value>,
    pub error_mensaje: Option<String>,
    pub target_base_url: Option<String>,
    pub target_entity_type: Option<String>,
    pub target_entity_id: Option<Uuid>,
    pub authorization_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Resultado de una restauración.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct RestoreResult {
    pub snapshot_id: Uuid,
    pub tipo: String,
    pub registros_restaurados: u32,
    pub errores: u32,
    pub detalles: String,
}

pub struct BdpBackupService;

impl BdpBackupService {
    /* [187A-1] La evidencia que habilita escritura pertenece a una conexión
     * exacta. El hash evita persistir secretos y hace inelegible cualquier
     * snapshot tomado con otras credenciales o parámetros operativos. */
    pub fn canonical_target(config: &ConfiguracionRestaurante) -> Result<String, String> {
        let raw = config.bdp_base_url.trim().trim_end_matches('/');
        let parsed = reqwest::Url::parse(raw)
            .map_err(|_| "La URL BDP configurada no es válida".to_string())?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.path() != "/"
        {
            return Err(
                "La URL BDP debe ser un origen HTTP(S) sin credenciales, query ni fragmento"
                    .to_string(),
            );
        }
        Ok(raw.to_string())
    }

    pub fn connection_fingerprint(config: &ConfiguracionRestaurante) -> Result<String, String> {
        let mut hasher = Sha256::new();
        for value in [
            Self::canonical_target(config)?,
            config.bdp_login.trim().to_string(),
            config.bdp_password.clone(),
            config.bdp_integrator_code.trim().to_string(),
            config.bdp_pos_id.to_string(),
            config.bdp_employee_id.to_string(),
            config.bdp_items_profile_id.to_string(),
        ] {
            hasher.update(value.as_bytes());
            hasher.update([0]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    // =========================================================================
    // SNAPSHOT BDP COMPLETO — lee TODOS los endpoints de lectura
    // =========================================================================

    /// Snapshot completo de BDP. Lee todos los endpoints de lectura.
    /// Costo: 5 llamadas a BDP (artículos, clientes, departamentos, salones, empleados).
    pub async fn snapshot_bdp_completo(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        notas: Option<String>,
    ) -> Result<BdpSnapshot, String> {
        let client = BdpWeblinkClient::new(config);

        /* Login primero para validar credenciales */
        let _session = client
            .login()
            .await
            .map_err(|e| format!("Error login BDP: {e}"))?;

        /* Recolectar datos de cada endpoint */
        let articulos = Self::fetch_articles(&client, config).await?;
        let clientes = Self::fetch_customers(&client).await?;
        let departamentos = Self::fetch_departments(&client).await?;
        let salones = Self::fetch_rooms(&client).await?;
        let empleados = Self::fetch_employees(&client).await?;

        let datos = serde_json::json!({
            "articulos": articulos,
            "clientes": clientes,
            "departamentos": departamentos,
            "salones": salones,
            "empleados": empleados,
        });

        let metadata = serde_json::json!({
            "endpoints": ["ExportArticles", "ExportCustomers", "ExportDepartments", "GetRoomsTables", "GetEmployees"],
        });

        /* Calcular expiración */
        let retention_days = Self::get_retention_days(pool, user_id).await?;
        let expires_at = if retention_days > 0 {
            Some(chrono::Utc::now() + chrono::Duration::days(i64::from(retention_days)))
        } else {
            None
        };

        Self::insert_snapshot(
            pool,
            user_id,
            "completo",
            "bdp",
            "manual",
            datos,
            Some(metadata),
            Some(Self::canonical_target(config)?),
            Some(Self::connection_fingerprint(config)?),
            expires_at,
            notas,
        )
        .await
    }

    // =========================================================================
    // SNAPSHOT PARCIAL BDP — solo los tipos seleccionados
    // =========================================================================

    /// Snapshot parcial de BDP. Solo los tipos de datos seleccionados.
    /// Tipos válidos: 'articulos', 'clientes', 'departamentos', 'salones', 'empleados'
    pub async fn snapshot_bdp_parcial(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        tipos: &[String],
        notas: Option<String>,
    ) -> Result<BdpSnapshot, String> {
        const VALID_TYPES: &[&str] = &[
            "articulos",
            "clientes",
            "departamentos",
            "salones",
            "empleados",
        ];
        if tipos.is_empty()
            || tipos
                .iter()
                .any(|tipo| !VALID_TYPES.contains(&tipo.as_str()))
            || tipos.iter().collect::<std::collections::HashSet<_>>().len() != tipos.len()
        {
            return Err(
                "Snapshot parcial bloqueado: tipos vacíos, repetidos o desconocidos".to_string(),
            );
        }
        let client = BdpWeblinkClient::new(config);
        let _session = client
            .login()
            .await
            .map_err(|e| format!("Error login BDP: {e}"))?;

        let mut datos = serde_json::json!({});
        let mut endpoints_used = Vec::new();

        for tipo in tipos {
            match tipo.as_str() {
                "articulos" => {
                    let val = Self::fetch_articles(&client, config).await?;
                    datos["articulos"] = val;
                    endpoints_used.push("ExportArticles");
                }
                "clientes" => {
                    let val = Self::fetch_customers(&client).await?;
                    datos["clientes"] = val;
                    endpoints_used.push("ExportCustomers");
                }
                "departamentos" => {
                    let val = Self::fetch_departments(&client).await?;
                    datos["departamentos"] = val;
                    endpoints_used.push("ExportDepartments");
                }
                "salones" => {
                    let val = Self::fetch_rooms(&client).await?;
                    datos["salones"] = val;
                    endpoints_used.push("GetRoomsTables");
                }
                "empleados" => {
                    let val = Self::fetch_employees(&client).await?;
                    datos["empleados"] = val;
                    endpoints_used.push("GetEmployees");
                }
                _ => unreachable!("los tipos se validaron antes de contactar BDP"),
            }
        }

        let metadata = serde_json::json!({
            "endpoints": endpoints_used,
            "tipos_solicitados": tipos,
        });

        let retention_days = Self::get_retention_days(pool, user_id).await?;
        let expires_at = if retention_days > 0 {
            Some(chrono::Utc::now() + chrono::Duration::days(i64::from(retention_days)))
        } else {
            None
        };

        Self::insert_snapshot(
            pool,
            user_id,
            &format!("parcial_{}", tipos.join("_")),
            "bdp",
            "manual",
            datos,
            Some(metadata),
            Some(Self::canonical_target(config)?),
            Some(Self::connection_fingerprint(config)?),
            expires_at,
            notas,
        )
        .await
    }

    // =========================================================================
    // SNAPSHOT GLORY — exporta tablas locales de Glory
    // =========================================================================

    /// Snapshot de datos locales de Glory (query local, 0 llamadas BDP).
    /// Tipos válidos: 'ventas', 'clientes', 'mapeos'
    pub async fn snapshot_glory(
        pool: &PgPool,
        user_id: Uuid,
        tipos: &[String],
        notas: Option<String>,
    ) -> Result<BdpSnapshot, String> {
        const VALID_TYPES: &[&str] = &["ventas", "clientes", "mapeos"];
        if tipos.is_empty()
            || tipos
                .iter()
                .any(|tipo| !VALID_TYPES.contains(&tipo.as_str()))
            || tipos.iter().collect::<std::collections::HashSet<_>>().len() != tipos.len()
        {
            return Err(
                "Snapshot Glory bloqueado: tipos vacíos, repetidos o desconocidos".to_string(),
            );
        }
        let mut datos = serde_json::json!({});

        for tipo in tipos {
            match tipo.as_str() {
                "ventas" => {
                    let val: serde_json::Value = sqlx::query_scalar(
                        r"SELECT COALESCE(json_agg(row_to_json(v)), '[]'::json)
                        FROM (
                            SELECT id, user_id, cliente_id, canal, total, estado,
                                   bdp_synced, bdp_order_id, bdp_order_status,
                                   bdp_sync_error, bdp_invoiced
                            FROM ventas
                            WHERE user_id = $1
                            ORDER BY created_at DESC
                            LIMIT 5000
                        ) v",
                    )
                    .bind(user_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error exportando ventas Glory: {e}"))?;
                    datos["ventas"] = val;
                }
                "clientes" => {
                    let val: serde_json::Value = sqlx::query_scalar(
                        r"SELECT COALESCE(json_agg(row_to_json(c)), '[]'::json)
                        FROM (
                            SELECT id, user_id, nombre, email, telefono,
                                   bdp_customer_code, bdp_synced, bdp_synced_at, bdp_sync_error
                            FROM clientes
                            WHERE user_id = $1
                            ORDER BY created_at DESC
                            LIMIT 5000
                        ) c",
                    )
                    .bind(user_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error exportando clientes Glory: {e}"))?;
                    datos["clientes"] = val;
                }
                "mapeos" => {
                    let val: serde_json::Value = sqlx::query_scalar(
                        r"SELECT COALESCE(json_agg(row_to_json(m)), '[]'::json)
                        FROM (
                            SELECT id, user_id, articulo_glory_codigo, articulo_bdp_codigo,
                                   articulo_bdp_nombre, descripcion, precio_tarifa1, iva_pct,
                                   departamento, familia, subfamilia, activo, barcode
                            FROM bdp_article_map
                            WHERE user_id = $1
                            ORDER BY articulo_bdp_nombre
                            LIMIT 10000
                        ) m",
                    )
                    .bind(user_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error exportando mapeos Glory: {e}"))?;
                    datos["mapeos"] = val;
                }
                _ => unreachable!("los tipos Glory se validaron antes de consultar la base"),
            }
        }

        let metadata = serde_json::json!({
            "tipos_solicitados": tipos,
            "source": "glory_local_db",
        });

        let retention_days = Self::get_retention_days(pool, user_id).await?;
        let expires_at = if retention_days > 0 {
            Some(chrono::Utc::now() + chrono::Duration::days(i64::from(retention_days)))
        } else {
            None
        };

        Self::insert_snapshot(
            pool,
            user_id,
            &format!("glory_{}", tipos.join("_")),
            "glory",
            "manual",
            datos,
            Some(metadata),
            None,
            None,
            expires_at,
            notas,
        )
        .await
    }

    // =========================================================================
    // PRE-WRITE AUDIT LOG — selectivo, máximo 1 llamada BDP
    // =========================================================================

    /// Prepara la evidencia previa a una escritura. Para pago y factura el
    /// estado remoto de la comanda es obligatorio; si no se puede capturar,
    /// la autorización permanece intacta y no se crea una intención de envío.
    pub async fn preparar_snapshot_escritura(
        pool: &PgPool,
        user_id: Uuid,
        operacion: &str,
        config: &ConfiguracionRestaurante,
        bdp_order_id: Option<i64>,
    ) -> Result<Option<Uuid>, String> {
        let auto_backup = Self::get_auto_backup(pool, user_id).await?;
        if !auto_backup {
            return Err("Escritura BDP bloqueada: auto-backup pre-write desactivado".to_string());
        }

        if !matches!(operacion, "add_payment" | "invoice") {
            return Ok(None);
        }

        let order_id = bdp_order_id
            .ok_or_else(|| format!("Snapshot pre-write {operacion} bloqueado: falta order_id"))?;
        let snapshot = Self::snapshot_order_state(config, order_id).await?;
        let snap = Self::insert_snapshot(
            pool,
            user_id,
            "pre_write_order",
            "bdp",
            "pre_write",
            snapshot,
            Some(serde_json::json!({"operacion": operacion, "order_id": order_id})),
            Some(Self::canonical_target(config)?),
            Some(Self::connection_fingerprint(config)?),
            None,
            None,
        )
        .await?;
        Ok(Some(snap.id))
    }

    /// Actualiza el resultado de una entrada de auditoría.
    pub async fn actualizar_resultado(
        pool: &PgPool,
        audit_id: Uuid,
        resultado: &str,
        datos_respuesta: Option<&serde_json::Value>,
        error_mensaje: Option<&str>,
    ) -> Result<(), String> {
        let result = sqlx::query(
            r"UPDATE bdp_audit_log
            SET resultado = $2, datos_respuesta = $3, error_mensaje = $4, updated_at = NOW()
            WHERE id = $1",
        )
        .bind(audit_id)
        .bind(resultado)
        .bind(datos_respuesta)
        .bind(error_mensaje)
        .execute(pool)
        .await
        .map_err(|e| format!("Error actualizando audit log: {e}"))?;

        if result.rows_affected() != 1 {
            return Err(format!(
                "No se actualizó la auditoría BDP {audit_id}: registro inexistente"
            ));
        }
        Ok(())
    }

    // =========================================================================
    // CONSULTAS
    // =========================================================================

    /// Lista snapshots del usuario, ordenados por fecha descendente.
    pub async fn listar_snapshots(
        pool: &PgPool,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<BdpSnapshot>, String> {
        sqlx::query_as::<_, BdpSnapshot>(
            r"SELECT id, user_id, tipo, direccion, trigger_tipo, datos, metadata,
                      target_base_url, connection_fingerprint, created_at, expires_at, notas
            FROM bdp_snapshots
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error listando snapshots: {e}"))
    }

    /// Obtiene un snapshot por ID.
    pub async fn obtener_snapshot(
        pool: &PgPool,
        snapshot_id: Uuid,
    ) -> Result<Option<BdpSnapshot>, String> {
        sqlx::query_as::<_, BdpSnapshot>(
            r"SELECT id, user_id, tipo, direccion, trigger_tipo, datos, metadata,
                      target_base_url, connection_fingerprint, created_at, expires_at, notas
            FROM bdp_snapshots
            WHERE id = $1",
        )
        .bind(snapshot_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Error obteniendo snapshot: {e}"))
    }

    /// Elimina un snapshot.
    pub async fn eliminar_snapshot(
        pool: &PgPool,
        snapshot_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, String> {
        let result = sqlx::query(r"DELETE FROM bdp_snapshots WHERE id = $1 AND user_id = $2")
            .bind(snapshot_id)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| format!("Error eliminando snapshot: {e}"))?;

        Ok(result.rows_affected() > 0)
    }

    /// Lista entradas del audit log.
    pub async fn listar_audit(
        pool: &PgPool,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<BdpAuditEntry>, String> {
        sqlx::query_as::<_, BdpAuditEntry>(
            r"SELECT id, user_id, operacion, direccion, snapshot_pre_id,
                      datos_enviados, resultado, datos_respuesta, error_mensaje,
                      target_base_url, target_entity_type, target_entity_id,
                      authorization_reason, created_at, updated_at
            FROM bdp_audit_log
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error listando audit log: {e}"))
    }

    // =========================================================================
    // RESTAURACIÓN (solo datos locales de Glory)
    // =========================================================================

    /// Restaura datos de Glory desde un snapshot.
    /// NOTA: BDP no permite delete/update via API — solo podemos restaurar datos locales.
    /* [187A-1] Restauración legacy limitada a datos locales; mantener ambos
     * recorridos juntos facilita contar y reportar fallos parciales sin ocultarlos. */
    #[allow(clippy::too_many_lines)]
    pub async fn restaurar_glory(
        pool: &PgPool,
        snapshot_id: Uuid,
        user_id: Uuid,
    ) -> Result<RestoreResult, String> {
        let snapshot = Self::obtener_snapshot(pool, snapshot_id)
            .await?
            .ok_or_else(|| "Snapshot no encontrado".to_string())?;

        if snapshot.user_id != user_id {
            return Err("No autorizado: el snapshot pertenece a otro usuario".to_string());
        }

        if snapshot.direccion != "glory" {
            return Err(format!(
                "Solo se pueden restaurar snapshots de Glory (este es de {})",
                snapshot.direccion
            ));
        }

        let mut registros_restaurados: u32 = 0;
        let mut errores: u32 = 0;
        let mut detalles = Vec::new();

        /* [207A-3] S14-H1: Envolver toda la restauración en una transacción
         * atómica. Si falla cualquier UPDATE intermedio, se revierte todo
         * para evitar estado parcial (mapeos restaurados pero clientes no). */
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("Error iniciando tx de restauración: {e}"))?;

        /* Restaurar mapeos de artículos */
        if let Some(mapeos) = snapshot
            .datos
            .get("mapeos")
            .and_then(serde_json::Value::as_array)
        {
            for mapeo in mapeos {
                let Some(id) = mapeo.get("id").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                let Ok(id) = Uuid::parse_str(id) else {
                    continue;
                };
                let descripcion = mapeo.get("descripcion").and_then(serde_json::Value::as_str);
                let precio = mapeo
                    .get("precio_tarifa1")
                    .and_then(serde_json::Value::as_f64);
                let iva = mapeo.get("iva_pct").and_then(serde_json::Value::as_f64);
                let activo = mapeo.get("activo").and_then(serde_json::Value::as_bool);

                let r = sqlx::query(
                    r"UPDATE bdp_article_map
                    SET descripcion = COALESCE($3, descripcion),
                        precio_tarifa1 = COALESCE($4, precio_tarifa1),
                        iva_pct = COALESCE($5, iva_pct),
                        activo = COALESCE($6, activo)
                    WHERE id = $1 AND user_id = $2",
                )
                .bind(id)
                .bind(user_id)
                .bind(descripcion)
                .bind(precio.and_then(|p| rust_decimal::Decimal::try_from(p).ok()))
                .bind(iva.and_then(|i| rust_decimal::Decimal::try_from(i).ok()))
                .bind(activo)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Error restaurando mapeo {id}: {e}"))?;

                if r.rows_affected() > 0 {
                    registros_restaurados += 1;
                } else {
                    detalles.push(format!("Mapeo {id} no encontrado"));
                    errores += 1;
                }
            }
        }

        /* Restaurar campos BDP de clientes */
        if let Some(clientes) = snapshot
            .datos
            .get("clientes")
            .and_then(serde_json::Value::as_array)
        {
            for cliente in clientes {
                let Some(id) = cliente.get("id").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                let Ok(id) = Uuid::parse_str(id) else {
                    continue;
                };
                let bdp_code = cliente
                    .get("bdp_customer_code")
                    .and_then(serde_json::Value::as_i64)
                    .and_then(|code| i32::try_from(code).ok());

                let r = sqlx::query(
                    r"UPDATE clientes
                    SET bdp_customer_code = COALESCE($3, bdp_customer_code)
                    WHERE id = $1 AND user_id = $2",
                )
                .bind(id)
                .bind(user_id)
                .bind(bdp_code)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Error restaurando cliente {id}: {e}"))?;

                if r.rows_affected() > 0 {
                    registros_restaurados += 1;
                } else {
                    detalles.push(format!("Cliente {id} no encontrado"));
                    errores += 1;
                }
            }
        }

        tx.commit()
            .await
            .map_err(|e| format!("Error confirmando restauración: {e}"))?;

        let detalles_str = if detalles.is_empty() {
            format!("Restaurados {registros_restaurados} registros sin errores")
        } else {
            format!(
                "Restaurados {registros_restaurados} registros, {errores} errores. {}",
                detalles.join("; ")
            )
        };

        Ok(RestoreResult {
            snapshot_id,
            tipo: snapshot.tipo,
            registros_restaurados,
            errores,
            detalles: detalles_str,
        })
    }

    // =========================================================================
    // LIMPIEZA DE SNAPSHOTS EXPIRADOS
    // =========================================================================

    /// Elimina snapshots expirados.
    /* [D3] No eliminar snapshots que aún están referenciados por auditorías
     * pendientes o ambiguas. Sin esta protección, una auditoría pendiente
     * podría perder su evidencia pre-escritura. */
    pub async fn limpiar_expirados(pool: &PgPool) -> Result<u64, String> {
        let result = sqlx::query(
            r"DELETE FROM bdp_snapshots
              WHERE expires_at IS NOT NULL AND expires_at < NOW()
                AND NOT EXISTS (
                    SELECT 1 FROM bdp_audit_log a
                    WHERE a.snapshot_pre_id = bdp_snapshots.id
                      AND a.resultado IN ('pendiente', 'ambiguo')
                )",
        )
        .execute(pool)
        .await
        .map_err(|e| format!("Error limpiando snapshots expirados: {e}"))?;

        Ok(result.rows_affected())
    }

    // =========================================================================
    // HELPERS PRIVADOS
    // =========================================================================

    /* [187A-1] Helper interno de persistencia: los argumentos reflejan uno a
     * uno el registro inmutable de evidencia y todos los llamadores son locales. */
    #[allow(clippy::too_many_arguments)]
    async fn insert_snapshot(
        pool: &PgPool,
        user_id: Uuid,
        tipo: &str,
        direccion: &str,
        trigger_tipo: &str,
        datos: serde_json::Value,
        metadata: Option<serde_json::Value>,
        target_base_url: Option<String>,
        connection_fingerprint: Option<String>,
        expires_at: Option<DateTime<Utc>>,
        notas: Option<String>,
    ) -> Result<BdpSnapshot, String> {
        sqlx::query_as::<_, BdpSnapshot>(
            r"INSERT INTO bdp_snapshots
               (user_id, tipo, direccion, trigger_tipo, datos, metadata,
                target_base_url, connection_fingerprint, expires_at, notas)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, user_id, tipo, direccion, trigger_tipo, datos, metadata,
                      target_base_url, connection_fingerprint, created_at, expires_at, notas",
        )
        .bind(user_id)
        .bind(tipo)
        .bind(direccion)
        .bind(trigger_tipo)
        .bind(datos)
        .bind(metadata)
        .bind(target_base_url)
        .bind(connection_fingerprint)
        .bind(expires_at)
        .bind(notas)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Error insertando snapshot: {e}"))
    }

    async fn snapshot_order_state(
        config: &ConfiguracionRestaurante,
        order_id: i64,
    ) -> Result<serde_json::Value, String> {
        let client = BdpWeblinkClient::new(config);
        let order_data = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|e| format!("Error obteniendo estado de comanda {order_id}: {e}"))?;
        if let Some(error) = response_error_message(&order_data) {
            return Err(format!(
                "Snapshot pre-write bloqueado: GetOrder {order_id} devolvió {error}"
            ));
        }
        if !order_data
            .get("Order")
            .is_some_and(serde_json::Value::is_object)
        {
            return Err(format!(
                "Snapshot pre-write bloqueado: GetOrder {order_id} no devolvió una orden válida"
            ));
        }
        Ok(order_data)
    }

    async fn get_retention_days(pool: &PgPool, user_id: Uuid) -> Result<i32, String> {
        sqlx::query_scalar::<_, i32>(
            r"SELECT COALESCE(bdp_backup_retention_days, 30)
            FROM configuracion_restaurante WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("Error leyendo retención de snapshots BDP: {error}"))?
        .ok_or_else(|| "No existe configuración para retención de snapshots BDP".to_string())
    }

    async fn get_auto_backup(pool: &PgPool, user_id: Uuid) -> Result<bool, String> {
        sqlx::query_scalar::<_, bool>(
            r"SELECT COALESCE(bdp_auto_backup_before_write, true)
            FROM configuracion_restaurante WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("Error leyendo auto-backup BDP: {error}"))?
        .ok_or_else(|| "No existe configuración para auto-backup BDP".to_string())
    }

    async fn fetch_articles(
        client: &BdpWeblinkClient<'_>,
        config: &ConfiguracionRestaurante,
    ) -> Result<serde_json::Value, String> {
        let value = client
            .export_articles(&BdpExportArticlesRequest::all_web_articles(
                config.bdp_pos_id,
            ))
            .await
            .map_err(|error| format!("Snapshot BDP abortado al leer artículos: {error}"))?;
        Self::validate_snapshot_response(
            "artículos",
            value,
            &[
                "ArticlesListData",
                "ArticleListData",
                "Articles",
                "ArticleList",
            ],
        )
    }

    async fn fetch_customers(client: &BdpWeblinkClient<'_>) -> Result<serde_json::Value, String> {
        let value = client
            .export_customers(&BdpExportCustomersRequest::default())
            .await
            .map_err(|error| format!("Snapshot BDP abortado al leer clientes: {error}"))?;
        Self::validate_snapshot_response("clientes", value, &["Customers", "CustomerList"])
    }

    async fn fetch_departments(client: &BdpWeblinkClient<'_>) -> Result<serde_json::Value, String> {
        let value = client
            .export_departments(&BdpExportDepartmentsRequest::default())
            .await
            .map_err(|error| format!("Snapshot BDP abortado al leer departamentos: {error}"))?;
        Self::validate_snapshot_response(
            "departamentos",
            value,
            &["Departments", "Department", "DepartmentList"],
        )
    }

    async fn fetch_rooms(client: &BdpWeblinkClient<'_>) -> Result<serde_json::Value, String> {
        let value = client
            .get_rooms_tables(&BdpGetRoomsTablesRequest::default())
            .await
            .map_err(|error| format!("Snapshot BDP abortado al leer salones: {error}"))?;
        Self::validate_snapshot_response("salones", value, &["Rooms", "RoomList"])
    }

    async fn fetch_employees(client: &BdpWeblinkClient<'_>) -> Result<serde_json::Value, String> {
        let value = client
            .get_employees(&BdpGetEmployeesRequest {
                ids: vec![],
                only_salespeople: None,
            })
            .await
            .map_err(|error| format!("Snapshot BDP abortado al leer empleados: {error}"))?;
        Self::validate_snapshot_response(
            "empleados",
            value,
            &["Employees", "Employee", "EmployeeList"],
        )
    }

    fn validate_snapshot_response(
        label: &str,
        value: serde_json::Value,
        expected_keys: &[&str],
    ) -> Result<serde_json::Value, String> {
        if let Some(error) = response_error_message(&value) {
            return Err(format!(
                "Snapshot BDP abortado: la lectura de {label} devolvió {error}"
            ));
        }
        let has_payload = expected_keys
            .iter()
            .any(|key| value.get(*key).is_some_and(|payload| !payload.is_null()));
        if !has_payload {
            return Err(format!(
                "Snapshot BDP abortado: la lectura de {label} no contiene una colección reconocible"
            ));
        }
        Ok(value)
    }
}

/* [S16-H3] Tests unitarios para canonical_target y connection_fingerprint.
 * Valida que la URL se canonicaliza correctamente y que se rechazan URLs
 * con componentes peligrosos (credenciales, query, fragmento, path). */
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use uuid::Uuid;

    fn config_with_url(url: &str) -> ConfiguracionRestaurante {
        ConfiguracionRestaurante {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            reserva_email_obligatorio: false,
            reserva_telefono_obligatorio: true,
            reserva_nombre_obligatorio: true,
            reserva_apellidos_obligatorio: false,
            iva_por_defecto: Decimal::new(10, 0),
            nombre_restaurante: "Test".to_string(),
            groq_api_key: None,
            auto_venta_reserva: false,
            hora_desayuno_inicio: chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            hora_desayuno_fin: chrono::NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
            hora_comida_inicio: chrono::NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
            hora_comida_fin: chrono::NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            hora_cena_inicio: chrono::NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
            hora_cena_fin: chrono::NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
            url_haddock: String::new(),
            haddock_api_token: String::new(),
            haddock_sync_enabled: false,
            bdp_base_url: url.to_string(),
            bdp_login: "user".to_string(),
            bdp_password: "pass".to_string(),
            bdp_integrator_code: "INT".to_string(),
            bdp_sync_enabled: true,
            bdp_pos_id: 1,
            bdp_employee_id: 1,
            bdp_items_profile_id: 1,
            bdp_default_article_code: String::new(),
            bdp_default_article_name: String::new(),
            bdp_tender_map: serde_json::json!({}),
            bdp_order_type_map: serde_json::json!({}),
            bdp_default_customer_code: String::new(),
            bdp_poll_interval_secs: 60,
            bdp_poll_enabled: false,
            google_review_url: String::new(),
            telefono_restaurante: String::new(),
            url_reservas: String::new(),
            bdp_auto_sync_customers: false,
            bdp_sync_mode: "read_only".to_string(),
            bdp_backup_retention_days: 30,
            bdp_auto_backup_before_write: true,
            bdp_env_bootstrap_applied_at: None,
            ff_bdp_auto_arm: false,
            ff_bdp_partial_payments: false,
            ff_bdp_cancel_order: false,
            ff_bdp_purchase_notes_read: false,
            ff_bdp_purchase_notes_draft: false,
            ff_bdp_purchase_notes_receive: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /* [S16-H3] canonical_target acepta orígenes HTTP/HTTPS limpios */
    #[test]
    fn canonical_target_accepts_clean_http_origin() {
        let c = config_with_url("http://bdp.example.com:8068");
        assert_eq!(BdpBackupService::canonical_target(&c).unwrap(), "http://bdp.example.com:8068");
    }

    #[test]
    fn canonical_target_accepts_https_origin() {
        let c = config_with_url("https://bdp.cliente.com");
        assert_eq!(BdpBackupService::canonical_target(&c).unwrap(), "https://bdp.cliente.com");
    }

    #[test]
    fn canonical_target_strips_trailing_slash() {
        let c = config_with_url("http://127.0.0.1:8068/");
        assert_eq!(BdpBackupService::canonical_target(&c).unwrap(), "http://127.0.0.1:8068");
    }

    /* [S16-H3] canonical_target rechaza URLs con componentes peligrosos */
    #[test]
    fn canonical_target_rejects_url_with_path() {
        let c = config_with_url("http://bdp.example.com/api");
        assert!(BdpBackupService::canonical_target(&c).is_err());
    }

    #[test]
    fn canonical_target_rejects_url_with_query() {
        let c = config_with_url("http://bdp.example.com?token=abc");
        assert!(BdpBackupService::canonical_target(&c).is_err());
    }

    #[test]
    fn canonical_target_rejects_url_with_fragment() {
        let c = config_with_url("http://bdp.example.com#section");
        assert!(BdpBackupService::canonical_target(&c).is_err());
    }

    #[test]
    fn canonical_target_rejects_url_with_credentials() {
        let c = config_with_url("http://user:pass@bdp.example.com");
        assert!(BdpBackupService::canonical_target(&c).is_err());
    }

    #[test]
    fn canonical_target_rejects_empty_url() {
        let c = config_with_url("");
        assert!(BdpBackupService::canonical_target(&c).is_err());
    }

    /* [S16-H3] connection_fingerprint produce hash determinista */
    #[test]
    fn connection_fingerprint_is_deterministic() {
        let c = config_with_url("http://bdp.example.com:8068");
        let fp1 = BdpBackupService::connection_fingerprint(&c).unwrap();
        let fp2 = BdpBackupService::connection_fingerprint(&c).unwrap();
        assert_eq!(fp1, fp2);
        assert!(!fp1.is_empty());
    }

    #[test]
    fn connection_fingerprint_differs_for_different_urls() {
        let c1 = config_with_url("http://bdp-a.example.com");
        let c2 = config_with_url("http://bdp-b.example.com");
        assert_ne!(
            BdpBackupService::connection_fingerprint(&c1).unwrap(),
            BdpBackupService::connection_fingerprint(&c2).unwrap()
        );
    }
}
