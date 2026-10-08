/* [06AA-3 F3] Capa `Agente→Envío` del turno WhatsApp (plan 03AA-4
 * §Arquitectura): el turno IA en background con su acuse diferido (E-fluido),
 * partido en partes, escalada con aviso y fallback — todo lo que era la
 * segunda mitad de `handlers/whatsapp.rs` vive aquí.
 * - `TurnoFondo` + `disparar_turno_fondo` es el gancho propio que anticipaba
 *   F2 (`politica.rs`): el webhook entrega el texto CRUDO y el trato, y el
 *   tono (`texto_para_turno`) se aplica aquí, junto al modelo. Lo persistido
 *   y el panel siguen intactos. Sin cambios de conducta: verbatim. */

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

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use crate::repositories::ClienteRepository;
use crate::services::sesion::{AudioPendiente, FotoPendiente, MediosPendientes};
use crate::services::triage::Trato;
use crate::services::{
    clave_idempotencia, corte_cubre, debe_usar_clave, encolar_outbox_idem, politica, Encolado,
};
use glory_agent::errors::AgentError;

/// Espera antes del acuse: la mayoría de turnos con tools cierra en 5-15 s;
/// a los 6 s sin respuesta vale un "ya voy"; antes es ruido.
/* [Fase3-H6] Además el acuse se omite si el turno ya encoló texto IA: en F1
 * salía DESPUÉS de las tarjetas y se leía como cierre ("dame un momentico"
 * tras la oferta). Ver `hay_avance_turno`. */
pub(crate) const ESPERA_ACUSE_MS: u64 = 6_000;
/// Máx de partes de texto por turno (intro + cierre; el resto se funde).
pub(crate) const MAX_PARTES: usize = 3;
pub(crate) const ACUSE_TEXTO: &str = "Ya lo estoy revisando, dame un momentico 👀";
pub(crate) const FALLBACK_TEXTO: &str =
    "Se me complicó con eso, ¿me lo repites en un momentico? 🙏";
pub(crate) const AVISO_ASESOR_TEXTO: &str = "Dame un momentico que ya te atiende un asesor 🙏";

/// Claves de `agent_config` con el tono editable (F4 las expone en admin).
pub(crate) const CLAVE_ACUSE: &str = "whatsapp_acuse_texto";
pub(crate) const CLAVE_FALLBACK: &str = "whatsapp_fallback_texto";
pub(crate) const CLAVE_AVISO_ASESOR: &str = "whatsapp_aviso_asesor_texto";

/// Tono efectivo del turno (dueño de los `String` para el background).
pub(crate) struct TextosTono {
    pub(crate) acuse: String,
    pub(crate) fallback: String,
    pub(crate) aviso_asesor: String,
}

/// Valor efectivo: `None` o vacío = constante por defecto (el admin que
/// borra el campo vuelve al tono de fábrica, nunca al silencio).
/* [07AA-1 F4] Pura para testear; la lectura real es `leer_tono`. */
pub(crate) fn texto_efectivo<'a>(valor: Option<&'a str>, defecto: &'a str) -> &'a str {
    match valor {
        Some(v) if !v.trim().is_empty() => v,
        _ => defecto,
    }
}

/// Lee el tono desde `agent_config` (3 lecturas en camino excepcional: turno
/// lento, escalada o fallo — nunca en el camino caliente del 2xx). Fallo o
/// vacío = constantes (`ACUSE_TEXTO`, `FALLBACK_TEXTO`, `AVISO_ASESOR_TEXTO`).
pub(crate) async fn leer_tono(pool: &sqlx::PgPool) -> TextosTono {
    let acuse = glory_agent::persistence::get_config(pool, CLAVE_ACUSE)
        .await
        .ok()
        .flatten();
    let fallback = glory_agent::persistence::get_config(pool, CLAVE_FALLBACK)
        .await
        .ok()
        .flatten();
    let aviso = glory_agent::persistence::get_config(pool, CLAVE_AVISO_ASESOR)
        .await
        .ok()
        .flatten();
    TextosTono {
        acuse: texto_efectivo(acuse.as_deref(), ACUSE_TEXTO).to_string(),
        fallback: texto_efectivo(fallback.as_deref(), FALLBACK_TEXTO).to_string(),
        aviso_asesor: texto_efectivo(aviso.as_deref(), AVISO_ASESOR_TEXTO).to_string(),
    }
}

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
 * `manual` nunca lleva clave (lo excluye `debe_usar_clave`).
 * [08AA-13] FP `sqlite-carga-N-consultas`: los 2 `await` son dependientes
 * (la escritura necesita el veredicto del corte); no hay bucle ni N+1.
 * Detalle en `Agente/prevencion/prevencion-sentinel-secuencial-dependiente-fp-2026-10-08.md`. */
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

/// [08AA-3 B3] Resuelve una `clave` de storage bajo `dir` sin escape:
/// solo sobreviven componentes `Normal` (`whatsapp/<digitos>/<uuid>.<ext>`);
/// cualquier `..`, ruta absoluta o prefijo devuelve `None`. La `clave` la
/// generan `guardar_archivo`/`guardar_audio` (servidor), pero la lectura se
/// blinda igual en el boundary: chequeo léxico puro, sin IO extra.
fn ruta_clave(dir: &str, clave: &str) -> Option<PathBuf> {
    let rel = Path::new(clave);
    if rel.components().all(|c| matches!(c, Component::Normal(_))) {
        Some(Path::new(dir).join(rel))
    } else {
        None
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
    let Some(ruta) = ruta_clave(&dir, &foto.clave) else {
        tracing::warn!(
            "webhook WhatsApp: clave de foto fuera de UPLOAD_DIR: {}",
            foto.clave
        );
        return;
    };
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
    let descripcion = match crate::handlers::ia::describir_foto(&data_url, &foto.pie).await {
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
pub(crate) fn cuerpo_audio_con_texto(cuerpo_base: &str, texto: &str) -> String {
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
pub(crate) async fn transcribir_y_anexar(pool: &sqlx::PgPool, audio: AudioPendiente) {
    /* Mismo tope que `guardar_audio` (10 MiB): Groq acepta 25 MB, pero lo
     * que ya se archivó en local es lo que hay. */
    const TOPE_BYTES: usize = 10 * 1024 * 1024;
    let dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    let Some(ruta) = ruta_clave(&dir, &audio.clave) else {
        tracing::warn!(
            "webhook WhatsApp: clave de audio fuera de UPLOAD_DIR: {}",
            audio.clave
        );
        return;
    };
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
    let texto = match crate::handlers::ia::transcribir_audio(&bytes, nombre, &audio.mime).await {
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

/// Parámetros del turno en background (extraídos del `webhook` por el lint
/// de 100 líneas y para que el tono de la política viaje junto al texto).
pub struct TurnoFondo {
    pub(crate) estado: glory_agent::transport::AgentState,
    pub(crate) pool: sqlx::PgPool,
    /// Texto CRUDO del visitante (lo persistido); el tono se aplica dentro.
    pub(crate) texto_crudo: String,
    pub(crate) trato: Trato,
    pub(crate) remitente: String,
    pub(crate) canal: String,
    pub(crate) sesion: Uuid,
    pub(crate) secuencia: i64,
    pub(crate) medios: MediosPendientes,
}

/// Dispara el turno IA en background: responde 2xx rápido al gateway mientras
/// la IA corre aparte (mismo `responder_turno_persistido` del chat web).
/// Aplica el tono de la política SOLO al texto del modelo (lo persistido no
/// cambia). Sobre presupuesto no hay turno, pero el 2xx ya salió igual.
pub(crate) fn disparar_turno_fondo(params: TurnoFondo) {
    /* [289A-1] Turno IA en background: el webhook responde 2xx rápido al
     * gateway; la IA (núcleo `responder_turno_persistido`, misma vía que el
     * chat web) corre aparte y su texto se encola en outbox `whatsapp` con
     * el `via` del canal, que el worker manda por la sesión Baileys que
     * recibió. `Ok(None)` (LLM sin texto tras tools, sin key, gate humano)
     * NO es silencio: escala la sesión a `consultando` para que el staff la
     * tome. `Err` se loguea: nunca 500 al gateway por fallos del LLM. */
    let TurnoFondo {
        estado: fondo,
        pool: pool_fondo,
        texto_crudo,
        trato,
        remitente: remitente_fondo,
        canal: canal_fondo,
        sesion: sesion_fondo,
        secuencia: secuencia_fondo,
        medios,
    } = params;
    /* [06AA-2] El modelo ve el tono de la política; lo persistido no cambia. */
    let texto_fondo = politica::texto_para_turno(trato, texto_crudo.trim());
    if !fondo.timing.check_budget(&remitente_fondo) {
        tracing::warn!("webhook WhatsApp: {sesion_fondo} sobre presupuesto, sin turno IA");
        return;
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
        /* [299A-1 E11] La foto se describe en background ANTES del turno para que
         * el historial ya traiga el `— se ve:`.
         * [309A-4] El audio se transcribe igual (`— dice:`). El 2xx al gateway
         * no espera a ninguno. */
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
        let tono = leer_tono(&pool).await;
        encolar_texto_ia(&pool, sesion, &destino, &canal, "acuse", &tono.acuse).await;
    });
}

/// ¿El turno ya encoló texto IA o tarjetas en los últimos 2 min? Evita el
/// acuse tardío (tras las tarjetas confundía). `false` ante error de BD:
/// mejor un acuse de más que romper el turno por una consulta auxiliar.
pub(crate) async fn hay_avance_turno(pool: &sqlx::PgPool, session_id: Uuid) -> bool {
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
pub(crate) async fn atender_resultado_turno(
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
            let tono = leer_tono(pool).await;
            encolar_texto_ia(pool, sesion, destino, canal, "asesor", &tono.aviso_asesor).await;
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
            let tono = leer_tono(pool).await;
            encolar_texto_ia(pool, sesion, destino, canal, "fallback", &tono.fallback).await;
        }
    }
}

/* [299A-1 E16] Decisión pura: un turno vacío escala a `consultando` salvo
 * ciclo `answered` (el staff ya respondió y la IA venía conversando: un hipo
 * del LLM no congela una conversación sana, caso 739bb63e). */
#[must_use]
pub(crate) fn debe_escalar_consultando(ciclo: Option<&str>) -> bool {
    ciclo != Some("answered")
}

#[cfg(test)]
mod pruebas {
    use super::{texto_efectivo, ACUSE_TEXTO};

    /* [07AA-1 F4] El tono configurado manda; `None` o vacío = fábrica. */
    #[test]
    fn texto_efectivo_respeta_config_o_fabrica() {
        assert_eq!(texto_efectivo(Some("Ya voy 👀"), ACUSE_TEXTO), "Ya voy 👀");
        assert_eq!(texto_efectivo(None, ACUSE_TEXTO), ACUSE_TEXTO);
        assert_eq!(texto_efectivo(Some(""), ACUSE_TEXTO), ACUSE_TEXTO);
        assert_eq!(texto_efectivo(Some("   "), ACUSE_TEXTO), ACUSE_TEXTO);
    }
}
