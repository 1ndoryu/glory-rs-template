/* [06AA-3 F3] Capa `Sesion` (plan 03AA-4 §Arquitectura): vincular el evento
 * ya normalizado a cliente×canal (registro con origen, resolución del hilo,
 * persistencia del `client` + archivado de media entrante). Lo que era el
 * centro de `handlers/whatsapp.rs` (`repartir_y_vincular` + media-ingesta)
 * vive aquí; el webhook solo orquesta. Sin cambios de conducta: verbatim. */

use serde::Serialize;
use uuid::Uuid;

use crate::handlers::chat_tools::telefono_valido;
use crate::repositories::ClienteRepository;
use crate::services::transporte::{destino_jubilado, reparto, EntradaWhatsapp};
use crate::services::{CanalResolver, InmuebleService};
use glory_agent::channels::Resolver;
use glory_agent::errors::AgentError;

/// Foto local pendiente de descripción visual (E11): el turno IA es texto
/// puro, así que la foto se describe con visión real y el texto se anexa al
/// mensaje ANTES del turno. `clave` es relativa a `UPLOAD_DIR`
/// (`whatsapp/<tel>/<archivo>`), `mime` el Content-Type validado.
/// `pub` porque la devuelve `repartir_y_vincular` (también `pub`).
pub struct FotoPendiente {
    pub(crate) mensaje_id: Uuid,
    pub(crate) cuerpo_base: String,
    pub(crate) clave: String,
    pub(crate) mime: String,
    pub(crate) pie: String,
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
    pub(crate) mensaje_id: Uuid,
    pub(crate) cuerpo_base: String,
    pub(crate) clave: String,
    pub(crate) mime: String,
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

#[derive(Debug, Serialize)]
pub struct RepartoWhatsapp {
    pub(crate) ok: bool,
    pub(crate) canal: String,
    pub(crate) modo: String,
    pub(crate) session_id: Uuid,
    pub(crate) cliente_id: Uuid,
    pub(crate) mensaje_id: Uuid,
    /* [06AA-1] Decisión del triage (`atiende:cliente`, `no:delegada`, ...):
     * la fija el webhook tras `repartir_y_vincular`; el harness vivo la
     * verifica sin leer logs. */
    pub(crate) decision: String,
    /* [06AA-2] Rol (`publico`/`autorizado`) y trato (`cliente`/`neutral`)
     * de F2 `Politica`: los fija el webhook junto a la decisión. */
    pub(crate) rol: String,
    pub(crate) trato: String,
    /* [289A-1] Secuencia del `client` recién persistido: el turno IA la
     * excluye del historial y la re-anexa como actual. */
    pub(crate) secuencia: i64,
}

impl RepartoWhatsapp {
    /* [07AA-2 F5] Recibo sintético del destino jubilado: `ok:false` (el
     * webhook lo devuelve 2xx sin triage ni turno), sin nada persistido.
     * Los ids `nil` marcan "sin sesión/cliente/mensaje": nunca llegan al
     * panel porque nada se guarda; solo viajan en el 2xx para el gateway. */
    pub(crate) fn jubilado() -> Self {
        Self {
            ok: false,
            canal: "wa_b".to_string(),
            modo: "jubilado".to_string(),
            session_id: Uuid::nil(),
            cliente_id: Uuid::nil(),
            mensaje_id: Uuid::nil(),
            decision: "no:canal-jubilado".to_string(),
            rol: "pendiente".to_string(),
            trato: "pendiente".to_string(),
            secuencia: 0,
        }
    }
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
    if !telefono_valido(remitente_txt) {
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
    let Some((canal, modo)) = reparto(numero_a, destino_txt) else {
        /* [07AA-2 F5] B jubilado: 2xx con motivo, sin persistir ni turno
         * (el gateway reintentaría ante un 400). Lo desconocido sigue 400. */
        if destino_jubilado(numero_b, destino_txt) {
            return Ok((
                RepartoWhatsapp::jubilado(),
                MediosPendientes {
                    foto: None,
                    audio: None,
                },
            ));
        }
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
