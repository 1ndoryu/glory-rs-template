use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::repositories::ClienteRepository;
use crate::services::InmuebleService;
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
async fn numeros_configurados(pool: &sqlx::PgPool) -> (String, String) {
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
#[must_use]
pub fn reparto(
    numero_a: &str,
    numero_b: &str,
    numero_destino: &str,
) -> Option<(&'static str, &'static str)> {
    let destino = ClienteRepository::normalizar_telefono(numero_destino);
    if destino == ClienteRepository::normalizar_telefono(numero_a) {
        Some(("wa_a", "completo"))
    } else if destino == ClienteRepository::normalizar_telefono(numero_b) {
        Some(("wa_b", "inicial"))
    } else {
        None
    }
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

/// Storage decidido 2026-09-28: disco local `UPLOAD_DIR/whatsapp/<tel>/`
/// (en prod el mismo volumen bind que las fotos de inmueble, sin infra
/// nueva; se sirven por `/uploads/whatsapp/...`). Descarga la `media_url`
/// que deja el gateway, valida el tipo real por Content-Type (nunca por la
/// URL) y guarda con `guardar_archivo` (tope de tamaño + magic-bytes).
/// Si algo falla se conserva la URL remota: media a mano antes que media
/// perdida. Devuelve el cuerpo del mensaje (`[foto]` o `[audio]`).
/// [299A-1 E12] Audios: `audio/ogg` (notas de voz), `audio/mpeg`, `audio/mp4`.
async fn cuerpo_media(
    http: &reqwest::Client,
    upload_dir: &std::path::Path,
    telefono_norm: &str,
    url: &str,
) -> String {
    match descargar_y_guardar(http, upload_dir, telefono_norm, url).await {
        Ok((clase, clave)) => format!("[{clase}] /uploads/{clave}"),
        Err(e) => {
            tracing::warn!("webhook WhatsApp: no se pudo archivar {url}: {e}; se conserva remota");
            format!("[media] {url}")
        }
    }
}

async fn descargar_y_guardar(
    http: &reqwest::Client,
    upload_dir: &std::path::Path,
    telefono_norm: &str,
    url: &str,
) -> Result<(String, String), String> {
    let resp = http.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("http {}", resp.status()));
    }
    let (clase, extension) = match resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
    {
        "image/jpeg" => ("foto", ".jpg"),
        "image/png" => ("foto", ".png"),
        "image/webp" => ("foto", ".webp"),
        "audio/ogg" => ("audio", ".ogg"),
        "audio/mpeg" => ("audio", ".mp3"),
        "audio/mp4" => ("audio", ".m4a"),
        _ => return Err("content-type no soportado".to_string()),
    };
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let clave = InmuebleService::guardar_archivo(
        upload_dir,
        &format!("whatsapp/{telefono_norm}"),
        &format!("{clase}{extension}"),
        &bytes,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok((clase.to_string(), clave))
}

#[derive(Debug, Deserialize)]
pub struct EntradaWhatsapp {
    numero_destino: String,
    remitente: String,
    texto: String,
    nombre: Option<String>,
    media_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RepartoWhatsapp {
    ok: bool,
    canal: String,
    modo: String,
    session_id: Uuid,
    cliente_id: Uuid,
    mensaje_id: Uuid,
    /* [289A-1] Secuencia del `client` recién persistido: el turno IA la
     * excluye del historial y la re-anexa como actual. */
    secuencia: i64,
}

/// Persiste la media entrante (`[foto]`/`[audio]`) y la emite por el hub.
/// Best-effort con aviso: el mensaje de texto ya quedó guardado; la media no
/// debe tumbarlo.
async fn persistir_media(
    pool: &sqlx::PgPool,
    hub: &glory_agent::session::ChatHub,
    sesion: Uuid,
    cuerpo: String,
) {
    /* Secuencia asignada por `insert_message_seq` (reseed desde BD +
     * retry 23505): el hub en memoria vuelve a 1 en cada reinicio y sin esto
     * el primer mensaje post-reinicio a una sesión vieja colisiona. */
    match glory_agent::persistence::insert_message_seq(
        pool, hub, sesion, "client", &cuerpo, None, None,
    )
    .await
    {
        Ok(msg) => {
            let _ = hub.broadcast(sesion, &glory_agent::models::WsServerMessage::live(msg));
        }
        Err(e) => {
            tracing::warn!("webhook WhatsApp: no se pudo persistir media: {e}");
        }
    }
}

/// Entrada única del webhook (lógica testeable): reparte, registra el cliente
/// con origen del canal, reutiliza su hilo o crea uno, persiste el mensaje
/// como `client` (+ segundo mensaje `[foto]`/`[audio]` si trae `media_url`) y emite por
/// el hub para que el panel staff lo vea en realtime.
pub async fn repartir_y_vincular(
    pool: &sqlx::PgPool,
    hub: &glory_agent::session::ChatHub,
    numero_a: &str,
    numero_b: &str,
    entrada: &EntradaWhatsapp,
) -> Result<RepartoWhatsapp, AgentError> {
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
    let sesion = if let Some(previa) =
        ClienteRepository::buscar_sesion_por_cliente_canal(pool, cliente.id, canal)
            .await
            .map_err(|e| AgentError::Db(e.to_string()))?
    {
        ClienteRepository::vincular_canal(pool, previa, cliente.id, &remitente_norm, canal, modo)
            .await
            .map_err(|e| AgentError::Db(e.to_string()))?;
        previa
    } else {
        let nueva = Uuid::new_v4();
        glory_agent::persistence::ensure_session(pool, nueva).await?;
        ClienteRepository::vincular_canal(pool, nueva, cliente.id, &remitente_norm, canal, modo)
            .await
            .map_err(|e| AgentError::Db(e.to_string()))?;
        nueva
    };
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
    if let Some(u) = media {
        let dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .unwrap_or_default();
        let cuerpo = cuerpo_media(&http, std::path::Path::new(&dir), &remitente_norm, u).await;
        persistir_media(pool, hub, sesion, cuerpo).await;
    }
    Ok(RepartoWhatsapp {
        ok: true,
        canal: canal.to_string(),
        modo: modo.to_string(),
        session_id: sesion,
        cliente_id: cliente.id,
        mensaje_id: msg.id,
        secuencia: msg.sequence_num,
    })
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
    let rep = repartir_y_vincular(&pool, &state.hub, &a, &b, &entrada).await?;
    /* [289A-1] Turno IA en background: el webhook responde 2xx rápido al
     * gateway; la IA (núcleo `responder_turno_persistido`, misma vía que el
     * chat web) corre aparte y su texto se encola en outbox `whatsapp` con
     * el `via` del canal, que el worker manda por la sesión Baileys que
     * recibió. `Ok(None)` (LLM sin texto tras tools, sin key, gate humano)
     * NO es silencio: escala la sesión a `consultando` para que el staff la
     * tome. `Err` se loguea: nunca 500 al gateway por fallos del LLM. */
    let fondo = state.clone();
    let pool_fondo = pool.clone();
    let texto_fondo = entrada.texto.trim().to_string();
    let remitente_fondo = ClienteRepository::normalizar_telefono(entrada.remitente.trim());
    let canal_fondo = rep.canal.clone();
    let sesion_fondo = rep.session_id;
    let secuencia_fondo = rep.secuencia;
    if !fondo.timing.check_budget(&remitente_fondo) {
        tracing::warn!("webhook WhatsApp: {sesion_fondo} sobre presupuesto, sin turno IA");
        return Ok(Json(rep));
    }
    tokio::spawn(async move {
        match glory_agent::transport::responder_turno_persistido(
            &fondo,
            sesion_fondo,
            &texto_fondo,
            secuencia_fondo,
        )
        .await
        {
            Ok(Some(respuesta)) => {
                /* [289A-4] Log de cierre de turno: sin esto un turno que
                 * termina con texto parcial (preámbulo sin listado tras
                 * tools) es indistinguible de un turno sano. */
                tracing::info!(
                    "webhook WhatsApp: {sesion_fondo} turno IA ok ({} chars), encolando via {canal_fondo}",
                    respuesta.chars().count()
                );
                let payload = serde_json::json!({
                    "session_id": sesion_fondo.to_string(),
                    "destino": remitente_fondo,
                    "texto": respuesta,
                    "via": canal_fondo,
                    "motivo": "ia",
                });
                if let Err(e) =
                    glory_agent::persistence::enqueue_outbox(&pool_fondo, "whatsapp", payload).await
                {
                    tracing::error!(
                        "webhook WhatsApp: {sesion_fondo} no se pudo encolar respuesta IA: {e}"
                    );
                }
            }
            Ok(None) => {
                /* IA sin texto (quemó tools sin redactar, sin key, gate
                 * humano): escalar a `consultando` en vez de soltar el
                 * mensaje al vacío — el staff lo ve y responde.
                 * [299A-1 E16] Pero un hipo transitorio (respuesta vacía
                 * aislada) no puede congelar una conversación sana: si el
                 * ciclo es `answered` (el staff ya respondió y la IA venía
                 * conversando, caso 739bb63e) se mantiene `activa` y solo
                 * se deja WARN en el log. */
                let ciclo = glory_agent::persistence::get_response_cycle(&pool_fondo, sesion_fondo)
                    .await
                    .ok()
                    .flatten();
                /* [299A-1 E16] Hipo transitorio con ciclo `answered`: se
                 * mantiene `activa` y solo se deja WARN en el log. */
                if !debe_escalar_consultando(ciclo.as_ref().map(|c| c.status.as_str())) {
                    tracing::warn!(
                        "webhook WhatsApp: {sesion_fondo} sin respuesta IA pero ciclo answered, se mantiene activa"
                    );
                    return;
                }
                if let Err(e) =
                    ClienteRepository::marcar_atencion(&pool_fondo, sesion_fondo, "consultando")
                        .await
                {
                    tracing::error!(
                        "webhook WhatsApp: {sesion_fondo} sin respuesta IA y no se pudo escalar: {e}"
                    );
                } else {
                    tracing::warn!(
                        "webhook WhatsApp: {sesion_fondo} sin respuesta IA, escalada a consultando"
                    );
                }
            }
            Err(e) => {
                tracing::warn!("webhook WhatsApp: {sesion_fondo} turno IA falló: {e}");
            }
        }
    });
    Ok(Json(rep))
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

    fn pool_si_hay() -> Option<sqlx::PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
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
        };
        let r1 = repartir_y_vincular(&pool, &hub, numero_a, numero_b, &entrada_a)
            .await
            .unwrap();
        assert_eq!((r1.canal.as_str(), r1.modo.as_str()), ("wa_a", "completo"));
        let r2 = repartir_y_vincular(&pool, &hub, numero_a, numero_b, &entrada_a)
            .await
            .unwrap();
        assert_eq!(r1.session_id, r2.session_id);
        let entrada_b = EntradaWhatsapp {
            numero_destino: numero_b.to_string(),
            remitente: remitente.clone(),
            texto: "hola B".to_string(),
            nombre: None,
            media_url: Some("https://example.com/foto.jpg".to_string()),
        };
        let r3 = repartir_y_vincular(&pool, &hub, numero_a, numero_b, &entrada_b)
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
}
