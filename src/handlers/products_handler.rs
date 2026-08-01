use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AdminUser;
use crate::models::product::{
    CheckoutRequest, CreateProductRequest, Order, Product, UpdateProductRequest,
};
use crate::services::product_svc::ProductService;
use crate::AppState;

/// Crear producto (admin) — nace inactivo/private por defecto
pub async fn create_product(
    State(state): State<AppState>,
    _auth: AdminUser,
    Json(req): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<Product>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let product = ProductService::create(&state.pool, req).await?;
    Ok((StatusCode::CREATED, Json(product)))
}

/// Obtener producto por ID (admin)
pub async fn get_product(
    State(state): State<AppState>,
    _auth: AdminUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Product>, AppError> {
    let product = ProductService::get(&state.pool, id).await?;
    Ok(Json(product))
}

/// Listar todos los productos (admin)
pub async fn list_all_products(
    State(state): State<AppState>,
    _auth: AdminUser,
) -> Result<Json<Vec<Product>>, AppError> {
    let products = ProductService::list_all(&state.pool).await?;
    Ok(Json(products))
}

/// Listar productos de un articulo (publico — solo activos)
pub async fn list_products_by_article(
    State(state): State<AppState>,
    Path(article_id): Path<Uuid>,
) -> Result<Json<Vec<Product>>, AppError> {
    let products = ProductService::list_by_article(&state.pool, article_id).await?;
    Ok(Json(products))
}

/// Catálogo público de la Tienda.
pub async fn list_public_products(
    State(state): State<AppState>,
) -> Result<Json<Vec<Product>>, AppError> {
    Ok(Json(ProductService::list_public(&state.pool).await?))
}

/// Actualizar producto (admin) — sincroniza envelope en transacción
pub async fn update_product(
    State(state): State<AppState>,
    _auth: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<Product>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let product = ProductService::update(&state.pool, id, req).await?;
    Ok(Json(product))
}

/// Eliminar producto (admin) — soft delete del envelope
pub async fn delete_product(
    State(state): State<AppState>,
    _auth: AdminUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    ProductService::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_stripe_checkout(
    state: &AppState,
    stripe_key: &str,
    product: &Product,
    order: &Order,
    email: &str,
) -> Result<Json<serde_json::Value>, AppError> {
    let success_url = format!(
        "{}/checkout/success?session_id={{CHECKOUT_SESSION_ID}}&order={}",
        state.site_url, order.id
    );
    let cancel_url = format!("{}/checkout/cancel?order={}", state.site_url, order.id);
    let mut params = vec![
        ("mode", "payment".to_string()),
        ("customer_email", email.to_string()),
        ("success_url", success_url),
        ("cancel_url", cancel_url),
        ("line_items[0][quantity]", "1".to_string()),
        (
            "line_items[0][price_data][currency]",
            product.currency.to_lowercase(),
        ),
        (
            "line_items[0][price_data][unit_amount]",
            product.price_cents.to_string(),
        ),
        (
            "line_items[0][price_data][product_data][name]",
            product.name.clone(),
        ),
    ];
    if !product.description.is_empty() {
        params.push((
            "line_items[0][price_data][product_data][description]",
            product.description.clone(),
        ));
    }
    params.push(("metadata[order_id]", order.id.to_string()));
    params.push(("metadata[product_id]", product.id.to_string()));

    let stripe_idempotency = order
        .idempotency_key
        .clone()
        .unwrap_or_else(|| order.id.to_string());
    let response = reqwest::Client::new()
        .post("https://api.stripe.com/v1/checkout/sessions")
        .header("Authorization", format!("Bearer {stripe_key}"))
        .header("Idempotency-Key", stripe_idempotency)
        .form(&params)
        .send()
        .await
        .map_err(|error| AppError::Internal(format!("Error creando Stripe session: {error}")))?;
    let status = response.status();
    if !status.is_success() {
        let detail = response.text().await.map_err(|error| {
            AppError::Internal(format!("Error leyendo respuesta de Stripe: {error}"))
        })?;
        tracing::error!("Stripe API error {status}: {detail}");
        return Err(AppError::Internal(format!(
            "Error creando checkout: {status}"
        )));
    }

    let session: serde_json::Value = response.json().await.map_err(|error| {
        AppError::Internal(format!("Error parseando respuesta de Stripe: {error}"))
    })?;
    let checkout_url = session["url"]
        .as_str()
        .ok_or_else(|| AppError::Internal("Stripe no retorno URL de checkout".into()))?;
    let session_id = session["id"]
        .as_str()
        .ok_or_else(|| AppError::Internal("Stripe no retorno ID de sesion".into()))?;

    crate::repositories::product_repo::OrderRepository::update_stripe_session(
        &state.pool,
        order.id,
        session_id,
    )
    .await?;
    tracing::info!(
        "Stripe checkout creado: session={session_id}, order={}",
        order.id
    );

    Ok(Json(serde_json::json!({
        "checkout_url": checkout_url,
        "order_id": order.id,
        "session_id": session_id,
    })))
}

/// Iniciar checkout de Stripe.
/// [297A-7] Solo acepta productos activos. Modo demo deshabilitado.
/// [297A-14] `get_public` exige envelope active + public e `is_active` en SQL.
pub async fn checkout(
    State(state): State<AppState>,
    Path(product_id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<CheckoutRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let product = ProductService::get_public(&state.pool, product_id).await?;

    /* [297A-7] Modo demo deshabilitado — requiere Stripe configurado */
    let stripe_key = state
        .stripe_secret_key
        .as_ref()
        .ok_or_else(|| AppError::Internal("Pasarela de pago no configurada".into()))?;

    /* [297A-15] El header tiene precedencia; el body permite clientes que no
     * puedan añadir headers. Si falta, se genera una clave única para no
     * romper clientes legacy, pero los reintentos seguros deben reutilizarla. */
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or(req.idempotency_key)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let order = crate::repositories::product_repo::OrderRepository::create_with_idempotency(
        &state.pool,
        product.id,
        &req.email,
        &idempotency_key,
    )
    .await?;

    create_stripe_checkout(&state, stripe_key, &product, &order, &req.email).await
}

pub fn routes() -> Router<AppState> {
    Router::new()
        /* Públicos */
        .route(
            "/articles/:article_id/products",
            get(list_products_by_article),
        )
        .route("/products", get(list_public_products))
        .route("/products/:id/checkout", post(checkout))
        /* Admin — contrato canónico /admin/products */
        .route(
            "/admin/products",
            get(list_all_products).post(create_product),
        )
        .route(
            "/admin/products/:id",
            get(get_product).put(update_product).delete(delete_product),
        )
}
