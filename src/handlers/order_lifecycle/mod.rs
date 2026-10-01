/* [174A-2] Handlers de lifecycle de ordenes.
 * POST /api/auth/switch-role
 * POST /api/orders/:id/cancel
 * POST /api/orders/:id/phases/:n/approve
 * POST /api/orders/:id/phases/:n/revision
 * PATCH /api/orders/:id/ai-intermediary
 * GET /api/orders/:id/activity
 *
 * [01AA-4-F2] Partido por dominio (era god-object 706 + limite 706):
 * cancel.rs (cancelación), approve.rs (aprobación/completado),
 * misc.rs (switch-role, revisiones, AI-intermediary, activity). */

mod approve;
mod cancel;
mod misc;

pub use approve::approve_phase;
pub use cancel::cancel_order_handler;
pub use misc::{
    get_order_activity, request_revision, switch_role, toggle_ai_intermediary, ActivityEntry,
};

use axum::routing::{get, post, put};
use axum::Router;

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orders/:order_id/cancel", post(cancel_order_handler))
        .route(
            "/orders/:order_id/phases/:phase_number/approve",
            put(approve_phase),
        )
        .route(
            "/orders/:order_id/phases/:phase_number/revision",
            put(request_revision),
        )
        .route(
            "/orders/:order_id/ai-intermediary",
            put(toggle_ai_intermediary),
        )
        .route("/orders/:order_id/activity", get(get_order_activity))
        .route("/auth/switch-role", post(switch_role))
}
