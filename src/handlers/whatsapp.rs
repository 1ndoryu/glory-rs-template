use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use uuid::Uuid;

use crate::repositories::ClienteRepository;
use crate::services::{
    politica,
    sesion::{repartir_y_vincular, RepartoWhatsapp},
    transporte::{numeros_configurados, secreto_valido, EntradaWhatsapp},
    triage,
    turno::{disparar_turno_fondo, TurnoFondo},
};
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
/* [06AA-3 F3] Capas partidas sin cambiar conducta (plan 03AA-4 F3): el
 * Transporte (`services/transporte.rs`: DTO, secreto, números, reparto), la
 * Sesión (`services/sesion.rs`: vincular + media-ingesta) y el Turno
 * (`services/turno.rs`: fondo + acuse + E-fluido) viven en `services/`; aquí
 * queda la orquesta (webhook + triage + ruta + pruebas). */

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
        triage::Decision::NoAtiende { .. } => triage::evaluar_trato(entrada.texto.trim()).trato,
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
    /* [06AA-3 F3] El turno corre en su capa: el webhook entrega el texto
     * crudo + trato y responde el 2xx sin esperar al modelo. */
    disparar_turno_fondo(TurnoFondo {
        estado: state.clone(),
        pool,
        texto_crudo: entrada.texto.clone(),
        trato: regla.trato,
        remitente: remitente_norm,
        canal: rep.canal.clone(),
        sesion: rep.session_id,
        secuencia: rep.secuencia,
        medios,
    });
    Ok(Json(rep))
}

/// Ruta pública del gateway (simulado hoy, Baileys mañana): el secreto del
/// gateway llegará con F2 real; hoy la frontera es la validación estricta.
pub fn whatsapp_routes() -> Router<glory_agent::transport::AgentState> {
    Router::new().route("/agent/whatsapp/webhook", post(webhook))
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use uuid::Uuid;

    /* [06AA-3 F3] Las pruebas se quedaron en la orquesta: importan lo que se
     * mudó a `services/` (el resto sigue llegando por `super::*`). */
    use crate::services::sesion::AudioPendiente;
    use crate::services::transporte::reparto;
    use crate::services::turno::{
        atender_resultado_turno, cuerpo_audio_con_texto, debe_escalar_consultando,
        hay_avance_turno, partir_respuesta, transcribir_y_anexar, ACUSE_TEXTO, AVISO_ASESOR_TEXTO,
        ESPERA_ACUSE_MS, FALLBACK_TEXTO, MAX_PARTES,
    };

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
