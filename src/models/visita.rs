use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::{Validate, ValidationError};

/* [E15] Visitas que agenda la IA: el visitante pide ver un inmueble del
 * catálogo, la tool deja la fila en `pendiente` y el admin la confirma
 * (con `fecha`) o la cancela desde el panel. `cuando` guarda literal lo
 * que dijo el visitante; `fecha` solo existe cuando hay día cerrado. */

/// Estados del ciclo de la visita (CHECK en BD)
pub const ESTADOS_VISITA: &[&str] = &["pendiente", "confirmada", "cancelada"];

/// Fila de `visitas` tal cual la devuelve Postgres
#[derive(Debug, Clone, FromRow)]
pub struct VisitaRow {
    pub id: Uuid,
    pub inmueble_id: Uuid,
    pub session_id: Uuid,
    pub nombre: String,
    pub telefono: String,
    pub cuando: String,
    pub fecha: Option<NaiveDate>,
    pub estado: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fila del listado admin: la visita más el título del inmueble (JOIN)
#[derive(Debug, Clone, FromRow)]
pub struct VisitaAdmin {
    pub id: Uuid,
    pub inmueble_id: Uuid,
    pub session_id: Uuid,
    pub nombre: String,
    pub telefono: String,
    pub cuando: String,
    pub fecha: Option<NaiveDate>,
    pub estado: String,
    pub titulo_inmueble: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Visita tal como la expone la API admin
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Visita {
    pub id: Uuid,
    pub inmueble_id: Uuid,
    pub session_id: Uuid,
    pub nombre: String,
    pub telefono: String,
    pub cuando: String,
    pub fecha: Option<NaiveDate>,
    pub estado: String,
    pub titulo_inmueble: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Visita {
    /// Ensambla la vista del listado (ya trae el título por el JOIN)
    #[must_use]
    pub fn from_admin(row: VisitaAdmin) -> Self {
        Self {
            id: row.id,
            inmueble_id: row.inmueble_id,
            session_id: row.session_id,
            nombre: row.nombre,
            telefono: row.telefono,
            cuando: row.cuando,
            fecha: row.fecha,
            estado: row.estado,
            titulo_inmueble: row.titulo_inmueble,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }

    /// Ensambla la vista tras crear/confirmar (el título se busca aparte)
    #[must_use]
    pub fn from_row(row: VisitaRow, titulo_inmueble: String) -> Self {
        Self {
            id: row.id,
            inmueble_id: row.inmueble_id,
            session_id: row.session_id,
            nombre: row.nombre,
            telefono: row.telefono,
            cuando: row.cuando,
            fecha: row.fecha,
            estado: row.estado,
            titulo_inmueble,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn validar_estado_visita(estado: &str) -> Result<(), ValidationError> {
    if ESTADOS_VISITA.contains(&estado) {
        Ok(())
    } else {
        let mut err = ValidationError::new("estado_visita");
        err.message = Some("Estado de visita no válido".into());
        Err(err)
    }
}

/// Cambio de estado por el admin (confirmar exige `fecha`)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateEstadoVisita {
    #[validate(custom(function = "validar_estado_visita"))]
    pub estado: String,
    /// Día cerrado `YYYY-MM-DD` (obligatorio al confirmar)
    pub fecha: Option<NaiveDate>,
}

/// Página de visitas para el panel admin
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedVisitas {
    pub items: Vec<Visita>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/// Query del listado admin: filtro opcional por estado + paginación
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct VisitasAdminParams {
    /// Filtra por estado (`pendiente`, `confirmada`, `cancelada`)
    pub estado: Option<String>,
    #[serde(default = "pagina_por_defecto")]
    pub page: i64,
    #[serde(default = "por_pagina_por_defecto")]
    pub per_page: i64,
}

fn pagina_por_defecto() -> i64 {
    1
}

fn por_pagina_por_defecto() -> i64 {
    20
}

#[cfg(test)]
mod tests {
    use super::validar_estado_visita;

    #[test]
    fn estados_visita_solo_los_tres() {
        assert!(validar_estado_visita("pendiente").is_ok());
        assert!(validar_estado_visita("confirmada").is_ok());
        assert!(validar_estado_visita("cancelada").is_ok());
        assert!(validar_estado_visita("revisada").is_err());
    }
}
