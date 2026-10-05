/* [05AA-1] MN implementa el `Resolver` del núcleo (F10 paso 4): la
 * resolución cliente×canal sale del webhook en línea y pasa por el trait
 * (`sesion_por_canal`), sin cambio de conducta — reutiliza el hilo o crea
 * uno (`ensure_session` + `vincular_canal`). El núcleo define el contrato;
 * el producto pone la BD (`canal_sesiones`, `atencion_sesiones`); nunca al
 * revés. Fallo de BD → `Interno` (500, igual que el `Db` anterior: ambos
 * responden 500 en el núcleo). */

use std::{future::Future, pin::Pin};

use glory_agent::channels::{Resolver, ResolverError};
use sqlx::PgPool;
use uuid::Uuid;

use crate::repositories::ClienteRepository;

/// Modo de atención por canal (fuente única: antes vivía dentro de
/// `reparto()` en `whatsapp.rs`; el resolutor también la necesita y el
/// trait no recibe `modo`). Canal desconocido → `None`.
#[must_use]
pub fn modo_por_canal(canal: &str) -> Option<&'static str> {
    match canal {
        "wa_a" => Some("completo"),
        "wa_b" => Some("inicial"),
        _ => None,
    }
}

fn interno(e: impl std::fmt::Display) -> ResolverError {
    ResolverError::Interno(e.to_string())
}

/// Resolutor cliente×canal de MN. El `cliente_id` llega como `str` por el
/// trait (agnóstico); aquí debe ser un UUID válido.
pub struct CanalResolver {
    pool: PgPool,
}

impl CanalResolver {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn resolver(
        &self,
        cliente_id: &str,
        canal: &str,
        numero: &str,
    ) -> Result<Uuid, ResolverError> {
        let modo = modo_por_canal(canal).ok_or_else(|| ResolverError::CanalDesconocido {
            canal: canal.to_string(),
        })?;
        let cliente = Uuid::parse_str(cliente_id)
            .map_err(|e| interno(format!("cliente_id no es UUID: {e}")))?;
        let telefono = ClienteRepository::normalizar_telefono(numero);
        if let Some(previa) =
            ClienteRepository::buscar_sesion_por_cliente_canal(&self.pool, cliente, canal)
                .await
                .map_err(interno)?
        {
            ClienteRepository::vincular_canal(&self.pool, previa, cliente, &telefono, canal, modo)
                .await
                .map_err(interno)?;
            return Ok(previa);
        }
        let nueva = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&self.pool, nueva)
            .await
            .map_err(interno)?;
        ClienteRepository::vincular_canal(&self.pool, nueva, cliente, &telefono, canal, modo)
            .await
            .map_err(interno)?;
        Ok(nueva)
    }
}

impl Resolver for CanalResolver {
    fn sesion_por_canal<'a>(
        &'a self,
        cliente_id: &'a str,
        canal: &'a str,
        numero: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Uuid, ResolverError>> + Send + 'a>> {
        Box::pin(self.resolver(cliente_id, canal, numero))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool_si_hay() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    fn telefono_unico() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        format!("34609{:05}", nanos % 100_000)
    }

    #[test]
    fn modo_cubre_canales_conocidos() {
        assert_eq!(modo_por_canal("wa_a"), Some("completo"));
        assert_eq!(modo_por_canal("wa_b"), Some("inicial"));
        assert_eq!(modo_por_canal("web"), None);
    }

    /* El trait se ejerce contra BD viva: canal raro falla sin tocar nada;
     * el mismo cliente×canal devuelve siempre la misma sesión. Sin
     * `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn resolutor_reutiliza_hilo_y_rechaza_canal() {
        let Some(pool) = pool_si_hay() else { return };
        let resolutor = CanalResolver::new(pool.clone());
        let err = resolutor
            .sesion_por_canal(&Uuid::new_v4().to_string(), "sms", "3460911111")
            .await
            .unwrap_err();
        assert!(matches!(err, ResolverError::CanalDesconocido { .. }));
        let telefono = telefono_unico();
        let cliente = ClienteRepository::registrar_con_origen(&pool, None, &telefono, "wa_b")
            .await
            .unwrap();
        let cid = cliente.id.to_string();
        let primera = resolutor
            .sesion_por_canal(&cid, "wa_b", &telefono)
            .await
            .unwrap();
        let segunda = resolutor
            .sesion_por_canal(&cid, "wa_b", &telefono)
            .await
            .unwrap();
        assert_eq!(primera, segunda);
        for (tabla, columna) in [
            ("canal_sesiones", "session_id"),
            ("atencion_sesiones", "session_id"),
        ] {
            sqlx::query(&format!("DELETE FROM {tabla} WHERE {columna} = $1"))
                .bind(primera)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(primera)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM clientes WHERE id = $1")
            .bind(cliente.id)
            .execute(&pool)
            .await
            .unwrap();
    }
}
