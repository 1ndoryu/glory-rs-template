/* [044A-38 Fase 7] Modelos de reembolsos.
 * RefundStatus enum, OrderRefund struct, request/response types.
 * Tabla: order_refunds (ya existe en migración). */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "refund_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RefundStatus {
    Requested,
    UnderReview,
    Approved,
    Processing,
    Completed,
    Failed,
    Rejected,
}

#[derive(Debug, Clone, FromRow)]
pub struct OrderRefund {
    pub id: Uuid,
    pub order_id: Uuid,
    pub payment_id: Uuid,
    pub requested_by: Uuid,
    pub reviewed_by: Option<Uuid>,
    pub amount_cents: i32,
    pub reason: String,
    pub admin_response: Option<String>,
    pub status: RefundStatus,
    pub stripe_refund_id: Option<String>,
    /* [277A-7] Retry tracking para reembolsos fallidos en Stripe */
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub requested_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/* ============================================================
REQUESTS
============================================================ */

#[derive(Debug, Deserialize, ToSchema)]
pub struct RequestRefundBody {
    pub reason: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewRefundBody {
    pub action: ReviewAction,
    pub admin_response: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReviewAction {
    Approve,
    Reject,
}

/* ============================================================
RESPONSES
============================================================ */

#[derive(Debug, Serialize, ToSchema)]
pub struct RefundResponse {
    pub id: Uuid,
    pub order_id: Uuid,
    pub amount_cents: i32,
    pub reason: String,
    pub admin_response: Option<String>,
    pub status: RefundStatus,
    pub requested_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<OrderRefund> for RefundResponse {
    fn from(r: OrderRefund) -> Self {
        Self {
            id: r.id,
            order_id: r.order_id,
            amount_cents: r.amount_cents,
            reason: r.reason,
            admin_response: r.admin_response,
            status: r.status,
            requested_at: r.requested_at,
            reviewed_at: r.reviewed_at,
            completed_at: r.completed_at,
        }
    }
}
