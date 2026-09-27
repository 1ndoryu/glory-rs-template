use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/* [279A-2 F1] Cliente de la inmobiliaria: una fila por teléfono (UNIQUE).
 * Lo alimenta `registrar_contacto` (tool IA) y `POST .../contacto` (widget);
 * el webhook F2 lo vinculará a `canal_sesiones` con canal wa_a/wa_b.
 * `interes`/`presupuesto`/`zona` los completa F4 (memoria); hoy quedan NULL. */

/// Fila de `clientes` tal cual la devuelve Postgres
#[derive(Debug, Clone, FromRow)]
pub struct ClienteRow {
    pub id: Uuid,
    pub nombre: Option<String>,
    pub telefono: String,
    pub origen: String,
    pub interes: Option<String>,
    pub presupuesto: Option<String>,
    pub zona: Option<String>,
    pub notas: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Cliente tal como lo expondrá la consola dueña (F5)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Cliente {
    pub id: Uuid,
    pub nombre: Option<String>,
    pub telefono: String,
    pub origen: String,
    pub interes: Option<String>,
    pub presupuesto: Option<String>,
    pub zona: Option<String>,
    pub notas: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Cliente {
    #[must_use]
    pub fn from_row(row: ClienteRow) -> Self {
        Self {
            id: row.id,
            nombre: row.nombre,
            telefono: row.telefono,
            origen: row.origen,
            interes: row.interes,
            presupuesto: row.presupuesto,
            zona: row.zona,
            notas: row.notas,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Alta de cliente — solo nombre+teléfono (el repositorio normaliza)
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateClienteRequest {
    pub nombre: Option<String>,
    pub telefono: String,
}
