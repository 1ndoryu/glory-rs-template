use sqlx::PgPool;

/* [169A-4] Avisos WhatsApp por escalacion (patron Nakomi `chat_alert_worker`,
 * solo lectura como referencia). Poll cada 15 s sobre `agent_outbox`
 * (`kind='whatsapp'`): POST al gateway y marca `sent|failed`. Sin gateway
 * configurado avisa una vez en logs y no itera: los avisos quedan
 * `pending` y visibles en el panel staff (nunca silencio). */

/// Bucle del worker. No retorna (tarea de fondo; ver `main.rs`).
pub async fn vigilar(pool: PgPool, gateway: Option<String>) {
    let Some(url) = gateway.filter(|u| !u.trim().is_empty()) else {
        tracing::warn!(
            "alerta WhatsApp sin gateway (GLORY_ALERT_GATEWAY_URL vacio): \
             las escalaciones quedan 'pending' en agent_outbox hasta configurar"
        );
        return;
    };
    tracing::info!("alerta WhatsApp activa hacia gateway configurado");
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .unwrap_or_default();
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        let pendientes = match glory_agent::persistence::fetch_pending_outbox(&pool, 10).await {
            Ok(list) => list,
            Err(e) => {
                tracing::error!("alerta WhatsApp: no se pudo leer outbox: {e}");
                continue;
            }
        };
        for entry in pendientes.into_iter().filter(|e| e.kind == "whatsapp") {
            let estado = procesar_aviso(&pool, &http, &url, &entry).await;
            if let Err(e) = glory_agent::persistence::mark_outbox(&pool, entry.id, estado).await {
                tracing::error!(
                    "alerta WhatsApp: no se pudo marcar outbox {}: {e}",
                    entry.id
                );
            }
        }
    }
}

/// Envía un aviso. Retorna el estado final para `mark_outbox`.
async fn procesar_aviso(
    pool: &PgPool,
    http: &reqwest::Client,
    url: &str,
    entry: &glory_agent::models::OutboxEntry,
) -> &'static str {
    /* [279A-2 F3] `destino` explícito en el payload (aviso al otro número
     * con ficha) con fallback a `whatsapp_admin` (modo completo clásico).
     * Todo async real: sin bloqueos (`get_config_blocking` no existe). */
    let destino_payload = entry
        .payload
        .get("destino")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let destino = if destino_payload.is_some() {
        destino_payload
    } else {
        glory_agent::persistence::get_config(pool, "whatsapp_admin")
            .await
            .ok()
            .flatten()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let Some(destino) = destino else {
        tracing::warn!(
            "alerta WhatsApp {}: sin 'whatsapp_admin' en agent_config, se reintenta",
            entry.id
        );
        return "pending";
    };
    let texto = texto_aviso(pool, entry).await;
    /* [279A-2 F2] Passthrough de `media_url` (fotos solo-enviar con pie):
     * si el outbox la trae (envío manual F5 o ficha con foto), el gateway
     * Baileys la manda; si no, aviso solo-texto como siempre. */
    let media_url = entry
        .payload
        .get("media_url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let cuerpo = if let Some(url_media) = media_url {
        serde_json::json!({"destino": destino, "texto": texto, "media_url": url_media})
    } else {
        serde_json::json!({"destino": destino, "texto": texto})
    };
    let r = http.post(url).json(&cuerpo).send().await;
    match r {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!("alerta WhatsApp {} enviada", entry.id);
            "sent"
        }
        Ok(resp) => {
            tracing::error!("alerta WhatsApp {}: gateway {}", entry.id, resp.status());
            "failed"
        }
        Err(e) => {
            tracing::error!("alerta WhatsApp {}: error de red: {e}", entry.id);
            "failed"
        }
    }
}

/// Texto del aviso con ficha comercial (decisión usuaria 2026-09-27:
/// nombre+teléfono+resumen+interés+presupuesto+zona). Si la sesión no deja
/// ficha (uuid inválido o sin filas), cae al texto mínimo con sesión+motivo:
/// mejor aviso parcial que ninguno (nunca silencio).
async fn texto_aviso(pool: &PgPool, entry: &glory_agent::models::OutboxEntry) -> String {
    let sesion_txt = entry
        .payload
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let motivo = entry
        .payload
        .get("motivo")
        .and_then(|v| v.as_str())
        .unwrap_or("sin motivo");
    let resumen = entry
        .payload
        .get("resumen")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("-");
    let Ok(sesion_id) = sesion_txt.parse::<uuid::Uuid>() else {
        return format!(
            "Chat inmobiliaria: la sesion {sesion_txt} necesita un humano ({motivo}). \
             Atiendela en /admin (Mensajes)."
        );
    };
    let Ok(ficha) = crate::repositories::ClienteRepository::ficha_para_aviso(pool, sesion_id).await
    else {
        return format!(
            "Chat inmobiliaria: la sesion {sesion_txt} necesita un humano ({motivo}). \
             Atiendela en /admin (Mensajes)."
        );
    };
    format!(
        "Chat MN ({}): {} necesita humano ({}) — nombre: {}, tel: {}, resumen: {}, \
         interes: {}, presupuesto: {}, zona: {}. Atiendela en /admin (Mensajes).",
        ficha.modo,
        sesion_txt,
        motivo,
        ficha.nombre.as_deref().unwrap_or("-"),
        ficha.telefono.as_deref().unwrap_or("-"),
        resumen,
        ficha.interes.as_deref().unwrap_or("-"),
        ficha.presupuesto.as_deref().unwrap_or("-"),
        ficha.zona.as_deref().unwrap_or("-"),
    )
}

/* [169A-4] Sin gateway real no hay E2E: estos tests verifican las ramas de
 * estado contra la BD de rama (sin `DATABASE_URL` se omiten). */
#[cfg(test)]
mod pruebas {
    use super::*;

    fn pool_si_hay() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    #[tokio::test]
    async fn gateway_caido_marca_failed_no_pending_eterno() {
        let Some(pool) = pool_si_hay() else { return };
        glory_agent::persistence::set_config(&pool, "whatsapp_admin", "+34600000000")
            .await
            .unwrap();
        let sid = uuid::Uuid::new_v4().to_string();
        let entrada = glory_agent::persistence::enqueue_outbox(
            &pool,
            "whatsapp",
            serde_json::json!({"session_id": sid, "motivo": "prueba"}),
        )
        .await
        .unwrap();
        let id = entrada.id;
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let estado = procesar_aviso(&pool, &http, "http://127.0.0.1:9/inexistente", &entrada).await;
        assert_eq!(estado, "failed");
        glory_agent::persistence::mark_outbox(&pool, id, estado)
            .await
            .unwrap();
        let queda: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE id = $1 AND status = 'failed'",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(queda, 1);

        sqlx::query("DELETE FROM agent_outbox WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_config WHERE key = 'whatsapp_admin'")
            .execute(&pool)
            .await
            .unwrap();
    }
}
