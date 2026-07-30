use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::product::{CheckoutRequest, CreateProductRequest, Order, Product};
use crate::services::email::EmailService;
use crate::services::product_svc::ProductService;
use crate::AppState;

/// Crear producto (admin)
pub async fn create_product(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<Product>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let product = ProductService::create(&state.pool, req).await?;
    Ok((StatusCode::CREATED, Json(product)))
}

/// Listar productos de un articulo (publico)
pub async fn list_products_by_article(
    State(state): State<AppState>,
    Path(article_id): Path<Uuid>,
) -> Result<Json<Vec<Product>>, AppError> {
    let products = ProductService::list_by_article(&state.pool, article_id).await?;
    Ok(Json(products))
}

/// Eliminar producto (admin)
pub async fn delete_product(
    State(state): State<AppState>,
    _auth: AuthUser,
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

    let response = reqwest::Client::new()
        .post("https://api.stripe.com/v1/checkout/sessions")
        .header("Authorization", format!("Bearer {stripe_key}"))
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

    sqlx::query("UPDATE orders SET stripe_session_id = $1 WHERE id = $2")
        .bind(session_id)
        .bind(order.id)
        .execute(&state.pool)
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

async fn create_demo_checkout(
    state: &AppState,
    product: &Product,
    order: &Order,
    email: &str,
) -> Result<Json<serde_json::Value>, AppError> {
    if let (Some(api_key), Some(download_path)) = (&state.resend_api_key, &product.download_path) {
        let download_url = format!("{}{download_path}", state.site_url);
        EmailService::send_download_link(
            api_key,
            &state.email_from,
            email,
            &product.name,
            &download_url,
        )
        .await?;
        crate::repositories::product_repo::OrderRepository::mark_delivered(&state.pool, order.id)
            .await?;
    }

    Ok(Json(serde_json::json!({
        "checkout_url": format!("{}/checkout/demo?order={}", state.site_url, order.id),
        "order_id": order.id,
        "message": "modo demo — si hay email configurado, recibiras tu descarga por correo."
    })))
}

/// Iniciar checkout de Stripe.
/// Si hay `STRIPE_SECRET_KEY`, crea una Checkout Session real; si no, usa modo demo.
pub async fn checkout(
    State(state): State<AppState>,
    Path(product_id): Path<Uuid>,
    Json(req): Json<CheckoutRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let product = ProductService::get(&state.pool, product_id).await?;
    let order = crate::repositories::product_repo::OrderRepository::create(
        &state.pool,
        product.id,
        &req.email,
        None,
    )
    .await?;

    if let Some(stripe_key) = &state.stripe_secret_key {
        create_stripe_checkout(&state, stripe_key, &product, &order, &req.email).await
    } else {
        create_demo_checkout(&state, &product, &order, &req.email).await
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/products", post(create_product))
        .route(
            "/articles/{article_id}/products",
            get(list_products_by_article),
        )
        .route("/products/{id}", axum::routing::delete(delete_product))
        .route("/products/{id}/checkout", post(checkout))
}
