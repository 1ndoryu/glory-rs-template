/* [20CA-11] Plantillas HTML centralizadas para emails.
 * Elimina duplicación entre email.rs (envío) y email_preview.rs (previews).
 * Cada función render_* genera el HTML completo usando helpers de layout.
 * email_preview.rs reutiliza estas funciones con datos SAMPLE.
 * [01AA-4-F3h] Partido por destinatario (era god-object 600+): layout =
 * helpers HTML; orders_client/orders_admin = plantillas de pedidos;
 * ops = escalación, facturas chat, VPS, perfil, registro y continuación. */

mod layout;
mod ops;
mod orders_admin;
mod orders_client;

pub use ops::{
    render_chat_client_message_admin, render_chat_continuation, render_chat_invoice_paid_admin,
    render_chat_invoice_paid_client, render_escalation, render_new_user_registered_admin,
    render_profile_email_changed_new, render_profile_email_changed_old,
    render_profile_password_changed, render_vps_approved, render_vps_pending_approval,
    render_vps_rejected,
};
pub use orders_admin::{
    render_new_order_admin, render_order_cancelled_admin, render_order_completed_admin,
    render_payment_received_admin, render_problem_reported_admin, render_refund_requested_admin,
};
pub use orders_client::{
    render_order_cancelled_client, render_order_completed_client, render_order_confirmation,
    render_phase_delivered_client, render_problem_reported_client,
};
