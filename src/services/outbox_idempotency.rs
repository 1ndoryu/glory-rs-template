//! Outbox idempotente `WhatsApp` ([011A-5] Fase1, `F5 strangler`).
//!
//! El núcleo declara `agent_outbox.idempotency_key UNIQUE` (migración
//! `20261001000019`, espejo de `0004_canal.sql`): este módulo es el único
//! que la escribe. La clave es `sha256("{ambito}:{motivo}:{texto}")` en hex
//! (hash de contenido, NO HMAC: ningún punto de encolado tiene secreto).
//!
//! Semántica (decisión del plan 011A-5):
//! - `encolar` devuelve `None` si la clave ya existe = duplicado tragado
//!   (doble-clic, reintento, acuse concurrente). Solo filas `pending`
//!   conservan clave: `marcar` la pone a NULL al pasar a `sent`/`failed`,
//!   así un texto idéntico futuro es un mensaje nuevo, no un duplicado.
//! - `manual` nunca lleva clave (intención explícita del staff).
//! - La clave solo se usa bajo corte (`corte_cubre`): con corte apagado el
//!   comportamiento es el legacy (sin clave), cero riesgo de regresión.
//! - `purgar_resueltos` borra `sent`/`failed` de +7 días (TTL del plan);
//!   `pending` jamás se purga (perdería mensajes).

use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Días que vive una fila resuelta antes de la purga (`DoD` del plan).
const TTL_RESUELTOS_DIAS: i64 = 7;

/// Clave determinista `sha256("{ambito}:{motivo}:{texto}")` en hex.
/// `ambito` = `session_id` (o `"global"` para avisos sin sesión como el
/// tope). Pura y testeable sin BD.
#[must_use]
pub fn clave_idempotencia(ambito: &str, motivo: &str, texto: &str) -> String {
    let mut h = Sha256::new();
    h.update(ambito.as_bytes());
    h.update([0x1f]);
    h.update(motivo.as_bytes());
    h.update([0x1f]);
    h.update(texto.as_bytes());
    format!("{:x}", h.finalize())
}

/// ¿Este motivo usa clave? `manual` queda fuera (doble-clic del staff =
/// intención explícita, nunca se traga).
#[must_use]
pub fn debe_usar_clave(motivo: &str) -> bool {
    motivo != "manual"
}

/// ¿El corte F5 cubre este canal? Lee `agent_config corte_whatsapp`:
/// `total` = ambos, `wa_b` = solo B. Ante error o valor desconocido,
/// `false` (fail-closed al legacy, nunca se bloquea un envío).
pub async fn corte_cubre(pool: &PgPool, canal: &str) -> bool {
    let corte = glory_agent::persistence::get_config(pool, "corte_whatsapp")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    match corte.trim() {
        "total" => true,
        "wa_b" => canal == "wa_b",
        _ => false,
    }
}

/// Encola en `agent_outbox` con clave opcional. `Ok(None)` = la clave ya
/// existía (duplicado tragado, no error). Sin clave el INSERT es directo
/// (NULL nunca colisiona en el UNIQUE parcial).
/// Retorna el `id` insertado cuando sí se encoló.
pub async fn encolar(
    pool: &PgPool,
    kind: &str,
    payload: serde_json::Value,
    clave: Option<&str>,
) -> Result<Option<Uuid>, sqlx::Error> {
    match sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO agent_outbox (kind, payload, idempotency_key) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(kind)
    .bind(payload)
    .bind(clave)
    .fetch_one(pool)
    .await
    {
        Ok(id) => Ok(Some(id)),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Ok(None),
        Err(e) => Err(e),
    }
}

/// Marca terminal del worker (sustituye a `mark_outbox` del núcleo aquí):
/// mismo UPDATE de estado, pero la clave se pone a NULL al salir de
/// `pending` para que un texto idéntico futuro no colisione.
pub async fn marcar(pool: &PgPool, id: Uuid, estado: &str) -> Result<(), sqlx::Error> {
    if !matches!(estado, "pending" | "sent" | "failed") {
        return Err(sqlx::Error::Protocol(
            "outbox estado debe ser pending|sent|failed".into(),
        ));
    }
    sqlx::query(
        "UPDATE agent_outbox SET status = $2, \
         idempotency_key = CASE WHEN $2 IN ('sent', 'failed') THEN NULL \
         ELSE idempotency_key END WHERE id = $1",
    )
    .bind(id)
    .bind(estado)
    .execute(pool)
    .await?;
    Ok(())
}

/// Purga TTL: borra `sent`/`failed` con más de 7 días. Retorna filas.
/// `pending` jamás se toca.
pub async fn purgar_resueltos(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let r = sqlx::query(
        "DELETE FROM agent_outbox WHERE status IN ('sent', 'failed') \
         AND created_at < NOW() - ($1 * INTERVAL '1 day')",
    )
    .bind(TTL_RESUELTOS_DIAS)
    .execute(pool)
    .await?;
    Ok(r.rows_affected())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn pool_si_hay() -> Option<sqlx::PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    /* [011A-5] La clave es estable y hex de 64. */
    #[test]
    fn clave_estable_y_hex() {
        let a = clave_idempotencia("sesion-1", "ia", "hola");
        let b = clave_idempotencia("sesion-1", "ia", "hola");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /* [011A-5] Ámbito, motivo o texto distintos = clave distinta. */
    #[test]
    fn clave_distingue_entradas() {
        let base = clave_idempotencia("s", "ia", "hola");
        assert_ne!(base, clave_idempotencia("s2", "ia", "hola"));
        assert_ne!(base, clave_idempotencia("s", "acuse", "hola"));
        assert_ne!(base, clave_idempotencia("s", "ia", "adios"));
    }

    /* [011A-5] `manual` nunca usa clave (doble-clic = intención). */
    #[test]
    fn manual_sin_clave() {
        assert!(!debe_usar_clave("manual"));
        assert!(debe_usar_clave("ia"));
        assert!(debe_usar_clave("tope"));
    }

    /* [011A-5] Doble enqueue con la misma clave inserta una sola fila;
     * el segundo retorna None (duplicado tragado, no error).
     * Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn doble_enqueue_misma_clave_inserta_una() {
        let Some(pool) = pool_si_hay() else { return };
        let sid = Uuid::new_v4();
        let clave = clave_idempotencia(&sid.to_string(), "ia", "texto unico");
        let payload = || serde_json::json!({"session_id": sid.to_string(), "motivo": "ia", "texto": "texto unico"});
        let primero = encolar(&pool, "whatsapp", payload(), Some(&clave))
            .await
            .unwrap();
        assert!(primero.is_some());
        let segundo = encolar(&pool, "whatsapp", payload(), Some(&clave))
            .await
            .unwrap();
        assert!(segundo.is_none());
        let n: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM agent_outbox WHERE idempotency_key = $1")
                .bind(&clave)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(n, 1);
        sqlx::query("DELETE FROM agent_outbox WHERE idempotency_key = $1")
            .bind(&clave)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [011A-5] Al marcar `sent` la clave se libera: el mismo texto futuro
     * vuelve a encolar (mensaje nuevo, no duplicado).
     * Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn marcar_sent_libera_clave() {
        let Some(pool) = pool_si_hay() else { return };
        let sid = Uuid::new_v4();
        let clave = clave_idempotencia(&sid.to_string(), "acuse", "acuse-test");
        let id = encolar(
            &pool,
            "whatsapp",
            serde_json::json!({"session_id": sid.to_string()}),
            Some(&clave),
        )
        .await
        .unwrap()
        .unwrap();
        marcar(&pool, id, "sent").await.unwrap();
        let libre: bool =
            sqlx::query_scalar("SELECT idempotency_key IS NULL FROM agent_outbox WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(libre);
        let re = encolar(
            &pool,
            "whatsapp",
            serde_json::json!({"session_id": sid.to_string()}),
            Some(&clave),
        )
        .await
        .unwrap();
        assert!(re.is_some());
        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sid.to_string())
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [011A-5] La purga solo borra resueltos viejos: `pending` viejo y
     * `sent` reciente sobreviven. Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn purga_solo_resueltos_viejos() {
        let Some(pool) = pool_si_hay() else { return };
        let sid = Uuid::new_v4().to_string();
        let payload = || serde_json::json!({"session_id": sid});
        let viejo_sent = encolar(&pool, "whatsapp", payload(), None)
            .await
            .unwrap()
            .unwrap();
        let viejo_pending = encolar(&pool, "whatsapp", payload(), None)
            .await
            .unwrap()
            .unwrap();
        marcar(&pool, viejo_sent, "sent").await.unwrap();
        for id in [viejo_sent, viejo_pending] {
            sqlx::query(
                "UPDATE agent_outbox SET created_at = NOW() - INTERVAL '8 days' WHERE id = $1",
            )
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        }
        let borradas = purgar_resueltos(&pool).await.unwrap();
        assert!(borradas >= 1);
        let queda_pending: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM agent_outbox WHERE id = $1")
                .bind(viejo_pending)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(queda_pending, 1);
        let queda_sent: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_outbox WHERE id = $1")
            .bind(viejo_sent)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(queda_sent, 0);
        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(&sid)
            .execute(&pool)
            .await
            .unwrap();
    }
}
