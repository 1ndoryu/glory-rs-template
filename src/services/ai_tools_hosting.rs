/* [259A-4e] Tools IA de hosting: `hosting_tool_defs` + exec fns de catálogo,
 * consulta y checkout. `tool_error`, `chat_public_base_url` y
 * `fetch_hosting_client` son `pub(crate)` porque los reutiliza `ai_tools_vps`. */

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::models::{
    sanitize_hosting_event_details, HostingPlanConfig, HostingSubscription, SelfSubscribeRequest,
};
use crate::repositories::{CreateHostingParams, HostingRepository, UserRepository};
use crate::services::{
    checkout_bypass_is_configured, is_checkout_bypass_email, CheckoutParams, HostingStripeService,
};

use super::ai_tools::{
    require_auth, requires_login, tool_json, tool_status, RichMessage, ToolAuthContext,
    ToolExecResult, ToolExecutionContext,
};

/* [095A-8] Tools de hosting: asesoría con catálogo real, consulta de hostings del cliente
 * y checkout mensual Stripe. No ejecutan provisioning/restart/stop: eso sigue siendo admin-only. */
pub(crate) fn hosting_tool_defs() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "list_hosting_plans",
                "description": "Lista planes reales de hosting normal y hosting WordPress desde la configuración de la base de datos. Úsalo para asesorar precios, recursos y diferencias antes de recomendar un plan.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_my_hostings",
                "description": "Lista las suscripciones de hosting del cliente registrado, con plan, dominio, estado y datos operativos básicos. Úsalo cuando el cliente pregunte por su hosting o quiera gestionarlo.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_hosting_checkout",
                "description": "Crea una suscripción pending de hosting y un checkout mensual real de Stripe. Úsalo solo cuando un cliente registrado confirme que quiere contratar un plan concreto. Si el cliente no está registrado, la tool indicará que debe iniciar sesión.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "plan": { "type": "string", "description": "Slug del plan: basico, pro o ecommerce" },
                        "domain": { "type": "string", "description": "Dominio opcional del cliente, sin https:// ni rutas" }
                    },
                    "required": ["plan"]
                }
            }
        }
    ])
}

pub(crate) async fn exec_list_hosting_plans(pool: &PgPool) -> ToolExecResult {
    match HostingRepository::list_plan_configs(pool).await {
        Ok(plans) => ToolExecResult {
            tool_result_json: json!({
                "status": "ok",
                "plans": plans.into_iter().map(|plan| json!({
                    "plan": plan.plan_name,
                    "monthly_price_cents": plan.monthly_price_cents,
                    "storage_limit_mb": plan.storage_limit_mb,
                    "bandwidth_limit_gb": plan.bandwidth_limit_gb,
                    "wp_cpu_millicores": plan.wp_cpu_millicores,
                    "wp_memory_mb": plan.wp_memory_mb,
                    "db_cpu_millicores": plan.db_cpu_millicores,
                    "db_memory_mb": plan.db_memory_mb,
                    "ssh_cpu_millicores": plan.ssh_cpu_millicores,
                    "ssh_memory_mb": plan.ssh_memory_mb,
                })).collect::<Vec<_>>()
            })
            .to_string(),
            rich_message: None,
        },
        Err(e) => {
            tracing::error!("Error listando planes hosting para AI tool: {e}");
            tool_status("error", "No se pudieron listar los planes de hosting")
        }
    }
}

pub(crate) async fn exec_list_my_hostings(
    pool: &PgPool,
    auth: Option<ToolAuthContext>,
) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };

    match HostingRepository::list_by_user_id(pool, auth.user_id).await {
        Ok(hostings) => {
            let hostings = with_hosting_events(pool, hostings).await;
            tool_json(json!({
                "status": "ok",
                "scope": auth.effective_role.to_string(),
                "hostings": hostings,
            }))
        }
        Err(e) => {
            tracing::error!(
                user_id = %auth.user_id,
                effective_role = %auth.effective_role,
                "Error listando hostings del usuario: {e}"
            );
            tool_status("error", "No se pudieron consultar tus hostings")
        }
    }
}

async fn with_hosting_events(pool: &PgPool, hostings: Vec<HostingSubscription>) -> Vec<Value> {
    let mut result = Vec::with_capacity(hostings.len());
    for hosting in hostings {
        let events = match HostingRepository::list_events(pool, hosting.id, 5).await {
            Ok(events) => events,
            Err(e) => {
                tracing::warn!(hosting_id = %hosting.id, "Error listando eventos hosting para AI tool: {e}");
                Vec::new()
            }
        }
        .into_iter()
            .map(|event| {
                json!({
                    "event_type": event.event_type,
                    "details": sanitize_hosting_event_details(event.details),
                    "created_at": event.created_at,
                })
            })
            .collect::<Vec<_>>();
        result.push(json!({
                    "id": hosting.id,
                    "plan": hosting.plan,
                    "domain": hosting.domain,
                    "status": hosting.status,
                    "monthly_price_cents": hosting.monthly_price_cents,
                    "storage_limit_mb": hosting.storage_limit_mb,
                    "server_ip": hosting.server_ip,
                    "coolify_site_name": hosting.coolify_site_name,
                    "sftp_user": hosting.sftp_user,
                    "sftp_port": hosting.sftp_port,
                    "events": events,
        }));
    }
    result
}

pub(crate) fn chat_public_base_url() -> String {
    std::env::var("GLORY_PUBLIC_URL")
        .or_else(|_| std::env::var("PUBLIC_URL"))
        .unwrap_or_else(|_| "https://nakomi.studio".to_string())
        .trim_end_matches('/')
        .to_string()
}

pub(crate) async fn exec_create_hosting_checkout(
    ctx: &ToolExecutionContext<'_>,
    args: &Value,
) -> ToolExecResult {
    let Ok(auth) = require_auth(ctx.auth) else {
        return hosting_requires_login();
    };
    if ctx.stripe_key.is_none() && !checkout_bypass_is_configured() {
        return tool_error("Stripe no configurado para checkout de hosting");
    }

    let req = match parse_hosting_checkout_request(args) {
        Ok(req) => req,
        Err(result) => return result,
    };
    let plan_config = match fetch_hosting_plan_config(ctx.pool, &req.plan).await {
        Ok(config) => config,
        Err(result) => return result,
    };
    let (client_name, client_email) = match fetch_hosting_client(ctx.pool, auth.user_id).await {
        Ok(client) => client,
        Err(result) => return result,
    };
    let sub = match create_chat_hosting_subscription(
        ctx.pool,
        auth.user_id,
        &req,
        &plan_config,
        &client_name,
        &client_email,
    )
    .await
    {
        Ok(sub) => sub,
        Err(result) => return result,
    };
    if is_checkout_bypass_email(&client_email) {
        return match activate_chat_hosting_bypass(ctx.pool, &sub, auth.user_id).await {
            Ok(updated) => hosting_bypass_success(&updated),
            Err(result) => result,
        };
    }

    let Some(stripe_key) = ctx.stripe_key else {
        return tool_error("Stripe no configurado para checkout de hosting");
    };

    let checkout_url =
        match create_chat_hosting_checkout_url(ctx.http_client, stripe_key, &sub, &client_email)
            .await
        {
            Ok(url) => url,
            Err(result) => return result,
        };

    hosting_checkout_success(&sub, &checkout_url)
}

async fn activate_chat_hosting_bypass(
    pool: &PgPool,
    sub: &HostingSubscription,
    user_id: Uuid,
) -> Result<HostingSubscription, ToolExecResult> {
    HostingRepository::update_status(pool, sub.id, "active")
        .await
        .map_err(|e| {
            tracing::error!(hosting_id = %sub.id, "Error activando hosting test desde chatbot: {e}");
            tool_error("No se pudo activar el hosting de prueba")
        })?;
    if let Err(e) = HostingRepository::add_event(
        pool,
        sub.id,
        "test_checkout_bypassed",
        Some(json!({"by": user_id.to_string(), "source": "chatbot-hosting"})),
    )
    .await
    {
        tracing::warn!(hosting_id = %sub.id, "Error registrando bypass hosting chatbot: {e}");
    }
    HostingRepository::find_by_id(pool, sub.id)
        .await
        .map_err(|e| {
            tracing::error!(hosting_id = %sub.id, "Error recargando hosting test chatbot: {e}");
            tool_error("No se pudo consultar el hosting creado")
        })?
        .ok_or_else(|| tool_error("Hosting creado no encontrado"))
}

pub(crate) fn tool_error(message: &str) -> ToolExecResult {
    tool_status("error", message)
}

fn hosting_requires_login() -> ToolExecResult {
    requires_login(
        "Para contratar hosting y generar checkout mensual debes iniciar sesión o crear una cuenta.",
    )
}

fn parse_hosting_checkout_request(args: &Value) -> Result<SelfSubscribeRequest, ToolExecResult> {
    let domain = args["domain"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let req = SelfSubscribeRequest {
        plan: args["plan"].as_str().unwrap_or("").trim().to_string(),
        domain,
        billing_cycle_months: Some(1),
        wp_admin_username: None,
        wp_admin_password: None,
        wp_language: None,
        sftp_user: None,
        sftp_password: None,
    };
    req.validate()
        .map(|()| req)
        .map_err(|e| tool_error(&format!("Datos de hosting inválidos: {e}")))
}

async fn fetch_hosting_plan_config(
    pool: &PgPool,
    plan: &str,
) -> Result<HostingPlanConfig, ToolExecResult> {
    match HostingRepository::get_plan_config(pool, plan).await {
        Ok(Some(config)) => Ok(config),
        Ok(None) => Err(tool_error(&format!("Plan de hosting inválido: {plan}"))),
        Err(e) => {
            tracing::error!("Error consultando plan hosting {plan}: {e}");
            Err(tool_error("No se pudo consultar el plan de hosting"))
        }
    }
}

pub(crate) async fn fetch_hosting_client(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(String, String), ToolExecResult> {
    match UserRepository::find_by_id(pool, user_id).await {
        Ok(Some(user)) => Ok((
            user.display_name.unwrap_or_else(|| user.email.clone()),
            user.email,
        )),
        Ok(None) => Err(tool_error("Usuario no encontrado")),
        Err(e) => {
            tracing::error!("Error consultando usuario {user_id}: {e}");
            Err(tool_error("No se pudo consultar el usuario"))
        }
    }
}

async fn create_chat_hosting_subscription(
    pool: &PgPool,
    user_id: Uuid,
    req: &SelfSubscribeRequest,
    plan_config: &HostingPlanConfig,
    client_name: &str,
    client_email: &str,
) -> Result<HostingSubscription, ToolExecResult> {
    let requested_domain = req
        .domain
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    let domain_verification_status = if requested_domain.is_some() {
        "pending_verification".to_string()
    } else {
        "none".to_string()
    };
    let domain_verification_token = requested_domain
        .as_ref()
        .map(|_| format!("nakomi-verification={}", Uuid::new_v4().simple()).to_ascii_lowercase());

    let sub = HostingRepository::create(
        pool,
        CreateHostingParams {
            user_id: Some(user_id),
            client_name,
            client_email,
            plan: &req.plan,
            domain: requested_domain.as_deref(),
            domain_verification_status: &domain_verification_status,
            domain_verification_token: domain_verification_token.as_deref(),
            domain_verified_at: None,
            runtime_kind: crate::services::HostingRuntimeService::runtime_kind_for_plan(&req.plan)
                .as_str(),
            deployment_id: None,
            coolify_site_name: None,
            monthly_price_cents: plan_config.monthly_price_cents,
            storage_limit_mb: plan_config.storage_limit_mb,
        },
    )
    .await
    .map_err(|e| {
        tracing::error!("Error creando suscripción hosting desde chatbot: {e}");
        tool_error("No se pudo crear la suscripción de hosting")
    })?;
    record_chat_hosting_event(pool, sub.id, user_id, &req.plan).await;
    Ok(sub)
}

async fn record_chat_hosting_event(pool: &PgPool, sub_id: Uuid, user_id: Uuid, plan: &str) {
    if let Err(e) = HostingRepository::add_event(
        pool,
        sub_id,
        "created",
        Some(json!({"plan": plan, "by": user_id.to_string(), "source": "chatbot-hosting"})),
    )
    .await
    {
        tracing::warn!("Error registrando evento chatbot-hosting para {sub_id}: {e}");
    }
}

async fn create_chat_hosting_checkout_url(
    http_client: &reqwest::Client,
    stripe_key: &str,
    sub: &HostingSubscription,
    client_email: &str,
) -> Result<String, ToolExecResult> {
    let base_url = chat_public_base_url();
    let success_url =
        format!("{base_url}/panel?hosting=success&session_id={{CHECKOUT_SESSION_ID}}");
    let cancel_url = format!("{base_url}/panel?hosting=cancelled");
    HostingStripeService::create_checkout_session(&CheckoutParams {
        http_client,
        stripe_key,
        subscription_id: sub.id,
        plan: &sub.plan,
        amount_cents: sub.monthly_price_cents,
        customer_email: client_email,
        success_url: &success_url,
        cancel_url: &cancel_url,
        billing_cycle_months: 1,
    })
    .await
    .map_err(|e| {
        tracing::error!(
            "Error creando checkout hosting desde chatbot para {}: {e}",
            sub.id
        );
        tool_error("No se pudo crear el checkout de hosting")
    })
}

fn hosting_checkout_success(sub: &HostingSubscription, checkout_url: &str) -> ToolExecResult {
    let product_name = if sub.plan.starts_with("normal-") {
        "Hosting"
    } else {
        "Hosting WordPress"
    };
    let description = format!("Suscripción mensual {product_name} {}", sub.plan);
    ToolExecResult {
        tool_result_json: json!({
            "status": "ok",
            "hosting_subscription_id": sub.id,
            "plan": sub.plan,
            "domain": sub.domain,
            "amount_cents": sub.monthly_price_cents,
            "message": "Checkout mensual de hosting creado. El cliente verá una tarjeta visual con botón de pago. NO repitas el link en texto."
        })
        .to_string(),
        rich_message: Some(RichMessage {
            content: format!(
                "Hosting {} — ${:.2} USD/mes",
                sub.plan,
                f64::from(sub.monthly_price_cents) / 100.0
            ),
            message_type: "invoice".to_string(),
            metadata: json!({
                "title": product_name,
                "payment_url": checkout_url,
                "amount_cents": sub.monthly_price_cents,
                "currency": "usd",
                "status": "open",
                "description": description,
                "hosting_subscription_id": sub.id,
                "plan": sub.plan,
                "domain": sub.domain,
            }),
        }),
    }
}

fn hosting_bypass_success(sub: &HostingSubscription) -> ToolExecResult {
    tool_json(json!({
        "status": "ok",
        "hosting_subscription_id": sub.id,
        "plan": sub.plan,
        "domain": sub.domain,
        "amount_cents": sub.monthly_price_cents,
        "bypassed": true,
        "message": "Suscripción de hosting creada para cuenta test sin cobro real. Indica al cliente que ya puede revisarla en su panel."
    }))
}
