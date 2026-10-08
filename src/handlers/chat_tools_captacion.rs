//! [08AA-8] Tools de captación, contacto y escalado a humano.
//!
//! Extraído de `chat_tools.rs` (god-object + límite 500): registrar contacto,
//! contacto público, delegación (`consultar`/`escalar` + aviso), captación
//! (`pedido_captacion`/`captar`) y visita (`agendar`). El dispatcher
//! `Herramientas::ejecutar` y los tests siguen resolviendo estos nombres
//! vía re-export en `chat_tools.rs`; `chat.rs` sigue usando
//! `chat_tools::contacto_publico`. `telefono_valido` queda en `chat_tools`
//! (lo comparten captación, staff y visitante).

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{CreateSolicitudRequest, OPERACIONES};
use crate::repositories::{ClienteRepository, NuevaVisita, VisitaRepository};
use crate::services::SolicitudService;
use crate::services::{
    clave_idempotencia, corte_cubre, debe_usar_clave, encolar_outbox_idem, Encolado,
};
use glory_agent::errors::AgentError;

use super::chat_tools::telefono_valido;

pub(super) async fn registrar(
    pool: &PgPool,
    session_id: Uuid,
    args: &Value,
) -> Result<Value, AgentError> {
    let nombre = args
        .get("nombre")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let telefono = args
        .get("telefono")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if nombre.is_empty() || nombre.len() > 80 {
        return Ok(json!({"error": "nombre requerido (1..80)"}));
    }
    if !telefono_valido(telefono) {
        return Ok(json!({"error": "telefono invalido"}));
    }
    glory_agent::persistence::set_session_contact(pool, session_id, Some(nombre), Some(telefono))
        .await?;
    /* [279A-2 F1] El contacto también vive en `clientes` (una fila por
     * teléfono, upsert idempotente): sin este paso la IA captaría datos
     * que nadie puede consultar. Si falla se propaga (nada silencioso).
     * [Fase3-H1] Ficha SIN re-vincular: el hilo pertenece al remitente real
     * y un número dictado no lo re-clavea (antes partía el hilo: segunda
     * vuelta sin historial). */
    ClienteRepository::registrar_sin_vincular(pool, Some(nombre), telefono)
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    Ok(json!({"ok": true}))
}

/// Teléfono oficial: config `contacto_telefono` o `AGENTE_CONTACTO`.
/// `whatsapp_url` listo para el widget (`wa.me`, solo dígitos).
/// Pública para reutilizar en `GET /api/agent/info` (misma fuente que la tool).
pub async fn contacto_publico(pool: &PgPool, defecto: &str) -> Result<Value, AgentError> {
    let telefono = glory_agent::persistence::get_config(pool, "contacto_telefono")
        .await
        .ok()
        .flatten()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| defecto.to_string());
    let admin = glory_agent::persistence::get_config(pool, "whatsapp_admin")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let digitos: String = admin.chars().filter(char::is_ascii_digit).collect();
    let url = if digitos.len() >= 6 {
        format!("https://wa.me/{digitos}")
    } else {
        String::new()
    };
    Ok(json!({"telefono": telefono, "whatsapp": admin, "whatsapp_url": url}))
}

/* [279A-2 F3] Delegación con congelamiento (máquina `atencion_sesiones`):
 * `consultar` = duda puntual (consultando + `ai_enabled=false` + ciclo
 * `waiting`; el humano devuelve con nota y la IA retoma); `escalar` =
 * delegación total (delegada + triple freno `escalated`+ciclo+`ai_enabled`;
 * la IA no vuelve sola). Ambas encolan `whatsapp` con ficha (el worker la
 * arma vía `ficha_para_aviso`) y `destino` explícito cuando hay
 * `whatsapp_admin` (si no, el worker usa el fallback y queda `pending`). */

pub(super) async fn destino_humano(pool: &PgPool) -> Option<String> {
    glory_agent::persistence::get_config(pool, "whatsapp_admin")
        .await
        .ok()
        .flatten()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub(super) fn resumen_breve(args: &Value) -> String {
    args.get("resumen")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(|| "-".to_string(), |s| s.chars().take(500).collect())
}

/* [E14] Aviso al humano compartido por `consultar`/`escalar`/`captar`:
 * `via` = canal de la sesión (el gateway responde por el mismo número
 * que escribió el cliente; sin vínculo cae a `wa_a`) y `destino`
 * explícito cuando hay `whatsapp_admin` (si no, el worker usa el
 * fallback y queda `pending`: aviso parcial antes que ninguno). */
pub(super) async fn aviso_humano(
    pool: &PgPool,
    session_id: Uuid,
    motivo: &str,
    resumen: &str,
) -> Result<(), AgentError> {
    let mut aviso = json!({
        "session_id": session_id.to_string(),
        "motivo": motivo,
        "resumen": resumen,
    });
    if let Ok(canal) = ClienteRepository::canal_de(pool, session_id).await {
        aviso["via"] = json!(canal.as_deref().unwrap_or("wa_a"));
    }
    if let Some(destino) = destino_humano(pool).await {
        aviso["destino"] = json!(destino);
    }
    /* [011A-5 Fase1] Clave bajo corte (motivos consultar/escalar/captar…);
     * `manual` queda excluido por `debe_usar_clave`. */
    let canal_aviso = aviso.get("via").and_then(|v| v.as_str()).unwrap_or("wa_a");
    let clave_aviso;
    let clave_aviso_ref = if debe_usar_clave(motivo) && corte_cubre(pool, canal_aviso).await {
        clave_aviso = clave_idempotencia(&session_id.to_string(), motivo, resumen);
        Some(clave_aviso.as_str())
    } else {
        None
    };
    /* [011A-5 Fase3] `Revivido` también envía (gemelo `failed` que vuelve
     * a `pending`): solo `Duplicado` se reporta como tragado. */
    if matches!(
        encolar_outbox_idem(pool, "whatsapp", aviso, clave_aviso_ref).await?,
        Encolado::Duplicado
    ) {
        tracing::info!(
            "aviso humano sesion={session_id} motivo={motivo}: duplicado tragado por idempotency_key"
        );
    }
    Ok(())
}

pub(super) async fn consultar(
    pool: &PgPool,
    session_id: Uuid,
    args: &Value,
) -> Result<Value, AgentError> {
    let motivo = args
        .get("motivo")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if motivo.is_empty() || motivo.len() > 300 {
        return Ok(json!({"error": "motivo requerido (1..300)"}));
    }
    let resumen = resumen_breve(args);
    ClienteRepository::marcar_atencion(pool, session_id, "consultando")
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    glory_agent::persistence::set_session_ai(pool, session_id, false).await?;
    glory_agent::persistence::upsert_response_cycle(pool, session_id, "waiting").await?;
    aviso_humano(pool, session_id, motivo, &resumen).await?;
    Ok(json!({"ok": true}))
}

pub(super) async fn escalar(
    pool: &PgPool,
    session_id: Uuid,
    args: &Value,
) -> Result<Value, AgentError> {
    let motivo = args
        .get("motivo")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if motivo.is_empty() || motivo.len() > 300 {
        return Ok(json!({"error": "motivo requerido (1..300)"}));
    }
    let resumen = resumen_breve(args);
    glory_agent::persistence::set_session_status(pool, session_id, "escalated").await?;
    glory_agent::persistence::upsert_response_cycle(pool, session_id, "escalated").await?;
    glory_agent::persistence::set_session_ai(pool, session_id, false).await?;
    ClienteRepository::marcar_atencion(pool, session_id, "delegada")
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    aviso_humano(pool, session_id, motivo, &resumen).await?;
    let telefono = glory_agent::persistence::get_config(pool, "contacto_telefono")
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty());
    Ok(json!({"ok": true, "telefono": telefono.unwrap_or_default()}))
}

/* [E14] Captación: el visitante quiere vender/alquilar SU propiedad.
 * Crea la `solicitud` (origen `whatsapp`, siempre `pendiente`: la crea
 * el visitante, nunca la IA) con el mismo servicio del formulario web,
 * marca `captacion` (la IA confirma y sigue disponible; el captador
 * llama por teléfono fuera del chat) y avisa al humano con la ficha. */
pub(super) fn arg_texto(args: &Value, clave: &str) -> String {
    args.get(clave)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

pub(super) fn mapear_error_solicitud(e: AppError) -> AgentError {
    match e {
        AppError::Validation(m) | AppError::BadRequest(m) => AgentError::BadRequest(m),
        AppError::NotFound(m) => AgentError::Internal(format!("solicitud no encontrada: {m}")),
        AppError::Conflict(m) | AppError::Forbidden(m) => {
            AgentError::Internal(format!("rechazada: {m}"))
        }
        AppError::Unauthorized => AgentError::Internal("no autorizado".to_string()),
        AppError::PayloadMuyGrande => AgentError::Internal("carga demasiado grande".to_string()),
        AppError::Internal(m) => AgentError::Internal(m),
        AppError::Database(e) => AgentError::Db(e.to_string()),
        AppError::Io(e) => AgentError::Internal(e.to_string()),
    }
}

/* Valida los args de `registrar_captacion` y arma el pedido para el
 * servicio web. `Err` = `{"error": ...}` para que la IA se corrija. */
pub(super) fn pedido_captacion(
    args: &Value,
) -> Result<(CreateSolicitudRequest, String, String), Value> {
    let nombre = arg_texto(args, "nombre");
    let telefono = arg_texto(args, "telefono");
    let ubicacion = arg_texto(args, "ubicacion");
    let descripcion = arg_texto(args, "descripcion");
    if nombre.is_empty() || nombre.len() > 200 {
        return Err(json!({"error": "nombre requerido (1..200)"}));
    }
    if !telefono_valido(&telefono) {
        return Err(json!({"error": "telefono invalido"}));
    }
    if ubicacion.is_empty() || ubicacion.len() > 300 {
        return Err(json!({"error": "ubicacion requerida (1..300)"}));
    }
    if descripcion.is_empty() || descripcion.len() > 2000 {
        return Err(json!({"error": "descripcion requerida (1..2000)"}));
    }
    let operacion = arg_texto(args, "operacion");
    let operacion = if operacion.is_empty() {
        "venta".to_string()
    } else {
        operacion
    };
    if !OPERACIONES.contains(&operacion.as_str()) {
        return Err(json!({"error": "operacion invalida (venta|alquiler)"}));
    }
    let req = CreateSolicitudRequest {
        nombre: nombre.clone(),
        telefono,
        email: {
            let e = arg_texto(args, "email");
            if e.is_empty() {
                None
            } else {
                Some(e)
            }
        },
        descripcion,
        ubicacion,
        puestos: args
            .get("puestos")
            .and_then(Value::as_i64)
            .and_then(|p| i32::try_from(p).ok())
            .unwrap_or(0)
            .max(0),
        residencia: arg_texto(args, "residencia"),
        precio_estimado: args.get("precio_estimado").and_then(Value::as_f64),
        operacion: operacion.clone(),
        fotos: vec![],
        origen_contacto: Some("whatsapp".to_string()),
    };
    Ok((req, operacion, nombre))
}

pub(super) async fn captar(
    pool: &PgPool,
    session_id: Uuid,
    args: &Value,
) -> Result<Value, AgentError> {
    let (req, operacion, nombre) = match pedido_captacion(args) {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let telefono = req.telefono.clone();
    let ubicacion = req.ubicacion.clone();
    let descripcion = req.descripcion.clone();
    let solicitud = match SolicitudService::create(
        pool,
        req,
        None,
        Some("glory-ia/whatsapp".to_string()),
    )
    .await
    {
        Ok(s) => s,
        /* Fallo de validación del servicio = la IA pasó algo mal:
         * se devuelve como `error` para que se corrija en el turno. */
        Err(AppError::Validation(m) | AppError::BadRequest(m)) => {
            return Ok(json!({"error": m}));
        }
        Err(e) => return Err(mapear_error_solicitud(e)),
    };
    ClienteRepository::marcar_atencion(pool, session_id, "captacion")
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    let mut resumen =
        format!("Captación {operacion} en {ubicacion} — {nombre} {telefono}: {descripcion}");
    if let Some(p) = solicitud.precio_estimado {
        resumen = format!("{resumen} (estima {p})");
    }
    let resumen: String = resumen.chars().take(500).collect();
    aviso_humano(pool, session_id, "captacion", &resumen).await?;
    Ok(json!({"ok": true, "solicitud_id": solicitud.id}))
}

/* [E15] Visita: el visitante quiere ver un inmueble del catálogo. Valida
 * que el inmueble exista y esté publicado, abre la fila en `pendiente`
 * (el agente confirma día/hora) y congela la IA como `consultar` (el
 * humano confirma con nota y la IA retoma): la cita sin día cerrado no
 * es una delegación total. `fecha` solo viaja si el visitante dio un
 * día exacto; si no, el humano la fija al confirmar. */
pub(super) async fn agendar(
    pool: &PgPool,
    session_id: Uuid,
    args: &Value,
) -> Result<Value, AgentError> {
    let id: Uuid = args
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .parse()
        .map_err(|_| AgentError::BadRequest("id de inmueble invalido".to_string()))?;
    /* Barato primero: sin nombre/teléfono/cuándo/fecha válidos no se toca
     * la BD (la IA se corrige en el turno sin costo de consulta). */
    let nombre = arg_texto(args, "nombre");
    let telefono = arg_texto(args, "telefono");
    let cuando = arg_texto(args, "cuando");
    if nombre.is_empty() || nombre.len() > 200 {
        return Ok(json!({"error": "nombre requerido (1..200)"}));
    }
    if !telefono_valido(&telefono) {
        return Ok(json!({"error": "telefono invalido"}));
    }
    if cuando.is_empty() || cuando.len() > 200 {
        return Ok(json!({"error": "cuando requerido (1..200)"}));
    }
    let fecha = match arg_texto(args, "fecha") {
        f if f.is_empty() => None,
        f => match chrono::NaiveDate::parse_from_str(&f, "%Y-%m-%d") {
            Ok(d) => Some(d),
            Err(_) => return Ok(json!({"error": "fecha invalida (YYYY-MM-DD)"})),
        },
    };
    let titulo: Option<String> =
        sqlx::query_scalar("SELECT titulo FROM inmuebles WHERE id = $1 AND publicado")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AgentError::Db(e.to_string()))?;
    let Some(titulo) = titulo.filter(|t| !t.trim().is_empty()) else {
        return Ok(json!({"error": "inmueble no disponible"}));
    };
    let visita = VisitaRepository::crear(
        pool,
        NuevaVisita {
            inmueble_id: id,
            session_id,
            nombre: nombre.clone(),
            telefono: telefono.clone(),
            cuando: cuando.clone(),
            fecha,
        },
    )
    .await
    .map_err(|e| AgentError::Db(e.to_string()))?;
    /* Congelar como `consultar`: el humano confirma día/hora y la IA
     * retoma con la nota (una cita no es delegación total). */
    ClienteRepository::marcar_atencion(pool, session_id, "consultando")
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    glory_agent::persistence::set_session_ai(pool, session_id, false).await?;
    glory_agent::persistence::upsert_response_cycle(pool, session_id, "waiting").await?;
    let resumen: String = format!(
        "Visita {} — {cuando} — {nombre} {telefono} (cita {})",
        titulo.trim(),
        visita.id
    )
    .chars()
    .take(500)
    .collect();
    aviso_humano(pool, session_id, "visita", &resumen).await?;
    Ok(json!({"ok": true, "visita_id": visita.id}))
}
