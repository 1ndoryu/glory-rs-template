/* [154A-15a] Handlers de wallet: saldo virtual de usuarios.
 * GET /api/wallet â€” obtener saldo actual
 * GET /api/wallet/transactions â€” historial paginado
 * Incluye endpoints de cancellation requests y withdrawal requests.
 * [184A-1] POST /api/wallet/withdraw â€” solicitar retiro de fondos.
 * GET /api/wallet/withdrawals â€” listar solicitudes del usuario.
 * PATCH /api/admin/withdrawals/:id â€” admin aprueba/rechaza. */

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CreateNotification, CreateWithdrawalRequest, ResolveWithdrawalRequest, UserRole,
    WithdrawalRequestResponse, WithdrawalRequestsPage,
};
use crate::repositories::{UserRepository, WalletRepository, WithdrawalRequestRepository};
use crate::services::WalletService;
use crate::AppState;

/* ============================================================
GET /api/wallet â€” Saldo actual del usuario
============================================================ */

#[utoipa::path(
    get,
    path = "/api/wallet",
    responses(
        (status = 200, description = "Saldo actual", body = WalletResponse),
    ),
    security(("bearer" = []))
)]
pub async fn get_balance(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let wallet = WalletService::get_balance(&state.pool, auth.user_id).await?;
    Ok(Json(wallet))
}

/* ============================================================
GET /api/wallet/transactions â€” Historial paginado
============================================================ */

#[derive(Debug, Deserialize)]
pub struct TransactionQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/wallet/transactions",
    params(
        ("page" = Option<i64>, Query, description = "PÃ¡gina (1-indexed)"),
        ("per_page" = Option<i64>, Query, description = "Registros por pÃ¡gina (max 100)"),
    ),
    responses(
        (status = 200, description = "Historial de transacciones", body = WalletTransactionsPage),
    ),
    security(("bearer" = []))
)]
pub async fn list_transactions(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<TransactionQuery>,
) -> Result<impl IntoResponse, AppError> {
    let page = WalletService::list_transactions(
        &state.pool,
        auth.user_id,
        q.page.unwrap_or(1),
        q.per_page.unwrap_or(20),
    )
    .await?;
    Ok(Json(page))
}

/* ============================================================
POST /api/wallet/withdraw — Solicitar retiro de fondos
[184A-1] El usuario solicita retirar saldo a un medio de pago externo.
Solo se permite una solicitud pendiente a la vez.
============================================================ */

#[utoipa::path(
    post,
    path = "/api/wallet/withdraw",
    request_body = CreateWithdrawalRequest,
    responses(
        (status = 201, description = "Solicitud creada", body = WithdrawalRequestResponse),
        (status = 400, description = "Monto invÃ¡lido, fondos insuficientes o ya hay solicitud pendiente"),
    ),
    security(("bearer" = []))
)]
pub async fn create_withdrawal(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateWithdrawalRequest>,
) -> Result<impl IntoResponse, AppError> {
    if body.amount_cents <= 0 {
        return Err(AppError::BadRequest("El monto debe ser mayor a 0".into()));
    }

    /* [204A-11] Verificar saldo disponible para retiro (crÃ©ditos con >7 dÃ­as).
     * Las ganancias recientes no son retirables hasta cumplir 7 dÃ­as. */
    let withdrawable =
        WalletRepository::get_withdrawable_balance(&state.pool, auth.user_id).await?;
    if withdrawable < body.amount_cents {
        return Err(AppError::BadRequest(
            format!(
                "Fondos disponibles para retiro insuficientes. Disponible: ${:.2} (las ganancias tardan 7 dÃ­as en ser retirables)",
                f64::from(withdrawable) / 100.0
            ),
        ));
    }

    /* Solo una solicitud pendiente a la vez */
    if WithdrawalRequestRepository::has_pending(&state.pool, auth.user_id).await? {
        return Err(AppError::BadRequest(
            "Ya tienes una solicitud de retiro pendiente".into(),
        ));
    }

    let req = WithdrawalRequestRepository::create(
        &state.pool,
        auth.user_id,
        body.amount_cents,
        body.payment_method.as_deref(),
        body.payment_details.as_deref(),
    )
    .await?;

    /* Notificar a admins */
    let amount_fmt = format!("${:.2}", f64::from(body.amount_cents) / 100.0);
    let admins = UserRepository::admin_ids(&state.pool)
        .await
        .unwrap_or_default();
    let base = CreateNotification {
        user_id: Uuid::nil(),
        notification_type: "withdrawal_requested".to_string(),
        title: "Solicitud de retiro".to_string(),
        body: Some(format!("Nueva solicitud de retiro por {amount_fmt}")),
        link: Some("/panel?seccion=retiros".to_string()),
        reference_type: Some("withdrawal".to_string()),
        reference_id: Some(req.id),
    };
    let _ = state.notification_hub.notify_many(&admins, &base).await;

    Ok((
        StatusCode::CREATED,
        Json(WithdrawalRequestResponse::from(&req)),
    ))
}

/* ============================================================
GET /api/wallet/withdrawals â€” Listar solicitudes del usuario
============================================================ */

#[utoipa::path(
    get,
    path = "/api/wallet/withdrawals",
    params(
        ("page" = Option<i64>, Query, description = "PÃ¡gina (1-indexed)"),
        ("per_page" = Option<i64>, Query, description = "Registros por pÃ¡gina"),
    ),
    responses(
        (status = 200, description = "Solicitudes paginadas", body = WithdrawalRequestsPage),
    ),
    security(("bearer" = []))
)]
pub async fn list_withdrawals(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<TransactionQuery>,
) -> Result<impl IntoResponse, AppError> {
    let per_page = q.per_page.unwrap_or(20).clamp(1, 100);
    let page = q.page.unwrap_or(1).max(1);

    let (rows, total) =
        WithdrawalRequestRepository::list_by_user(&state.pool, auth.user_id, page, per_page)
            .await?;

    let requests = rows.iter().map(WithdrawalRequestResponse::from).collect();
    Ok(Json(WithdrawalRequestsPage {
        requests,
        total,
        page,
        per_page,
    }))
}

/* ============================================================
GET /api/admin/withdrawals â€” Admin: listar pendientes
============================================================ */

#[utoipa::path(
    get,
    path = "/api/admin/withdrawals",
    params(
        ("page" = Option<i64>, Query, description = "PÃ¡gina"),
        ("per_page" = Option<i64>, Query, description = "Registros por pÃ¡gina"),
    ),
    responses(
        (status = 200, description = "Solicitudes pendientes", body = WithdrawalRequestsPage),
    ),
    security(("bearer" = []))
)]
pub async fn admin_list_withdrawals(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<TransactionQuery>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let per_page = q.per_page.unwrap_or(20).clamp(1, 100);
    let page = q.page.unwrap_or(1).max(1);

    let (rows, total) =
        WithdrawalRequestRepository::list_pending(&state.pool, page, per_page).await?;

    let requests = rows.iter().map(WithdrawalRequestResponse::from).collect();
    Ok(Json(WithdrawalRequestsPage {
        requests,
        total,
        page,
        per_page,
    }))
}

/* ============================================================
PATCH /api/admin/withdrawals/:id â€” Admin aprueba/rechaza
[184A-1] Al aprobar, debita del wallet del usuario.
============================================================ */

#[utoipa::path(
    patch,
    path = "/api/admin/withdrawals/{id}",
    request_body = ResolveWithdrawalRequest,
    responses(
        (status = 200, description = "Solicitud resuelta", body = WithdrawalRequestResponse),
        (status = 404, description = "Solicitud no encontrada"),
    ),
    params(("id" = Uuid, Path, description = "ID de la solicitud")),
    security(("bearer" = []))
)]
pub async fn admin_resolve_withdrawal(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ResolveWithdrawalRequest>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_role(&[UserRole::Admin])?;

    let req = WithdrawalRequestRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Solicitud no encontrada".into()))?;

    if req.status != "pending" {
        return Err(AppError::BadRequest("Ya fue resuelta".into()));
    }

    if body.approve {
        /* Debitar del wallet antes de aprobar */
        WalletService::debit(
            &state.pool,
            req.user_id,
            req.amount_cents,
            "withdrawal",
            Some("withdrawal"),
            Some(req.id),
            Some("Retiro aprobado por administrador"),
        )
        .await?;
    }

    let resolved = WithdrawalRequestRepository::resolve(
        &state.pool,
        id,
        auth.user_id,
        body.approve,
        body.admin_notes.as_deref(),
    )
    .await?;

    /* Notificar al usuario */
    let status_msg = if body.approve {
        "aprobada"
    } else {
        "rechazada"
    };
    let amount_fmt = format!("${:.2}", f64::from(req.amount_cents) / 100.0);
    let notif = CreateNotification {
        user_id: req.user_id,
        notification_type: "withdrawal_resolved".to_string(),
        title: format!("Retiro {status_msg}"),
        body: Some(format!(
            "Tu solicitud de retiro por {amount_fmt} fue {status_msg}"
        )),
        link: Some("/panel?seccion=wallet".to_string()),
        reference_type: Some("withdrawal".to_string()),
        reference_id: Some(id),
    };
    let _ = state.notification_hub.notify(notif).await;

    Ok(Json(WithdrawalRequestResponse::from(&resolved)))
}

/* ============================================================
ROUTES
============================================================ */

pub fn wallet_routes() -> Router<AppState> {
    Router::new()
        .route("/wallet", get(get_balance))
        .route("/wallet/transactions", get(list_transactions))
        .route("/wallet/withdraw", post(create_withdrawal))
        .route("/wallet/withdrawals", get(list_withdrawals))
}

pub fn withdrawal_admin_routes() -> Router<AppState> {
    Router::new()
        .route("/admin/withdrawals", get(admin_list_withdrawals))
        .route("/admin/withdrawals/:id", patch(admin_resolve_withdrawal))
}
