use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::product::{CheckoutRequest, CreateProductRequest, Product};
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

/// Iniciar checkout de Stripe
/// Si hay STRIPE_SECRET_KEY configurado, crea una Checkout Session real.
/// Si no, modo demo: envia email de descarga directamente.
pub async fn checkout(
    State(state): State<AppState>,
    Path(product_id): Path<Uuid>,
    Json(req): Json<CheckoutRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let product = ProductService::get(&state.pool, product_id).await?;

    /* Crear orden pendiente */
    let order = crate::repositories::product_repo::OrderRepository::create(
        &state.pool,
        product.id,
        &req.email,
        None,
    )
    .await?;

    if let Some(ref stripe_key) = state.stripe_secret_key {
        /* === Stripe Checkout Session real === */
        let client = reqwest::Client::new();

        let success_url = format!(
            "{}/checkout/success?session_id={{CHECKOUT_SESSION_ID}}&order={}",
            state.site_url, order.id
        );
        let cancel_url = format!(
            "{}/checkout/cancel?order={}",
            state.site_url, order.id
        );

        /* Construir form data para Stripe API */
        let mut params = vec![
            ("mode", "payment".to_string()),
            ("customer_email", req.email.clone()),
            ("success_url", success_url),
            ("cancel_url", cancel_url),
            ("line_items[0][quantity]", "1".to_string()),
            ("line_items[0][price_data][currency]", product.currency.to_lowercase()),
            ("line_items[0][price_data][unit_amount]", product.price_cents.to_string()),
            ("line_items[0][price_data][product_data][name]", product.name.clone()),
        ];

        if !product.description.is_empty() {
            params.push(("line_items[0][price_data][product_data][description]", product.description.clone()));
        }

        /* Metadata para el webhook */
        params.push(("metadata[order_id]", order.id.to_string()));
        params.push(("metadata[product_id]", product.id.to_string()));

        let resp = client
            .post("https://api.stripe.com/v1/checkout/sessions")
            .header("Authorization", format!("Bearer {stripe_key}"))
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Error creando Stripe session: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            tracing::error!("Stripe API error {status}: {text}");
            return Err(AppError::Internal(format!(
                "Error creando checkout: {status}"
            )));
        }

        let session: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Error parseando respuesta de Stripe: {e}")))?;

        let checkout_url = session["url"]
            .as_str()
            .ok_or_else(|| AppError::Internal("Stripe no retorno URL de checkout".into()))?
            .to_string();

        let session_id = session["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        /* Actualizar orden con session_id */
        let _ = sqlx::query("UPDATE orders SET stripe_session_id = $1 WHERE id = $2")
            .bind(&session_id)
            .bind(order.id)
            .execute(&state.pool)
            .await;

        tracing::info!("Stripe checkout creado: session={session_id}, order={}", order.id);

        Ok(Json(serde_json::json!({
            "checkout_url": checkout_url,
            "order_id": order.id,
            "session_id": session_id,
        })))
    } else {
        /* === Modo demo: enviar email de descarga directamente === */
        if let Some(ref api_key) = state.resend_api_key {
            if let Some(ref download_path) = product.download_path {
                let download_url = format!(
                    "{}{}",
                    state.site_url,
                    download_path
                );
                let _ = EmailService::send_download_link(
                    api_key,
                    &state.email_from,
                    &req.email,
                    &product.name,
                    &download_url,
                )
                .await;

                /* Marcar orden como entregada */
                let _ = crate::repositories::product_repo::OrderRepository::mark_delivered(
                    &state.pool,
                    order.id,
                )
                .await;
            }
        }

        Ok(Json(serde_json::json!({
            "checkout_url": format!("{}/checkout/demo?order={}", state.site_url, order.id),
            "order_id": order.id,
            "message": "modo demo — si hay email configurado, recibiras tu descarga por correo."
        })))
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
