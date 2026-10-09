//! [08AA-8] Proveedores de IA (`GloryAPI` + `OpenCode` Go + Groq STT).
//!
//! Extraído de `ia.rs` (god-object + límite 500): cliente HTTP, `ping`
//! de ambos proveedores, `completar` de ambos, transcripción de audio con
//! rotación de claves y descripción de fotos. `ia.rs` conserva el boundary
//! HTTP (`routes`, estado, config, probar, completar) y re-exporta lo que
//! usan sus rutas, sus tests y terceros (`marketplace`, `turno`).

use std::time::Duration;

use crate::errors::AppError;

use super::ia::{glory_base, leer_env, MAX_FOTOS};
/* [09AA-5] Eventos de la tab de Logs (sin PII). */
use super::mp_logs::{mp_log, LogNivel};

fn cliente_http(segs: u64) -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(segs))
        .build()
        .map_err(|e| AppError::Internal(format!("AI http: {e}")))
}

pub(super) async fn ping_glory(base: &str, key: &str) -> Result<String, String> {
    let cliente = cliente_http(30).map_err(|e| e.to_string())?;
    let resp = cliente
        .post(format!("{base}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "model": "auto",
            "stream": false,
            "max_tokens": 16,
            "messages": [
                {"role": "system", "content": "Responde exactamente: OK"},
                {"role": "user", "content": "ping"},
            ],
        }))
        .send()
        .await
        .map_err(|e| format!("GloryAPI red: {e}"))?;
    if resp.status() == 401 || resp.status() == 403 {
        return Err("GloryAPI rechazo la clave (revisa GLORY_API_KEY)".to_string());
    }
    if !resp.status().is_success() {
        return Err(format!("GloryAPI HTTP {}", resp.status()));
    }
    let cuerpo: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("GloryAPI respuesta no JSON: {e}"))?;
    Ok(cuerpo
        .pointer("/model")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("auto")
        .to_string())
}

pub(super) async fn ping_opencode(key: &str) -> Result<String, String> {
    let cliente = cliente_http(30).map_err(|e| e.to_string())?;
    let config = glory_agent::providers::ProviderConfig::opencode_go(key.to_string());
    let cuerpo = glory_agent::providers::build_responses_body(
        &config.model,
        &[serde_json::json!({"role": "user", "content": "Responde exactamente: OK"})],
        None,
        glory_agent::providers::ChatApiOptions::terse(16),
    );
    let resp = cliente
        .post(config.responses_url())
        .header("Authorization", format!("Bearer {key}"))
        .header("x-opencode-session", uuid::Uuid::new_v4().to_string())
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| format!("OpenCode Go red: {e}"))?;
    if resp.status() == 401 || resp.status() == 403 {
        return Err("OpenCode Go rechazo la clave (revisa OPENCODE_GO_API_KEY)".to_string());
    }
    if !resp.status().is_success() {
        let trozo: String = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect();
        return Err(format!("OpenCode Go HTTP: {trozo}"));
    }
    Ok(config.model.clone())
}

fn texto_glory(cuerpo: &serde_json::Value) -> Option<String> {
    let contenido = cuerpo.pointer("/choices/0/message/content")?;
    let texto = contenido.as_str()?;
    let texto = texto.trim();
    if texto.is_empty() {
        None
    } else {
        Some(texto.to_string())
    }
}

pub(super) async fn completar_glory(
    system: &str,
    texto: &str,
    fotos: &[String],
) -> Result<(String, String), String> {
    let key = leer_env("GLORY_API_KEY");
    if key.is_empty() {
        return Err("Sin GLORY_API_KEY en .env".to_string());
    }
    let base = glory_base();
    let mut partes = vec![serde_json::json!({"type": "text", "text": texto})];
    for foto in fotos.iter().take(MAX_FOTOS) {
        partes.push(serde_json::json!({"type": "image_url", "image_url": {"url": foto}}));
    }
    let cliente = cliente_http(120).map_err(|e| e.to_string())?;
    let resp = cliente
        .post(format!("{base}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "model": "auto",
            "stream": false,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": partes},
            ],
        }))
        .send()
        .await
        .map_err(|e| format!("GloryAPI red: {e}"))?;
    if resp.status() == 401 || resp.status() == 403 {
        return Err("GloryAPI rechazo la clave (revisa GLORY_API_KEY)".to_string());
    }
    if !resp.status().is_success() {
        let trozo: String = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect();
        return Err(format!("GloryAPI HTTP: {trozo}"));
    }
    let cuerpo: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("GloryAPI respuesta no JSON: {e}"))?;
    let modelo = cuerpo
        .pointer("/model")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("auto")
        .to_string();
    texto_glory(&cuerpo)
        .map(|t| (t, modelo))
        .ok_or_else(|| "GloryAPI devolvio una respuesta sin texto".to_string())
}

/* [03AA-3 M3] Se expone al handler `marketplace` para generar borradores;
 * sigue sin ruta HTTP propia (solo `probar`/`completar` del centro de IA).
 * [09AA-4] Diagnóstico sin PII de una respuesta Responses sin texto: solo
 * forma (estado, tipos de `output[]`, uso, motivo de `incomplete`), nunca el
 * contenido. El vacío existe de verdad (2026-10-09: 200 sin `message`, sin
 * `AI incompleta` en el log = no es tope de tokens; el relay estaba sano en
 * replay mínimo), así que ante un vacío persistente el WARN dice QUÉ vino. */
pub(super) fn diagnostico_respuesta_vacia(resp: &serde_json::Value) -> String {
    let estado = resp
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    let mut tipos: Vec<&str> = Vec::new();
    if let Some(items) = resp.get("output").and_then(serde_json::Value::as_array) {
        for item in items {
            if let Some(t) = item.get("type").and_then(serde_json::Value::as_str) {
                tipos.push(t);
            }
        }
    }
    let motivo = resp
        .get("incomplete_details")
        .and_then(|d| d.get("reason"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("-");
    let uso = resp.get("uso").map_or_else(
        || {
            resp.get("usage")
                .map_or_else(|| "-".to_string(), uso_responses)
        },
        uso_responses,
    );
    format!(
        "estado={estado} tipos=[{}] motivo={motivo} uso={uso}",
        tipos.join(",")
    )
}

/// `in/out` de `usage` como texto; ausente o no numérico → `-`.
fn uso_responses(uso: &serde_json::Value) -> String {
    let num = |c: &str| uso.get(c).and_then(serde_json::Value::as_u64).unwrap_or(0);
    format!("in={} out={}", num("input_tokens"), num("output_tokens"))
}

async fn llamar_opencode(
    config: &glory_agent::providers::ProviderConfig,
    entrada: &[serde_json::Value],
    cliente: &reqwest::Client,
    tope: u32,
    sesion: &str,
) -> Result<serde_json::Value, String> {
    glory_agent::providers::call_provider(
        config,
        entrada,
        None,
        glory_agent::providers::ChatApiOptions {
            max_output_tokens: tope,
            timeout_secs: 120,
        },
        Some(sesion),
        cliente,
    )
    .await
    .map_err(|e| format!("OpenCode Go: {e}"))
}

/* [09AA-15] Entrada Responses compartida por la vía estándar y la rápida
 * (mismo system + texto + fotos): lo único que cambia es el cuerpo
 * (tope + `reasoning.effort`) y el timeout. */
fn entrada_borrador(system: &str, texto: &str, fotos: &[String]) -> Vec<serde_json::Value> {
    let mut contenido: Vec<serde_json::Value> =
        vec![serde_json::json!({"type": "input_text", "text": texto})];
    for foto in fotos.iter().take(MAX_FOTOS) {
        contenido.push(serde_json::json!({"type": "input_image", "image_url": foto}));
    }
    vec![
        serde_json::json!({"role": "system", "content": system}),
        serde_json::json!({"role": "user", "content": contenido}),
    ]
}

/* `sesion`: id estable por conversación para `x-opencode-session` (afinidad
 * de ruteo + prompt caching del relay; ver docs de Go). Estable = mismo
 * valor para el mismo hilo (un hash, jamás PII en claro); quien no tiene
 * hilo pasa una etiqueta fija (`"centro-ia"`, `"fotos"`). Un uuid fresco por
 * llamada también evita el 400, pero rompe la afinidad y parece abuso. */
pub(crate) async fn completar_opencode(
    system: &str,
    texto: &str,
    fotos: &[String],
    sesion: &str,
) -> Result<(String, String), String> {
    /* [08AA-19] 4000, no 2500: el modelo razona antes de redactar
     * y un prompt normal ya quema ~1788 tokens de razonamiento
     * (medido 2026-10-08 contra el endpoint real); con 2500 el
     * borrador largo caía en `incomplete` sin `message` y acababa
     * en `reserva`. El texto útil son ~150 tokens.
     * [09AA-4] 8000, no 4000: el vacío del 2026-10-09 no era tope
     * (sin `AI incompleta`), pero el razonamiento varía por hilo y
     * 4000 se midió una sola vez; 8000 da aire como `standard()`
     * (8192) sin cambiar el coste del texto útil. */
    const TOPE_BORRADOR: u32 = 8000;
    let key = leer_env("OPENCODE_GO_API_KEY");
    if key.is_empty() {
        return Err("Sin OPENCODE_GO_API_KEY en .env".to_string());
    }
    let config = glory_agent::providers::ProviderConfig::opencode_go(key.clone());
    let entrada = entrada_borrador(system, texto, fotos);
    let cliente = cliente_http(120).map_err(|e| e.to_string())?;
    let mut respuesta = llamar_opencode(&config, &entrada, &cliente, TOPE_BORRADOR, sesion).await?;
    let mut texto_ia = glory_agent::providers::extract_first_text(&respuesta);
    /* [09AA-4] Un reintento ante vacío: el relay a veces devuelve 200 sin
     * `message` de forma transitoria (incidente 2026-10-09 con relay sano
     * en replay). Solo en el camino de fallo, como mucho una llamada más. */
    if texto_ia.is_none() {
        respuesta = llamar_opencode(&config, &entrada, &cliente, TOPE_BORRADOR, sesion).await?;
        texto_ia = glory_agent::providers::extract_first_text(&respuesta);
        /* [09AA-5] Lo que hizo el reintento queda en la tab de Logs (el
         * WARN de abajo sigue yendo además al log de texto del vivo). */
        if texto_ia.is_some() {
            mp_log(
                LogNivel::Info,
                "ia.reintento_ok",
                "ia",
                "el reintento trajo texto; se usa la IA".to_string(),
                &[("modelo", serde_json::json!(config.model))],
            );
        } else {
            mp_log(
                LogNivel::Warn,
                "ia.vacia",
                "fallback",
                format!(
                    "OpenCode Go vacío tras reintento ({}); va fallback",
                    diagnostico_respuesta_vacia(&respuesta)
                ),
                &[("modelo", serde_json::json!(config.model))],
            );
        }
    }
    texto_ia.map(|t| (t, config.model.clone())).ok_or_else(|| {
        tracing::warn!(
            "OpenCode Go vacío persistente ({}); va fallback",
            diagnostico_respuesta_vacia(&respuesta)
        );
        "OpenCode Go devolvio una respuesta sin texto".to_string()
    })
}

/* [09AA-15] Vía rápida sin razonamiento para borradores (pedido de ella:
 * el borrador tardó 75 s en un hilo). Sonda real contra el relay: SÍ
 * acepta `"reasoning": {"effort": "low"}` (la nota de 09AA-3 que decía que
 * no había perilla era inferencia sin probar; lo que da 400 es
 * `reasoningSummary`, no `effort`). Con esfuerzo bajo el modelo piensa
 * ~60 tokens en vez de ~2000 (medido con prompt mínimo: 61 de
 * razonamiento en 2,1 s), así que el tope puede ser chico (2500 cubre
 * razonamiento + ~150 de texto útil). Fail-open: ante 400, vacío,
 * `incomplete`, timeout o red se usa la vía estándar y queda
 * `ia.rapido_fallback` en Logs; el llamador no cambia de forma. Solo la
 * usa el borrador de Marketplace; centro IA y fotos siguen estándar. */
const TOPE_BORRADOR_RAPIDO: u32 = 2500;
const TIMEOUT_RAPIDO_SEGS: u64 = 45;

/// Cuerpo Responses igual al estándar más `reasoning.effort=low`.
/// Pura (testeable sin red).
fn cuerpo_borrador_rapido(modelo: &str, entrada: &[serde_json::Value]) -> serde_json::Value {
    let mut cuerpo = glory_agent::providers::build_responses_body(
        modelo,
        entrada,
        None,
        glory_agent::providers::ChatApiOptions {
            max_output_tokens: TOPE_BORRADOR_RAPIDO,
            timeout_secs: TIMEOUT_RAPIDO_SEGS,
        },
    );
    cuerpo["reasoning"] = serde_json::json!({"effort": "low"});
    cuerpo
}

/// Un intento rápido; `None` = ir a la vía estándar (nunca error: el
/// llamador decide el fallback y lo anota en Logs).
async fn intento_borrador_rapido(
    config: &glory_agent::providers::ProviderConfig,
    entrada: &[serde_json::Value],
    sesion: &str,
) -> Option<(String, String)> {
    let cliente = cliente_http(TIMEOUT_RAPIDO_SEGS).ok()?;
    let cuerpo = cuerpo_borrador_rapido(&config.model, entrada);
    let resp = cliente
        .post(config.responses_url())
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .header("x-opencode-session", sesion)
        .timeout(std::time::Duration::from_secs(TIMEOUT_RAPIDO_SEGS))
        .json(&cuerpo)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    if json.get("status").and_then(serde_json::Value::as_str) == Some("incomplete") {
        return None;
    }
    let texto = glory_agent::providers::extract_first_text(&json)?;
    Some((texto, config.model.clone()))
}

pub(crate) async fn completar_opencode_rapido(
    system: &str,
    texto: &str,
    fotos: &[String],
    sesion: &str,
) -> Result<(String, String), String> {
    let key = leer_env("OPENCODE_GO_API_KEY");
    if key.is_empty() {
        return Err("Sin OPENCODE_GO_API_KEY en .env".to_string());
    }
    let config = glory_agent::providers::ProviderConfig::opencode_go(key);
    let entrada = entrada_borrador(system, texto, fotos);
    if let Some(rapido) = intento_borrador_rapido(&config, &entrada, sesion).await {
        return Ok(rapido);
    }
    /* [09AA-5] La caída a la vía estándar queda en la tab de Logs (el
     * WARN de abajo sigue yendo además al log de texto del vivo). */
    mp_log(
        LogNivel::Info,
        "ia.rapido_fallback",
        "ia",
        "la vía rápida no trajo texto; va la estándar".to_string(),
        &[("modelo", serde_json::json!(config.model))],
    );
    completar_opencode(system, texto, fotos, sesion).await
}

/// [309A-4] Transcribe una nota de voz con Groq Whisper
/// (`whisper-large-v3-turbo`, endpoint OpenAI-compatible). Opencode Go no
/// pasa audio por ninguna vía (verificado: Responses pela `input_audio`,
/// chat pela `audio_url`, sin endpoint `/audio/transcriptions`), así que el
/// STT sale por aquí. Best-effort: cualquier `Err` y el llamador conserva el
/// `[audio]` pelado + el prompt pide que lo escriban.
/// [309A-5] Rotación: prueba `GROQ_API_KEY` + respaldos `_2..=_9` en orden;
/// ante 401/403/429/5xx/red rota a la siguiente (WARN con el # de clave,
/// nunca la clave). Agotadas todas → `Err` y fail-open en el hilo.
pub(crate) async fn transcribir_audio(
    bytes: &[u8],
    nombre: &str,
    mime: &str,
) -> Result<String, String> {
    const GROQ_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
    transcribir_audio_con(bytes, nombre, mime, &claves_groq(), GROQ_URL).await
}

/// [309A-5] Claves STT en orden (primaria + respaldos no vacíos).
fn claves_groq() -> Vec<String> {
    let mut todas = vec![leer_env("GROQ_API_KEY")];
    for n in 2..=9 {
        todas.push(leer_env(&format!("GROQ_API_KEY_{n}")));
    }
    todas.into_iter().filter(|k| !k.is_empty()).collect()
}

/// [309A-5] Núcleo con claves+URL inyectables (testeable con mock local).
/// 400 no rota (petición malformada: reintentar no ayuda); 200 vacío sí rota
/// (puede ser flakiness del modelo). Timeout 120 s por intento: corre en el
/// spawn del webhook, nunca en el camino del 2xx ni del turno.
pub(super) async fn transcribir_audio_con(
    bytes: &[u8],
    nombre: &str,
    mime: &str,
    claves: &[String],
    url: &str,
) -> Result<String, String> {
    const MODELO: &str = "whisper-large-v3-turbo";
    const MAX_TEXTO: usize = 2000;
    if claves.is_empty() {
        return Err("Sin GROQ_API_KEY en .env".to_string());
    }
    let mut ultimo = "sin intentos".to_string();
    for (i, key) in claves.iter().enumerate() {
        let parte = reqwest::multipart::Part::bytes(bytes.to_vec())
            .file_name(nombre.to_string())
            .mime_str(mime)
            .map_err(|e| format!("Groq audio invalido: {e}"))?;
        let forma = reqwest::multipart::Form::new()
            .part("file", parte)
            .text("model", MODELO)
            .text("language", "es")
            .text("response_format", "text");
        let cliente = cliente_http(120).map_err(|e| e.to_string())?;
        let resp = match cliente
            .post(url)
            .header("Authorization", format!("Bearer {key}"))
            .multipart(forma)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                ultimo = format!("Groq red: {e}");
                tracing::warn!("STT: clave #{} sin red, roto a la siguiente", i + 1);
                continue;
            }
        };
        let estado = resp.status();
        if estado == 401 || estado == 403 || estado == 429 || estado.is_server_error() {
            ultimo = format!("Groq HTTP {estado} con clave #{}", i + 1);
            tracing::warn!("STT: {ultimo}, roto a la siguiente");
            continue;
        }
        if !estado.is_success() {
            let trozo: String = resp
                .text()
                .await
                .unwrap_or_default()
                .chars()
                .take(200)
                .collect();
            return Err(format!("Groq HTTP {estado}: {trozo}"));
        }
        let texto: String = resp
            .text()
            .await
            .map_err(|e| format!("Groq respuesta no texto: {e}"))?
            .trim()
            .chars()
            .take(MAX_TEXTO)
            .collect();
        if texto.is_empty() {
            ultimo = "Groq devolvio transcripcion vacia".to_string();
            tracing::warn!("STT: {ultimo}, roto a la siguiente");
            continue;
        }
        return Ok(texto);
    }
    Err(ultimo)
}

/// [299A-1 E11] La IA del turno de `WhatsApp` es texto puro (el transporte de
/// `glory-agent` arma `input` Responses solo con strings, sin `input_image`):
/// para que "vea" la foto entrante se describe aquí con visión real
/// (`completar_opencode`, probado con `input_image`) y el llamador anexa el
/// texto al mensaje `[foto]` ANTES del turno. Best-effort: cualquier `Err` y
/// el llamador conserva el `[foto]` pelado + el prompt pide que la describan.
pub(crate) async fn describir_foto(data_url: &str, pie: &str) -> Result<String, String> {
    let pie = pie.trim();
    let texto = if pie.is_empty() || pie == "(foto sin pie)" {
        "Describe lo que se ve en esta foto en 2-3 frases: qué tipo de ambiente \
         o lugar es, su estado y los detalles visibles relevantes para alguien \
         que busca inmueble."
            .to_string()
    } else {
        format!(
            "El visitante envió esta foto con el pie: \"{pie}\". Describe lo que \
             se ve en 2-3 frases (ambiente, estado, detalles visibles para \
             alguien que busca inmueble)."
        )
    };
    let (texto, _) = completar_opencode(
        "Eres el ojo del Asistente de IA de MN Inmobiliaria. Respondes solo con \
         la descripción de la foto, texto plano, sin adornos.",
        &texto,
        &[data_url.to_string()],
        "fotos",
    )
    .await?;
    Ok(texto.chars().take(1200).collect())
}

/* [09AA-4] Puras, sin DB: el diagnóstico solo describe la forma. */
#[cfg(test)]
mod pruebas {
    use super::{cuerpo_borrador_rapido, diagnostico_respuesta_vacia};

    /* [09AA-15] La vía rápida pide esfuerzo bajo con tope chico, sin
     * cambiar modelo ni entrada. */
    #[test]
    fn cuerpo_rapido_pide_esfuerzo_bajo() {
        let entrada = vec![serde_json::json!({"role": "user", "content": "hola"})];
        let cuerpo = cuerpo_borrador_rapido("modelo-x", &entrada);
        assert_eq!(cuerpo["model"], serde_json::json!("modelo-x"));
        assert_eq!(cuerpo["reasoning"], serde_json::json!({"effort": "low"}));
        assert_eq!(cuerpo["max_output_tokens"], serde_json::json!(2500));
        assert_eq!(cuerpo["input"], serde_json::json!(entrada));
    }

    #[test]
    fn diagnostico_describe_respuesta_vacia_sin_pii() {
        let vacia = serde_json::json!({
            "status": "completed",
            "output": [{"type": "reasoning"}],
            "usage": {"input_tokens": 12, "output_tokens": 118},
        });
        assert_eq!(
            diagnostico_respuesta_vacia(&vacia),
            "estado=completed tipos=[reasoning] motivo=- uso=in=12 out=118"
        );
        let incompleta = serde_json::json!({
            "status": "incomplete",
            "output": [{"type": "reasoning"}],
            "incomplete_details": {"reason": "max_output_tokens"},
            "usage": {"input_tokens": 2000, "output_tokens": 8000},
        });
        assert_eq!(
            diagnostico_respuesta_vacia(&incompleta),
            "estado=incomplete tipos=[reasoning] motivo=max_output_tokens uso=in=2000 out=8000"
        );
        let rota: serde_json::Value = serde_json::json!({"output": "no-es-arreglo"});
        assert_eq!(
            diagnostico_respuesta_vacia(&rota),
            "estado=? tipos=[] motivo=- uso=-"
        );
    }
}
