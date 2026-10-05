/* [01AA-4-F3b] Creación de intents y alta post-pago (era parte de
 * services/payment.rs). */

use reqwest::Client;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::order_slugs::{find_plan_for_order, find_service_for_order};
use super::PaymentService;
use crate::errors::AppError;
use crate::models::{
    OrderStatus, PaymentIntentResponse, PaymentMode, PaymentStatus, PhaseStatus,
};
use crate::repositories::{
    CreateOrderParams, CreatePaymentParams, CreatePhaseParams, OrderRepository, PaymentRepository,
    ServiceRepository, UserRepository,
};

/* Tipo mínimo para parsear la respuesta de Stripe PaymentIntent */
#[derive(Debug, serde::Deserialize)]
struct StripePaymentIntentMin {
    id: String,
    client_secret: String,
}

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

        /* [277A-7] Idempotency-Key para checkout directo */
        let idempotency_key = Uuid::new_v4().to_string();

        let resp = http_client
            .post("https://api.stripe.com/v1/payment_intents")
            .bearer_auth(stripe_key)
            .header("Idempotency-Key", &idempotency_key)
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
        stripe_amount_cents: i32,
    ) -> Result<(), AppError> {
        /* [20CA-1] Buscar o crear usuario por email */
        let user = Self::find_or_create_checkout_user(pool, email).await?;
        let user_id = user.id;

        let svc = find_service_for_order(pool, service_slug).await?;
        let plan = find_plan_for_order(pool, svc.id, plan_slug).await?;

        let base_price = plan.price_cents;
        let discount = Self::discount_for_mode(payment_mode);
        let final_price = base_price - (base_price * discount / 100);

        /* [277A-7] Validar que el monto de Stripe coincida con el esperado.
         * Previene manipulación del monto en el webhook (spoofed amount). */
        if stripe_amount_cents > 0 && stripe_amount_cents != final_price {
            tracing::error!(
                "[277A-7] Mismatch de monto checkout: Stripe={stripe_amount_cents}, esperado={final_price} para {service_slug}/{plan_slug}"
            );
            return Err(AppError::BadRequest(
                "El monto del pago no coincide con el esperado".into(),
            ));
        }

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
        Self::create_order_phases_from_templates(pool, order.id, plan.id, final_price, payment_mode)
            .await?;

        /* Crear registro de pago en order_payments y marcarlo como held */
        let phase_1_id = if payment_mode == PaymentMode::Phased {
            OrderRepository::list_order_phases(pool, order.id)
                .await
                .ok()
                .and_then(|phases| phases.first().map(|p| p.id))
        } else {
            None
        };

        let description = format!("{} — {} ({:?})", svc.title, plan.name, payment_mode);

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

    /* [01AA-4-f3s] Genera las fases de la orden desde las plantillas del plan
     * (extraído de handle_checkout_payment_succeeded). La fase 1 hereda el
     * estado inicial según el modo de pago; el resto nace Locked. */
    async fn create_order_phases_from_templates(
        pool: &PgPool,
        order_id: Uuid,
        plan_id: Uuid,
        final_price: i32,
        payment_mode: PaymentMode,
    ) -> Result<(), AppError> {
        let plan_phases = ServiceRepository::list_plan_phases(pool, plan_id).await?;
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
                    order_id,
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
        Ok(())
    }

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
        tracing::info!(
            "[20CA-1] Usuario {} creado post-pago para email {email}",
            user.id
        );
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
                    return Err(AppError::BadRequest(
                        "La orden ya fue pagada o no está disponible para pago".into(),
                    ));
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

        /* [277A-7] Idempotency-Key: previene PaymentIntents duplicados si Stripe
         * recibe la misma petición más de una vez (timeout, retry de red). */
        let idempotency_key = Uuid::new_v4().to_string();

        let resp = client
            .post("https://api.stripe.com/v1/payment_intents")
            .basic_auth(api_key, None::<&str>)
            .header("Idempotency-Key", &idempotency_key)
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
}
