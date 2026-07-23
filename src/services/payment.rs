/* [044A-38 Fase 3] Servicio de pagos: integración con Stripe REST API.
 * PaymentIntent con capture_method manual (escrow).
 * 3 modos: full (20% desc), half_half (10% desc), phased (sin desc).
 * Webhook verifica firma HMAC-SHA256 con constant-time comparison. */

use reqwest::Client;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    OrderPayment, OrderStatus, PaymentIntentResponse, PaymentMode, PaymentResponse, PaymentStatus,
    PhaseStatus,
};
use crate::repositories::{CreateOrderParams, CreatePaymentParams, CreatePhaseParams, OrderRepository,
    PaymentRepository, ServiceRepository, UserRepository};
use super::order_slugs::{find_plan_for_order, find_service_for_order};

pub struct PaymentService;

impl PaymentService {
    /// Inicia un pago: crea `PaymentIntent` en Stripe con capture manual, guarda en BD.
    /// [064A-65] `client_id` es None si el caller es admin (no aplica ownership check).
    /// [064A-59] `receipt_email` pre-llena el email en Stripe para no pedirlo en checkout.
    pub async fn initiate_payment(
        pool: &PgPool,
        http_client: &Client,
        stripe_key: &str,
        order_id: Uuid,
        client_id: Option<Uuid>,
        phase_number: Option<i32>,
        receipt_email: Option<&str>,
    ) -> Result<PaymentIntentResponse, AppError> {
        let order = OrderRepository::find_order_by_id(pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

        if let Some(cid) = client_id {
            if order.client_id != cid {
                return Err(AppError::Forbidden("No tienes acceso a esta orden".into()));
            }
        }

        let (amount_cents, phase_id, description) =
            Self::resolve_payment_amount(pool, &order, phase_number).await?;

        let stripe_resp = Self::create_stripe_intent(
            http_client,
            stripe_key,
            amount_cents,
            &order.currency,
            order_id,
            phase_number,
            receipt_email,
        )
        .await?;

        let payment = PaymentRepository::create_payment(
            pool,
            CreatePaymentParams {
                order_id,
                phase_id,
                amount_cents,
                currency: &order.currency,
                payment_mode: order.payment_mode,
                stripe_payment_intent_id: &stripe_resp.id,
                description: Some(&description),
            },
        )
        .await
        .map_err(|e| AppError::Internal(format!("Error guardando pago: {e}")))?;

        Ok(PaymentIntentResponse {
            payment_id: payment.id,
            client_secret: stripe_resp.client_secret,
            amount_cents,
            currency: order.currency,
            bypassed: false,
        })
    }

    /* [155A-11] Flujo de prueba sin Stripe para cuentas allowlisted.
     * Crea el pago en BD con ID sintetico, lo marca held y ejecuta la misma maquina
     * de estados que el webhook exitoso para que servicios/proyectos avancen igual. */
    pub async fn initiate_bypassed_payment(
        pool: &PgPool,
        order_id: Uuid,
        client_id: Option<Uuid>,
        phase_number: Option<i32>,
    ) -> Result<PaymentIntentResponse, AppError> {
        let order = OrderRepository::find_order_by_id(pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

        if let Some(cid) = client_id {
            if order.client_id != cid {
                return Err(AppError::Forbidden("No tienes acceso a esta orden".into()));
            }
        }

        let (amount_cents, phase_id, description) =
            Self::resolve_payment_amount(pool, &order, phase_number).await?;
        let synthetic_intent_id = format!("test_bypass_{}", Uuid::new_v4());
        let bypass_description = format!("{description} (checkout test sin cobro)");

        let payment = PaymentRepository::create_payment(
            pool,
            CreatePaymentParams {
                order_id,
                phase_id,
                amount_cents,
                currency: &order.currency,
                payment_mode: order.payment_mode,
                stripe_payment_intent_id: &synthetic_intent_id,
                description: Some(&bypass_description),
            },
        )
        .await
        .map_err(|e| AppError::Internal(format!("Error guardando pago de prueba: {e}")))?;

        let held_payment = PaymentRepository::update_status_held(pool, payment.id).await?;
        Self::handle_payment_success(pool, &held_payment).await?;

        Ok(PaymentIntentResponse {
            payment_id: held_payment.id,
            client_secret: String::new(),
            amount_cents,
            currency: order.currency,
            bypassed: true,
        })
    }

    /* [166A-2] Crea PaymentIntent de checkout directo SIN crear orden.
     * [20CA-1] Ya NO requiere usuario existente: acepta email directamente.
     * El email se guarda en metadata de Stripe para que el webhook pueda
     * crear/encontrar el usuario DESPUÉS del pago confirmado. */
    pub async fn create_checkout_intent(
        pool: &PgPool,
        http_client: &Client,
        stripe_key: &str,
        email: &str,
        service_slug: &str,
        plan_slug: &str,
        payment_mode: PaymentMode,
    ) -> Result<crate::models::CheckoutIntentResponse, AppError> {
        /* Resolver servicio y plan (misma lógica que create_order) */
        let svc = find_service_for_order(pool, service_slug).await?;
        let plan = find_plan_for_order(pool, svc.id, plan_slug).await?;

        let base_price = plan.price_cents;
        let discount = Self::discount_for_mode(payment_mode);
        let final_price = base_price - (base_price * discount / 100);
        let currency = "usd".to_string();

        /* Validar que phased tenga fases configuradas */
        if payment_mode == PaymentMode::Phased {
            let plan_phases = ServiceRepository::list_plan_phases(pool, plan.id).await?;
            if plan_phases.is_empty() {
                return Err(AppError::BadRequest(
                    "Este plan no tiene fases configuradas para pago por fases".into(),
                ));
            }
        }

        /* Crear PaymentIntent en Stripe con metadata de checkout.
         * [20CA-1] Guardamos email en vez de user_id — el usuario se crea en el webhook. */
        let params = vec![
            ("amount", final_price.to_string()),
            ("currency", currency.clone()),
            ("capture_method", "manual".to_string()),
            ("metadata[source]", "checkout".to_string()),
            ("metadata[email]", email.to_string()),
            ("metadata[service_slug]", service_slug.to_string()),
            ("metadata[plan_slug]", plan_slug.to_string()),
            ("metadata[payment_mode]", format!("{:?}", payment_mode)),
            ("receipt_email", email.to_string()),
        ];

        let resp = http_client
            .post("https://api.stripe.com/v1/payment_intents")
            .bearer_auth(stripe_key)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Error llamando a Stripe: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("Stripe error {status}: {body}");
            return Err(AppError::Internal(format!(
                "Stripe rechazó la solicitud ({status})"
            )));
        }

        let pi: StripePaymentIntentMin = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Error parseando respuesta Stripe: {e}")))?;

        Ok(crate::models::CheckoutIntentResponse {
            client_secret: pi.client_secret,
            amount_cents: final_price,
            currency,
        })
    }

    /* [166A-2] Descuento según modo de pago (misma lógica que OrderService). */
    fn discount_for_mode(mode: PaymentMode) -> i32 {
        match mode {
            PaymentMode::Full => 20,
            PaymentMode::HalfHalf => 10,
            PaymentMode::Phased => 0,
        }
    }

    /* [166A-2] Procesa pago exitoso de checkout: crea orden + pago + fases.
     * [20CA-1] Recibe email en vez de user_id: crea el usuario si no existe
     * (quick_register post-pago), o reutiliza el existente. Esto garantiza que
     * NO se crean cuentas huérfanas — el usuario solo se crea tras pago confirmado. */
    pub async fn handle_checkout_payment_succeeded(
        pool: &PgPool,
        email: &str,
        service_slug: &str,
        plan_slug: &str,
        payment_mode: PaymentMode,
        stripe_intent_id: &str,
        charge_id: Option<&str>,
        _amount_cents: i32,
    ) -> Result<(), AppError> {
        /* [20CA-1] Buscar o crear usuario por email */
        let user = Self::find_or_create_checkout_user(pool, email).await?;
        let user_id = user.id;

        let svc = find_service_for_order(pool, service_slug).await?;
        let plan = find_plan_for_order(pool, svc.id, plan_slug).await?;

        let base_price = plan.price_cents;
        let discount = Self::discount_for_mode(payment_mode);
        let final_price = base_price - (base_price * discount / 100);

        /* Crear la orden (status default: payment_held en BD) */
        let order = OrderRepository::create_order(
            pool,
            CreateOrderParams {
                client_id: user_id,
                service_id: svc.id,
                plan_id: plan.id,
                payment_mode,
                base_price_cents: base_price,
                discount_percent: discount,
                final_price_cents: final_price,
                project_description: None,
                client_notes: None,
            },
        )
        .await?;

        /* Generar fases de la orden desde plantillas del plan */
        let plan_phases = ServiceRepository::list_plan_phases(pool, plan.id).await?;
        for tmpl in &plan_phases {
            let phase_price = final_price * tmpl.percentage_of_total / 100;
            let status = if tmpl.phase_number == 1 {
                crate::services::OrderService::initial_phase_status(payment_mode)
            } else {
                PhaseStatus::Locked
            };
            OrderRepository::create_order_phase(
                pool,
                CreatePhaseParams {
                    order_id: order.id,
                    phase_number: tmpl.phase_number,
                    title: &tmpl.title,
                    description: tmpl.description.as_deref(),
                    price_cents: phase_price,
                    status,
                    max_revisions: tmpl.max_revisions,
                    estimated_days: tmpl.estimated_days,
                },
            )
            .await?;
        }

        /* Crear registro de pago en order_payments y marcarlo como held */
        let phase_1_id = if payment_mode == PaymentMode::Phased {
            OrderRepository::list_order_phases(pool, order.id)
                .await
                .ok()
                .and_then(|phases| phases.first().map(|p| p.id))
        } else {
            None
        };

        let description = format!(
            "{} — {} ({:?})",
            svc.title, plan.name, payment_mode
        );

        let payment = PaymentRepository::create_payment(
            pool,
            CreatePaymentParams {
                order_id: order.id,
                phase_id: phase_1_id,
                amount_cents: final_price,
                currency: "usd",
                payment_mode,
                stripe_payment_intent_id: stripe_intent_id,
                description: Some(&description),
            },
        )
        .await
        .map_err(|e| AppError::Internal(format!("Error guardando pago de checkout: {e}")))?;

        if let Some(cid) = charge_id {
            PaymentRepository::update_charge_id(pool, payment.id, cid).await?;
        }

        /* Marcar pago como held y ejecutar máquina de estados */
        let held_payment = PaymentRepository::update_status_held(pool, payment.id).await?;
        Self::handle_payment_success(pool, &held_payment).await?;

        /* Log de actividad */
        let _ = crate::repositories::ActivityLogRepository::log(
            pool,
            user_id,
            "order_created",
            "order",
            order.id,
            Some(serde_json::json!({
                "service": svc.title,
                "plan": plan.name,
                "payment_mode": format!("{:?}", payment_mode),
                "source": "checkout",
            })),
        )
        .await;

        tracing::info!(
            "[166A-2] Orden {} creada via checkout para usuario {user_id}",
            order.id
        );

        Ok(())
    }

    /// Procesa webhook de Stripe (ya verificada la firma).
    /// `event_data` es opcional para mantener compatibilidad con llamadas existentes;
    /// cuando se pasa, permite detectar checkout flows por metadata.
    pub async fn handle_webhook(
        pool: &PgPool,
        event_type: &str,
        data: &serde_json::Value,
        event_data: Option<&serde_json::Value>,
    ) -> Result<(), AppError> {
        match event_type {
            "payment_intent.succeeded" => {
                let pi_id = data["object"]["id"]
                    .as_str()
                    .ok_or_else(|| AppError::BadRequest("Missing payment_intent id".into()))?;
                let charge_id = data["object"]["latest_charge"].as_str();

                /* [166A-2] Detectar checkout flow: si metadata.source == "checkout",
                 * crear la orden + pago desde cero. El intent NO tiene order_id asociado. */
                let meta_source = event_data
                    .and_then(|d| d["object"]["metadata"]["source"].as_str());

                if meta_source == Some("checkout") {
                    /* [20CA-1] Extraer email de metadata (en vez de user_id).
                     * El usuario se crea/encuentra dentro de handle_checkout_payment_succeeded. */
                    let email = event_data
                        .and_then(|d| d["object"]["metadata"]["email"].as_str())
                        .ok_or_else(|| AppError::BadRequest("Missing email in checkout metadata".into()))?;
                    let service_slug = event_data
                        .and_then(|d| d["object"]["metadata"]["service_slug"].as_str())
                        .ok_or_else(|| AppError::BadRequest("Missing service_slug in metadata".into()))?;
                    let plan_slug = event_data
                        .and_then(|d| d["object"]["metadata"]["plan_slug"].as_str())
                        .ok_or_else(|| AppError::BadRequest("Missing plan_slug in metadata".into()))?;
                    let payment_mode_str = event_data
                        .and_then(|d| d["object"]["metadata"]["payment_mode"].as_str())
                        .unwrap_or("Full");
                    let payment_mode: PaymentMode = serde_json::from_str(
                        &format!("\"{}\"", payment_mode_str.to_lowercase())
                    ).unwrap_or(PaymentMode::Full);

                    let amount_cents = data["object"]["amount"]
                        .as_i64()
                        .unwrap_or(0) as i32;

                    Self::handle_checkout_payment_succeeded(
                        pool, email, service_slug, plan_slug, payment_mode,
                        pi_id, charge_id, amount_cents,
                    ).await?;

                    return Ok(());
                }

                /* Flujo original: pago contra orden existente */
                let payment = PaymentRepository::find_by_stripe_intent(pool, pi_id)
                    .await?
                    .ok_or_else(|| {
                        AppError::NotFound(format!("Payment for intent {pi_id} not found"))
                    })?;

                PaymentRepository::update_status_held(pool, payment.id).await?;
                if let Some(cid) = charge_id {
                    PaymentRepository::update_charge_id(pool, payment.id, cid).await?;
                }

                Self::handle_payment_success(pool, &payment).await?;
            }
            "payment_intent.payment_failed" => {
                let pi_id = data["object"]["id"]
                    .as_str()
                    .ok_or_else(|| AppError::BadRequest("Missing payment_intent id".into()))?;

                if let Some(payment) = PaymentRepository::find_by_stripe_intent(pool, pi_id).await?
                {
                    PaymentRepository::update_status(pool, payment.id, PaymentStatus::Failed)
                        .await?;
                }
            }
            _ => {
                tracing::debug!("Evento Stripe no manejado: {event_type}");
            }
        }
        Ok(())
    }

    /// Captura todos los pagos retenidos de una orden (al completarse)
    pub async fn capture_held_payments(
        pool: &PgPool,
        http_client: &Client,
        stripe_key: &str,
        order_id: Uuid,
    ) -> Result<(), AppError> {
        let held = PaymentRepository::find_held_for_order(pool, order_id).await?;
        for payment in held {
            if let Some(ref pi_id) = payment.stripe_payment_intent_id {
                /* [155A-11] Los pagos de prueba usan intents sinteticos: avanzan la orden
                 * sin cobro real y no deben capturarse contra Stripe al completar. */
                if Self::is_test_bypass_intent(pi_id) {
                    PaymentRepository::update_status_released(pool, payment.id).await?;
                    continue;
                }
                Self::capture_stripe_intent(http_client, stripe_key, pi_id).await?;
                PaymentRepository::update_status_released(pool, payment.id).await?;
            }
        }
        Ok(())
    }

    /// Lista pagos de una orden con números de fase resueltos
    pub async fn list_payments(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<Vec<PaymentResponse>, AppError> {
        let payments = PaymentRepository::list_for_order(pool, order_id).await?;
        let phases = OrderRepository::list_order_phases(pool, order_id).await?;

        Ok(payments
            .into_iter()
            .map(|p| {
                let bypassed = p
                    .stripe_payment_intent_id
                    .as_deref()
                    .is_some_and(Self::is_test_bypass_intent);
                let phase_number = p.phase_id.and_then(|pid| {
                    phases
                        .iter()
                        .find(|ph| ph.id == pid)
                        .map(|ph| ph.phase_number)
                });
                PaymentResponse {
                    id: p.id,
                    order_id: p.order_id,
                    phase_number,
                    amount_cents: p.amount_cents,
                    currency: p.currency,
                    status: p.status,
                    payment_mode: p.payment_mode,
                    description: p.description,
                    bypassed,
                    created_at: p.created_at,
                }
            })
            .collect())
    }

    /// Verifica firma HMAC-SHA256 del webhook de Stripe (constant-time comparison)
    pub fn verify_webhook_signature(
        payload: &[u8],
        signature_header: &str,
        webhook_secret: &str,
    ) -> Result<(), AppError> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let mut timestamp = None;
        let mut expected_sig = None;

        for part in signature_header.split(',') {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "t" => timestamp = Some(value),
                    "v1" => expected_sig = Some(value),
                    _ => {}
                }
            }
        }

        let ts = timestamp
            .ok_or_else(|| AppError::BadRequest("Missing timestamp in Stripe signature".into()))?;
        let sig = expected_sig
            .ok_or_else(|| AppError::BadRequest("Missing v1 in Stripe signature".into()))?;

        let signed_payload = format!("{ts}.{}", String::from_utf8_lossy(payload));

        let mut mac = Hmac::<Sha256>::new_from_slice(webhook_secret.as_bytes())
            .map_err(|e| AppError::Internal(format!("HMAC init failed: {e}")))?;
        mac.update(signed_payload.as_bytes());

        let decoded_sig = hex::decode(sig)
            .map_err(|_| AppError::BadRequest("Invalid hex in Stripe signature".into()))?;

        mac.verify_slice(&decoded_sig)
            .map_err(|_| AppError::Forbidden("Invalid webhook signature".into()))?;

        /* [064A-73] Verificar freshness: máximo 2 minutos de tolerancia (antes 5 min).
         * Reducido para estrechar la ventana de replay attacks. */
        if let Ok(ts_num) = ts.parse::<i64>() {
            let now = chrono::Utc::now().timestamp();
            if (now - ts_num).unsigned_abs() > 120 {
                return Err(AppError::BadRequest("Webhook timestamp too old".into()));
            }
        }

        Ok(())
    }

    /* ============================================================
    HELPERS PRIVADOS
    ============================================================ */

    /* [20CA-1] Busca usuario por email; si no existe, lo crea con contraseña
     * aleatoria (mismo flujo que quick_register). Usado por el webhook de
     * checkout para crear el usuario DESPUÉS del pago confirmado. */
    async fn find_or_create_checkout_user(
        pool: &PgPool,
        email: &str,
    ) -> Result<crate::models::User, AppError> {
        if let Some(user) = UserRepository::find_by_email(pool, email).await? {
            return Ok(user);
        }
        let random_password: String = {
            use argon2::password_hash::rand_core::RngCore;
            let mut buf = [0u8; 32];
            argon2::password_hash::rand_core::OsRng.fill_bytes(&mut buf);
            hex::encode(buf)
        };
        let password_hash = crate::services::auth::hash_password(&random_password)?;
        let user = UserRepository::create(pool, email, &password_hash, false).await?;
        let user = if crate::services::auth::is_admin_email(email) {
            UserRepository::update_role(pool, user.id, crate::models::UserRole::Admin).await?
        } else {
            user
        };
        tracing::info!("[20CA-1] Usuario {} creado post-pago para email {email}", user.id);
        Ok(user)
    }

    /// Resuelve monto, `phase_id` y descripción según `payment_mode`
    async fn resolve_payment_amount(
        pool: &PgPool,
        order: &crate::models::Order,
        phase_number: Option<i32>,
    ) -> Result<(i32, Option<Uuid>, String), AppError> {
        match order.payment_mode {
            PaymentMode::Full => {
                /* [166A-1] Aceptar pending_payment (orden nueva sin pago) y payment_held
                 * (orden legacy o después de pago parcial en half_half). Rechazar otros estados. */
                if order.status != OrderStatus::PendingPayment
                    && order.status != OrderStatus::PaymentHeld
                {
                    return Err(AppError::BadRequest("La orden ya fue pagada o no está disponible para pago".into()));
                }
                Ok((
                    order.final_price_cents,
                    None,
                    format!("Pago completo - Orden #{}", order.order_number),
                ))
            }
            PaymentMode::HalfHalf => {
                let existing = PaymentRepository::list_for_order(pool, order.id).await?;
                let paid_count = existing
                    .iter()
                    .filter(|p| {
                        p.status == PaymentStatus::Held || p.status == PaymentStatus::Released
                    })
                    .count();

                if paid_count >= 2 {
                    return Err(AppError::BadRequest(
                        "Ambos pagos ya fueron realizados".into(),
                    ));
                }

                let half = order.final_price_cents / 2;
                let (amount, desc) = if paid_count == 0 {
                    (
                        half,
                        format!("Primer pago 50% - Orden #{}", order.order_number),
                    )
                } else {
                    (
                        order.final_price_cents - half,
                        format!("Segundo pago 50% - Orden #{}", order.order_number),
                    )
                };
                Ok((amount, None, desc))
            }
            PaymentMode::Phased => {
                let pn = phase_number.ok_or_else(|| {
                    AppError::BadRequest("Se requiere phase_number para pago por fases".into())
                })?;
                let phase = OrderRepository::find_phase_by_number(pool, order.id, pn)
                    .await?
                    .ok_or_else(|| AppError::NotFound(format!("Fase {pn} no encontrada")))?;

                if phase.status != PhaseStatus::PendingPayment {
                    return Err(AppError::BadRequest(format!(
                        "La fase {pn} no está pendiente de pago (estado: {:?})",
                        phase.status
                    )));
                }

                Ok((
                    phase.price_cents,
                    Some(phase.id),
                    format!("Fase {} - Orden #{}", pn, order.order_number),
                ))
            }
        }
    }

    /// Crea `PaymentIntent` en Stripe con `capture_method` manual (escrow)
    /// [064A-59] `receipt_email` pre-llena el email — Stripe no lo pide de nuevo en checkout
    async fn create_stripe_intent(
        client: &Client,
        api_key: &str,
        amount: i32,
        currency: &str,
        order_id: Uuid,
        phase_number: Option<i32>,
        receipt_email: Option<&str>,
    ) -> Result<StripePaymentIntentMin, AppError> {
        let mut form = vec![
            ("amount".to_string(), amount.to_string()),
            ("currency".to_string(), currency.to_lowercase()),
            ("capture_method".to_string(), "manual".to_string()),
            ("metadata[order_id]".to_string(), order_id.to_string()),
        ];
        if let Some(pn) = phase_number {
            form.push(("metadata[phase_number]".to_string(), pn.to_string()));
        }
        if let Some(email) = receipt_email {
            form.push(("receipt_email".to_string(), email.to_string()));
        }

        let resp = client
            .post("https://api.stripe.com/v1/payment_intents")
            .basic_auth(api_key, None::<&str>)
            .form(&form)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Error comunicando con Stripe: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            /* [064A-73] Log explícito de errores Stripe para debugging.
             * [074A-24] Clasificar el error Stripe para dar feedback útil al usuario
             * sin exponer detalles internos (ej: claves API). */
            tracing::error!("Stripe create_payment_intent falló: {body}");
            return Err(Self::classify_stripe_error(&body));
        }

        resp.json::<StripePaymentIntentMin>()
            .await
            .map_err(|e| AppError::Internal(format!("Error parseando respuesta Stripe: {e}")))
    }

    /// Captura un `PaymentIntent` en Stripe (libera fondos retenidos)
    async fn capture_stripe_intent(
        client: &Client,
        api_key: &str,
        payment_intent_id: &str,
    ) -> Result<(), AppError> {
        let url = format!("https://api.stripe.com/v1/payment_intents/{payment_intent_id}/capture");
        let resp = client
            .post(&url)
            .basic_auth(api_key, None::<&str>)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Error capturando pago Stripe: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            /* [064A-73] Log explícito de errores Stripe para debugging */
            tracing::error!("Stripe capture falló: {body}");
            return Err(AppError::Internal(format!("Stripe capture error: {body}")));
        }
        Ok(())
    }

    /// Post-pago: avanza la máquina de estados de la orden según `payment_mode`
    async fn handle_payment_success(pool: &PgPool, payment: &OrderPayment) -> Result<(), AppError> {
        let order = OrderRepository::find_order_by_id(pool, payment.order_id)
            .await?
            .ok_or_else(|| AppError::Internal("Orden no encontrada post-pago".into()))?;

        match order.payment_mode {
            PaymentMode::Full => {
                /* Pago completo → todas las fases a Paid, orden a awaiting_assignment */
                OrderRepository::set_awaiting_assignment(pool, order.id).await?;
                let phases = OrderRepository::list_order_phases(pool, order.id).await?;
                for phase in phases {
                    if phase.status == PhaseStatus::PendingPayment
                        || phase.status == PhaseStatus::Locked
                    {
                        OrderRepository::update_phase_status(pool, phase.id, PhaseStatus::Paid)
                            .await?;
                    }
                }
            }
            PaymentMode::HalfHalf => {
                let all_payments = PaymentRepository::list_for_order(pool, order.id).await?;
                let held_count = all_payments
                    .iter()
                    .filter(|p| p.status == PaymentStatus::Held)
                    .count();

                if held_count >= 1 && order.status == OrderStatus::PaymentHeld {
                    /* Primer 50% → awaiting_assignment, desbloquear primera mitad de fases */
                    OrderRepository::set_awaiting_assignment(pool, order.id).await?;
                    let phases = OrderRepository::list_order_phases(pool, order.id).await?;
                    let midpoint = phases.len().div_ceil(2);
                    for (i, phase) in phases.iter().enumerate() {
                        if i < midpoint
                            && (phase.status == PhaseStatus::PendingPayment
                                || phase.status == PhaseStatus::Locked)
                        {
                            OrderRepository::update_phase_status(pool, phase.id, PhaseStatus::Paid)
                                .await?;
                        }
                    }
                }
                /* Segundo 50% → desbloquear fases restantes */
                if held_count >= 2 {
                    let phases = OrderRepository::list_order_phases(pool, order.id).await?;
                    for phase in phases {
                        if phase.status == PhaseStatus::Locked
                            || phase.status == PhaseStatus::PendingPayment
                        {
                            OrderRepository::update_phase_status(pool, phase.id, PhaseStatus::Paid)
                                .await?;
                        }
                    }
                }
            }
            PaymentMode::Phased => {
                /* Pago de fase individual → actualizar esa fase a Paid */
                if let Some(phase_id) = payment.phase_id {
                    OrderRepository::update_phase_status(pool, phase_id, PhaseStatus::Paid).await?;
                }
                /* Si la orden estaba en payment_held y la primera fase se pagó, avanzar */
                if order.status == OrderStatus::PaymentHeld {
                    OrderRepository::set_awaiting_assignment(pool, order.id).await?;
                }
            }
        }
        Ok(())
    }

    /// [044A-38 Fase 7] Ejecutar reembolso en Stripe.
    /// Para pagos con `capture_method=manual` (held): cancela el `PaymentIntent`.
    /// Para pagos ya capturados (released): crea un `Refund`.
    pub async fn refund_payment(
        http_client: &Client,
        stripe_key: &str,
        payment: &OrderPayment,
    ) -> Result<String, AppError> {
        let pi_id = payment
            .stripe_payment_intent_id
            .as_deref()
            .ok_or_else(|| AppError::Internal("Pago sin PaymentIntent de Stripe".into()))?;

        match payment.status {
            PaymentStatus::Held => {
                /* Fondos retenidos → cancelar PaymentIntent libera el dinero */
                let url = format!("https://api.stripe.com/v1/payment_intents/{pi_id}/cancel");
                let resp = http_client
                    .post(&url)
                    .basic_auth(stripe_key, None::<&str>)
                    .send()
                    .await
                    .map_err(|e| {
                        AppError::Internal(format!("Error cancelando PaymentIntent Stripe: {e}"))
                    })?;

                if !resp.status().is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    tracing::error!("Stripe cancel falló: {body}");
                    return Err(AppError::Internal(format!("Stripe cancel error: {body}")));
                }
                /* Para cancel, el refund_id es el PI id mismo */
                Ok(format!("cancel_{pi_id}"))
            }
            PaymentStatus::Released => {
                /* Fondos ya capturados → crear Refund en Stripe */
                let resp = http_client
                    .post("https://api.stripe.com/v1/refunds")
                    .basic_auth(stripe_key, None::<&str>)
                    .form(&[("payment_intent", pi_id)])
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("Error creando refund Stripe: {e}")))?;

                if !resp.status().is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    tracing::error!("Stripe refund falló: {body}");
                    return Err(AppError::Internal(format!("Stripe refund error: {body}")));
                }

                let refund_resp = resp.json::<StripeRefundMin>().await.map_err(|e| {
                    AppError::Internal(format!("Error parseando refund Stripe: {e}"))
                })?;
                Ok(refund_resp.id)
            }
            _ => Err(AppError::BadRequest(
                "El pago no está en un estado reembolsable".into(),
            )),
        }
    }

    /* [074A-24] Clasifica errores de Stripe API para dar feedback útil al usuario
     * sin exponer detalles internos (claves API, IDs internos de Stripe).
     * Stripe devuelve: { "error": { "type": "...", "message": "...", "code": "..." } }
     * [074A-27] Todos retornan BadRequest para que el mensaje llegue al cliente.
     * AppError::Internal oculta el mensaje — seguro pero inútil para el usuario. */
    fn classify_stripe_error(body: &str) -> AppError {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);
        let (error_type, message) = match &parsed {
            Ok(json) => (
                json["error"]["type"].as_str().unwrap_or("unknown"),
                json["error"]["message"]
                    .as_str()
                    .unwrap_or("Error desconocido de Stripe"),
            ),
            Err(_) => return AppError::BadRequest("Error inesperado del servicio de pagos".into()),
        };

        match error_type {
            "authentication_error" => {
                AppError::BadRequest("Error de configuración de pagos — contacta soporte".into())
            }
            "invalid_request_error" | "card_error" => {
                AppError::BadRequest(format!("Error de pago: {message}"))
            }
            "rate_limit_error" => AppError::BadRequest(
                "Demasiadas solicitudes de pago, intenta de nuevo en unos segundos".into(),
            ),
            _ => AppError::BadRequest("Servicio de pagos no disponible temporalmente".into()),
        }
    }

    fn is_test_bypass_intent(intent_id: &str) -> bool {
        intent_id.starts_with("test_bypass_")
    }
}

/* Tipo mínimo para parsear la respuesta de Stripe PaymentIntent */
#[derive(Debug, serde::Deserialize)]
struct StripePaymentIntentMin {
    id: String,
    client_secret: String,
}

/* [044A-38 Fase 7] Tipo mínimo para parsear respuesta de Stripe Refund */
#[derive(Debug, serde::Deserialize)]
struct StripeRefundMin {
    id: String,
}
