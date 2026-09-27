use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ClienteRow;

/* [279A-2 F1] Acceso a `clientes` + `canal_sesiones` + `atencion_sesiones`
 * con prepared statements. El alta es un upsert atómico por teléfono
 * (sin buscar-crear secuencial): repetir no duplica, solo refresca nombre
 * y `updated_at`. `registrar_y_vincular` es el único punto de entrada
 * (tool `registrar_contacto` y `POST .../contacto` pasan por aquí). */

pub struct ClienteRepository;

impl ClienteRepository {
    /// Solo dígitos; `0...` local de 11 dígitos → `58...` (E.164 sin `+`).
    #[must_use]
    pub fn normalizar_telefono(tel: &str) -> String {
        let digitos: String = tel.chars().filter(char::is_ascii_digit).collect();
        if digitos.len() == 11 && digitos.starts_with('0') {
            format!("58{}", &digitos[1..])
        } else {
            digitos
        }
    }

    /// Upsert por teléfono; el nombre vacío no pisa el ya guardado.
    /// Columnas del RETURNING en el orden de `ClienteRow` (`FromRow` mapea
    /// por posición en `query_as` sin macro).
    pub async fn registrar(
        pool: &PgPool,
        nombre: Option<&str>,
        telefono: &str,
    ) -> Result<ClienteRow, sqlx::Error> {
        let nombre_limpio = nombre.map(str::trim).filter(|n| !n.is_empty());
        sqlx::query_as::<_, ClienteRow>(
            "INSERT INTO clientes (id, nombre, telefono) VALUES (gen_random_uuid(), $1, $2) \
             ON CONFLICT (telefono) DO UPDATE SET \
               nombre = COALESCE(NULLIF(EXCLUDED.nombre, ''), clientes.nombre), \
               updated_at = NOW() \
             RETURNING id, nombre, telefono, origen, interes, presupuesto, zona, \
               notas, created_at, updated_at",
        )
        .bind(nombre_limpio)
        .bind(telefono)
        .fetch_one(pool)
        .await
    }

    /// Vincula sesión↔cliente sin pisar `canal`/`modo` ya fijados (los pone
    /// F2 al repartir; en web se crean con `web`/`completo`). Asegura la fila
    /// de `atencion_sesiones` sin tocar un estado ya existente (la máquina
    /// F3 es la única que cambia `estado`).
    pub async fn vincular_sesion(
        pool: &PgPool,
        session_id: Uuid,
        cliente_id: Uuid,
        telefono: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO canal_sesiones (session_id, cliente_id, canal, telefono, modo) \
             VALUES ($1, $2, 'web', $3, 'completo') \
             ON CONFLICT (session_id) DO UPDATE SET \
               cliente_id = EXCLUDED.cliente_id, telefono = EXCLUDED.telefono",
        )
        .bind(session_id)
        .bind(cliente_id)
        .bind(telefono)
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO atencion_sesiones (session_id, estado, modo) \
             VALUES ($1, 'activa', 'completo') \
             ON CONFLICT (session_id) DO NOTHING",
        )
        .bind(session_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Entrada única: normaliza → upsert → vincula. Sin duplicados por
    /// teléfono aunque la IA llame dos veces (upserts idempotentes).
    pub async fn registrar_y_vincular(
        pool: &PgPool,
        session_id: Uuid,
        nombre: Option<&str>,
        telefono: &str,
    ) -> Result<ClienteRow, sqlx::Error> {
        let normalizado = Self::normalizar_telefono(telefono);
        let cliente = Self::registrar(pool, nombre, &normalizado).await?;
        Self::vincular_sesion(pool, session_id, cliente.id, &normalizado).await?;
        Ok(cliente)
    }
}

/* Las consultas SQL no usan macros verificadas en compilación: estos tests
 * las ejecutan contra la BD real de rama (`DATABASE_URL`). Sin
 * `DATABASE_URL` se omiten (gate local sin BD sigue verde). */
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

    #[test]
    fn normalizar_pasa_0412_a_58() {
        assert_eq!(
            ClienteRepository::normalizar_telefono("0412 0825234"),
            "584120825234"
        );
        assert_eq!(
            ClienteRepository::normalizar_telefono("+34611111111"),
            "34611111111"
        );
        assert_eq!(
            ClienteRepository::normalizar_telefono("600123456"),
            "600123456"
        );
    }

    #[tokio::test]
    async fn registrar_no_duplica_por_telefono() {
        let Some(pool) = pool_si_hay() else { return };
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();

        let primero = ClienteRepository::registrar_y_vincular(
            &pool,
            sesion,
            Some("Humo Uno"),
            "+34933333333",
        )
        .await
        .unwrap();
        assert_eq!(primero.telefono, "34933333333");
        let segundo =
            ClienteRepository::registrar_y_vincular(&pool, sesion, Some("Humo Dos"), "34933333333")
                .await
                .unwrap();
        assert_eq!(primero.id, segundo.id);
        assert_eq!(segundo.nombre.as_deref(), Some("Humo Dos"));
        let canal: String =
            sqlx::query_scalar("SELECT canal FROM canal_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(canal, "web");
        let estado: String =
            sqlx::query_scalar("SELECT estado FROM atencion_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(estado, "activa");

        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM clientes WHERE id = $1")
            .bind(primero.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }
}
