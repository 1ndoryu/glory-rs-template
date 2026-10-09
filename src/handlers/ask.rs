use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{validar_extras, FichaAskRequest, FichaAskResponse};
use crate::repositories::InmuebleRepository;
use crate::AppState;

/* [279A-3] Ficha /ask: la dueña completa huecos por inmueble (`extras`) +
 * precio mínimo privado. Requiere JWT; lo privado solo sale por aquí:
 * `Inmueble` (vistas públicas y tools) ni declara `precio_minimo`. */

/// Leer la ficha /ask de un inmueble (incluye lo privado)
#[utoipa::path(
    get,
    path = "/api/admin/inmuebles/{id}/ficha",
    params(("id" = Uuid, Path, description = "ID del inmueble")),
    responses(
        (status = 200, description = "Ficha /ask", body = FichaAskResponse),
        (status = 404, description = "No encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_ficha(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<FichaAskResponse>, AppError> {
    let (extras, precio_minimo) = InmuebleRepository::get_ficha(&state.pool, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
    Ok(Json(FichaAskResponse {
        inmueble_id: id,
        extras: extras.0,
        precio_minimo,
    }))
}

/// Guardar la ficha /ask de un inmueble
#[utoipa::path(
    put,
    path = "/api/admin/inmuebles/{id}/ficha",
    params(("id" = Uuid, Path, description = "ID del inmueble")),
    request_body = FichaAskRequest,
    responses(
        (status = 200, description = "Ficha guardada", body = FichaAskResponse),
        (status = 404, description = "No encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn set_ficha(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<FichaAskRequest>,
) -> Result<Json<FichaAskResponse>, AppError> {
    validar_extras(&req.extras).map_err(AppError::Validation)?;
    if req.precio_minimo.is_some_and(|p| p < 0.0 || !p.is_finite()) {
        return Err(AppError::Validation(
            "precio_minimo debe ser un número >= 0".to_string(),
        ));
    }
    let fila = InmuebleRepository::set_ficha(
        &state.pool,
        id,
        sqlx::types::Json(req.extras),
        req.precio_minimo,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
    .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
    Ok(Json(FichaAskResponse {
        inmueble_id: fila.id,
        extras: fila.extras.0,
        precio_minimo: fila.precio_minimo,
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/inmuebles/:id/ficha", get(get_ficha).put(set_ficha))
}

/* [279A-3] Sin `DATABASE_URL` solo corren las puras (`validar_extras`):
 * con BD se verifica el roundtrip y que lo público nunca trae
 * `precio_minimo` aunque esté guardado. */
#[cfg(test)]
mod pruebas {
    use super::validar_extras;
    use sqlx::PgPool;

    fn pool_si_hay() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    #[test]
    fn extras_acepta_objeto_plano_y_rechaza_raro() {
        assert!(validar_extras(&serde_json::json!({"piso": 3})).is_ok());
        assert!(validar_extras(&serde_json::json!({})).is_ok());
        assert!(validar_extras(&serde_json::json!({"a": "x", "b": true, "c": null})).is_ok());
        assert!(validar_extras(&serde_json::json!([1, 2])).is_err());
        assert!(validar_extras(&serde_json::json!({"a": {"b": 1}})).is_err());
        assert!(validar_extras(&serde_json::json!({"a": [1]})).is_err());
        assert!(validar_extras(&serde_json::json!({"mala-clave": 1})).is_err());
        assert!(validar_extras(&serde_json::json!({"a": "x".repeat(501)})).is_err());
    }

    #[tokio::test]
    async fn ficha_roundtrip_y_publico_sin_minimo() {
        use crate::models::CreateInmuebleRequest;
        use crate::repositories::InmuebleRepository;
        use crate::services::InmuebleService;
        let Some(pool) = pool_si_hay() else { return };
        let creado = InmuebleService::create(
            &pool,
            CreateInmuebleRequest {
                titulo: "Ask humo".to_string(),
                descripcion: String::new(),
                ubicacion: String::new(),
                puestos: 0,
                residencia: String::new(),
                precio: 0.0,
                tipo: "apartamento".to_string(),
                operacion: "venta".to_string(),
                habitaciones: 0,
                banos: 0,
                metros: 0.0,
                metros_terreno: 0.0,
                estado: "disponible".to_string(),
                marketplace_id: None,
                copy: None,
            },
        )
        .await
        .unwrap();
        let extras = sqlx::types::Json(serde_json::json!({"piso": 3}));
        let fila = InmuebleRepository::set_ficha(&pool, creado.id, extras, Some(40000.0))
            .await
            .unwrap()
            .expect("la fila existe");
        assert_eq!(fila.precio_minimo, Some(40000.0));
        let (leidos, minimo) = InmuebleRepository::get_ficha(&pool, creado.id)
            .await
            .unwrap()
            .expect("la ficha existe");
        assert_eq!(leidos.0, serde_json::json!({"piso": 3}));
        assert_eq!(minimo, Some(40000.0));
        /* Frontera: la vista pública serializa sin `precio_minimo`. */
        let publico = serde_json::to_value(&creado).unwrap();
        assert!(publico.get("precio_minimo").is_none());
        assert_eq!(publico["extras"], serde_json::json!({}));
        InmuebleService::delete(&pool, std::path::Path::new("."), creado.id)
            .await
            .unwrap();
    }
}
