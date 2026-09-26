/* sentinel-disable-file sqlx-query-sin-macro: payment handler usa runtime query para
 * UPDATE de estado tras webhook Stripe (tipo dinámico por contexto). */
/* [044A-38 Fase 3] Handlers de pagos: Stripe checkout, webhook, historial.
 * Webhook no requiere auth — se verifica con firma HMAC-SHA256. */

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CheckoutIntentResponse, CreateCheckoutIntentRequest, CreateNotification,
    InitiatePaymentRequest, PaymentIntentResponse, PaymentResponse, UserRole,
    NOTIF_CHAT_INVOICE_PAID, NOTIF_PAYMENT_RECEIVED,
};
use crate::repositories::{OrderRepository, PaymentRepository, UserRepository};
use crate::services::{
    is_checkout_bypass_email, AuditService, BillingStripeService, DomainStripeService,
    EmailService, HostingStripeService, PaymentService, VpsStripeService,
};
use crate::AppState;

/// Iniciar pago de una orden (crea `PaymentIntent` en Stripe)
#[utoipa::path(
    post,
    path = "/api/orders/{order_id}/pay",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    request_body = InitiatePaymentRequest,
    responses(
        (status = 200, description = "PaymentIntent creado", body = PaymentIntentResponse),
        (status = 400, description = "Datos inválidos", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "payments"
)]
pub async fn initiate_payment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
    Json(req): Json<InitiatePaymentRequest>,
) -> Result<Json<PaymentIntentResponse>, AppError> {
    auth.require_role(&[UserRole::Client, UserRole::Admin])?;

    /* [064A-65] Admin puede pagar cualquier orden (testing/soporte).
     * Para clientes, el servicio verifica ownership por client_id. */
    let caller_id = if auth.role == UserRole::Admin {
        None
    } else {
        Some(auth.user_id)
    };

    /* [064A-59] Obtener email del usuario para pre-llenar en Stripe (receipt_email).
     * Así no se le pide email de nuevo en el checkout.
     * [074A-24] Log si falla la consulta en vez de silenciar con .ok(). */
    let user_email = match UserRepository::find_by_id(&state.pool, auth.user_id).await {
        Ok(Some(u)) => Some(u.email),
        Ok(None) => None,
        Err(e) => {
            tracing::warn!("No se pudo obtener email del usuario para receipt_email: {e}");
            None
        }
    };

    if user_email.as_deref().is_some_and(is_checkout_bypass_email) {
        let result = PaymentService::initiate_bypassed_payment(
            &state.pool,
            order_id,
            caller_id,
            req.phase_number,
        )
        .await?;
        return Ok(Json(result));
    }

    let stripe_key = state
        .stripe_secret_key
        .as_ref()
        .ok_or_else(|| AppError::Internal("Stripe no está configurado".into()))?;

    let result = PaymentService::initiate_payment(
        &state.pool,
        &state.http_client,
        stripe_key,
        order_id,
        caller_id,
        req.phase_number,
        user_email.as_deref(),
    )
    .await?;

    Ok(Json(result))
}

/* [166A-2] Crear PaymentIntent de checkout directo (sin orden previa).
 * [20CA-1] Ya NO requiere autenticación: acepta email en el body para crear
 * el PaymentIntent con metadata. El usuario se crea DESPUÉS del pago exitoso
 * en el webhook (handle_checkout_payment_succeeded). Esto evita cuentas
 * huérfanas cuando el usuario abandona el checkout de Stripe. */
pub async fn create_checkout_intent(
    State(state): State<AppState>,
    Json(req): Json<CreateCheckoutIntentRequest>,
) -> Result<Json<CheckoutIntentResponse>, AppError> {
    let stripe_key = state
        .stripe_secret_key
        .as_ref()
        .ok_or_else(|| AppError::Internal("Stripe no está configurado".into()))?;

    /* [20CA-1] El email es obligatorio para checkout sin auth */
    let email = req
        .email
        .as_deref()
        .filter(|e| !e.trim().is_empty())
        .ok_or_else(|| AppError::BadRequest("Email es requerido para checkout".into()))?;

    /* Validar formato básico de email */
    if !email.contains('@') || email.len() < 5 {
        return Err(AppError::BadRequest("Email inválido".into()));
    }

    let result = PaymentService::create_checkout_intent(
        &state.pool,
        &state.http_client,
        stripe_key,
        email,
        &req.service_slug,
        &req.plan_slug,
        req.payment_mode,
    )
    .await?;

    Ok(Json(result))
}

/// Webhook de Stripe — sin autenticación, verificado por firma HMAC
#[utoipa::path(
    post,
    path = "/api/webhooks/stripe",
    responses(
        (status = 200, description = "Webhook procesado"),
        (status = 400, description = "Firma inválida", body = crate::errors::ErrorResponse),
    ),
    tag = "payments"
)]
#[allow(clippy::too_many_lines)]
/* sentinel-disable-next-line funcion-larga-rs: webhook central de Stripe que coordina orders, chat invoices, hosting y VPS en un único punto de deduplicación. */
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    let webhook_secret = state
        .stripe_webhook_secret
        .as_ref()
        .ok_or_else(|| AppError::Internal("Stripe webhook secret no configurado".into()))?;

    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing Stripe-Signature header".into()))?;

    PaymentService::verify_webhook_signature(&body, signature, webhook_secret)?;

    let event: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| AppError::BadRequest(format!("JSON inválido: {e}")))?;

    let event_type = event["type"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("Missing event type".into()))?;

    /* [064A-73] Deduplicación: si el event_id ya fue procesado, retornar 200 sin
     * reprocesar. Stripe reenvía webhooks si no obtiene 200, sin esto un pago
     * podría acreditarse doble. */
    let event_id = event["id"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("Missing event id".into()))?;

    let already_processed: bool =
        PaymentRepository::is_event_processed(&state.pool, event_id).await;

    if already_processed {
        tracing::info!("Webhook duplicado ignorado: {event_id}");
        return Ok(StatusCode::OK);
    }

    PaymentService::handle_webhook(&state.pool, event_type, &event["data"], Some(&event)).await?;

    /* [104A-38] Notificar al cliente cuando su pago se procesa exitosamente.
     * Buscamos el payment por stripe_intent → order → client_id.
     * [166A-1] Re-leemos la orden DESPUÉS de handle_webhook para tener el status actualizado. */
    if event_type == "payment_intent.succeeded" {
        if let Some(pi_id) = event["data"]["object"]["id"].as_str() {
            if let Ok(Some(payment)) =
                PaymentRepository::find_by_stripe_intent(&state.pool, pi_id).await
            {
                /* Re-leer orden con status post-webhook (handle_payment_success actualiza status) */
                if let Ok(Some(order)) =
                    OrderRepository::find_order_by_id(&state.pool, payment.order_id).await
                {
                    let amount_display = format!("${:.2}", f64::from(payment.amount_cents) / 100.0);
                    let _ = state
                        .notification_hub
                        .notify(CreateNotification {
                            user_id: order.client_id,
                            notification_type: NOTIF_PAYMENT_RECEIVED.to_string(),
                            title: format!("Pago recibido — Orden #{}", order.order_number),
                            body: Some(format!(
                                "Tu pago de {amount_display} fue procesado exitosamente"
                            )),
                            link: Some(format!("/panel/orders/{}", order.id)),
                            reference_type: Some("order".to_string()),
                            reference_id: Some(order.id),
                        })
                        .await;

                    /* [311A-1] Email a admins notificando pago recibido */
                    if let Some(ref email_cfg) = state.email_config {
                        if let Ok(admin_emails) = UserRepository::admin_emails(&state.pool).await {
                            if !admin_emails.is_empty() {
                                let site_url = std::env::var("SITE_URL")
                                    .unwrap_or_else(|_| "https://nakomi.studio".to_string());
                                let client_email =
                                    UserRepository::get_email(&state.pool, order.client_id)
                                        .await
                                        .ok()
                                        .flatten()
                                        .unwrap_or_else(|| "desconocido".to_string());
                                let client_name =
                                    UserRepository::get_display_name(&state.pool, order.client_id)
                                        .await
                                        .ok()
                                        .flatten()
                                        .unwrap_or_else(|| "Cliente".to_string());
                                EmailService::send_payment_received_admin(
                                    email_cfg,
                                    &state.pool,
                                    &admin_emails,
                                    &client_email,
                                    &client_name,
                                    order.order_number,
                                    &amount_display,
                                    order.id,
                                    &site_url,
                                )
                                .await;
                            }
                        }
                    }

                    /* [166A-1] Post-pago: auto-asignar, crear chat y enviar emails.
                     * handle_payment_success movió la orden a awaiting_assignment (Full/HalfHalf 1er pago)
                     * o la dejó en payment_held (si no cambió status). En ambos casos, necesita activación.
                     * Para HalfHalf 2do pago o Phased subsiguiente, la orden ya está in_progress → skip. */
                    let needs_activation = order.status
                        == crate::models::OrderStatus::AwaitingAssignment
                        || order.status == crate::models::OrderStatus::PaymentHeld;

                    if needs_activation {
                        /* Obtener nombres de servicio y plan para emails/chat */
                        let (svc_title, _svc_slug, plan_name) =
                            OrderRepository::get_order_display_info(
                                &state.pool,
                                order.service_id,
                                order.plan_id,
                            )
                            .await
                            .unwrap_or_else(|_| ("Servicio".into(), String::new(), "Plan".into()));

                        let admins = UserRepository::admin_ids(&state.pool)
                            .await
                            .unwrap_or_default();
                        if let Some(&admin_id) = admins.first() {
                            match OrderRepository::assign_order(&state.pool, order.id, admin_id)
                                .await
                            {
                                Ok(_) => {
                                    let _ = crate::repositories::ActivityLogRepository::log(
                                        &state.pool,
                                        admin_id,
                                        "order_assigned",
                                        "order",
                                        order.id,
                                        Some(
                                            serde_json::json!({"auto": true, "post_payment": true}),
                                        ),
                                    )
                                    .await;

                                    /* Notificar admins de nueva orden */
                                    let notif_base = CreateNotification {
                                        user_id: Uuid::nil(),
                                        notification_type: crate::models::NOTIF_NEW_ORDER
                                            .to_string(),
                                        title: format!("Nueva orden #{}", order.order_number),
                                        body: Some(format!(
                                            "{} — Pago confirmado",
                                            order.order_number
                                        )),
                                        link: Some(format!(
                                            "/panel?seccion=ordenes&id={}",
                                            order.id
                                        )),
                                        reference_type: Some("order".to_string()),
                                        reference_id: Some(order.id),
                                    };
                                    let _ = state
                                        .notification_hub
                                        .notify_many(&admins, &notif_base)
                                        .await;
                                }
                                Err(e) => tracing::error!(
                                    "[166A-1] Error auto-asignando orden {} post-pago: {e}",
                                    order.id
                                ),
                            }
                        }

                        /* Crear chat session + mensaje de bienvenida */
                        let chat_hub = &state.chat_hub;
                        match chat_hub
                            .get_or_create_order_session(order.id, order.client_id)
                            .await
                        {
                            Ok(session) => {
                                let greeting = format!(
                                    "¡Felicidades! Tu pedido #{} ha sido recibido. \
                                     Será atendido dentro de las próximas 48 horas por nuestro equipo. \
                                     Puedes usar este chat para cualquier duda sobre tu pedido.",
                                    order.order_number,
                                );
                                let _ = chat_hub
                                    .send_message(session.id, "system", None, &greeting)
                                    .await;
                            }
                            Err(e) => tracing::error!(
                                "[166A-1] Error creando chat session para orden {}: {e}",
                                order.id
                            ),
                        }

                        /* Email de confirmación al cliente */
                        if let Some(ref email_cfg) = state.email_config {
                            let client_email_opt =
                                UserRepository::get_email(&state.pool, order.client_id)
                                    .await
                                    .ok()
                                    .flatten();
                            let client_name_opt =
                                UserRepository::get_display_name(&state.pool, order.client_id)
                                    .await
                                    .ok()
                                    .flatten();

                            if let Some(email) = client_email_opt {
                                let cfg = email_cfg.clone();
                                let pool = state.pool.clone();
                                let name = client_name_opt.unwrap_or_else(|| "Cliente".to_string());
                                let svc = svc_title.clone();
                                let plan = plan_name.clone();
                                let price = crate::services::format_price_cents(
                                    order.final_price_cents,
                                    &order.currency,
                                );
                                let order_num = order.order_number;
                                tokio::spawn(async move {
                                    EmailService::send_order_confirmation(
                                        &cfg, &pool, &email, &name, order_num, &svc, &plan, &price,
                                    )
                                    .await;
                                });
                            }
                        }

                        /* Email a admins notificando nueva orden */
                        if let Some(ref email_cfg) = state.email_config {
                            let admin_emails = UserRepository::admin_emails(&state.pool)
                                .await
                                .unwrap_or_default();
                            if !admin_emails.is_empty() {
                                let cfg = email_cfg.clone();
                                let pool = state.pool.clone();
                                let client_email_admin =
                                    UserRepository::get_email(&state.pool, order.client_id)
                                        .await
                                        .ok()
                                        .flatten()
                                        .unwrap_or_else(|| "desconocido".to_string());
                                let client_name_admin =
                                    UserRepository::get_display_name(&state.pool, order.client_id)
                                        .await
                                        .ok()
                                        .flatten()
                                        .unwrap_or_else(|| "Cliente".to_string());
                                let svc = svc_title;
                                let plan = plan_name;
                                let price = crate::services::format_price_cents(
                                    order.final_price_cents,
                                    &order.currency,
                                );
                                let pmode = format!("{:?}", order.payment_mode);
                                let oid = order.id;
                                let onum = order.order_number;
                                let site_url = std::env::var("SITE_URL")
                                    .unwrap_or_else(|_| "https://nakomi.studio".to_string());
                                tokio::spawn(async move {
                                    EmailService::send_new_order_admin(
                                        &cfg,
                                        &pool,
                                        &admin_emails,
                                        &client_email_admin,
                                        &client_name_admin,
                                        onum,
                                        &svc,
                                        &plan,
                                        &price,
                                        &pmode,
                                        oid,
                                        &site_url,
                                    )
                                    .await;
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    /* [124A-INV] Detectar cuando una factura generada por IA en el chat fue pagada.
     * La factura tiene metadata: source=chat_invoice, session_id, client_email.
     * Al confirmar el pago: notificar admins via hub + email, y enviar email al
     * cliente invitándolo a registrarse con el email de pago para acceder al panel. */
    if event_type == "invoice.paid" {
        let meta = &event["data"]["object"]["metadata"];
        if meta["source"].as_str() == Some("chat_invoice") {
            let client_email = meta["client_email"]
                .as_str()
                .or_else(|| event["data"]["object"]["customer_email"].as_str())
                .unwrap_or_default()
                .to_string();
            let session_id_str = meta["session_id"].as_str().unwrap_or_default();
            let session_id = session_id_str.parse::<Uuid>().unwrap_or(Uuid::nil());
            let amount_cents = event["data"]["object"]["amount_paid"].as_i64().unwrap_or(0);
            #[allow(clippy::cast_precision_loss)]
            let amount_usd = amount_cents as f64 / 100.0;
            let site_url =
                std::env::var("SITE_URL").unwrap_or_else(|_| "https://nakomi.studio".to_string());

            /* Notificar admins via notification hub */
            if let Ok(admin_ids) = UserRepository::admin_ids(&state.pool).await {
                if !admin_ids.is_empty() {
                    let _ = state
                        .notification_hub
                        .notify_many(
                            &admin_ids,
                            &CreateNotification {
                                user_id: Uuid::nil(),
                                notification_type: NOTIF_CHAT_INVOICE_PAID.to_string(),
                                title: format!(
                                    "Factura pagada: ${amount_usd:.2} — {client_email}"
                                ),
                                body: Some(
                                    "Una factura del chatbot fue pagada. El cliente puede registrarse."
                                        .to_string(),
                                ),
                                link: Some(format!("/panel/chat?session={session_id}")),
                                reference_type: None,
                                reference_id: None,
                            },
                        )
                        .await;
                }
            }

            /* Email a admins y al cliente (no fatal si falla) */
            if let Some(cfg) = &state.email_config {
                if let Ok(admin_emails) = UserRepository::admin_emails(&state.pool).await {
                    EmailService::send_chat_invoice_paid_admin(
                        cfg,
                        &state.pool,
                        &admin_emails,
                        &client_email,
                        amount_usd,
                        session_id,
                        &site_url,
                    )
                    .await;
                }
                if !client_email.is_empty() {
                    /* Encoding mínimo para email en query param (@→%40) */
                    let encoded_email = client_email.replace('@', "%40").replace('+', "%2B");
                    let register_url = format!("{site_url}/registro?email={encoded_email}");
                    EmailService::send_chat_invoice_paid_client(
                        cfg,
                        &state.pool,
                        &client_email,
                        amount_usd,
                        &site_url,
                        &register_url,
                    )
                    .await;
                }
            }

            tracing::info!(
                "Factura chat pagada: {client_email} ${amount_usd:.2} sesión {session_id}"
            );
        }
    }

    /* [084A-24] Hosting subscriptions: procesar eventos de checkout.session.completed,
     * invoice.paid, invoice.payment_failed, customer.subscription.deleted.
     * [104A-42] Se pasa http_client y coolify_config para provisioning automático.
     * HostingStripeService retorna bool indicando si procesó el evento. */
    HostingStripeService::handle_webhook(
        &state.pool,
        &state.http_client,
        state.coolify_config.as_ref(),
        event_type,
        &event["data"],
    )
    .await?;

    VpsStripeService::handle_webhook(
        &state.pool,
        &state.notification_hub,
        state.email_config.as_ref(),
        event_type,
        &event["data"],
    )
    .await?;

    DomainStripeService::handle_webhook(&state.pool, event_type, &event["data"]).await?;

    BillingStripeService::handle_webhook(&state.pool, event_type, &event["data"]).await?;

    /* [064A-73] Audit: webhook procesado exitosamente */
    AuditService::log(
        &state.pool,
        "stripe_webhook",
        None,
        None,
        serde_json::json!({"event_id": event_id, "event_type": event_type}),
    )
    .await;

    /* Registrar evento como procesado */
    PaymentRepository::mark_event_processed(&state.pool, event_id, event_type)
        .await
        .map_err(AppError::Database)?;

    Ok(StatusCode::OK)
}

/// Historial de pagos de una orden
#[utoipa::path(
    get,
    path = "/api/orders/{order_id}/payments",
    params(("order_id" = Uuid, Path, description = "ID de la orden")),
    responses(
        (status = 200, description = "Historial de pagos", body = Vec<PaymentResponse>),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 404, description = "Orden no encontrada", body = crate::errors::ErrorResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "payments"
)]
pub async fn list_payments(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(order_id): Path<Uuid>,
) -> Result<Json<Vec<PaymentResponse>>, AppError> {
    /* Verificar acceso: dueño, asignado o admin */
    let order = OrderRepository::find_order_by_id(&state.pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

    /* [074A-50] Admin real siempre tiene acceso, effective_role solo afecta UI */
    if auth.role != UserRole::Admin {
        match auth.effective_role {
            UserRole::Admin => {}
            UserRole::Client => {
                if order.client_id != auth.user_id {
                    return Err(AppError::Forbidden("No tienes acceso a esta orden".into()));
                }
            }
            UserRole::Employee => {
                if order.assigned_employee_id != Some(auth.user_id) {
                    return Err(AppError::Forbidden("No tienes acceso a esta orden".into()));
                }
            }
        }
    }

    let payments = PaymentService::list_payments(&state.pool, order_id).await?;
    Ok(Json(payments))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orders/:order_id/pay", post(initiate_payment))
        .route("/orders/:order_id/payments", get(list_payments))
        .route("/webhooks/stripe", post(stripe_webhook))
        .route("/checkout/intent", post(create_checkout_intent))
}
