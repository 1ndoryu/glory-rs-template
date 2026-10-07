use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::repositories::ClienteRepository;
use crate::services::InmuebleService;
use crate::services::{
    clave_idempotencia, corte_cubre, debe_usar_clave, encolar_outbox_idem, modo_por_canal,
    politica, triage, CanalResolver, Encolado,
};
use glory_agent::channels::Resolver;
use glory_agent::errors::AgentError;

/* [279A-2 F2] Webhook simulado + reparto por número (sin Baileys/QR todavía).
 * Baileys tendrá 2 sesiones (una por número) y llamará aquí con
 * `numero_destino` (el número MN que recibió) + `remitente` (el cliente) +
 * `texto` (+ `media_url` opcional para fotos solo-enviar/recibir).
 * El reparto es por destino: A→`wa_a`/`completo`, B→`wa_b`/`inicial`.
 * Un hilo por cliente×canal (web y WhatsApp no mezclan hilos, se enlazan por
 * `clientes`); el mensaje entra como `client` (mismo `sender` del núcleo) y
 * el trigger `registrar_uso_estimado()` lo cuenta en `uso_mensajes`.
 * La IA responde por el transporte normal cuando haya gateway; hoy el
 * simulado persiste + vincula + devuelve el reparto (verificable por HTTP).
 * Gotcha: el `modo` de `atencion_sesiones` sigue al canal sin tocar el
 * `estado` (una ráfaga no reabre un hilo delegado: ver `vincular_canal`). */

/* [E-fluido] Conversación por partes (decisión usuaria 2026-09-29): la IA
 * habla como persona en WhatsApp (acuse breve → piezas → cierre), no en un
 * solo bloque tras 30-60s de silencio.
 * - F1 Acuse diferido: si el turno supera `ESPERA_ACUSE` se encola un acuse
 *   breve (sin hook del núcleo: el loop F6 descarta el texto intermedio y el
 *   núcleo es agnóstico; el acuse es comportamiento del producto WhatsApp).
 *   Si el turno termina antes, no se envía nada (cero ruido en turnos rápidos).
 * - El texto final se parte por líneas en blanco (máx `MAX_PARTES`): el modelo
 *   separa intro y cierre con línea en blanco (ver prompt en `chat.rs`).
 * - F4 Fallback: un turno fallido tras acuse no puede ser silencio total. */
/// Espera antes del acuse: la mayoría de turnos con tools cierra en 5-15 s;
/// a los 6 s sin respuesta vale un "ya voy"; antes es ruido.
/* [Fase3-H6] Además el acuse se omite si el turno ya encoló texto IA: en F1
 * salía DESPUÉS de las tarjetas y se leía como cierre ("dame un momentico"
 * tras la oferta). Ver `hay_avance_turno`. */
const ESPERA_ACUSE_MS: u64 = 6_000;
/// Máx de partes de texto por turno (intro + cierre; el resto se funde).
const MAX_PARTES: usize = 3;
const ACUSE_TEXTO: &str = "Ya lo estoy revisando, dame un momentico 👀";
const FALLBACK_TEXTO: &str = "Se me complicó con eso, ¿me lo repites en un momentico? 🙏";
const AVISO_ASESOR_TEXTO: &str = "Dame un momentico que ya te atiende un asesor 🙏";

/// Parte el texto final en mensajes breves (por líneas en blanco, máx
/// `MAX_PARTES`; el sobrante se funde en la última parte). Pura para testear.
/* [011A-5 Fase2] `pub(crate)`: la sombra compara este partido contra el
 * del núcleo (`partir_respuesta` de `channels::adapters`). */
pub(crate) fn partir_respuesta(texto: &str) -> Vec<String> {
    let partes: Vec<String> = texto
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    if partes.len() <= MAX_PARTES {
        return partes;
    }
    let mut cortadas = partes[..MAX_PARTES - 1].to_vec();
    cortadas.push(partes[MAX_PARTES - 1..].join(" "));
    cortadas
}

/// Encola un texto IA en outbox `whatsapp` (misma forma que el turno normal).
/* [011A-5 Fase1] Encolado idempotente: bajo corte (`corte_whatsapp` cubre
 * `via`) la fila lleva `idempotency_key = sha256(sesion:motivo:texto)`;
 * un duplicado en vuelo retorna `None` y se registra (no es error).
 * `manual` nunca lleva clave (lo excluye `debe_usar_clave`). */
async fn encolar_texto_ia(
    pool: &sqlx::PgPool,
    sesion: Uuid,
    destino: &str,
    via: &str,
    motivo: &str,
    texto: &str,
) {
    let payload = serde_json::json!({
        "session_id": sesion.to_string(),
        "destino": destino,
        "texto": texto,
        "via": via,
        "motivo": motivo,
    });
    let clave;
    let clave_ref = if debe_usar_clave(motivo) && corte_cubre(pool, via).await {
        clave = clave_idempotencia(&sesion.to_string(), motivo, texto);
        Some(clave.as_str())
    } else {
        None
    };
    match encolar_outbox_idem(pool, "whatsapp", payload, clave_ref).await {
        Ok(Encolado::Duplicado) => tracing::info!(
            "webhook WhatsApp: {sesion} duplicado {motivo} tragado por idempotency_key"
        ),
        /* [011A-5 Fase3] Revivido = el gemelo estaba `failed` (el envío
         * anterior nunca llegó) y vuelve a `pending`: se enviará. */
        Ok(Encolado::Revivido(id)) => {
            tracing::info!("webhook WhatsApp: {sesion} {motivo} revivido de failed ({id})");
        }
        Err(e) => tracing::error!("webhook WhatsApp: {sesion} no se pudo encolar {motivo}: {e}"),
        Ok(Encolado::Nuevo(_)) => {}
    }
}

/// Números MN por defecto (E.164 sin `+`; se normalizan igual que el resto).
/// El B es el vivo del negocio: no usar hasta el final (plan §Estado).
fn numero_a_defecto() -> String {
    std::env::var("WA_NUMERO_A").unwrap_or_else(|_| "584120825234".to_string())
}

fn numero_b_defecto() -> String {
    std::env::var("WA_NUMERO_B").unwrap_or_else(|_| "584249208855".to_string())
}

/// Números configurados: `agent_config` (`wa_numero_a`/`wa_numero_b`, editables
/// en consola F5) con fallback a env (`WA_NUMERO_A`/`WA_NUMERO_B`).
pub(crate) async fn numeros_configurados(pool: &sqlx::PgPool) -> (String, String) {
    let a = glory_agent::persistence::get_config(pool, "wa_numero_a")
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(numero_a_defecto);
    let b = glory_agent::persistence::get_config(pool, "wa_numero_b")
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(numero_b_defecto);
    (
        ClienteRepository::normalizar_telefono(&a),
        ClienteRepository::normalizar_telefono(&b),
    )
}

/// Reparto puro por destino (testeable sin BD): `wa_a`/`completo`, `wa_b`/`inicial`.
/// El `modo` sale de `modo_por_canal` (fuente única, compartida con el resolutor).
#[must_use]
pub fn reparto(
    numero_a: &str,
    numero_b: &str,
    numero_destino: &str,
) -> Option<(&'static str, &'static str)> {
    let destino = ClienteRepository::normalizar_telefono(numero_destino);
    let canal = if destino == ClienteRepository::normalizar_telefono(numero_a) {
        "wa_a"
    } else if destino == ClienteRepository::normalizar_telefono(numero_b) {
        "wa_b"
    } else {
        return None;
    };
    Some((canal, modo_por_canal(canal)?))
}

/// Secreto compartido con el gateway Baileys (llega con F2 real): si
/// `WA_WEBHOOK_SECRETO` está definido, el webhook exige la cabecera
/// `X-Gateway-Secret` idéntica (401 si falta o difiere). Sin definir
/// (simulado/dev local) acepta todo: ni el simulado ni los tests mandan
/// cabecera. Comparación exacta, sin normalizar (un secreto con espacios
/// es válido y se respeta tal cual).
#[must_use]
pub fn secreto_valido(esperado: Option<&str>, recibido: Option<&str>) -> bool {
    let Some(sec) = esperado.filter(|s| !s.is_empty()) else {
        return true;
    };
    recibido.is_some_and(|r| r == sec)
}

/// Foto local pendiente de descripción visual (E11): el turno IA es texto
/// puro, así que la foto se describe con visión real y el texto se anexa al
/// mensaje ANTES del turno. `clave` es relativa a `UPLOAD_DIR`
/// (`whatsapp/<tel>/<archivo>`), `mime` el Content-Type validado.
/// `pub` porque la devuelve `repartir_y_vincular` (también `pub`).
pub struct FotoPendiente {
    mensaje_id: Uuid,
    cuerpo_base: String,
    clave: String,
    mime: String,
    pie: String,
}

/// Storage decidido 2026-09-28: disco local `UPLOAD_DIR/whatsapp/<tel>/`
/// (en prod el mismo volumen bind que las fotos de inmueble, sin infra
/// nueva; se sirven por `/uploads/whatsapp/...`). Descarga la `media_url`
/// que deja el gateway, valida el tipo real por Content-Type (nunca por la
/// URL) y guarda con `guardar_archivo` (tope de tamaño + magic-bytes).
/// Si algo falla se conserva la URL remota: media a mano antes que media
/// perdida. Devuelve el cuerpo del mensaje (`[foto]` o `[audio]`) y los
/// pendientes de enriquecimiento: foto a describir (E11) y/o audio a
/// transcribir ([309A-4], `None` si no aplica).
/// [299A-1 E12] Audios: `audio/ogg` (notas de voz), `audio/mpeg`, `audio/mp4`.
async fn cuerpo_media(
    http: &reqwest::Client,
    upload_dir: &std::path::Path,
    telefono_norm: &str,
    url: &str,
    pie: &str,
) -> (
    String,
    Option<FotoPendienteSinId>,
    Option<AudioPendienteSinId>,
) {
    match descargar_y_guardar(http, upload_dir, telefono_norm, url).await {
        Ok((kind, clave, mime)) => {
            let cuerpo = format!("[{kind}] /uploads/{clave}");
            let foto = if kind == "foto" {
                Some(FotoPendienteSinId {
                    cuerpo_base: cuerpo.clone(),
                    clave: clave.clone(),
                    mime: mime.clone(),
                    pie: pie.to_string(),
                })
            } else {
                None
            };
            /* [309A-4] El audio sí deja pendiente (antes `None`): el
             * `mime` ya validado decide el `multipart` del STT. */
            let audio = if kind == "audio" {
                Some(AudioPendienteSinId {
                    cuerpo_base: cuerpo.clone(),
                    clave,
                    mime,
                })
            } else {
                None
            };
            (cuerpo, foto, audio)
        }
        Err(e) => {
            tracing::warn!("webhook WhatsApp: no se pudo archivar {url}: {e}; se conserva remota");
            (format!("[media] {url}"), None, None)
        }
    }
}

/// Foto archivada aún sin `mensaje_id` (se conoce tras persistir).
struct FotoPendienteSinId {
    cuerpo_base: String,
    clave: String,
    mime: String,
    pie: String,
}

/// [309A-4] Audio archivado aún sin `mensaje_id`: se transcribe con Groq
/// Whisper y el texto se anexa al mensaje ANTES del turno (espejo del flujo
/// E11 de fotos). `mime` decide el `filename`/tipo del `multipart`.
struct AudioPendienteSinId {
    cuerpo_base: String,
    clave: String,
    mime: String,
}

/// [309A-4] Medios pendientes del turno: foto (E11) y/o audio (STT). Va como
/// segundo elemento de `repartir_y_vincular` para que el webhook los procese
/// en background antes del turno IA.
pub struct MediosPendientes {
    pub foto: Option<FotoPendiente>,
    pub audio: Option<AudioPendiente>,
}

/// [309A-4] Audio local pendiente de transcripción (ver `AudioPendienteSinId`).
/// `pub` porque viaja en `MediosPendientes` (también `pub`).
pub struct AudioPendiente {
    mensaje_id: Uuid,
    cuerpo_base: String,
    clave: String,
    mime: String,
}

async fn descargar_y_guardar(
    http: &reqwest::Client,
    upload_dir: &std::path::Path,
    telefono_norm: &str,
    url: &str,
) -> Result<(String, String, String), String> {
    let resp = http.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("http {}", resp.status()));
    }
    let tipo = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    let (kind, extension) = match tipo.as_str() {
        "image/jpeg" => ("foto", ".jpg"),
        "image/png" => ("foto", ".png"),
        "image/webp" => ("foto", ".webp"),
        "audio/ogg" => ("audio", ".ogg"),
        "audio/mpeg" => ("audio", ".mp3"),
        "audio/mp4" => ("audio", ".m4a"),
        _ => return Err("content-type no soportado".to_string()),
    };
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    /* [299A-3] El audio NO pasa por `guardar_archivo` (solo valida fotos y
     * toda nota de voz caía al fallback `[media]`): usa `guardar_audio`. */
    let carpeta = format!("whatsapp/{telefono_norm}");
    let nombre = format!("{kind}{extension}");
    let clave = if kind == "audio" {
        InmuebleService::guardar_audio(upload_dir, &carpeta, &nombre, &bytes).await
    } else {
        InmuebleService::guardar_archivo(upload_dir, &carpeta, &nombre, &bytes).await
    }
    .map_err(|e| e.to_string())?;
    Ok((kind.to_string(), clave, tipo))
}

#[derive(Debug, Deserialize)]
pub struct EntradaWhatsapp {
    numero_destino: String,
    remitente: String,
    texto: String,
    nombre: Option<String>,
    media_url: Option<String>,
    /* [06AA-1] Campos del transporte real (F-transporte): el simulado no los
     * manda (`None` = dato ausente = el triage atiende, regla de oro). */
    #[serde(default)]
    ts_ms: Option<i64>,
    #[serde(default)]
    from_me: Option<bool>,
    #[serde(default)]
    es_sistema: Option<bool>,
    #[serde(default)]
    client_seq: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RepartoWhatsapp {
    ok: bool,
    canal: String,
    modo: String,
    session_id: Uuid,
    cliente_id: Uuid,
    mensaje_id: Uuid,
    /* [06AA-1] Decisión del triage (`atiende:cliente`, `no:delegada`, ...):
     * la fija el webhook tras `repartir_y_vincular`; el harness vivo la
     * verifica sin leer logs. */
    decision: String,
    /* [06AA-2] Rol (`publico`/`autorizado`) y trato (`cliente`/`neutral`)
     * de F2 `Politica`: los fija el webhook junto a la decisión. */
    rol: String,
    trato: String,
    /* [289A-1] Secuencia del `client` recién persistido: el turno IA la
     * excluye del historial y la re-anexa como actual. */
    secuencia: i64,
}

/// Persiste la media entrante (`[foto]`/`[audio]`) y la emite por el hub.
/// Best-effort con aviso: el mensaje de texto ya quedó guardado; la media no
/// debe tumbarlo. Devuelve el id del mensaje para anexar la descripción (E11).
async fn persistir_media(
    pool: &sqlx::PgPool,
    hub: &glory_agent::session::ChatHub,
    sesion: Uuid,
    cuerpo: String,
) -> Option<Uuid> {
    /* Secuencia asignada por `insert_message_seq` (reseed desde BD +
     * retry 23505): el hub en memoria vuelve a 1 en cada reinicio y sin esto
     * el primer mensaje post-reinicio a una sesión vieja colisiona. */
    match glory_agent::persistence::insert_message_seq(
        pool, hub, sesion, "client", &cuerpo, None, None,
    )
    .await
    {
        Ok(msg) => {
            let id = msg.id;
            let _ = hub.broadcast(sesion, &glory_agent::models::WsServerMessage::live(msg));
            Some(id)
        }
        Err(e) => {
            tracing::warn!("webhook WhatsApp: no se pudo persistir media: {e}");
            None
        }
    }
}

/// [299A-1 E11] Describe la foto con visión real y anexa el texto al mensaje
/// ANTES del turno IA, para que el historial la "vea". Corre en el spawn del
/// webhook (no en el camino rápido del 2xx). Best-effort total: cualquier
/// fallo deja el `[foto]` pelado y solo queda WARN. Sin rebroadcast: el
/// `WsServerMessage` solo conoce `live/history` y re-emitir duplicaría la
/// burbuja en el panel; el staff la ve al recargar y la IA en el turno.
async fn describir_y_anexar(pool: &sqlx::PgPool, foto: FotoPendiente) {
    const TOPE_BYTES: u64 = 4_000_000;
    let dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    let ruta = std::path::Path::new(&dir).join(&foto.clave);
    let bytes = match tokio::fs::read(&ruta).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(
                "webhook WhatsApp: foto sin bytes para describir {}: {e}",
                foto.clave
            );
            return;
        }
    };
    if bytes.len() as u64 > TOPE_BYTES {
        tracing::warn!(
            "webhook WhatsApp: foto {} pesa {} bytes, se describe a mano",
            foto.clave,
            bytes.len()
        );
        return;
    }
    let data_url = format!(
        "data:{};base64,{}",
        foto.mime,
        base64::engine::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes)
    );
    let descripcion = match super::ia::describir_foto(&data_url, &foto.pie).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("webhook WhatsApp: no se pudo describir {}: {e}", foto.clave);
            return;
        }
    };
    let cuerpo = format!("{} — se ve: {descripcion}", foto.cuerpo_base);
    if let Err(e) = sqlx::query("UPDATE agent_messages SET body = $1 WHERE id = $2")
        .bind(&cuerpo)
        .bind(foto.mensaje_id)
        .execute(pool)
        .await
    {
        tracing::warn!("webhook WhatsApp: no se pudo anexar descripción: {e}");
    }
}

/// Arma el cuerpo con la transcripción anexada (pura para testear).
#[must_use]
fn cuerpo_audio_con_texto(cuerpo_base: &str, texto: &str) -> String {
    let dicho = texto.trim();
    if dicho.is_empty() {
        return cuerpo_base.trim_end().to_string();
    }
    format!("{} — dice: {dicho}", cuerpo_base.trim_end())
}

/// [309A-4] Transcribe la nota de voz con Groq Whisper y anexa el texto al
/// mensaje ANTES del turno IA, para que el historial la "oiga" (espejo de
/// `describir_y_anexar`). Corre en el spawn del webhook (no en el camino
/// rápido del 2xx). Best-effort total: cualquier fallo deja el `[audio]`
/// pelado y solo queda WARN. Sin rebroadcast (misma razón que E11).
async fn transcribir_y_anexar(pool: &sqlx::PgPool, audio: AudioPendiente) {
    /* Mismo tope que `guardar_audio` (10 MiB): Groq acepta 25 MB, pero lo
     * que ya se archivó en local es lo que hay. */
    const TOPE_BYTES: usize = 10 * 1024 * 1024;
    let dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    let ruta = std::path::Path::new(&dir).join(&audio.clave);
    let bytes = match tokio::fs::read(&ruta).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(
                "webhook WhatsApp: audio sin bytes para transcribir {}: {e}",
                audio.clave
            );
            return;
        }
    };
    if bytes.len() > TOPE_BYTES {
        tracing::warn!(
            "webhook WhatsApp: audio {} pesa {} bytes, se transcribe a mano",
            audio.clave,
            bytes.len()
        );
        return;
    }
    let nombre = ruta.file_name().and_then(|n| n.to_str()).unwrap_or("nota");
    let texto = match super::ia::transcribir_audio(&bytes, nombre, &audio.mime).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(
                "webhook WhatsApp: no se pudo transcribir {}: {e}",
                audio.clave
            );
            return;
        }
    };
    let cuerpo = cuerpo_audio_con_texto(&audio.cuerpo_base, &texto);
    if let Err(e) = sqlx::query("UPDATE agent_messages SET body = $1 WHERE id = $2")
        .bind(&cuerpo)
        .bind(audio.mensaje_id)
        .execute(pool)
        .await
    {
        tracing::warn!("webhook WhatsApp: no se pudo anexar transcripción: {e}");
    }
}

/// [299A-1 E11] Archiva la media entrante y devuelve los pendientes de
/// enriquecimiento (extraído de `repartir_y_vincular` por tope de líneas).
async fn archivar_media_entrante(
    pool: &sqlx::PgPool,
    hub: &glory_agent::session::ChatHub,
    sesion: Uuid,
    remitente_norm: &str,
    texto: &str,
    url: &str,
) -> MediosPendientes {
    let dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .unwrap_or_default();
    let (cuerpo, foto, audio) = cuerpo_media(
        &http,
        std::path::Path::new(&dir),
        remitente_norm,
        url,
        texto,
    )
    .await;
    let mensaje_id = persistir_media(pool, hub, sesion, cuerpo).await;
    let (foto, audio) = match mensaje_id {
        Some(id) => (
            foto.map(|f| FotoPendiente {
                mensaje_id: id,
                cuerpo_base: f.cuerpo_base,
                clave: f.clave,
                mime: f.mime,
                pie: f.pie,
            }),
            audio.map(|a| AudioPendiente {
                mensaje_id: id,
                cuerpo_base: a.cuerpo_base,
                clave: a.clave,
                mime: a.mime,
            }),
        ),
        None => (None, None),
    };
    MediosPendientes { foto, audio }
}
/// Entrada única del webhook (lógica testeable): reparte, registra el cliente
/// con origen del canal, reutiliza su hilo o crea uno, persiste el mensaje
/// como `client` (+ segundo mensaje `[foto]`/`[audio]` si trae `media_url`) y emite por
/// el hub para que el panel staff lo vea en realtime. Además del reparto
/// devuelve los medios pendientes de enriquecimiento (E11 foto,
/// [309A-4] audio): el webhook los procesa en background antes del turno IA.
pub async fn repartir_y_vincular(
    pool: &sqlx::PgPool,
    hub: &glory_agent::session::ChatHub,
    numero_a: &str,
    numero_b: &str,
    entrada: &EntradaWhatsapp,
) -> Result<(RepartoWhatsapp, MediosPendientes), AgentError> {
    let destino_txt = entrada.numero_destino.trim();
    let remitente_txt = entrada.remitente.trim();
    let texto = entrada.texto.trim();
    if destino_txt.is_empty() || destino_txt.len() > 32 {
        return Err(AgentError::BadRequest(
            "numero_destino requerido (1..32)".to_string(),
        ));
    }
    if !super::chat_tools::telefono_valido(remitente_txt) {
        return Err(AgentError::BadRequest("remitente invalido".to_string()));
    }
    if texto.is_empty() || texto.len() > 4000 {
        return Err(AgentError::BadRequest(
            "texto requerido (1..4000)".to_string(),
        ));
    }
    let media = entrada
        .media_url
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty());
    if let Some(u) = media {
        if u.len() > 2048 || !(u.starts_with("http://") || u.starts_with("https://")) {
            return Err(AgentError::BadRequest(
                "media_url debe ser http(s) (<=2048)".to_string(),
            ));
        }
    }
    let nombre = entrada
        .nombre
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty());
    if let Some(n) = nombre {
        if n.len() > 80 {
            return Err(AgentError::BadRequest("nombre <=80".to_string()));
        }
    }
    let Some((canal, modo)) = reparto(numero_a, numero_b, destino_txt) else {
        return Err(AgentError::BadRequest(
            "numero_destino desconocido".to_string(),
        ));
    };
    let remitente_norm = ClienteRepository::normalizar_telefono(remitente_txt);
    if remitente_norm.chars().filter(char::is_ascii_digit).count() < 7 {
        return Err(AgentError::BadRequest("remitente invalido".to_string()));
    }
    let cliente = ClienteRepository::registrar_con_origen(pool, nombre, &remitente_norm, canal)
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    /* [05AA-1] La resolución va por el `Resolver` del núcleo (mismo
     * resultado: reutiliza el hilo o crea uno con `ensure_session` +
     * `vincular_canal`). Fallo de BD → 500 igual que antes (`Db` e
     * `Internal` responden 500 en el núcleo). */
    let resolutor = CanalResolver::new(pool.clone());
    let sesion = resolutor
        .sesion_por_canal(&cliente.id.to_string(), canal, &remitente_norm)
        .await?;
    /* La secuencia la asigna `insert_message_seq` (reseed desde BD +
     * retry 23505): nunca `hub.next_sequence` directo. */
    let msg = glory_agent::persistence::insert_message_seq(
        pool, hub, sesion, "client", texto, None, None,
    )
    .await?;
    let _ = hub.broadcast(
        sesion,
        &glory_agent::models::WsServerMessage::live(msg.clone()),
    );
    /* Media entrante (foto o nota de voz): se archiva en
     * `UPLOAD_DIR/whatsapp/<tel>/` y queda como mensaje `[foto]` o `[audio]`
     * con ruta local servible para el staff. Si el archivo no baja o el tipo
     * no es soportado, se conserva la URL remota antes que perderla. */
    let medios = if let Some(u) = media {
        archivar_media_entrante(pool, hub, sesion, &remitente_norm, texto, u).await
    } else {
        MediosPendientes {
            foto: None,
            audio: None,
        }
    };
    Ok((
        RepartoWhatsapp {
            ok: true,
            canal: canal.to_string(),
            modo: modo.to_string(),
            session_id: sesion,
            cliente_id: cliente.id,
            mensaje_id: msg.id,
            secuencia: msg.sequence_num,
            decision: "pendiente".to_string(),
            rol: "pendiente".to_string(),
            trato: "pendiente".to_string(),
        },
        medios,
    ))
}

/// [06AA-1] Registro de `client_seq` del proceso (duplicados del
/// transporte): TTL 180 s igual que el eco del gateway, tope 500.
static REGISTRO_DUPLICADOS: LazyLock<Mutex<triage::RegistroDuplicados>> = LazyLock::new(|| {
    Mutex::new(triage::RegistroDuplicados::nuevo(
        Duration::from_secs(180),
        500,
    ))
});

/// [06AA-1] Triage del webhook (extraída por el lint de 100 líneas):
/// resuelve el remitente normalizado y la decisión (`decidir_para`). El
/// mensaje ya quedó persistido por `repartir_y_vincular` (el staff lo ve en
/// el panel aunque no se atienda); aquí solo se decide si corre el turno.
/// `Mutex` del registro: sección crítica breve, sin `await` dentro.
async fn triar_entrada(
    pool: &sqlx::PgPool,
    numero_a: &str,
    numero_b: &str,
    entrada: &EntradaWhatsapp,
    sesion: Uuid,
) -> (String, triage::Decision) {
    let remitente = ClienteRepository::normalizar_telefono(entrada.remitente.trim());
    let texto = entrada.texto.trim();
    let tiene_media = entrada
        .media_url
        .as_deref()
        .is_some_and(|u| !u.trim().is_empty());
    let marca = if entrada.from_me.unwrap_or(false) {
        triage::MarcaTransporte::Eco
    } else if entrada.es_sistema.unwrap_or(false) {
        triage::MarcaTransporte::Sistema
    } else {
        triage::MarcaTransporte::Normal
    };
    let sin_resolver = triage::EntradaSinResolver {
        procedencia: if remitente == numero_a || remitente == numero_b {
            triage::Procedencia::Propio
        } else {
            triage::Procedencia::Externo
        },
        marca,
        contenido: if texto.is_empty() && !tiene_media {
            triage::Contenido::Vacio
        } else {
            triage::Contenido::Util
        },
        texto,
        ts_ms: entrada.ts_ms,
        client_seq: entrada.client_seq.as_deref(),
        sesion,
    };
    let decision = triage::decidir_para(pool, &sin_resolver, &REGISTRO_DUPLICADOS).await;
    (remitente, decision)
}

async fn webhook(
    State(state): State<glory_agent::transport::AgentState>,
    cabeceras: HeaderMap,
    Json(entrada): Json<EntradaWhatsapp>,
) -> Result<Json<RepartoWhatsapp>, AgentError> {
    let esperado = std::env::var("WA_WEBHOOK_SECRETO").ok();
    let recibido = cabeceras
        .get("x-gateway-secret")
        .and_then(|v| v.to_str().ok());
    if !secreto_valido(esperado.as_deref(), recibido) {
        return Err(AgentError::Unauthorized);
    }
    let pool = state
        .pool
        .clone()
        .ok_or_else(|| AgentError::Internal("sin BD".to_string()))?;
    let (a, b) = numeros_configurados(&pool).await;
    let (mut rep, medios) = repartir_y_vincular(&pool, &state.hub, &a, &b, &entrada).await?;
    /* [06AA-1] Triage F1: el mensaje ya persistió (auditoría/panel); si la
     * decisión es `no`, el 2xx lleva el motivo y no corre turno.
     * [06AA-2] Política F2: rol por allowlist + trato→tono. Se resuelve en
     * ambas ramas para que el 2xx siempre traiga `rol`/`trato`; el tono
     * neutral viaja como prefijo de contexto solo al modelo (lo persistido
     * y el panel quedan intactos). */
    let (remitente_norm, decision) = triar_entrada(&pool, &a, &b, &entrada, rep.session_id).await;
    rep.decision = decision.codigo().to_string();
    let autorizados = politica::leer_autorizados(&pool).await;
    let trato = match decision {
        triage::Decision::Atiende { trato } => trato,
        triage::Decision::NoAtiende { .. } => {
            triage::evaluar_trato(entrada.texto.trim()).trato
        }
    };
    let regla = politica::resolver(&autorizados, &remitente_norm, trato);
    rep.rol = regla.rol.codigo().to_string();
    rep.trato = politica::codigo_trato(regla.trato).to_string();
    match decision {
        triage::Decision::NoAtiende { motivo } => {
            tracing::warn!(
                "webhook WhatsApp: {} triage no atiende ({}) rol={} trato={}",
                rep.session_id,
                motivo.codigo(),
                regla.rol.codigo(),
                politica::codigo_trato(regla.trato)
            );
            return Ok(Json(rep));
        }
        triage::Decision::Atiende { .. } => {
            tracing::info!(
                "webhook WhatsApp: {} triage atiende (rol={} trato={})",
                rep.session_id,
                regla.rol.codigo(),
                politica::codigo_trato(regla.trato)
            );
        }
    }
    /* [299A-1 E11] La foto se describe en background ANTES del turno para que
     * el historial ya traiga el `— se ve:`.
     * [309A-4] El audio se transcribe igual (`— dice:`). El 2xx al gateway
     * no espera a ninguno. */
    /* [289A-1] Turno IA en background: el webhook responde 2xx rápido al
     * gateway; la IA (núcleo `responder_turno_persistido`, misma vía que el
     * chat web) corre aparte y su texto se encola en outbox `whatsapp` con
     * el `via` del canal, que el worker manda por la sesión Baileys que
     * recibió. `Ok(None)` (LLM sin texto tras tools, sin key, gate humano)
     * NO es silencio: escala la sesión a `consultando` para que el staff la
     * tome. `Err` se loguea: nunca 500 al gateway por fallos del LLM. */
    let fondo = state.clone();
    let pool_fondo = pool.clone();
    /* [06AA-2] El modelo ve el tono de la política; lo persistido no cambia. */
    let texto_fondo = politica::texto_para_turno(regla.trato, entrada.texto.trim());
    let remitente_fondo = remitente_norm;
    let canal_fondo = rep.canal.clone();
    let sesion_fondo = rep.session_id;
    let secuencia_fondo = rep.secuencia;
    if !fondo.timing.check_budget(&remitente_fondo) {
        tracing::warn!("webhook WhatsApp: {sesion_fondo} sobre presupuesto, sin turno IA");
        return Ok(Json(rep));
    }
    /* [E-fluido F1] Acuse diferido: si el turno sigue vivo tras
     * `ESPERA_ACUSE_MS` y la IA sigue al mando, se avisa que ya se está
     * revisando. Turno rápido = sin acuse. */
    let turno_vivo = Arc::new(AtomicBool::new(true));
    programar_acuse(
        pool_fondo.clone(),
        sesion_fondo,
        remitente_fondo.clone(),
        canal_fondo.clone(),
        turno_vivo.clone(),
    );
    tokio::spawn(async move {
        if let Some(f) = medios.foto {
            describir_y_anexar(&pool_fondo, f).await;
        }
        if let Some(a) = medios.audio {
            transcribir_y_anexar(&pool_fondo, a).await;
        }
        let resultado = glory_agent::transport::responder_turno_persistido(
            &fondo,
            sesion_fondo,
            &texto_fondo,
            secuencia_fondo,
        )
        .await;
        atender_resultado_turno(
            &pool_fondo,
            sesion_fondo,
            &remitente_fondo,
            &canal_fondo,
            resultado,
            &turno_vivo,
        )
        .await;
    });
    Ok(Json(rep))
}

/* [E-fluido F1] El acuse vive fuera de `webhook` (el lint no deja pasar la
 * función de 100 líneas): programa el aviso de turno lento en background.
 * [Fase3-v2] Triple guarda: turno vivo + IA al mando + SIN avance encolado
 * (`ia` o `tarjeta`). Si el turno ya mostró producto (tarjetas) el acuse
 * "dame un momentico" tras la oferta se leía como cierre raro (H6 v2). */
fn programar_acuse(
    pool: sqlx::PgPool,
    sesion: Uuid,
    destino: String,
    canal: String,
    vivo: Arc<AtomicBool>,
) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(ESPERA_ACUSE_MS)).await;
        if !vivo.load(Ordering::Relaxed) {
            return;
        }
        let sigue_ia = glory_agent::persistence::get_session(&pool, sesion)
            .await
            .ok()
            .flatten()
            .is_some_and(|s| s.ai_enabled);
        if !sigue_ia {
            return;
        }
        if hay_avance_turno(&pool, sesion).await {
            return;
        }
        tracing::info!("webhook WhatsApp: {sesion} turno lento, encolando acuse");
        encolar_texto_ia(&pool, sesion, &destino, &canal, "acuse", ACUSE_TEXTO).await;
    });
}

/// ¿El turno ya encoló texto IA o tarjetas en los últimos 2 min? Evita el
/// acuse tardío (tras las tarjetas confundía). `false` ante error de BD:
/// mejor un acuse de más que romper el turno por una consulta auxiliar.
async fn hay_avance_turno(pool: &sqlx::PgPool, session_id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM agent_outbox \
         WHERE payload->>'session_id' = $1 AND payload->>'motivo' IN ('ia', 'tarjeta') \
         AND created_at > NOW() - INTERVAL '2 minutes')",
    )
    .bind(session_id.to_string())
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

/// Atiende el resultado del turno IA (extraída de `webhook` por el lint):
/// parte el texto final en mensajes, escala con aviso al asesor o cae al
/// fallback. Nunca deja al visitante en silencio tras un acuse.
async fn atender_resultado_turno(
    pool: &sqlx::PgPool,
    sesion: Uuid,
    destino: &str,
    canal: &str,
    resultado: Result<Option<String>, AgentError>,
    vivo: &AtomicBool,
) {
    match resultado {
        Ok(Some(respuesta)) => {
            /* [289A-4] Log de cierre de turno: sin esto un turno que
             * termina con texto parcial (preámbulo sin listado tras
             * tools) es indistinguible de un turno sano.
             * [E-fluido] El texto final viaja por partes (intro + cierre
             * por separado): cada parte es un mensaje WhatsApp. */
            vivo.store(false, Ordering::Relaxed);
            let partes = partir_respuesta(&respuesta);
            tracing::info!(
                "webhook WhatsApp: {sesion} turno IA ok ({} chars, {} partes), encolando via {canal}",
                respuesta.chars().count(),
                partes.len()
            );
            for parte in &partes {
                encolar_texto_ia(pool, sesion, destino, canal, "ia", parte).await;
            }
        }
        Ok(None) => {
            vivo.store(false, Ordering::Relaxed);
            /* IA sin texto (quemó tools sin redactar, sin key, gate
             * humano): escalar a `consultando` en vez de soltar el
             * mensaje al vacío — el staff lo ve y responde.
             * [299A-1 E16] Pero un hipo transitorio (respuesta vacía
             * aislada) no puede congelar una conversación sana: si el
             * ciclo es `answered` (el staff ya respondió y la IA venía
             * conversando, caso 739bb63e) se mantiene `activa` y solo
             * se deja WARN en el log. */
            let ciclo = glory_agent::persistence::get_response_cycle(pool, sesion)
                .await
                .ok()
                .flatten();
            /* [299A-1 E16] Hipo transitorio con ciclo `answered`: se
             * mantiene `activa` y solo se deja WARN en el log. */
            if !debe_escalar_consultando(ciclo.as_ref().map(|c| c.status.as_str())) {
                tracing::warn!(
                    "webhook WhatsApp: {sesion} sin respuesta IA pero ciclo answered, se mantiene activa"
                );
                return;
            }
            /* [E-fluido F4] La escalada ya no es silencio para el
             * visitante: se le dice que viene un asesor. */
            encolar_texto_ia(pool, sesion, destino, canal, "asesor", AVISO_ASESOR_TEXTO).await;
            if let Err(e) = ClienteRepository::marcar_atencion(pool, sesion, "consultando").await {
                tracing::error!(
                    "webhook WhatsApp: {sesion} sin respuesta IA y no se pudo escalar: {e}"
                );
            } else {
                tracing::warn!(
                    "webhook WhatsApp: {sesion} sin respuesta IA, escalada a consultando"
                );
            }
        }
        Err(e) => {
            /* [E-fluido F4] Turno fallido (el acuse ya pudo salir): nunca
             * silencio total; la sesión sigue activa y la IA retoma en el
             * próximo mensaje. */
            vivo.store(false, Ordering::Relaxed);
            tracing::warn!("webhook WhatsApp: {sesion} turno IA falló: {e}");
            encolar_texto_ia(pool, sesion, destino, canal, "fallback", FALLBACK_TEXTO).await;
        }
    }
}

/// Ruta pública del gateway (simulado hoy, Baileys mañana): el secreto del
/// gateway llegará con F2 real; hoy la frontera es la validación estricta.
pub fn whatsapp_routes() -> Router<glory_agent::transport::AgentState> {
    Router::new().route("/agent/whatsapp/webhook", post(webhook))
}

/* [299A-1 E16] Decisión pura: un turno vacío escala a `consultando` salvo
 * ciclo `answered` (el staff ya respondió y la IA venía conversando: un hipo
 * del LLM no congela una conversación sana, caso 739bb63e). */
#[must_use]
fn debe_escalar_consultando(ciclo: Option<&str>) -> bool {
    ciclo != Some("answered")
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn reparto_lleva_cada_numero_a_su_modo() {
        assert_eq!(
            reparto("584120825234", "584249208855", "0412 0825234"),
            Some(("wa_a", "completo"))
        );
        assert_eq!(
            reparto("584120825234", "584249208855", "0424 9208855"),
            Some(("wa_b", "inicial"))
        );
        assert_eq!(
            reparto("584120825234", "584249208855", "+584120825234"),
            Some(("wa_a", "completo"))
        );
        assert_eq!(reparto("584120825234", "584249208855", "04120000000"), None);
    }

    /* [279A-2] Secreto del gateway: sin configurar acepta todo (simulado);
     * configurado exige coincidencia exacta (falla cerrado). */
    #[test]
    fn secreto_solo_exige_si_esta_configurado() {
        assert!(secreto_valido(None, None));
        assert!(secreto_valido(None, Some("x")));
        assert!(secreto_valido(Some(""), None));
        assert!(secreto_valido(Some("s3cr3to"), Some("s3cr3to")));
        assert!(!secreto_valido(Some("s3cr3to"), None));
        assert!(!secreto_valido(Some("s3cr3to"), Some("otro")));
        assert!(!secreto_valido(Some("s3cr3to"), Some(" s3cr3to")));
    }

    /* [299A-1 E16] Un turno vacío solo congela sin ciclo `answered`. */
    #[test]
    fn turno_vacio_no_congela_ciclo_respondido() {
        assert!(debe_escalar_consultando(None));
        assert!(debe_escalar_consultando(Some("waiting")));
        assert!(debe_escalar_consultando(Some("escalated")));
        assert!(debe_escalar_consultando(Some("open")));
        assert!(!debe_escalar_consultando(Some("answered")));
    }

    /* [E-fluido] El texto final viaja por partes (intro + cierre por separado);
     * el sobrante se funde en la última parte, nunca se pierde. */
    #[test]
    fn respuesta_se_parte_por_lineas_en_blanco() {
        assert_eq!(partir_respuesta("hola"), vec!["hola".to_string()]);
        assert_eq!(
            partir_respuesta("Mira lo que hay:\n\nTe mando fotos si quieres"),
            vec![
                "Mira lo que hay:".to_string(),
                "Te mando fotos si quieres".to_string()
            ]
        );
        assert!(partir_respuesta("  \n\n  ").is_empty());
        let cinco = partir_respuesta("a\n\nb\n\nc\n\nd\n\ne");
        assert_eq!(cinco.len(), MAX_PARTES);
        assert_eq!(cinco[..MAX_PARTES - 1], ["a".to_string(), "b".to_string()]);
        assert!(cinco[MAX_PARTES - 1].contains('c'));
        assert!(cinco[MAX_PARTES - 1].contains('e'));
    }

    /* [309A-4] La transcripción se anexa tras `— dice:` igual que la
     * descripción de foto tras `— se ve:`; el front la muestra como `Dice:`. */
    #[test]
    fn audio_anexa_transcripcion_tras_dice() {
        assert_eq!(
            cuerpo_audio_con_texto("[audio] /uploads/whatsapp/18149575613/a.mp3", "¡Hola!"),
            "[audio] /uploads/whatsapp/18149575613/a.mp3 — dice: ¡Hola!".to_string()
        );
        assert_eq!(
            cuerpo_audio_con_texto("[audio] /u/a.mp3", "  "),
            "[audio] /u/a.mp3".to_string()
        );
    }

    /* [309A-4] Verificación viva con el mp3 real: solo corre con
     * `GROQ_LIVE_TEST=1` (necesita `GROQ_API_KEY` + red sin bloqueo a Groq,
     * hoy VPN). Confirma el camino completo: leer disco → Whisper → UPDATE
     * con `— dice:` en el hilo. Sin el opt-in se omite como `pool_si_hay`. */
    #[tokio::test]
    async fn audio_vivo_transcribe_y_anexa_dice() {
        if std::env::var("GROQ_LIVE_TEST").is_err() {
            return;
        }
        let Some(pool) = pool_si_hay() else { return };
        let hub = glory_agent::session::ChatHub::new();
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let cuerpo =
            "[audio] /uploads/whatsapp/18149575613/4353d43d-b676-41f9-9df3-9c7f241c783f.mp3";
        let msg = glory_agent::persistence::insert_message_seq(
            &pool, &hub, sesion, "client", cuerpo, None, None,
        )
        .await
        .unwrap();
        transcribir_y_anexar(
            &pool,
            AudioPendiente {
                mensaje_id: msg.id,
                cuerpo_base: cuerpo.to_string(),
                clave: "whatsapp/18149575613/4353d43d-b676-41f9-9df3-9c7f241c783f.mp3".to_string(),
                mime: "audio/mpeg".to_string(),
            },
        )
        .await;
        let final_: String = sqlx::query_scalar("SELECT body FROM agent_messages WHERE id = $1")
            .bind(msg.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(
            final_.contains("— dice:"),
            "el hilo debe traer la transcripción, quedó: {final_}"
        );
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }

    fn pool_si_hay() -> Option<sqlx::PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    /* [E-fluido F4] Un turno fallido encola el fallback (nunca silencio tras
     * un acuse) y no congela la sesión. Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn turno_fallido_encola_fallback() {
        let Some(pool) = pool_si_hay() else { return };
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let vivo = AtomicBool::new(true);
        atender_resultado_turno(
            &pool,
            sesion,
            "34600000000",
            "wa_b",
            Err(glory_agent::errors::AgentError::Internal(
                "boom".to_string(),
            )),
            &vivo,
        )
        .await;
        assert!(!vivo.load(Ordering::Relaxed));
        let texto: String = sqlx::query_scalar(
            "SELECT payload->>'texto' FROM agent_outbox WHERE payload->>'session_id' = $1",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(texto, FALLBACK_TEXTO);
        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sesion.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [279A-2 F2] El mismo cliente×canal reutiliza hilo; otro canal abre otro
     * (web y WhatsApp no mezclan hilos). Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn webhook_reutiliza_hilo_por_cliente_canal() {
        let Some(pool) = pool_si_hay() else { return };
        let hub = glory_agent::session::ChatHub::new();
        let numero_a = "584120825234";
        let numero_b = "584249208855";
        let remitente = format!("34609{:05}", rand_num());
        let entrada_a = EntradaWhatsapp {
            numero_destino: numero_a.to_string(),
            remitente: remitente.clone(),
            texto: "hola A".to_string(),
            nombre: Some("Humo WA".to_string()),
            media_url: None,
            ts_ms: None,
            from_me: None,
            es_sistema: None,
            client_seq: None,
        };
        let (r1, _) = repartir_y_vincular(&pool, &hub, numero_a, numero_b, &entrada_a)
            .await
            .unwrap();
        assert_eq!((r1.canal.as_str(), r1.modo.as_str()), ("wa_a", "completo"));
        let (r2, _) = repartir_y_vincular(&pool, &hub, numero_a, numero_b, &entrada_a)
            .await
            .unwrap();
        assert_eq!(r1.session_id, r2.session_id);
        let entrada_b = EntradaWhatsapp {
            numero_destino: numero_b.to_string(),
            remitente: remitente.clone(),
            texto: "hola B".to_string(),
            nombre: None,
            media_url: Some("https://example.com/foto.jpg".to_string()),
            ts_ms: None,
            from_me: None,
            es_sistema: None,
            client_seq: None,
        };
        let (r3, _) = repartir_y_vincular(&pool, &hub, numero_a, numero_b, &entrada_b)
            .await
            .unwrap();
        assert_eq!((r3.canal.as_str(), r3.modo.as_str()), ("wa_b", "inicial"));
        assert_ne!(r1.session_id, r3.session_id);
        let modo_b: String =
            sqlx::query_scalar("SELECT modo FROM atencion_sesiones WHERE session_id = $1")
                .bind(r3.session_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(modo_b, "inicial");

        for sid in [r1.session_id, r3.session_id] {
            sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
                .bind(sid)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
                .bind(sid)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
                .bind(sid)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1")
                .bind(sid)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
                .bind(sid)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query("DELETE FROM clientes WHERE id = $1")
            .bind(r1.cliente_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    fn rand_num() -> u32 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(12345, |d| (d.subsec_nanos() % 90000) + 10000)
    }

    /* [011A-1] Foto F5-Paso0: constantes del flujo WhatsApp congeladas. Si
     * alguna cambia, el strangler debe enterarse (acuse 6s, partes, textos). */
    #[test]
    fn foto_constantes_flujo_whatsapp() {
        assert_eq!(ESPERA_ACUSE_MS, 6_000);
        assert_eq!(MAX_PARTES, 3);
        assert_eq!(ACUSE_TEXTO, "Ya lo estoy revisando, dame un momentico 👀");
        assert_eq!(
            FALLBACK_TEXTO,
            "Se me complicó con eso, ¿me lo repites en un momentico? 🙏"
        );
        assert_eq!(
            AVISO_ASESOR_TEXTO,
            "Dame un momentico que ya te atiende un asesor 🙏"
        );
    }

    /* [011A-1] Foto F5-Paso0: el acuse solo se salta si el turno ya encoló
     * texto IA (`ia`) o tarjetas en los últimos 2 min; el acuse previo u
     * otros motivos no cuentan como avance. Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn foto_acuse_solo_si_no_hay_avance() {
        let Some(pool) = pool_si_hay() else { return };
        let sid = Uuid::new_v4();
        assert!(!hay_avance_turno(&pool, sid).await);
        for motivo in ["acuse", "fallback", "manual"] {
            glory_agent::persistence::enqueue_outbox(
                &pool,
                "whatsapp",
                serde_json::json!({"session_id": sid.to_string(), "motivo": motivo}),
            )
            .await
            .unwrap();
            assert!(!hay_avance_turno(&pool, sid).await, "motivo {motivo}");
        }
        for motivo in ["ia", "tarjeta"] {
            glory_agent::persistence::enqueue_outbox(
                &pool,
                "whatsapp",
                serde_json::json!({"session_id": sid.to_string(), "motivo": motivo}),
            )
            .await
            .unwrap();
            assert!(hay_avance_turno(&pool, sid).await, "motivo {motivo}");
        }
        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sid.to_string())
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [011A-1] Foto F5-Paso0: `trg_uso_mensajes` copia el `usage` exacto del
     * núcleo a `uso_mensajes` y estima `GREATEST(1,(len+3)/4)` cuando no hay
     * exacto. La sombra comparará contra esto. Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn foto_trigger_uso_mide_exacto_y_estima() {
        let Some(pool) = pool_si_hay() else { return };
        let hub = glory_agent::session::ChatHub::new();
        let sid = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sid)
            .await
            .unwrap();
        let cuerpo = "Hola, busco apartamento"; // 23 chars → estima (23+3)/4 = 6
        let msg = glory_agent::persistence::insert_message_seq(
            &pool,
            &hub,
            sid,
            "ai",
            cuerpo,
            Some(11),
            Some(23),
        )
        .await
        .unwrap();
        let (est, tin, tout): (i32, Option<i32>, Option<i32>) = sqlx::query_as(
            "SELECT tokens_est, tokens_in, tokens_out FROM uso_mensajes WHERE message_id = $1",
        )
        .bind(msg.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((est, tin, tout), (6, Some(11), Some(23)));
        let msg2 = glory_agent::persistence::insert_message_seq(
            &pool, &hub, sid, "client", "ok", None, None,
        )
        .await
        .unwrap();
        let (est2, tin2, tout2): (i32, Option<i32>, Option<i32>) = sqlx::query_as(
            "SELECT tokens_est, tokens_in, tokens_out FROM uso_mensajes WHERE message_id = $1",
        )
        .bind(msg2.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((est2, tin2, tout2), (1, None, None));
        sqlx::query("DELETE FROM uso_mensajes WHERE session_id = $1")
            .bind(sid)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sid)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sid)
            .execute(&pool)
            .await
            .unwrap();
    }
}
