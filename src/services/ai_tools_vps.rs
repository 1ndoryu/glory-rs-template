/* [259A-4e] Tools IA de VPS: `vps_tool_defs` + exec fns de catálogo, consulta
 * y checkout. Reutiliza `fetch_hosting_client`, `tool_error` y
 * `chat_public_base_url` de `ai_tools_hosting` (mismo flujo de cliente). */

/* [155A-12] Tools de VPS: asesoría de catálogo, consulta de VPS del cliente y checkout mensual.
 * No aprovisionan ni aprueban: la aprobación manual sigue protegida en el panel/admin. */

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::models::{SelfSubscribeVpsRequest, VpsPlanConfig, VpsSubscription};
use crate::repositories::{CreateVpsSubscriptionParams, VpsRepository};
use crate::services::{
    checkout_bypass_is_configured, is_checkout_bypass_email, vps_stripe_fee_cents,
    VpsCheckoutParams, VpsStripeService,
};

use super::ai_tools::{
    require_auth, requires_login, tool_json, tool_status, RichMessage, ToolAuthContext,
    ToolExecResult, ToolExecutionContext,
};
use super::ai_tools_hosting::{chat_public_base_url, fetch_hosting_client, tool_error};

pub(crate) fn vps_tool_defs() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "list_vps_plans",
                "description": "Lista planes reales de VPS desde la configuración de la base de datos. Úsalo para asesorar precios, CPU, RAM, disco, región y diferencias antes de recomendar un VPS.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_my_vps",
                "description": "Lista las solicitudes y suscripciones VPS del cliente registrado, con tier, hostname, estado, IP y eventos recientes. Úsalo cuando pregunte por su VPS o quiera gestionarlo.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_vps_checkout",
                "description": "Crea una solicitud VPS pending_payment y un checkout mensual real de Stripe. Úsalo solo cuando un cliente registrado confirme que quiere contratar un tier concreto. Si el cliente no está registrado, la tool indicará que debe iniciar sesión.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "tier": { "type": "string", "description": "Slug del tier VPS, por ejemplo vps1, vps2 o vps3" },
                        "hostname": { "type": "string", "description": "Hostname opcional solicitado por el cliente" }
                    },
                    "required": ["tier"]
                }
            }
        }
    ])
}

pub(crate) async fn exec_list_vps_plans(pool: &PgPool) -> ToolExecResult {
    match VpsRepository::list_plan_configs(pool).await {
        Ok(plans) => tool_json(json!({
            "status": "ok",
            "plans": plans.iter().map(vps_plan_json).collect::<Vec<_>>(),
        })),
        Err(e) => {
            tracing::error!("Error listando planes VPS para AI tool: {e}");
            tool_status("error", "No se pudieron listar los planes VPS")
        }
    }
}

fn vps_plan_json(plan: &VpsPlanConfig) -> Value {
    json!({
        "tier": plan.tier_name,
        "display_name": plan.display_name,
        "description": plan.description,
        "monthly_price_cents": plan.monthly_price_cents,
        "setup_fee_cents": plan.setup_fee_cents,
        "cpu_cores": plan.cpu_cores,
        "ram_mb": plan.ram_mb,
        "disk_mb": plan.disk_mb,
        "storage_type": plan.storage_type,
        "storage_options": plan.storage_options,
        "port_speed_mbps": plan.port_speed_mbps,
        "bandwidth_label": plan.bandwidth_label,
        "snapshot_count": plan.snapshot_count,
        "region": plan.region,
        "approval_required": plan.approval_required,
    })
}

pub(crate) async fn exec_list_my_vps(
    pool: &PgPool,
    auth: Option<ToolAuthContext>,
) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };

    match VpsRepository::list_by_user_id(pool, auth.user_id).await {
        Ok(subscriptions) => {
            let subscriptions = with_vps_events(pool, subscriptions).await;
            tool_json(json!({
                "status": "ok",
                "scope": auth.effective_role.to_string(),
                "vps": subscriptions,
            }))
        }
        Err(e) => {
            tracing::error!(
                user_id = %auth.user_id,
                effective_role = %auth.effective_role,
                "Error listando VPS del usuario: {e}"
            );
            tool_status("error", "No se pudieron consultar tus VPS")
        }
    }
}

async fn with_vps_events(pool: &PgPool, subscriptions: Vec<VpsSubscription>) -> Vec<Value> {
    let mut result = Vec::with_capacity(subscriptions.len());
    for subscription in subscriptions {
        let events = match VpsRepository::list_events(pool, subscription.id, 5).await {
            Ok(events) => events,
            Err(e) => {
                tracing::warn!(vps_id = %subscription.id, "Error listando eventos VPS para AI tool: {e}");
                Vec::new()
            }
        }
        .into_iter()
        .map(|event| {
            json!({
                "event_type": event.event_type,
                "details": event.details,
                "created_at": event.created_at,
            })
        })
        .collect::<Vec<_>>();
        result.push(json!({
            "id": subscription.id,
            "tier": subscription.tier_name,
            "hostname": subscription.requested_hostname,
            "status": subscription.status,
            "monthly_price_cents": subscription.monthly_price_cents,
            "contabo_instance_id": subscription.contabo_instance_id,
            "provisioning_ip": subscription.provisioning_ip,
            "access_username": subscription.access_username,
            "client_notes": subscription.client_notes,
            "events": events,
        }));
    }
    result
}

pub(crate) async fn exec_create_vps_checkout(
    ctx: &ToolExecutionContext<'_>,
    args: &Value,
) -> ToolExecResult {
    let Ok(auth) = require_auth(ctx.auth) else {
        return requires_login(
            "Para contratar VPS y generar checkout mensual debes iniciar sesión o crear una cuenta.",
        );
    };
    if ctx.stripe_key.is_none() && !checkout_bypass_is_configured() {
        return tool_error("Stripe no configurado para checkout de VPS");
    }

    let req = match parse_vps_checkout_request(args) {
        Ok(req) => req,
        Err(result) => return result,
    };
    let plan_config = match fetch_vps_plan_config(ctx.pool, &req.tier).await {
        Ok(config) => config,
        Err(result) => return result,
    };
    let (client_name, client_email) = match fetch_hosting_client(ctx.pool, auth.user_id).await {
        Ok(client) => client,
        Err(result) => return result,
    };
    let subscription = match create_chat_vps_subscription(
        ctx.pool,
        auth.user_id,
        &req,
        &plan_config,
        &client_name,
        &client_email,
    )
    .await
    {
        Ok(subscription) => subscription,
        Err(result) => return result,
    };

    if is_checkout_bypass_email(&client_email) {
        return match activate_chat_vps_bypass(ctx.pool, &subscription, auth.user_id).await {
            Ok(updated) => vps_bypass_success(&updated),
            Err(result) => result,
        };
    }

    let Some(stripe_key) = ctx.stripe_key else {
        return tool_error("Stripe no configurado para checkout de VPS");
    };

    let checkout_url = match create_chat_vps_checkout_url(
        ctx.http_client,
        stripe_key,
        &plan_config,
        &subscription,
        &client_email,
    )
    .await
    {
        Ok(url) => url,
        Err(result) => return result,
    };

    vps_checkout_success(&subscription, &checkout_url)
}

fn parse_vps_checkout_request(args: &Value) -> Result<SelfSubscribeVpsRequest, ToolExecResult> {
    let hostname = args["hostname"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let req = SelfSubscribeVpsRequest {
        tier: args["tier"].as_str().unwrap_or("").trim().to_string(),
        hostname,
        storage_preference: None,
        region_preference: None,
        os_preference: None,
        server_password: None,
    };
    req.validate()
        .map(|()| req)
        .map_err(|e| tool_error(&format!("Datos de VPS inválidos: {e}")))
}

async fn fetch_vps_plan_config(pool: &PgPool, tier: &str) -> Result<VpsPlanConfig, ToolExecResult> {
    match VpsRepository::get_plan_config(pool, tier).await {
        Ok(Some(config)) if config.is_active => Ok(config),
        Ok(_) => Err(tool_error(&format!("Tier VPS inválido: {tier}"))),
        Err(e) => {
            tracing::error!("Error consultando tier VPS {tier}: {e}");
            Err(tool_error("No se pudo consultar el plan VPS"))
        }
    }
}

async fn create_chat_vps_subscription(
    pool: &PgPool,
    user_id: Uuid,
    req: &SelfSubscribeVpsRequest,
    plan_config: &VpsPlanConfig,
    client_name: &str,
    client_email: &str,
) -> Result<VpsSubscription, ToolExecResult> {
    let subscription = VpsRepository::create(
        pool,
        CreateVpsSubscriptionParams {
            user_id: Some(user_id),
            client_name,
            client_email,
            tier_name: &req.tier,
            requested_hostname: req.hostname.as_deref(),
            client_notes: None,
            monthly_price_cents: plan_config.monthly_price_cents,
        },
    )
    .await
    .map_err(|e| {
        tracing::error!("Error creando suscripción VPS desde chatbot: {e}");
        tool_error("No se pudo crear la solicitud VPS")
    })?;
    record_chat_vps_event(pool, subscription.id, user_id, &req.tier).await;
    Ok(subscription)
}

async fn record_chat_vps_event(pool: &PgPool, subscription_id: Uuid, user_id: Uuid, tier: &str) {
    if let Err(e) = VpsRepository::add_event(
        pool,
        subscription_id,
        "created",
        Some(json!({"tier": tier, "by": user_id.to_string(), "source": "chatbot-vps"})),
    )
    .await
    {
        tracing::warn!(vps_id = %subscription_id, "Error registrando evento chatbot-vps: {e}");
    }
}

async fn activate_chat_vps_bypass(
    pool: &PgPool,
    subscription: &VpsSubscription,
    user_id: Uuid,
) -> Result<VpsSubscription, ToolExecResult> {
    VpsRepository::update_status(pool, subscription.id, "pending_approval")
        .await
        .map_err(|e| {
            tracing::error!(vps_id = %subscription.id, "Error activando bypass VPS desde chatbot: {e}");
            tool_error("No se pudo registrar el VPS de prueba")
        })?;
    if let Err(e) = VpsRepository::add_event(
        pool,
        subscription.id,
        "test_checkout_bypassed",
        Some(json!({"by": user_id.to_string(), "source": "chatbot-vps"})),
    )
    .await
    {
        tracing::warn!(vps_id = %subscription.id, "Error registrando bypass VPS chatbot: {e}");
    }
    VpsRepository::find_by_id(pool, subscription.id)
        .await
        .map_err(|e| {
            tracing::error!(vps_id = %subscription.id, "Error recargando VPS test chatbot: {e}");
            tool_error("No se pudo consultar el VPS creado")
        })?
        .ok_or_else(|| tool_error("VPS creado no encontrado"))
}

async fn create_chat_vps_checkout_url(
    http_client: &reqwest::Client,
    stripe_key: &str,
    plan_config: &VpsPlanConfig,
    subscription: &VpsSubscription,
    client_email: &str,
) -> Result<String, ToolExecResult> {
    let base_url = chat_public_base_url();
    let success_url = format!("{base_url}/panel?vps=success&session_id={{CHECKOUT_SESSION_ID}}");
    let cancel_url = format!("{base_url}/panel?vps=cancelled");
    VpsStripeService::create_checkout_session(&VpsCheckoutParams {
        http_client,
        stripe_key,
        subscription_id: subscription.id,
        tier_name: &subscription.tier_name,
        amount_cents: subscription.monthly_price_cents,
        setup_fee_cents: plan_config.setup_fee_cents,
        processing_fee_cents: vps_stripe_fee_cents(
            subscription.monthly_price_cents + plan_config.setup_fee_cents,
        ),
        customer_email: client_email,
        success_url: &success_url,
        cancel_url: &cancel_url,
    })
    .await
    .map_err(|e| {
        tracing::error!(
            vps_id = %subscription.id,
            "Error creando checkout VPS desde chatbot: {e}"
        );
        tool_error("No se pudo crear el checkout de VPS")
    })
}

fn vps_checkout_success(subscription: &VpsSubscription, checkout_url: &str) -> ToolExecResult {
    ToolExecResult {
        tool_result_json: json!({
            "status": "ok",
            "vps_subscription_id": subscription.id,
            "tier": subscription.tier_name,
            "hostname": subscription.requested_hostname,
            "amount_cents": subscription.monthly_price_cents,
            "message": "Checkout mensual de VPS creado. El cliente verá una tarjeta visual con botón de pago. NO repitas el link en texto."
        })
        .to_string(),
        rich_message: Some(RichMessage {
            content: format!(
                "VPS {} — ${:.2} USD/mes",
                subscription.tier_name,
                f64::from(subscription.monthly_price_cents) / 100.0
            ),
            message_type: "invoice".to_string(),
            metadata: json!({
                "title": "VPS",
                "payment_url": checkout_url,
                "amount_cents": subscription.monthly_price_cents,
                "currency": "usd",
                "status": "open",
                "description": format!("Suscripción mensual VPS {}", subscription.tier_name),
                "vps_subscription_id": subscription.id,
                "tier": subscription.tier_name,
                "hostname": subscription.requested_hostname,
            }),
        }),
    }
}

fn vps_bypass_success(subscription: &VpsSubscription) -> ToolExecResult {
    tool_json(json!({
        "status": "ok",
        "vps_subscription_id": subscription.id,
        "tier": subscription.tier_name,
        "hostname": subscription.requested_hostname,
        "amount_cents": subscription.monthly_price_cents,
        "bypassed": true,
        "message": "Solicitud VPS creada para cuenta test sin cobro real. Queda pendiente de aprobación manual como si el pago ya hubiera sido confirmado."
    }))
}
