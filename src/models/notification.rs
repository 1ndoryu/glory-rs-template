use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Public/admin notification row. `read` is resolved per authenticated user.
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct Notification {
    pub id: Uuid,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub release_version: Option<i32>,
    pub status: String,
    pub created_by: Option<Uuid>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub read: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationList {
    pub items: Vec<Notification>,
    pub unread_count: i64,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateNotificationRequest {
    #[validate(length(min = 1, max = 40))]
    pub kind: String,
    #[validate(length(min = 1, max = 160))]
    pub title: String,
    #[validate(length(min = 1, max = 500))]
    pub body: String,
    pub release_version: Option<i32>,
    #[validate(length(min = 1, max = 16))]
    pub status: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateNotificationStatusRequest {
    #[validate(length(min = 1, max = 16))]
    pub status: String,
}
