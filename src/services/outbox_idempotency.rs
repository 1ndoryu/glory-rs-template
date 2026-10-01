//! Outbox idempotente `WhatsApp` ([011A-5] Fase1, `F5 strangler`);
//! Fase3: revive + requeue + corte explícito).
//!
//! El núcleo declara `agent_outbox.idempotency_key UNIQUE` parcial
//! (`WHERE idempotency_key IS NOT NULL`; migración `20261001000019`,
//! espejo de `0004_canal.sql`): este módulo es el único que la escribe.
//! La clave es `sha256("{ambito}:{motivo}:{texto}")` en hex (hash de
//! contenido, NO HMAC: ningún punto de encolado tiene secreto).
//!
//! Semántica (decisión del plan 011A-5, refinada en Fase3):
//! - `encolar` con clave puede dar `Nuevo`, `Revivido` o `Duplicado`.
//!   `Duplicado` = ya hay gemelo `pending` (doble-clic, reintento,
//!   acuse concurrente): se traga, no es error.
//! - `Revivido` = el gemelo estaba `failed` y vuelve a `pending` con el
//!   payload fresco (el envío anterior nunca llegó: reintentar NO es
//!   duplicar). Sin este revive, un gemelo `failed` varado tragaría
//!   reintentos futuros en silencio (= pérdida).
//! - Solo `sent` libera la clave (`marcar` la pone a NULL): un texto
//!   idéntico futuro tras entrega es mensaje nuevo. `failed` la
//!   CONSERVA para que el reintento colisione con el gemelo en vez de
//!   duplicarlo; la purga TTL (+7 días) la borra.
//! - `manual` nunca lleva clave (intención explícita del staff).
//! - La clave solo se usa bajo corte (`corte_cubre`): con corte apagado
//!   el comportamiento es el legacy (sin clave), cero regresión.
//! - `reencolar_fallidos` (rollback: volver el corte a `apagado`/`wa_b`)
//!   pasa `failed→pending` in situ, sin filas nuevas: seguro por las
//!   claves (un encolado concurrente con la misma clave colisiona con
//!   la fila reencolada en vez de duplicarla).
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
/// `total` = ambos, `wa_b` = solo B, `apagado`/ausente = ninguno (legacy).
/// Ante error o valor desconocido, `false` (fail-closed al legacy: nunca
/// se bloquea un envío por un flag mal escrito).
/// Sticky por construcción: esta clave solo se LEE aquí; ningún código
/// la escribe (el cambio es manual en consola, auditable, nunca
/// auto-revierte). Verificado por búsqueda: sin `set_config` de
/// `corte_whatsapp` en `src/`.
pub async fn corte_cubre(pool: &PgPool, canal: &str) -> bool {
    let corte = glory_agent::persistence::get_config(pool, "corte_whatsapp")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    /* Brazos explícitos a propósito (el `allow` calla `match_same_arms`):
     * `apagado`/ausente es el default documentado y el comodín es el
     * fail-closed ante un flag mal escrito; fusionarlos ocultaría la
     * distinción auditable. */
    #[allow(clippy::match_same_arms)]
    match corte.trim() {
        "total" => true,
        "wa_b" => canal == "wa_b",
        "apagado" | "" => false,
        _ => false,
    }
}

/// Resultado de `encolar` con clave. `Nuevo`/`Revivido` llevan el id de
/// la fila que el worker enviará; `Duplicado` = gemelo `pending` ya en
/// cola (nada que hacer).
pub enum Encolado {
    Nuevo(Uuid),
    Revivido(Uuid),
    Duplicado,
}

impl Encolado {
    /// `id` de la fila a enviar (`None` solo en `Duplicado`).
    #[must_use]
    pub fn id(&self) -> Option<Uuid> {
        match *self {
            Encolado::Nuevo(id) | Encolado::Revivido(id) => Some(id),
            Encolado::Duplicado => None,
        }
    }
}

/// Encola en `agent_outbox` con clave opcional.
/// Sin clave el INSERT es directo (NULL nunca colisiona en el UNIQUE
/// parcial) y siempre retorna `Nuevo`.
/// Con clave usa `ON CONFLICT DO UPDATE` contra el índice parcial
/// `uq_agent_outbox_idem` (el predicado del árbitro debe igualar el del
/// índice): gemelo `failed` → revive a `pending` con el payload fresco
/// (gana lo último: misma clave = mismo contenido semántico); gemelo
/// `pending` → no toca nada y retorna `Duplicado`. Una sola ida a BD,
/// sin carrera buscar-crear.
pub async fn encolar(
    pool: &PgPool,
    kind: &str,
    payload: serde_json::Value,
    clave: Option<&str>,
) -> Result<Encolado, sqlx::Error> {
    let Some(clave) = clave else {
        let id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO agent_outbox (kind, payload) VALUES ($1, $2) RETURNING id",
        )
        .bind(kind)
        .bind(payload)
        .fetch_one(pool)
        .await?;
        return Ok(Encolado::Nuevo(id));
    };
    /* `xmax = 0` distingue inserto (fila nueva) de actualizado (revive):
     * `xmax` es el xid de la transacción que tocó la fila por última vez. */
    let fila: Option<(Uuid, bool)> = sqlx::query_as(
        "INSERT INTO agent_outbox (kind, payload, idempotency_key) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (idempotency_key) WHERE idempotency_key IS NOT NULL \
         DO UPDATE SET status = 'pending', payload = EXCLUDED.payload \
         WHERE agent_outbox.status = 'failed' \
         RETURNING id, (xmax = 0) AS fue_insert",
    )
    .bind(kind)
    .bind(payload)
    .bind(clave)
    .fetch_optional(pool)
    .await?;
    match fila {
        Some((id, true)) => Ok(Encolado::Nuevo(id)),
        Some((id, false)) => Ok(Encolado::Revivido(id)),
        None => Ok(Encolado::Duplicado),
    }
}

/// Marca terminal del worker (sustituye a `mark_outbox` del núcleo aquí).
/// Solo `sent` libera la clave (entregado = un texto idéntico futuro es
/// mensaje nuevo). `failed` la CONSERVA: el envío nunca llegó y el
/// reintento debe colisionar con el gemelo (revivirlo) en vez de
/// duplicarlo; `reencolar_fallidos` lo devuelve a `pending` con su clave.
pub async fn marcar(pool: &PgPool, id: Uuid, estado: &str) -> Result<(), sqlx::Error> {
    if !matches!(estado, "pending" | "sent" | "failed") {
        return Err(sqlx::Error::Protocol(
            "outbox estado debe ser pending|sent|failed".into(),
        ));
    }
    sqlx::query(
        "UPDATE agent_outbox SET status = $2, \
         idempotency_key = CASE WHEN $2 = 'sent' THEN NULL \
         ELSE idempotency_key END WHERE id = $1",
    )
    .bind(id)
    .bind(estado)
    .execute(pool)
    .await?;
    Ok(())
}

/// Requeue de rollback ([011A-5] Fase3): `failed→pending` in situ de un
/// `kind` (típico `whatsapp` al volver el corte a `apagado`/`wa_b`).
/// No crea filas: seguro por las claves (las reencoladas conservan la
/// suya y cualquier encolado concurrente idéntico revive/colisiona en
/// vez de duplicar). Retorna filas movidas. Idempotente: repetirlo no
/// cambia nada.
pub async fn reencolar_fallidos(pool: &PgPool, kind: &str) -> Result<u64, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE agent_outbox SET status = 'pending' \
         WHERE status = 'failed' AND kind = $1",
    )
    .bind(kind)
    .execute(pool)
    .await?;
    Ok(r.rows_affected())
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
     * el segundo retorna Duplicado (tragado, no error).
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
        assert!(matches!(primero, Encolado::Nuevo(_)));
        let segundo = encolar(&pool, "whatsapp", payload(), Some(&clave))
            .await
            .unwrap();
        assert!(matches!(segundo, Encolado::Duplicado));
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

    /* [011A-5 Fase3] El reintento sobre un gemelo `failed` lo revive con
     * el MISMO id (no inserta fila nueva); el siguiente colisiona con el
     * gemelo ya `pending` y es Duplicado. Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn reintento_revive_fallido_con_mismo_id() {
        let Some(pool) = pool_si_hay() else { return };
        let sid = Uuid::new_v4();
        let clave = clave_idempotencia(&sid.to_string(), "ia", "revive-test");
        let payload = || serde_json::json!({"session_id": sid.to_string(), "motivo": "ia", "texto": "revive-test"});
        let Encolado::Nuevo(id) = encolar(&pool, "whatsapp", payload(), Some(&clave))
            .await
            .unwrap()
        else {
            panic!("primer encolado debe ser Nuevo");
        };
        marcar(&pool, id, "failed").await.unwrap();
        let revivido = encolar(&pool, "whatsapp", payload(), Some(&clave))
            .await
            .unwrap();
        assert!(matches!(revivido, Encolado::Revivido(rid) if rid == id));
        let estado: String = sqlx::query_scalar("SELECT status FROM agent_outbox WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(estado, "pending");
        let tercero = encolar(&pool, "whatsapp", payload(), Some(&clave))
            .await
            .unwrap();
        assert!(matches!(tercero, Encolado::Duplicado));
        sqlx::query("DELETE FROM agent_outbox WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [011A-5 Fase3] `failed` CONSERVA la clave (el envío nunca llegó);
     * solo `sent` la libera. Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn marcar_failed_conserva_clave() {
        let Some(pool) = pool_si_hay() else { return };
        let sid = Uuid::new_v4();
        let clave = clave_idempotencia(&sid.to_string(), "ia", "clave-failed");
        let id = encolar(
            &pool,
            "whatsapp",
            serde_json::json!({"session_id": sid.to_string()}),
            Some(&clave),
        )
        .await
        .unwrap()
        .id()
        .unwrap();
        marcar(&pool, id, "failed").await.unwrap();
        let guardada: Option<String> =
            sqlx::query_scalar("SELECT idempotency_key FROM agent_outbox WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(guardada.as_deref(), Some(clave.as_str()));
        sqlx::query("DELETE FROM agent_outbox WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [011A-5 Fase3] Requeue de rollback: `failed→pending` in situ, con
     * la clave intacta para que un concurrente idéntico colisione en vez
     * de duplicar; repetirlo no mueve nada. Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn reencolar_fallidos_devuelve_pending_con_clave() {
        let Some(pool) = pool_si_hay() else { return };
        let sid = Uuid::new_v4();
        let clave = clave_idempotencia(&sid.to_string(), "ia", "requeue-test");
        let id = encolar(
            &pool,
            "whatsapp",
            serde_json::json!({"session_id": sid.to_string()}),
            Some(&clave),
        )
        .await
        .unwrap()
        .id()
        .unwrap();
        marcar(&pool, id, "failed").await.unwrap();
        assert_eq!(reencolar_fallidos(&pool, "whatsapp").await.unwrap(), 1);
        let (estado, guardada): (String, Option<String>) =
            sqlx::query_as("SELECT status, idempotency_key FROM agent_outbox WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(estado, "pending");
        assert_eq!(guardada.as_deref(), Some(clave.as_str()));
        /* Concurrente idéntico colisiona con la reencolada: Duplicado. */
        let concurrente = encolar(
            &pool,
            "whatsapp",
            serde_json::json!({"session_id": sid.to_string()}),
            Some(&clave),
        )
        .await
        .unwrap();
        assert!(matches!(concurrente, Encolado::Duplicado));
        assert_eq!(reencolar_fallidos(&pool, "whatsapp").await.unwrap(), 0);
        sqlx::query("DELETE FROM agent_outbox WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [011A-5 Fase3] Matriz del corte: `wa_b` solo cubre B (`wa_b` no
     * toca `wa_a`, DoD del plan), `total` cubre ambos, `apagado`/ausente/
     * valor raro = legacy en ambos (fail-closed). Sin `DATABASE_URL` se
     * omite. Limpia la clave al salir. */
    #[tokio::test]
    async fn corte_matriz_cubre_por_canal_y_falla_cerrado() {
        let Some(pool) = pool_si_hay() else { return };
        let casos = [
            ("wa_b", true, false),
            ("total", true, true),
            ("apagado", false, false),
            ("cualquier-cosa", false, false),
        ];
        for (valor, cubre_b, cubre_a) in casos {
            glory_agent::persistence::set_config(&pool, "corte_whatsapp", valor)
                .await
                .unwrap();
            assert_eq!(corte_cubre(&pool, "wa_b").await, cubre_b, "{valor}/wa_b");
            assert_eq!(corte_cubre(&pool, "wa_a").await, cubre_a, "{valor}/wa_a");
        }
        sqlx::query("DELETE FROM agent_config WHERE key = 'corte_whatsapp'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(!corte_cubre(&pool, "wa_b").await);
        assert!(!corte_cubre(&pool, "wa_a").await);
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
        .id()
        .expect("primer encolado es Nuevo");
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
        assert!(matches!(re, Encolado::Nuevo(_)));
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
            .id()
            .expect("encolado sin clave es Nuevo");
        let viejo_pending = encolar(&pool, "whatsapp", payload(), None)
            .await
            .unwrap()
            .id()
            .expect("encolado sin clave es Nuevo");
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
