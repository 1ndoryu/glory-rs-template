use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use validator::Validate;

/// Producto vendible. [297A-10] article_id ahora es opcional (independiente de artículo).
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Product {
    pub id: Uuid,
    /// [297A-10] Opcional: producto independiente de artículo.
    pub article_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub price_cents: i32,
    pub currency: String,
    pub stripe_product_id: Option<String>,
    pub stripe_price_id: Option<String>,
    /// Legacy: download_path directo. Se mantiene hasta fase contract.
    pub download_path: Option<String>,
    /// Legacy: is_active. Se mantiene hasta fase contract.
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateProductRequest {
    pub article_id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub price_cents: i32,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub download_path: Option<String>,
}

fn default_currency() -> String {
    "USD".to_string()
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price_cents: Option<i32>,
    pub is_active: Option<bool>,
    pub download_path: Option<String>,
}

/// Orden de compra
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Order {
    pub id: Uuid,
    pub product_id: Uuid,
    pub stripe_session_id: Option<String>,
    pub stripe_payment_intent: Option<String>,
    pub customer_email: String,
    pub status: String,
    pub paid_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOrderRequest {
    pub product_id: Uuid,
    pub customer_email: String,
}

/// Request de checkout
#[derive(Debug, Deserialize, ToSchema)]
pub struct CheckoutRequest {
    pub email: String,
}
