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
        Self::registrar_con_origen(pool, nombre, telefono, "web").await
    }

    /// Alta con origen explícito (`web|wa_a|wa_b`). En conflicto solo refresca
    /// el nombre (nunca pisa el `origen` primero: es de dónde vino primero).
    /// El teléfono ya viene normalizado por el llamador.
    pub async fn registrar_con_origen(
        pool: &PgPool,
        nombre: Option<&str>,
        telefono: &str,
        origen: &str,
    ) -> Result<ClienteRow, sqlx::Error> {
        let nombre_limpio = nombre.map(str::trim).filter(|n| !n.is_empty());
        sqlx::query_as::<_, ClienteRow>(
            "INSERT INTO clientes (id, nombre, telefono, origen) VALUES (gen_random_uuid(), $1, $2, $3) \
             ON CONFLICT (telefono) DO UPDATE SET \
               nombre = COALESCE(NULLIF(EXCLUDED.nombre, ''), clientes.nombre), \
               updated_at = NOW() \
             RETURNING id, nombre, telefono, origen, interes, presupuesto, zona, \
               notas, created_at, updated_at",
        )
        .bind(nombre_limpio)
        .bind(telefono)
        .bind(origen)
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

    /// Sesión ya abierta de un cliente en un canal (F2: un hilo por
    /// cliente×canal; web y `WhatsApp` no mezclan hilos, se enlazan por
    /// `clientes`). `None` = hay que crear sesión nueva.
    pub async fn buscar_sesion_por_cliente_canal(
        pool: &PgPool,
        cliente_id: Uuid,
        canal: &str,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT session_id FROM canal_sesiones WHERE cliente_id = $1 AND canal = $2 LIMIT 1",
        )
        .bind(cliente_id)
        .bind(canal)
        .fetch_optional(pool)
        .await
    }

    /* [279A-2 F2] Vinculación del webhook: fija `canal`/`modo` del reparto
     * (wa_a→completo, wa_b→inicial) y sincroniza el `modo` de la máquina
     * propia sin tocar su `estado` (la máquina F3 es la única que lo cambia:
     * una ráfaga de WhatsApp no debe reabrir un hilo delegado). */
    pub async fn vincular_canal(
        pool: &PgPool,
        session_id: Uuid,
        cliente_id: Uuid,
        telefono: &str,
        canal: &str,
        modo: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO canal_sesiones (session_id, cliente_id, canal, telefono, modo) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (session_id) DO UPDATE SET \
               cliente_id = EXCLUDED.cliente_id, telefono = EXCLUDED.telefono, \
               canal = EXCLUDED.canal, modo = EXCLUDED.modo",
        )
        .bind(session_id)
        .bind(cliente_id)
        .bind(canal)
        .bind(telefono)
        .bind(modo)
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO atencion_sesiones (session_id, estado, modo) \
             VALUES ($1, 'activa', $2) \
             ON CONFLICT (session_id) DO UPDATE SET modo = EXCLUDED.modo",
        )
        .bind(session_id)
        .bind(modo)
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

    /// Fija el estado de atención (`activa|consultando|delegada`); el `modo`
    /// se hereda del canal (F2) o nace `completo`. Un estado inválido lo
    /// rechaza el CHECK (error explícito, nunca silencio).
    pub async fn marcar_atencion(
        pool: &PgPool,
        session_id: Uuid,
        estado: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO atencion_sesiones (session_id, estado, modo) \
             VALUES ($1, $2, COALESCE((SELECT modo FROM canal_sesiones WHERE session_id = $1), 'completo')) \
             ON CONFLICT (session_id) DO UPDATE SET estado = EXCLUDED.estado, updated_at = NOW()",
        )
        .bind(session_id)
        .bind(estado)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Ficha comercial para el aviso al humano (decisión usuaria 2026-09-27):
    /// nombre+teléfono+resumen+interés+presupuesto+zona. El resumen lo pone
    /// la tool que avisa; el resto sale del cliente vinculado (hoy NULL
    /// hasta F4) con fallback al contacto de la sesión.
    pub async fn ficha_para_aviso(
        pool: &PgPool,
        session_id: Uuid,
    ) -> Result<FichaAviso, sqlx::Error> {
        sqlx::query_as::<_, FichaAviso>(
            "SELECT COALESCE(c.nombre, s.visitor_name) AS nombre, \
               COALESCE(c.telefono, cs.telefono, s.contact) AS telefono, \
               c.interes, c.presupuesto, c.zona, \
               COALESCE(cs.modo, 'completo') AS modo \
             FROM agent_sessions s \
             LEFT JOIN canal_sesiones cs ON cs.session_id = s.id \
             LEFT JOIN clientes c ON c.id = cs.cliente_id \
             WHERE s.id = $1",
        )
        .bind(session_id)
        .fetch_one(pool)
        .await
    }

    /// Canal de la sesión (`wa_a|wa_b|web`, ...). Lo usa el worker de avisos
    /// para elegir la sesión Baileys de salida (`via`); `None` si la sesión
    /// no está vinculada (el worker cae a `wa_a`, nunca falla el aviso).
    /// [289A-1]
    pub async fn canal_de(pool: &PgPool, session_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT canal FROM canal_sesiones WHERE session_id = $1")
            .bind(session_id)
            .fetch_optional(pool)
            .await
    }

    /// Hilo `WhatsApp` de la sesión (`canal`, `telefono` de `canal_sesiones`).
    /// Lo usa Responder staff: si hay hilo, el mensaje humano también se
    /// encola al outbox `whatsapp` para que le llegue al cliente (antes solo
    /// quedaba en el panel). `None` = sesión web sin hilo. [289A-2]
    pub async fn hilo_whatsapp(
        pool: &PgPool,
        session_id: Uuid,
    ) -> Result<Option<(String, Option<String>)>, sqlx::Error> {
        sqlx::query_as("SELECT canal, telefono FROM canal_sesiones WHERE session_id = $1")
            .bind(session_id)
            .fetch_optional(pool)
            .await
    }
}

/// Ficha comercial de una sesión para avisar al humano (ver
/// `ficha_para_aviso`). Todo opcional salvo `modo`: sin cliente vinculado
/// llega lo que la sesión sepa (mejor aviso parcial que ninguno).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FichaAviso {
    pub nombre: Option<String>,
    pub telefono: Option<String>,
    pub interes: Option<String>,
    pub presupuesto: Option<String>,
    pub zona: Option<String>,
    pub modo: String,
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

    #[tokio::test]
    async fn atencion_y_ficha_siguen_a_la_sesion() {
        let Some(pool) = pool_si_hay() else { return };
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        glory_agent::persistence::set_session_contact(
            &pool,
            sesion,
            Some("Humo Ficha"),
            Some("+34955555555"),
        )
        .await
        .unwrap();

        ClienteRepository::marcar_atencion(&pool, sesion, "consultando")
            .await
            .unwrap();
        let ficha = ClienteRepository::ficha_para_aviso(&pool, sesion)
            .await
            .unwrap();
        assert_eq!(ficha.nombre.as_deref(), Some("Humo Ficha"));
        assert_eq!(ficha.telefono.as_deref(), Some("+34955555555"));
        assert_eq!(ficha.modo, "completo");
        ClienteRepository::marcar_atencion(&pool, sesion, "delegada")
            .await
            .unwrap();
        let estado: String =
            sqlx::query_scalar("SELECT estado FROM atencion_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(estado, "delegada");

        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
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
}
