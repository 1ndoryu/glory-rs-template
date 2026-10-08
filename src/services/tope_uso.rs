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

use super::outbox_idempotency::{clave_idempotencia, corte_cubre, debe_usar_clave, encolar};
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
    let (hoy, tope_cfg, alerta_cfg, uso_hoy) = leer_estado_uso(pool).await?;
    let tope = tope_cfg
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(TOPE_DEFECTO);
    let ya_alertado = alerta_cfg.is_some_and(|v| v == hoy);
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
    /* [011A-5 Fase1] Clave idempotente bajo corte (ámbito `global`: el
     * tope no tiene sesión). El guard `ia_tope_alertado` ya limita a 1/día;
     * la clave cubre reintentos concurrentes del watcher. */
    let clave_tope = clave_idempotencia("global", "tope", &texto);
    let clave_tope_ref = if debe_usar_clave("tope") && corte_cubre(pool, "wa_a").await {
        Some(clave_tope.as_str())
    } else {
        None
    };
    encolar(
        pool,
        "whatsapp",
        /* [289A-1] `via` explícito: el tope no tiene hilo, sale por A. */
        serde_json::json!({"motivo": "tope", "texto": texto, "via": "wa_a"}),
        clave_tope_ref,
    )
    .await
    .map_err(|e| e.to_string())?;
    glory_agent::persistence::set_config(pool, CLAVE_ALERTA, &hoy)
        .await
        .map_err(|e| e.to_string())?;
    tracing::warn!("tope diario LLM superado ({uso_hoy} >= {tope}): alerta encolada");
    Ok(true)
}

/* [08AA-13] Una sola ida a BD: fecha + las 2 claves + suma del día en un
 * único SELECT (misma foto instantánea, sin carrera entre lecturas;
 * antes eran 4 `await` directos y saltaba `sqlite-carga-N-consultas`).
 * La validación (`trim`/`parse`, default) sigue en Rust, idéntica. */
async fn leer_estado_uso(pool: &PgPool) -> Result<(String, Option<String>, Option<String>, i64), String> {
    sqlx::query_as(
        "SELECT CURRENT_DATE::TEXT, \
          (SELECT value FROM agent_config WHERE key = $1), \
          (SELECT value FROM agent_config WHERE key = $2), \
          (SELECT COALESCE(SUM(COALESCE(tokens_in, 0) + COALESCE(tokens_out, 0)), 0) \
           FROM uso_mensajes WHERE created_at >= CURRENT_DATE)",
    )
    .bind(CLAVE_TOPE)
    .bind(CLAVE_ALERTA)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())
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
    use super::{evaluar_tope, leer_estado_uso};

    fn pool_si_hay() -> Option<sqlx::PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

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

    /* [08AA-13] La foto única equivale a las 3 lecturas separadas anteriores
     * (misma instantánea) y decodifica (`SUM` BIGINT → `i64`). Solo SELECTs:
     * no escribe config ni outbox, así que no interfiere con otros tests.
     * Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn estado_una_sola_foto_equivale_a_lecturas() {
        let Some(pool) = pool_si_hay() else { return };
        let (hoy, tope_cfg, alerta_cfg, uso_hoy) =
            leer_estado_uso(&pool).await.expect("foto única");
        let hoy2: String = sqlx::query_scalar("SELECT CURRENT_DATE::TEXT")
            .fetch_one(&pool)
            .await
            .expect("fecha");
        assert_eq!(hoy, hoy2);
        let tope2 = glory_agent::persistence::get_config(&pool, super::CLAVE_TOPE)
            .await
            .expect("tope");
        assert_eq!(tope_cfg, tope2);
        let alerta2 = glory_agent::persistence::get_config(&pool, super::CLAVE_ALERTA)
            .await
            .expect("alerta");
        assert_eq!(alerta_cfg, alerta2);
        let uso2: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(COALESCE(tokens_in, 0) + COALESCE(tokens_out, 0)), 0) \
             FROM uso_mensajes WHERE created_at >= CURRENT_DATE",
        )
        .fetch_one(&pool)
        .await
        .expect("suma");
        assert_eq!(uso_hoy, uso2);
    }
}
