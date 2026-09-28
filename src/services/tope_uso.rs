/* [279A-2 tope] Tope diario de tokens LLM + alerta (plan §10, sin QR).
 *
 * Qué: `revisar_tope` suma los tokens LLM exactos de hoy (`tokens_in` +
 * `tokens_out` de `uso_mensajes`; la estima de cliente no es coste y no
 * cuenta) y compara con `ia_tope_tokens_dia` (default 2_000_000). Si se
 * supera sin alerta hoy (`ia_tope_alertado` = fecha), encola un aviso
 * `whatsapp` con `motivo: tope` (motivo explícito + `texto` listo: el
 * worker lo manda tal cual tras el fix de `texto_aviso`) y marca la fecha.
 *
 * Solo alerta, nunca apaga: con el uso actual (~10 por mensaje) el tope
 * tardaría años en saltar; el kill-switch (`ai_enabled=false` por sesión)
 * lo decide un humano en la consola, no un watcher. Si no hay
 * `whatsapp_admin`, el aviso queda `pending` visible en el panel: nunca
 * silencio.
 *
 * `evaluar_tope` es puro para testear la decisión sin BD. El `vigilar`
 * corre cada 5 min en segundo plano junto al watcher de alertas. */

use sqlx::PgPool;

const CLAVE_TOPE: &str = "ia_tope_tokens_dia";
const CLAVE_ALERTA: &str = "ia_tope_alertado";
const TOPE_DEFECTO: i64 = 2_000_000;

/// Decisión pura: tope positivo alcanzado y aún no alertado hoy.
#[must_use]
pub fn evaluar_tope(uso_hoy: i64, tope: i64, ya_alertado: bool) -> bool {
    tope > 0 && uso_hoy >= tope && !ya_alertado
}

/// Revisa el uso de hoy y alerta una vez al día. Devuelve `true` si alertó.
pub async fn revisar_tope(pool: &PgPool) -> Result<bool, String> {
    let tope = glory_agent::persistence::get_config(pool, CLAVE_TOPE)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(TOPE_DEFECTO);
    let hoy: String = sqlx::query_scalar("SELECT CURRENT_DATE::TEXT")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let ya_alertado = glory_agent::persistence::get_config(pool, CLAVE_ALERTA)
        .await
        .map_err(|e| e.to_string())?
        .is_some_and(|v| v == hoy);
    let uso_hoy: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(COALESCE(tokens_in, 0) + COALESCE(tokens_out, 0)), 0) \
         FROM uso_mensajes WHERE created_at >= CURRENT_DATE",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    if !evaluar_tope(uso_hoy, tope, ya_alertado) {
        return Ok(false);
    }
    let top: Vec<(Option<String>, Option<i64>)> = sqlx::query_as(
        "SELECT sender, SUM(COALESCE(tokens_in, 0) + COALESCE(tokens_out, 0)) \
         FROM uso_mensajes WHERE created_at >= CURRENT_DATE \
         GROUP BY sender ORDER BY 2 DESC LIMIT 3",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let detalle = top
        .iter()
        .map(|(r, t)| format!("{}: {}", r.as_deref().unwrap_or("?"), t.unwrap_or(0)))
        .collect::<Vec<_>>()
        .join(", ");
    let texto = format!(
        "Tope diario LLM superado: {uso_hoy} tokens (tope {tope}). Top: {detalle}. \
         Revisa /admin (Uso) y ajusta {CLAVE_TOPE} si el gasto es legítimo."
    );
    glory_agent::persistence::enqueue_outbox(
        pool,
        "whatsapp",
        serde_json::json!({"motivo": "tope", "texto": texto}),
    )
    .await
    .map_err(|e| e.to_string())?;
    glory_agent::persistence::set_config(pool, CLAVE_ALERTA, &hoy)
        .await
        .map_err(|e| e.to_string())?;
    tracing::warn!("tope diario LLM superado ({uso_hoy} >= {tope}): alerta encolada");
    Ok(true)
}

/// Bucle de fondo: revisa cada 5 min; los fallos se registran y se reintenta
/// (igual que el watcher de alertas: observación ruidosa, nunca pánico).
pub async fn vigilar(pool: PgPool) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        if let Err(e) = revisar_tope(&pool).await {
            tracing::warn!("revisión de tope LLM fallida: {e}");
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::evaluar_tope;

    #[test]
    fn tope_solo_alerta_una_vez_al_superar() {
        assert!(!evaluar_tope(10, 2_000_000, false));
        assert!(!evaluar_tope(1_999_999, 2_000_000, false));
        assert!(evaluar_tope(2_000_000, 2_000_000, false));
        assert!(evaluar_tope(3_000_000, 2_000_000, false));
        assert!(!evaluar_tope(3_000_000, 2_000_000, true));
        assert!(!evaluar_tope(3_000_000, 0, false));
        assert!(!evaluar_tope(3_000_000, -5, false));
    }
}
