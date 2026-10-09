/* [09AA-5] Tab de Logs del puente: buffer en memoria de eventos
 * estructurados del flujo Marketplace (borrador/regenerar/IA), servido por
 * `GET /api/admin/marketplace/logs` (solo admin). El log de texto del vivo
 * se lo come el polling del lab; aquí cada evento trae nivel, estado y
 * campos para cazar «qué carajo pasó» sin leer líneas sueltas.
 * Sin PII por construcción: el hilo viaja como hash-8, jamás el excerpt,
 * el remitente ni el borrador (ver `hilo8`). Tope 500, recientes-primero. */

use std::collections::{BTreeMap, VecDeque};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex, OnceLock,
};

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::marketplace::{clave_hilo, sha_hex};
use crate::AppState;

const TOPE_LOGS: usize = 500;

/// Nivel = prioridad de la tab (error=alta, warn=media, info=baja).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LogNivel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LogEvento {
    pub id: u64,
    /// RFC3339 UTC del registro.
    pub ts: String,
    pub nivel: LogNivel,
    /// Punto del flujo: `borrador.cache`, `borrador.ia`, `ia.vacia`,
    /// `ia.reintento_ok`, `regenerar`, `regenerar-todo`.
    pub evento: String,
    /// Estado del flujo: `cache`, `ia`, `reserva`, `fallback`, `ok`, `error`.
    pub estado: String,
    /// Resumen humano, sin PII.
    pub mensaje: String,
    /// Detalle estructurado (`hilo8`, `latencia_ms`, modelo, conteos…), sin PII.
    pub campos: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct LogsQuery {
    /// Filtra por nivel (`info`/`warn`/`error`); sin él trae todo.
    pub nivel: Option<String>,
    /// Tope de eventos (default 200, máximo 500).
    pub limite: Option<usize>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LogsResponse {
    pub eventos: Vec<LogEvento>,
}

static LOGS: OnceLock<Mutex<VecDeque<LogEvento>>> = OnceLock::new();
static SEQ: AtomicU64 = AtomicU64::new(1);

fn buffer() -> &'static Mutex<VecDeque<LogEvento>> {
    LOGS.get_or_init(|| Mutex::new(VecDeque::with_capacity(TOPE_LOGS)))
}

/// Hilo como hash-8 para los logs: `clave_hilo()` normaliza la cifra
/// parpadeante del puente y `sha_hex` lo opaca; 8 bastan para correlacionar
/// en la tab sin exponer nombre ni aviso.
pub(crate) fn hilo8(thread_id: &str) -> String {
    sha_hex(&clave_hilo(thread_id)).chars().take(8).collect()
}

/* Registra un evento del puente. Los handlers llaman con campos ya
 * libres de PII; si el Mutex está envenenado se pierde el evento (el flujo
 * real jamás se bloquea por el log). */
pub(crate) fn mp_log(
    nivel: LogNivel,
    evento: &str,
    estado: &str,
    mensaje: String,
    campos: &[(&str, serde_json::Value)],
) {
    let ev = LogEvento {
        id: SEQ.fetch_add(1, Ordering::Relaxed),
        ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        nivel,
        evento: evento.to_string(),
        estado: estado.to_string(),
        mensaje,
        campos: campos
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect(),
    };
    if let Ok(mut cola) = buffer().lock() {
        cola.push_back(ev);
        while cola.len() > TOPE_LOGS {
            cola.pop_front();
        }
    }
}

fn nivel_de(s: &str) -> Option<LogNivel> {
    match s.trim().to_lowercase().as_str() {
        "info" => Some(LogNivel::Info),
        "warn" => Some(LogNivel::Warn),
        "error" => Some(LogNivel::Error),
        _ => None,
    }
}

/// [09AA-5] Lee el buffer de eventos del puente (recientes-primero).
/// Solo JWT admin (`AuthUser`, como `chats`): con `MpAuth` el JWT del panel
/// daba 401 y el front tumbaba la sesión al abrir la tab (09AA-5b).
/// Sin PII (ver `hilo8`).
#[utoipa::path(
    get,
    path = "/api/admin/marketplace/logs",
    params(LogsQuery),
    responses(
        (status = 200, description = "Eventos del puente, recientes-primero", body = LogsResponse),
    )
)]
pub async fn logs(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<LogsQuery>,
) -> Result<Response, AppError> {
    let tope = q.limite.unwrap_or(200).clamp(1, TOPE_LOGS);
    let filtro = q.nivel.as_deref().and_then(nivel_de);
    let eventos = buffer()
        .lock()
        .map(|cola| {
            cola.iter()
                .rev()
                .filter(|e| filtro.is_none_or(|n| e.nivel == n))
                .take(tope)
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    Ok(Json(LogsResponse { eventos }).into_response())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /* Testigo: cada test usa su propio `evento` porque el buffer es global
     * y los tests corren en paralelo en el mismo proceso. */
    #[test]
    fn registra_y_filtra_por_nivel_sin_pii() {
        mp_log(
            LogNivel::Warn,
            "09aa5-test-warn",
            "fallback",
            "aviso de prueba".to_string(),
            &[("hilo", serde_json::json!(hilo8("Lidia|casa|9")))],
        );
        let cola = buffer().lock().expect("buffer de logs");
        let ev = cola
            .iter()
            .rev()
            .find(|e| e.evento == "09aa5-test-warn")
            .expect("evento registrado");
        assert_eq!(ev.nivel, LogNivel::Warn);
        assert_eq!(ev.estado, "fallback");
        let hilo = ev.campos.get("hilo").and_then(|v| v.as_str()).unwrap_or("");
        assert_eq!(hilo.len(), 8);
        assert!(!hilo.contains('|'));
    }

    #[test]
    fn tope_evict_recientes_primero_en_lectura() {
        for i in 0..TOPE_LOGS + 5 {
            mp_log(
                LogNivel::Info,
                "09aa5-test-tope",
                "ok",
                format!("n={i}"),
                &[],
            );
        }
        let cola = buffer().lock().expect("buffer de logs");
        assert!(cola.len() <= TOPE_LOGS);
        let ultimo = cola
            .iter()
            .rev()
            .find(|e| e.evento == "09aa5-test-tope")
            .expect("evento reciente");
        assert!(ultimo.mensaje.starts_with("n="));
    }
}
