use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{VisitaAdmin, VisitaRow};

/* [E15] Persistencia de visitas de la IA: crear en `pendiente`, listar
 * para el panel (con título por JOIN, sin N+1) y mover el estado. Las
 * validaciones de negocio (inmueble publicado, fecha al confirmar)
 * viven en la tool y el handler (boundary), no aquí. */

/// Datos para abrir una visita `pendiente`
pub struct NuevaVisita {
    pub inmueble_id: Uuid,
    pub session_id: Uuid,
    pub nombre: String,
    pub telefono: String,
    pub cuando: String,
    pub fecha: Option<NaiveDate>,
}

const COLUMNAS: &str = "id, inmueble_id, session_id, nombre, telefono, cuando, \
                        fecha, estado, created_at, updated_at";

pub struct VisitaRepository;

impl VisitaRepository {
    pub async fn crear(pool: &PgPool, nueva: NuevaVisita) -> Result<VisitaRow, sqlx::Error> {
        sqlx::query_as::<_, VisitaRow>(&format!(
            "INSERT INTO visitas (inmueble_id, session_id, nombre, telefono, cuando, fecha) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING {COLUMNAS}",
        ))
        .bind(nueva.inmueble_id)
        .bind(nueva.session_id)
        .bind(nueva.nombre)
        .bind(nueva.telefono)
        .bind(nueva.cuando)
        .bind(nueva.fecha)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<VisitaRow>, sqlx::Error> {
        sqlx::query_as::<_, VisitaRow>(&format!("SELECT {COLUMNAS} FROM visitas WHERE id = $1"))
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Título del inmueble para completar la vista (una sola fila)
    pub async fn titulo_de(
        pool: &PgPool,
        inmueble_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT titulo FROM inmuebles WHERE id = $1")
            .bind(inmueble_id)
            .fetch_optional(pool)
            .await
    }

    /// Listado del panel: filtro opcional por estado, paginado, con título
    pub async fn list_admin(
        pool: &PgPool,
        estado: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<VisitaAdmin>, i64), sqlx::Error> {
        let offset = (page - 1) * per_page;
        let base = "SELECT v.id, v.inmueble_id, v.session_id, v.nombre, v.telefono, \
                    v.cuando, v.fecha, v.estado, i.titulo AS titulo_inmueble, \
                    v.created_at, v.updated_at \
                    FROM visitas v JOIN inmuebles i ON i.id = v.inmueble_id \
                    WHERE ($1::TEXT IS NULL OR v.estado = $1)";
        let rows = sqlx::query_as::<_, VisitaAdmin>(&format!(
            "{base} ORDER BY v.created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(estado)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let (total,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM visitas WHERE ($1::TEXT IS NULL OR estado = $1)")
                .bind(estado)
                .fetch_one(pool)
                .await?;

        Ok((rows, total))
    }

    /// Mueve el estado; devuelve la fila o `None` si no existe
    pub async fn cambiar_estado(
        pool: &PgPool,
        id: Uuid,
        estado: &str,
        fecha: Option<NaiveDate>,
    ) -> Result<Option<VisitaRow>, sqlx::Error> {
        sqlx::query_as::<_, VisitaRow>(&format!(
            "UPDATE visitas SET estado = $2, fecha = COALESCE($3, fecha), updated_at = NOW() \
             WHERE id = $1 RETURNING {COLUMNAS}",
        ))
        .bind(id)
        .bind(estado)
        .bind(fecha)
        .fetch_optional(pool)
        .await
    }
}
