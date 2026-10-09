/* F0 estructuradas/idempotencia extraído de `marketplace.rs` (split
 * god-object: el boundary superaba 800 efectivas). Comportamiento idéntico:
 * solo se movió de archivo; `marketplace.rs` conserva el wiring (llamadas).
 * Sin PII en logs: solo conteos, códigos e `hilo8`. */

use axum::http::HeaderMap;
use axum::response::Response;

use crate::errors::AppError;
use crate::services::marketplace::{
    clave_hilo, estructuradas_apagadas, normalizar_excerpt_hilo, texto_para_prompt,
    validar_conversacion, validar_idempotency_key, BorradorRequest, ConversacionValidada,
    FIRMA_VERSION_V2,
};

use super::mp_logs::{hilo8, mp_log, LogNivel};

/* [09AA-20] F0: de qué conversación hablamos. La estructurada (`Some` +
 * kill-switch encendido) se valida, se firma v2 y se renderiza a
 * `excerpt.texto`; el resto del flujo (caché, vuelo, IA) no distingue.
 * `firma_cache` es la llave de caché/dedup; `crudo` lo que se guarda para
 * calibrar. Sin PII en logs: solo conteos y códigos. */
pub(crate) struct FuenteBorrador {
    pub(crate) firma_cache: String,
    pub(crate) firma_version: String,
    pub(crate) crudo: String,
}

pub(crate) fn resolver_fuente(r: &mut BorradorRequest) -> Result<FuenteBorrador, AppError> {
    if let Some(c) = r.conversacion.as_ref() {
        if estructuradas_apagadas() {
            mp_log(
                LogNivel::Warn,
                "borrador.estructurada",
                "apagada",
                "kill-switch activo: va por texto plano".to_string(),
                &[("hilo", serde_json::json!(hilo8(r.thread_id.trim())))],
            );
        } else {
            match validar_conversacion(c) {
                Ok(val) => {
                    mp_log(
                        LogNivel::Info,
                        "borrador.estructurada",
                        "ok",
                        format!(
                            "{} útiles ({} sistema fuera, {} sin dueño)",
                            val.utiles.len(),
                            val.descartadas_sistema,
                            val.desconocidas
                        ),
                        &[
                            ("hilo", serde_json::json!(hilo8(r.thread_id.trim()))),
                            ("recibidas", serde_json::json!(val.total_burbujas)),
                            ("utiles", serde_json::json!(val.utiles.len())),
                            (
                                "descartadas_sistema",
                                serde_json::json!(val.descartadas_sistema),
                            ),
                            ("desconocidas", serde_json::json!(val.desconocidas)),
                        ],
                    );
                    let crudo =
                        serde_json::to_string(c).unwrap_or_else(|_| "{\"v\":1}".to_string());
                    r.excerpt.texto = texto_para_prompt(&val);
                    return Ok(fuente_v2(&val, crudo));
                }
                Err(e) => {
                    mp_log(
                        LogNivel::Warn,
                        "borrador.estructurada",
                        "rechazada",
                        format!("{}: se espera texto plano", e.codigo),
                        &[
                            ("hilo", serde_json::json!(hilo8(r.thread_id.trim()))),
                            ("codigo", serde_json::json!(e.codigo)),
                        ],
                    );
                    return Err(e.into());
                }
            }
        }
    }
    Ok(fuente_v1(r))
}

/// F0 aceptada: la firma v2 manda sobre la `firma` del body (el flotante la
/// manda igual por compat, pero la llave real es la calculada aquí).
pub(crate) fn fuente_v2(val: &ConversacionValidada, crudo: String) -> FuenteBorrador {
    FuenteBorrador {
        firma_cache: val.firma_v2.clone(),
        firma_version: FIRMA_VERSION_V2.to_string(),
        crudo,
    }
}

/* Texto plano legacy: mismo comportamiento de siempre (limpia excerpt,
 * conserva el original si solo había ruido, crudo para calibrar). */
pub(crate) fn fuente_v1(r: &mut BorradorRequest) -> FuenteBorrador {
    let crudo = r.excerpt.texto.clone();
    let limpio = normalizar_excerpt_hilo(&clave_hilo(r.thread_id.trim()), &r.excerpt.texto);
    if !limpio.is_empty() {
        r.excerpt.texto = limpio;
    }
    FuenteBorrador {
        firma_cache: r.firma.clone(),
        firma_version: "firma-v1".to_string(),
        crudo,
    }
}

/// `Idempotency-Key` opcional: si viene se valida (422 si es basura) y se
/// devuelve tal cual en la respuesta; además entra al vuelo para que un
/// reintento colapse con el original en vez de disparar otra IA.
pub(crate) fn clave_idempotencia(headers: &HeaderMap) -> Result<Option<String>, AppError> {
    let Some(valor) = headers.get("idempotency-key") else {
        return Ok(None);
    };
    let texto = valor.to_str().map_err(|_| {
        AppError::Validation("Idempotency-Key con caracteres inválidos".to_string())
    })?;
    validar_idempotency_key(texto).map_err(AppError::from)?;
    Ok(Some(texto.to_string()))
}

/// Devuelve la llave en la respuesta para que el flotante correlacione.
pub(crate) fn con_idempotencia(mut resp: Response, clave: Option<&String>) -> Response {
    if let Some(k) = clave {
        if let Ok(v) = axum::http::HeaderValue::from_str(k) {
            resp.headers_mut()
                .insert(axum::http::HeaderName::from_static("idempotency-key"), v);
        }
    }
    resp
}
