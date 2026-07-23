/* sentinel-disable-file limite-lineas: servicio legacy de tools IA.
 * El archivo concentra múltiples tool definitions y su dispatcher para preservar la
 * interfaz del chatbot mientras se separa por dominio.
 */
/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: servicio legacy de tools IA.
 * Usa algunas queries runtime intencionalmente para no depender de caché offline en consultas dinámicas por rol.
 */
/* [T-2] Herramientas de IA para el chatbot (tool use / function calling).
 * Define las tools que la IA puede invocar y ejecuta las llamadas.
 * Groq soporta tool use compatible con OpenAI en llama-3.3-70b-versatile.
 * Cada tool retorna un resultado JSON que se reenvía a la IA + opcionalmente
 * un mensaje rico para el WS del visitante. */

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::models::{
    sanitize_hosting_event_details, HostingPlanConfig, HostingSubscription, SelfSubscribeRequest,
    SelfSubscribeVpsRequest, UserRole, VpsPlanConfig, VpsSubscription,
};
use crate::repositories::{
    ChatRepository, CreateHostingParams, CreateVpsSubscriptionParams, HostingRepository,
    ProblemRepository, UserRepository, VpsRepository,
};
use crate::services::{
    checkout_bypass_is_configured, is_checkout_bypass_email, CheckoutParams, HostingStripeService,
    VpsCheckoutParams, VpsStripeService, vps_stripe_fee_cents,
};

/* Resultado de ejecutar una tool: JSON para la IA y opcionalmente un
 * mensaje rico para mostrar en el chat del visitante. */
pub struct ToolExecResult {
    pub tool_result_json: String,
    pub rich_message: Option<RichMessage>,
}

/* Mensaje rico que se envía al WS como message_type + metadata.
 * content es el texto visible; message_type indica el render; metadata tiene datos extra. */
pub struct RichMessage {
    pub content: String,
    pub message_type: String,
    pub metadata: Value,
}

/* [095A-8] Contexto de ejecución de tools. Agrupa pool, HTTP, sesión y usuario
 * para no seguir ampliando firmas cada vez que una tool necesita contexto real. */
pub struct ToolExecutionContext<'a> {
    pub pool: &'a PgPool,
    pub http_client: &'a reqwest::Client,
    pub stripe_key: Option<&'a str>,
    pub visitor_id: Option<&'a str>,
    pub auth: Option<ToolAuthContext>,
    pub session_id: Uuid,
}

/* [095A-20] Identidad autenticada que acompaña cada tool call.
 * El prompt puede describir permisos, pero la autorización real vive aquí:
 * user_id = sujeto del JWT; role/effective_role = contrato firmado; impersonator = admin origen. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolAuthContext {
    pub user_id: Uuid,
    pub role: UserRole,
    pub effective_role: UserRole,
    pub impersonator: Option<Uuid>,
}

impl ToolAuthContext {
    #[must_use]
    pub const fn new(
        user_id: Uuid,
        role: UserRole,
        effective_role: UserRole,
        impersonator: Option<Uuid>,
    ) -> Self {
        Self {
            user_id,
            role,
            effective_role,
            impersonator,
        }
    }

    #[must_use]
    pub const fn is_effective_admin(self) -> bool {
        matches!(self.effective_role, UserRole::Admin)
    }
}

/* [T-2][T-3] Definiciones de tools en formato OpenAI/Groq.
 * Se incluyen en el body de la API junto con los mensajes.
 * Dividido en service_tools + visitor_tools para no exceder líneas. */
pub fn tool_definitions() -> Value {
    let mut tools = service_tool_defs();
    if let Some(arr) = tools.as_array_mut() {
        arr.extend(hosting_tool_defs().as_array().cloned().unwrap_or_default());
        arr.extend(vps_tool_defs().as_array().cloned().unwrap_or_default());
        arr.extend(visitor_tool_defs().as_array().cloned().unwrap_or_default());
        /* [T-9] Tools para clientes registrados */
        arr.extend(
            registered_client_tool_defs()
                .as_array()
                .cloned()
                .unwrap_or_default(),
        );
    }
    tools
}

/* [155A-12] Tools de VPS: asesoría de catálogo, consulta de VPS del cliente y checkout mensual.
 * No aprovisionan ni aprueban: la aprobación manual sigue protegida en el panel/admin. */
fn vps_tool_defs() -> Value {
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

/* [095A-8] Tools de hosting: asesoría con catálogo real, consulta de hostings del cliente
 * y checkout mensual Stripe. No ejecutan provisioning/restart/stop: eso sigue siendo admin-only. */
fn hosting_tool_defs() -> Value {
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

/* [084A-51] show_service y list_services removidos — las service cards son antinaturales
 * en un chat conversacional. Solo se mantienen tools de factura, escalación y captura. */
fn service_tool_defs() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "create_invoice",
                "description": "Genera una factura de Stripe con link de pago para el cliente. Solo úsalo cuando el cliente confirme que quiere pagar y tenga claro el servicio/monto.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "amount_cents": { "type": "integer", "description": "Monto en centavos USD (ej: 10000 = $100.00)" },
                        "description": { "type": "string", "description": "Descripción del concepto de la factura" },
                        "client_email": { "type": "string", "description": "Email del cliente para la factura de Stripe" }
                    },
                    "required": ["amount_cents", "description", "client_email"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "request_human_assistance",
                "description": "Solicita intervención humana. Úsalo cuando no puedas resolver la solicitud, el cliente esté frustrado, pida hablar con una persona, o el tema sea legal/contractual.",
                "parameters": {
                    "type": "object",
                    "properties": { "reason": { "type": "string", "description": "Motivo breve de la escalación" } },
                    "required": ["reason"]
                }
            }
        }
    ])
}

/* Tools de captura de email e info del cliente (T-3) */
fn visitor_tool_defs() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "capture_email",
                "description": "Guarda el email del cliente. Úsalo cuando el cliente comparta su correo electrónico durante la conversación. No lo pidas de forma forzada — espera a que surja naturalmente o cuando sea necesario para una factura.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "email": { "type": "string", "description": "Email del cliente" },
                        "display_name": { "type": "string", "description": "Nombre del cliente (si lo mencionó)" }
                    },
                    "required": ["email"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "save_client_info",
                "description": "Guarda información relevante del cliente para futuras conversaciones (industria, presupuesto, intereses, tipo de proyecto, nombre). Úsalo cuando el cliente mencione datos útiles sobre su negocio, necesidades, o cuando dé su nombre.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Nombre del visitante. Úsalo cuando el cliente diga su nombre" },
                        "industry": { "type": "string", "description": "Sector o industria del cliente" },
                        "budget_range": { "type": "string", "description": "Rango de presupuesto mencionado" },
                        "interests": { "type": "array", "items": { "type": "string" }, "description": "Servicios o temas de interés" },
                        "project_description": { "type": "string", "description": "Descripción breve del proyecto que necesita" },
                        "notes": { "type": "string", "description": "Cualquier otra info relevante" }
                    }
                }
            }
        }
    ])
}

/* Ejecutar una tool call retornada por la IA.
 * Despacha al handler correcto según el nombre de la función.
 * visitor_id necesario para tools que actualizan visitor_profiles (T-3).
 * [124A-CHAT2] session_id necesario para actualizar visitor_name en chat_sessions al capturar nombre. */
pub async fn execute_tool(
    ctx: ToolExecutionContext<'_>,
    tool_name: &str,
    arguments: &Value,
) -> ToolExecResult {
    match tool_name {
        "create_invoice" => {
            exec_create_invoice(ctx.http_client, ctx.stripe_key, ctx.session_id, arguments).await
        }
        "list_hosting_plans" => exec_list_hosting_plans(ctx.pool).await,
        "list_my_hostings" => exec_list_my_hostings(ctx.pool, ctx.auth).await,
        "create_hosting_checkout" => exec_create_hosting_checkout(&ctx, arguments).await,
        "list_vps_plans" => exec_list_vps_plans(ctx.pool).await,
        "list_my_vps" => exec_list_my_vps(ctx.pool, ctx.auth).await,
        "create_vps_checkout" => exec_create_vps_checkout(&ctx, arguments).await,
        "list_my_orders" => exec_list_my_orders(ctx.pool, ctx.auth).await,
        "list_my_payments" => exec_list_my_payments(ctx.pool, ctx.auth).await,
        "list_my_reports" => exec_list_my_reports(ctx.pool, ctx.auth).await,
        "create_order_report" => exec_create_order_report(ctx.pool, ctx.auth, arguments).await,
        "admin_operational_summary" => exec_admin_operational_summary(ctx.pool, ctx.auth).await,
        "request_human_assistance" => exec_request_human(arguments),
        "capture_email" => {
            exec_capture_email(ctx.pool, ctx.visitor_id, ctx.session_id, arguments).await
        }
        "save_client_info" => {
            exec_save_client_info(ctx.pool, ctx.visitor_id, ctx.session_id, arguments).await
        }
        "create_support_ticket" => {
            exec_create_support_ticket(ctx.pool, ctx.auth, ctx.visitor_id, arguments).await
        }
        _ => tool_status("error", "Tool desconocida"),
    }
}

fn tool_json(payload: impl serde::Serialize) -> ToolExecResult {
    ToolExecResult {
        tool_result_json: serde_json::to_string(&payload).unwrap_or_else(|e| {
            tracing::error!("Error serializando resultado de AI tool: {e}");
            json!({"status": "error", "message": "No se pudo serializar el resultado"}).to_string()
        }),
        rich_message: None,
    }
}

fn tool_status(status: &str, message: &str) -> ToolExecResult {
    tool_json(json!({"status": status, "message": message}))
}

fn requires_login(message: &str) -> ToolExecResult {
    tool_status("requires_login", message)
}

fn forbidden(message: &str) -> ToolExecResult {
    tool_status("forbidden", message)
}

fn not_found(message: &str) -> ToolExecResult {
    tool_status("not_found", message)
}

fn require_auth(auth: Option<ToolAuthContext>) -> Result<ToolAuthContext, ToolExecResult> {
    auth.ok_or_else(|| {
        requires_login("Para consultar datos de tu cuenta necesitas iniciar sesión.")
    })
}

fn can_access_order(
    auth: ToolAuthContext,
    client_id: Uuid,
    assigned_employee_id: Option<Uuid>,
) -> bool {
    match auth.effective_role {
        UserRole::Admin => true,
        UserRole::Employee => assigned_employee_id == Some(auth.user_id),
        UserRole::Client => client_id == auth.user_id,
    }
}

async fn exec_list_hosting_plans(pool: &PgPool) -> ToolExecResult {
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

async fn exec_list_my_hostings(pool: &PgPool, auth: Option<ToolAuthContext>) -> ToolExecResult {
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

fn chat_public_base_url() -> String {
    std::env::var("GLORY_PUBLIC_URL")
        .or_else(|_| std::env::var("PUBLIC_URL"))
        .unwrap_or_else(|_| "https://nakomi.studio".to_string())
        .trim_end_matches('/')
        .to_string()
}

async fn exec_create_hosting_checkout(
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

fn tool_error(message: &str) -> ToolExecResult {
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

async fn fetch_hosting_client(
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

async fn exec_list_vps_plans(pool: &PgPool) -> ToolExecResult {
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

async fn exec_list_my_vps(pool: &PgPool, auth: Option<ToolAuthContext>) -> ToolExecResult {
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

async fn exec_create_vps_checkout(ctx: &ToolExecutionContext<'_>, args: &Value) -> ToolExecResult {
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

#[derive(sqlx::FromRow)]
struct ToolOrderRow {
    id: Uuid,
    order_number: i32,
    client_id: Uuid,
    service_title: String,
    service_slug: String,
    plan_name: String,
    status: String,
    current_phase: i32,
    total_phases: i64,
    final_price_cents: i32,
    currency: String,
    assigned_employee_id: Option<Uuid>,
    assigned_employee_name: Option<String>,
    open_reports: i64,
    deliverables_count: i64,
    phases: Value,
    updated_at: DateTime<Utc>,
}

async fn exec_list_my_orders(pool: &PgPool, auth: Option<ToolAuthContext>) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    match query_orders_for_scope(pool, auth).await {
        Ok(orders) => tool_json(json!({
            "status": "ok",
            "scope": auth.effective_role.to_string(),
            "orders": orders.iter().map(order_row_json).collect::<Vec<_>>(),
        })),
        Err(e) => {
            tracing::error!(
                user_id = %auth.user_id,
                effective_role = %auth.effective_role,
                "Error listando pedidos para AI tool: {e}"
            );
            tool_status("error", "No se pudieron consultar los pedidos")
        }
    }
}

async fn query_orders_for_scope(
    pool: &PgPool,
    auth: ToolAuthContext,
) -> Result<Vec<ToolOrderRow>, sqlx::Error> {
    match auth.effective_role {
        UserRole::Admin => {
            let query = order_admin_query();
            sqlx::query_as::<_, ToolOrderRow>(&query)
                .fetch_all(pool)
                .await
        }
        UserRole::Employee => {
            let query = order_employee_query();
            sqlx::query_as::<_, ToolOrderRow>(&query)
                .bind(auth.user_id)
                .fetch_all(pool)
                .await
        }
        UserRole::Client => {
            let query = order_client_query();
            sqlx::query_as::<_, ToolOrderRow>(&query)
                .bind(auth.user_id)
                .fetch_all(pool)
                .await
        }
    }
}

fn order_row_json(row: &ToolOrderRow) -> Value {
    json!({
        "id": row.id,
        "order_number": row.order_number,
        "client_id": row.client_id,
        "service_title": row.service_title,
        "service_slug": row.service_slug,
        "plan_name": row.plan_name,
        "status": row.status,
        "current_phase": row.current_phase,
        "total_phases": row.total_phases,
        "final_price_cents": row.final_price_cents,
        "currency": row.currency,
        "assigned_employee_id": row.assigned_employee_id,
        "assigned_employee_name": row.assigned_employee_name,
        "open_reports": row.open_reports,
        "deliverables_count": row.deliverables_count,
        "phases": row.phases,
        "updated_at": row.updated_at,
    })
}

const ORDER_SELECT_BASE: &str = "
SELECT o.id,
       o.order_number,
       o.client_id,
       s.title AS service_title,
       s.slug AS service_slug,
       sp.name AS plan_name,
       o.status::text AS status,
       o.current_phase,
       (SELECT COUNT(*) FROM order_phases op WHERE op.order_id = o.id) AS total_phases,
       o.final_price_cents,
       o.currency,
       o.assigned_employee_id,
       COALESCE(employee.display_name, employee.email) AS assigned_employee_name,
       (SELECT COUNT(*) FROM order_problems prob WHERE prob.order_id = o.id AND prob.status IN ('open', 'in_review')) AS open_reports,
       (SELECT COUNT(*) FROM phase_deliverables d JOIN order_phases phase ON phase.id = d.phase_id WHERE phase.order_id = o.id) AS deliverables_count,
       COALESCE((
           SELECT jsonb_agg(jsonb_build_object(
               'phase_number', phase.phase_number,
               'title', phase.title,
               'status', phase.status::text,
               'revisions_used', phase.revisions_used,
               'max_revisions', phase.max_revisions,
               'deadline', phase.deadline
           ) ORDER BY phase.phase_number)
           FROM order_phases phase
           WHERE phase.order_id = o.id
       ), '[]'::jsonb) AS phases,
       o.updated_at
FROM orders o
JOIN services s ON s.id = o.service_id
JOIN service_plans sp ON sp.id = o.plan_id
LEFT JOIN users employee ON employee.id = o.assigned_employee_id";

fn order_admin_query() -> String {
    format!("{ORDER_SELECT_BASE} ORDER BY o.updated_at DESC LIMIT 12")
}

fn order_employee_query() -> String {
    format!(
        "{ORDER_SELECT_BASE} WHERE o.assigned_employee_id = $1 ORDER BY o.updated_at DESC LIMIT 12"
    )
}

fn order_client_query() -> String {
    format!("{ORDER_SELECT_BASE} WHERE o.client_id = $1 ORDER BY o.updated_at DESC LIMIT 12")
}

#[derive(sqlx::FromRow)]
struct ToolPaymentRow {
    payment_id: Uuid,
    order_id: Uuid,
    order_number: i32,
    phase_number: Option<i32>,
    amount_cents: i32,
    currency: String,
    status: String,
    payment_mode: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
}

async fn exec_list_my_payments(pool: &PgPool, auth: Option<ToolAuthContext>) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    match query_payments_for_scope(pool, auth).await {
        Ok(payments) => tool_json(json!({
            "status": "ok",
            "scope": auth.effective_role.to_string(),
            "payments": payments.iter().map(payment_row_json).collect::<Vec<_>>(),
        })),
        Err(e) => {
            tracing::error!(
                user_id = %auth.user_id,
                effective_role = %auth.effective_role,
                "Error listando pagos para AI tool: {e}"
            );
            tool_status("error", "No se pudieron consultar los pagos")
        }
    }
}

async fn query_payments_for_scope(
    pool: &PgPool,
    auth: ToolAuthContext,
) -> Result<Vec<ToolPaymentRow>, sqlx::Error> {
    match auth.effective_role {
        UserRole::Admin => {
            let query = payment_admin_query();
            sqlx::query_as::<_, ToolPaymentRow>(&query)
                .fetch_all(pool)
                .await
        }
        UserRole::Employee => {
            let query = payment_employee_query();
            sqlx::query_as::<_, ToolPaymentRow>(&query)
                .bind(auth.user_id)
                .fetch_all(pool)
                .await
        }
        UserRole::Client => {
            let query = payment_client_query();
            sqlx::query_as::<_, ToolPaymentRow>(&query)
                .bind(auth.user_id)
                .fetch_all(pool)
                .await
        }
    }
}

fn payment_row_json(row: &ToolPaymentRow) -> Value {
    json!({
        "payment_id": row.payment_id,
        "order_id": row.order_id,
        "order_number": row.order_number,
        "phase_number": row.phase_number,
        "amount_cents": row.amount_cents,
        "currency": row.currency,
        "status": row.status,
        "payment_mode": row.payment_mode,
        "description": row.description,
        "created_at": row.created_at,
        "can_retry_from_panel": row.status == "pending" || row.status == "failed",
    })
}

const PAYMENT_SELECT_BASE: &str = "
SELECT p.id AS payment_id,
       p.order_id,
       o.order_number,
       phase.phase_number,
       p.amount_cents,
       p.currency,
       p.status::text AS status,
       p.payment_mode::text AS payment_mode,
       p.description,
       p.created_at
FROM order_payments p
JOIN orders o ON o.id = p.order_id
LEFT JOIN order_phases phase ON phase.id = p.phase_id";

fn payment_admin_query() -> String {
    format!("{PAYMENT_SELECT_BASE} ORDER BY p.created_at DESC LIMIT 20")
}

fn payment_employee_query() -> String {
    format!("{PAYMENT_SELECT_BASE} WHERE o.assigned_employee_id = $1 ORDER BY p.created_at DESC LIMIT 20")
}

fn payment_client_query() -> String {
    format!("{PAYMENT_SELECT_BASE} WHERE o.client_id = $1 ORDER BY p.created_at DESC LIMIT 20")
}

#[derive(sqlx::FromRow)]
struct ToolReportRow {
    id: Uuid,
    order_id: Uuid,
    order_number: i32,
    reporter_id: Uuid,
    reporter_name: String,
    reporter_role: String,
    reason: String,
    status: String,
    admin_response: Option<String>,
    created_at: DateTime<Utc>,
}

async fn exec_list_my_reports(pool: &PgPool, auth: Option<ToolAuthContext>) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    match query_reports_for_scope(pool, auth).await {
        Ok(reports) => tool_json(json!({
            "status": "ok",
            "scope": auth.effective_role.to_string(),
            "reports": reports.iter().map(report_row_json).collect::<Vec<_>>(),
        })),
        Err(e) => {
            tracing::error!(
                user_id = %auth.user_id,
                effective_role = %auth.effective_role,
                "Error listando reportes para AI tool: {e}"
            );
            tool_status("error", "No se pudieron consultar los reportes")
        }
    }
}

async fn query_reports_for_scope(
    pool: &PgPool,
    auth: ToolAuthContext,
) -> Result<Vec<ToolReportRow>, sqlx::Error> {
    match auth.effective_role {
        UserRole::Admin => {
            let query = report_admin_query();
            sqlx::query_as::<_, ToolReportRow>(&query)
                .fetch_all(pool)
                .await
        }
        UserRole::Employee => {
            let query = report_employee_query();
            sqlx::query_as::<_, ToolReportRow>(&query)
                .bind(auth.user_id)
                .fetch_all(pool)
                .await
        }
        UserRole::Client => {
            let query = report_client_query();
            sqlx::query_as::<_, ToolReportRow>(&query)
                .bind(auth.user_id)
                .fetch_all(pool)
                .await
        }
    }
}

fn report_row_json(row: &ToolReportRow) -> Value {
    json!({
        "id": row.id,
        "order_id": row.order_id,
        "order_number": row.order_number,
        "reporter_id": row.reporter_id,
        "reporter_name": row.reporter_name,
        "reporter_role": row.reporter_role,
        "reason": row.reason,
        "status": row.status,
        "admin_response": row.admin_response,
        "created_at": row.created_at,
    })
}

const REPORT_SELECT_BASE: &str = "
SELECT prob.id,
       prob.order_id,
       o.order_number,
       prob.reporter_id,
       COALESCE(reporter.display_name, reporter.email) AS reporter_name,
       prob.reporter_role,
       prob.reason,
       prob.status::text AS status,
       prob.admin_response,
       prob.created_at
FROM order_problems prob
JOIN orders o ON o.id = prob.order_id
JOIN users reporter ON reporter.id = prob.reporter_id";

fn report_admin_query() -> String {
    format!("{REPORT_SELECT_BASE} ORDER BY prob.created_at DESC LIMIT 20")
}

fn report_employee_query() -> String {
    format!("{REPORT_SELECT_BASE} WHERE o.assigned_employee_id = $1 ORDER BY prob.created_at DESC LIMIT 20")
}

fn report_client_query() -> String {
    format!("{REPORT_SELECT_BASE} WHERE o.client_id = $1 ORDER BY prob.created_at DESC LIMIT 20")
}

#[derive(sqlx::FromRow)]
struct OrderAccessRow {
    id: Uuid,
    order_number: i32,
    client_id: Uuid,
    assigned_employee_id: Option<Uuid>,
}

async fn exec_create_order_report(
    pool: &PgPool,
    auth: Option<ToolAuthContext>,
    args: &Value,
) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    let reason = args["reason"].as_str().unwrap_or("").trim();
    if !(10..=2000).contains(&reason.chars().count()) {
        return tool_status(
            "error",
            "El reporte necesita una descripción entre 10 y 2000 caracteres.",
        );
    }
    let order = match resolve_order_for_report(pool, auth, args).await {
        Ok(order) => order,
        Err(result) => return result,
    };
    match ProblemRepository::create(
        pool,
        order.id,
        auth.user_id,
        &auth.effective_role.to_string(),
        reason,
    )
    .await
    {
        Ok(problem) => tool_json(json!({
            "status": "ok",
            "report_id": problem.id,
            "order_id": order.id,
            "order_number": order.order_number,
            "report_status": problem.status,
            "message": "Reporte creado. El equipo lo revisará desde el panel."
        })),
        Err(e) => {
            tracing::error!(
                user_id = %auth.user_id,
                order_id = %order.id,
                "Error creando reporte desde AI tool: {e}"
            );
            tool_status("error", "No se pudo crear el reporte")
        }
    }
}

async fn resolve_order_for_report(
    pool: &PgPool,
    auth: ToolAuthContext,
    args: &Value,
) -> Result<OrderAccessRow, ToolExecResult> {
    let row = if let Some(order_id) = args["order_id"]
        .as_str()
        .and_then(|v| Uuid::parse_str(v).ok())
    {
        find_order_access_by_id(pool, order_id).await
    } else if let Some(order_number) = args["order_number"]
        .as_i64()
        .and_then(|v| i32::try_from(v).ok())
    {
        find_order_access_by_number(pool, order_number).await
    } else {
        return Err(tool_status(
            "error",
            "Indica order_id u order_number para crear el reporte.",
        ));
    }
    .map_err(|e| {
        tracing::error!("Error resolviendo pedido para reporte AI: {e}");
        tool_status("error", "No se pudo verificar el pedido")
    })?
    .ok_or_else(|| not_found("Pedido no encontrado"))?;

    if can_access_order(auth, row.client_id, row.assigned_employee_id) {
        Ok(row)
    } else {
        Err(forbidden(
            "No tienes permisos para crear reportes sobre ese pedido.",
        ))
    }
}

async fn find_order_access_by_id(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<Option<OrderAccessRow>, sqlx::Error> {
    sqlx::query_as::<_, OrderAccessRow>(ORDER_ACCESS_SELECT_ID)
        .bind(order_id)
        .fetch_optional(pool)
        .await
}

async fn find_order_access_by_number(
    pool: &PgPool,
    order_number: i32,
) -> Result<Option<OrderAccessRow>, sqlx::Error> {
    sqlx::query_as::<_, OrderAccessRow>(ORDER_ACCESS_SELECT_NUMBER)
        .bind(order_number)
        .fetch_optional(pool)
        .await
}

const ORDER_ACCESS_SELECT_ID: &str = "
SELECT id, order_number, client_id, assigned_employee_id
FROM orders
WHERE id = $1";
const ORDER_ACCESS_SELECT_NUMBER: &str = "
SELECT id, order_number, client_id, assigned_employee_id
FROM orders
WHERE order_number = $1";

#[derive(sqlx::FromRow)]
struct AdminOrderStats {
    total: i64,
    payment_held: i64,
    awaiting_assignment: i64,
    in_progress: i64,
    under_review: i64,
    completed: i64,
    disputed: i64,
}

#[derive(sqlx::FromRow)]
struct AdminReportStats {
    open_reports: i64,
    in_review_reports: i64,
}

#[derive(sqlx::FromRow)]
struct StatusCountRow {
    status: String,
    total: i64,
}

async fn exec_admin_operational_summary(
    pool: &PgPool,
    auth: Option<ToolAuthContext>,
) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    if !auth.is_effective_admin() {
        return forbidden(
            "Solo un administrador efectivo puede consultar el resumen operativo global.",
        );
    }

    let order_stats = match query_admin_order_stats(pool).await {
        Ok(stats) => stats,
        Err(e) => {
            tracing::error!(user_id = %auth.user_id, "Error consultando stats admin de pedidos: {e}");
            return tool_status("error", "No se pudo consultar el resumen operativo");
        }
    };
    let report_stats = query_admin_report_stats(pool)
        .await
        .unwrap_or(AdminReportStats {
            open_reports: 0,
            in_review_reports: 0,
        });
    let hosting_by_status = query_hosting_status_counts(pool).await.unwrap_or_default();

    tool_json(json!({
        "status": "ok",
        "orders": {
            "total": order_stats.total,
            "payment_held": order_stats.payment_held,
            "awaiting_assignment": order_stats.awaiting_assignment,
            "in_progress": order_stats.in_progress,
            "under_review": order_stats.under_review,
            "completed": order_stats.completed,
            "disputed": order_stats.disputed,
        },
        "reports": {
            "open": report_stats.open_reports,
            "in_review": report_stats.in_review_reports,
        },
        "hosting_by_status": hosting_by_status.into_iter().map(|row| json!({
            "status": row.status,
            "total": row.total,
        })).collect::<Vec<_>>(),
    }))
}

async fn query_admin_order_stats(pool: &PgPool) -> Result<AdminOrderStats, sqlx::Error> {
    sqlx::query_as::<_, AdminOrderStats>(
        "SELECT COUNT(*) AS total,
            COUNT(*) FILTER (WHERE status = 'payment_held') AS payment_held,
            COUNT(*) FILTER (WHERE status = 'awaiting_assignment') AS awaiting_assignment,
            COUNT(*) FILTER (WHERE status = 'in_progress') AS in_progress,
            COUNT(*) FILTER (WHERE status = 'under_review') AS under_review,
            COUNT(*) FILTER (WHERE status = 'completed') AS completed,
            COUNT(*) FILTER (WHERE status = 'disputed') AS disputed
         FROM orders",
    )
    .fetch_one(pool)
    .await
}

async fn query_admin_report_stats(pool: &PgPool) -> Result<AdminReportStats, sqlx::Error> {
    sqlx::query_as::<_, AdminReportStats>(
        "SELECT COUNT(*) FILTER (WHERE status = 'open') AS open_reports,
                COUNT(*) FILTER (WHERE status = 'in_review') AS in_review_reports
         FROM order_problems",
    )
    .fetch_one(pool)
    .await
}

async fn query_hosting_status_counts(pool: &PgPool) -> Result<Vec<StatusCountRow>, sqlx::Error> {
    sqlx::query_as::<_, StatusCountRow>(
        "SELECT status, COUNT(*) AS total
         FROM hosting_subscriptions
         GROUP BY status
         ORDER BY status",
    )
    .fetch_all(pool)
    .await
}

/* [084A-51] exec_show_service y exec_list_services eliminados — herramientas desactivadas.
 * Las service cards son antinaturales en el chat conversacional. Si se reactivan en el futuro,
 * recuperar implementación del git history (commit anterior a 084A-51). */

/* create_invoice: crea factura en Stripe con link de pago.
 * Flujo: create customer → create invoice → add line item → finalize → get URL.
 * El resultado es un mensaje de tipo "invoice" con el link de pago.
 * [124A-INV] session_id se guarda en metadata de Stripe para detectar pago
 * via webhook invoice.paid y notificar al admin/cliente. */
async fn exec_create_invoice(
    http_client: &reqwest::Client,
    stripe_key: Option<&str>,
    session_id: uuid::Uuid,
    args: &Value,
) -> ToolExecResult {
    let Some(stripe_key) = stripe_key else {
        return tool_status("error", "Stripe no configurado");
    };

    /* [084A-52] Parsing robusto: algunos modelos envían amount_cents como float
     * (ej: 10000.0 en vez de 10000). Intentar i64, luego f64→i64. */
    #[allow(clippy::cast_possible_truncation)]
    let amount_cents = args["amount_cents"]
        .as_i64()
        .or_else(|| args["amount_cents"].as_f64().map(|f| f as i64))
        .unwrap_or(0);
    let description = args["description"]
        .as_str()
        .unwrap_or("Servicio Nakomi Studio");
    let client_email = args["client_email"].as_str().unwrap_or("");

    if amount_cents <= 0 || client_email.is_empty() {
        /* [084A-52] Log detallado para diagnosticar qué parámetro falta.
         * Causa común: Gemini retorna arguments como objeto en vez de string,
         * y el parsing ignoraba ese caso (ya corregido en ai_chat.rs). */
        tracing::error!(
            "create_invoice args inválidos: amount_cents={amount_cents}, \
             email_empty={}, args_raw={args}",
            client_email.is_empty()
        );
        return tool_json(json!({
            "status": "error",
            "error": "Monto y email son requeridos",
            "detail": format!(
                "amount_cents={amount_cents}, client_email={}",
                if client_email.is_empty() { "(vacío)" } else { "(presente)" }
            )
        }));
    }

    /* Paso 1: crear/buscar customer por email */
    let customer_id =
        match find_or_create_stripe_customer(http_client, stripe_key, client_email).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Stripe create customer error: {e}");
                return tool_status("error", "Error creando cliente en Stripe");
            }
        };

    /* Paso 2: crear invoice con session_id en metadata para detectar pago via webhook */
    let invoice = match create_stripe_invoice(
        http_client,
        stripe_key,
        &customer_id,
        client_email,
        description,
        amount_cents,
        session_id,
    )
    .await
    {
        Ok(inv) => inv,
        Err(e) => {
            tracing::error!("Stripe create invoice error: {e}");
            return tool_status("error", "Error creando factura");
        }
    };

    #[allow(clippy::cast_precision_loss)]
    let price_usd = amount_cents as f64 / 100.0;
    let metadata = json!({
        "stripe_invoice_id": invoice.id,
        "amount_cents": amount_cents,
        "currency": "usd",
        "status": invoice.status,
        "payment_url": invoice.hosted_invoice_url,
        "description": description,
    });

    ToolExecResult {
        /* [084A-38] No incluir payment_url en tool_result_json — la IA lo repite en texto plano.
         * El link de pago solo va en la RichMessage metadata (la card lo muestra con botón). */
        tool_result_json: json!({
            "status": "ok",
            "invoice_id": invoice.id,
            "amount_usd": price_usd,
            "invoice_status": invoice.status,
            "message": "Factura creada exitosamente. El cliente verá una tarjeta visual con botón de pago. NO repitas el link de pago en tu respuesta."
        })
        .to_string(),
        rich_message: Some(RichMessage {
            content: format!("Factura por ${price_usd:.2} USD — {description}"),
            message_type: "invoice".to_string(),
            metadata,
        }),
    }
}

/* [237A-7g] request_human_assistance: marca escalación + genera CTA de WhatsApp.
 * El resultado para la IA indica que se escaló. El rich_message contact_cta
 * muestra al cliente un botón para escribir por WhatsApp directamente.
 * La URL se construye desde PUBLIC_SUPPORT_WHATSAPP (número público del soporte). */
fn exec_request_human(args: &Value) -> ToolExecResult {
    let reason = args["reason"].as_str().unwrap_or("Sin motivo especificado");

    /* Construir CTA de WhatsApp si el número público está configurado */
    let support_whatsapp = std::env::var("PUBLIC_SUPPORT_WHATSAPP")
        .ok()
        .filter(|s| !s.is_empty());

    let rich_message = support_whatsapp.and_then(|raw_number| {
        /* Normalizar a dígitos para wa.me — requiere al menos 7 dígitos (número real) */
        let digits: String = raw_number.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() < 7 {
            tracing::warn!("PUBLIC_SUPPORT_WHATSAPP no tiene dígitos suficientes: {raw_number}");
            return None;
        }
        let prefill = "Hola, necesito ayuda con mi consulta.";
        let href = format!("https://wa.me/{digits}?text={}", urlencoding::encode(prefill));

        Some(RichMessage {
            content: "Este caso necesita atención personal.".to_string(),
            message_type: "contact_cta".to_string(),
            metadata: json!({
                "label": "Escribir por WhatsApp",
                "href": href,
                "fallback": "El equipo fue notificado y responderá por este chat.",
                "reason": reason,
            }),
        })
    });

    ToolExecResult {
        tool_result_json: json!({
            "status": "escalated",
            "reason": reason,
            "message": "Se ha notificado al equipo. Un especialista se conectará pronto."
        })
        .to_string(),
        rich_message,
    }
}

/* ============================================================
STRIPE HELPERS (invoice flow)
============================================================ */

#[derive(Debug, serde::Deserialize)]
struct StripeCustomerSearch {
    data: Vec<StripeCustomer>,
}

#[derive(Debug, serde::Deserialize)]
struct StripeCustomer {
    id: String,
}

#[derive(Debug, serde::Deserialize)]
struct StripeInvoice {
    id: String,
    status: Option<String>,
    hosted_invoice_url: Option<String>,
}

/* Busca customer por email en Stripe. Si no existe, lo crea. */
async fn find_or_create_stripe_customer(
    client: &reqwest::Client,
    key: &str,
    email: &str,
) -> Result<String, String> {
    /* Buscar por email */
    let search_resp = client
        .get("https://api.stripe.com/v1/customers/search")
        .header("Authorization", format!("Bearer {key}"))
        .query(&[("query", &format!("email:'{email}'"))])
        .send()
        .await
        .map_err(|e| format!("Stripe search error: {e}"))?;

    if search_resp.status().is_success() {
        let body: StripeCustomerSearch = search_resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {e}"))?;
        if let Some(cust) = body.data.first() {
            return Ok(cust.id.clone());
        }
    }

    /* Crear nuevo customer */
    let create_resp = client
        .post("https://api.stripe.com/v1/customers")
        .header("Authorization", format!("Bearer {key}"))
        .form(&[("email", email)])
        .send()
        .await
        .map_err(|e| format!("Stripe create error: {e}"))?;

    if !create_resp.status().is_success() {
        let text = create_resp.text().await.unwrap_or_default();
        return Err(format!("Stripe create customer failed: {text}"));
    }

    let cust: StripeCustomer = create_resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {e}"))?;
    Ok(cust.id)
}

/* Crea invoice en Stripe: invoice + line item + finalize.
 * Retorna invoice con hosted_invoice_url (link de pago).
 * [124A-INV] session_id y client_email van en metadata para detectar pago chat en webhook. */
async fn create_stripe_invoice(
    client: &reqwest::Client,
    key: &str,
    customer_id: &str,
    client_email: &str,
    description: &str,
    amount_cents: i64,
    session_id: uuid::Uuid,
) -> Result<StripeInvoice, String> {
    /* Crear invoice draft — currency explícito para evitar conflicto con
     * el default de la cuenta Stripe (puede ser MXN u otra moneda local).
     * [124A-INV] metadata[session_id] + metadata[client_email] para webhook invoice.paid. */
    let inv_resp = client
        .post("https://api.stripe.com/v1/invoices")
        .header("Authorization", format!("Bearer {key}"))
        .form(&[
            ("customer", customer_id),
            ("collection_method", "send_invoice"),
            ("days_until_due", "7"),
            ("auto_advance", "true"),
            ("currency", "usd"),
            ("metadata[session_id]", session_id.to_string().as_str()),
            ("metadata[client_email]", client_email),
            ("metadata[source]", "chat_invoice"),
        ])
        .send()
        .await
        .map_err(|e| format!("Create invoice error: {e}"))?;

    if !inv_resp.status().is_success() {
        let text = inv_resp.text().await.unwrap_or_default();
        return Err(format!("Create invoice failed: {text}"));
    }

    let draft: StripeInvoice = inv_resp
        .json()
        .await
        .map_err(|e| format!("Parse invoice error: {e}"))?;

    /* [084A-38] Agregar line item — VALIDAR respuesta. Sin line item, el invoice
     * se finaliza con $0.00 y Stripe lo marca como "paid" inmediatamente. */
    let item_resp = client
        .post("https://api.stripe.com/v1/invoiceitems")
        .header("Authorization", format!("Bearer {key}"))
        .form(&[
            ("customer", customer_id),
            ("invoice", draft.id.as_str()),
            ("amount", &amount_cents.to_string()),
            ("currency", "usd"),
            ("description", description),
        ])
        .send()
        .await
        .map_err(|e| format!("Create line item error: {e}"))?;

    if !item_resp.status().is_success() {
        let text = item_resp.text().await.unwrap_or_default();
        return Err(format!("Line item creation failed: {text}"));
    }

    /* Finalizar invoice (genera hosted_invoice_url) */
    let final_resp = client
        .post(format!(
            "https://api.stripe.com/v1/invoices/{}/finalize",
            draft.id
        ))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .map_err(|e| format!("Finalize invoice error: {e}"))?;

    if !final_resp.status().is_success() {
        let text = final_resp.text().await.unwrap_or_default();
        return Err(format!("Finalize invoice failed: {text}"));
    }

    let finalized: StripeInvoice = final_resp
        .json()
        .await
        .map_err(|e| format!("Parse finalized error: {e}"))?;

    Ok(finalized)
}

/* ============================================================
VISITOR PROFILE TOOLS (T-3 — Memoria y contexto)
============================================================ */

/* [124A-CHAT2] Helper: actualiza visitor_name en chat_sessions para que el panel
 * muestre el nombre real del visitante capturado por la IA.
 * Usa sqlx::query (sin macro) para evitar requerir caché offline. */
async fn update_session_visitor_name(
    pool: &PgPool,
    session_id: uuid::Uuid,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE chat_sessions SET visitor_name = $2, updated_at = NOW() WHERE id = $1")
        .bind(session_id)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(())
}

/* [T-3] capture_email: guarda email del visitante en visitor_profiles.
 * También actualiza display_name si lo proporcionó.
 * [124A-CHAT2] Actualiza visitor_name en chat_sessions para que el panel muestre el nombre real. */
/* [237A-10] Validación de email: formato básico RFC 5322 simplificado.
 * No es exhaustivo (DNS/MX check queda fuera), pero filtra la mayoría de
 * entradas inválidas que la IA pueda aceptar por error. */
fn is_valid_email(email: &str) -> bool {
    let trimmed = email.trim().to_lowercase();
    if trimmed.len() < 5 || trimmed.len() > 254 {
        return false;
    }
    /* Debe tener exactamente un @ con texto antes y después */
    let parts: Vec<&str> = trimmed.splitn(2, '@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    /* Dominio debe tener al menos un punto y no empezar/terminar con punto */
    if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }
    /* Local no puede empezar/terminar con punto ni tener dos puntos seguidos */
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }
    true
}

async fn exec_capture_email(
    pool: &PgPool,
    visitor_id: Option<&str>,
    session_id: uuid::Uuid,
    args: &Value,
) -> ToolExecResult {
    let Some(vid) = visitor_id else {
        return tool_status("error", "visitor_id no disponible");
    };

    let raw_email = args["email"].as_str().unwrap_or("");
    if raw_email.is_empty() {
        return tool_status("error", "Email no proporcionado");
    }

    /* [237A-10] Validación real de formato email */
    if !is_valid_email(raw_email) {
        return tool_status("error", "El email proporcionado no tiene un formato válido.");
    }

    let email_normalized = raw_email.trim().to_lowercase();
    let display_name = args["display_name"].as_str();

    /* Si la IA nos da el nombre junto con el email, actualizar visitor_name en la sesión */
    if let Some(name) = display_name {
        let _ = update_session_visitor_name(pool, session_id, name).await;
    }

    match ChatRepository::update_visitor_email(pool, vid, &email_normalized, display_name).await {
        Ok(profile) => {
            /* [237A-10] Guardar email normalizado + timestamp de captura + source */
            let _ = sqlx::query(
                "UPDATE visitor_profiles SET \
                   email_normalized = $2, \
                   email_captured_at = NOW(), \
                   continuation_consent_at = NOW(), \
                   email_source = 'chatbot' \
                 WHERE visitor_id = $1",
            )
            .bind(vid)
            .bind(&email_normalized)
            .execute(pool)
            .await;

            tracing::info!("Email capturado para visitor {vid}: {email_normalized}");
            ToolExecResult {
                tool_result_json: json!({
                    "status": "ok",
                    "email": profile.email,
                    "display_name": profile.display_name,
                    "message": "Email guardado correctamente."
                })
                .to_string(),
                rich_message: None,
            }
        }
        Err(e) => {
            tracing::error!("Error guardando email visitor {vid}: {e}");
            tool_status("error", "Error guardando email")
        }
    }
}

/* [T-3] save_client_info: guarda preferencias/datos del cliente en visitor_profiles.
 * Hace merge con preferencias existentes (JSON ||).
 * [124A-CHAT2] Si se incluye 'name', actualiza visitor_name en chat_sessions para el panel. */
async fn exec_save_client_info(
    pool: &PgPool,
    visitor_id: Option<&str>,
    session_id: uuid::Uuid,
    args: &Value,
) -> ToolExecResult {
    let Some(vid) = visitor_id else {
        return tool_status("error", "visitor_id no disponible");
    };

    /* Si la IA capturó el nombre, actualizar visitor_name en la sesión para el panel */
    if let Some(name) = args["name"].as_str() {
        if !name.trim().is_empty() {
            let _ = update_session_visitor_name(pool, session_id, name).await;
            /* También guardar en visitor_profiles */
            let _ = ChatRepository::update_visitor_email(pool, vid, "", Some(name)).await;
        }
    }

    /* Construir objeto de preferencias solo con campos que la IA proporcionó */
    let mut prefs = serde_json::Map::new();
    if let Some(v) = args.get("industry") {
        prefs.insert("industry".to_string(), v.clone());
    }
    if let Some(v) = args.get("budget_range") {
        prefs.insert("budget_range".to_string(), v.clone());
    }
    if let Some(v) = args.get("interests") {
        prefs.insert("interests".to_string(), v.clone());
    }
    if let Some(v) = args.get("project_description") {
        prefs.insert("project_description".to_string(), v.clone());
    }
    if let Some(v) = args.get("notes") {
        prefs.insert("notes".to_string(), v.clone());
    }

    if prefs.is_empty() && args["name"].as_str().is_none() {
        return ToolExecResult {
            tool_result_json: json!({"status": "ok", "message": "Sin datos nuevos"}).to_string(),
            rich_message: None,
        };
    }

    if prefs.is_empty() {
        return ToolExecResult {
            tool_result_json: json!({"status": "ok", "message": "Nombre guardado."}).to_string(),
            rich_message: None,
        };
    }

    let prefs_value = Value::Object(prefs);
    match ChatRepository::update_visitor_preferences(pool, vid, &prefs_value).await {
        Ok(()) => {
            tracing::info!("Preferencias actualizadas para visitor {vid}");
            ToolExecResult {
                tool_result_json: json!({
                    "status": "ok",
                    "saved_fields": prefs_value,
                    "message": "Información del cliente guardada."
                })
                .to_string(),
                rich_message: None,
            }
        }
        Err(e) => {
            tracing::error!("Error guardando preferencias visitor {vid}: {e}");
            tool_status("error", "Error guardando información")
        }
    }
}

/* [T-9][095A-20] Tools de cuenta registradas y protegidas por rol efectivo. */
fn registered_client_tool_defs() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "list_my_orders",
                "description": "Lista pedidos visibles para la cuenta autenticada. Cliente: sus propios pedidos. Empleado: pedidos asignados. Admin efectivo: pedidos recientes globales. Úsalo antes de responder estados, fases, entregables o empleado asignado.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_my_payments",
                "description": "Lista pagos/facturas de pedidos visibles para la cuenta autenticada. No revela secretos de Stripe; informa estado, monto, fase y si puede reintentarse desde el panel.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_my_reports",
                "description": "Lista reportes/problemas abiertos o recientes visibles según rol: cliente por sus pedidos, empleado por pedidos asignados, admin efectivo global.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_order_report",
                "description": "Crea un reporte de problema sobre un pedido visible para la cuenta autenticada. Requiere order_id u order_number y una descripción clara. Valida permisos en backend.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "order_id": { "type": "string", "description": "UUID del pedido si está disponible" },
                        "order_number": { "type": "integer", "description": "Número legible del pedido si el usuario lo menciona" },
                        "reason": { "type": "string", "description": "Problema reportado, entre 10 y 2000 caracteres" }
                    },
                    "required": ["reason"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "admin_operational_summary",
                "description": "Resumen operativo global para administradores efectivos: pedidos por estado, reportes abiertos y hosting. Prohibida para clientes, empleados e impersonaciones sin rol admin efectivo.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_support_ticket",
                "description": "Crea un ticket de soporte para el cliente. Úsalo cuando el cliente reporte un problema con su hosting, pedido, facturación o necesite asistencia técnica. Esto crea una nota interna que el equipo revisará.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "enum": ["hosting_issue", "order_issue", "billing_issue", "general"],
                            "description": "Categoría del problema"
                        },
                        "description": {
                            "type": "string",
                            "description": "Descripción detallada del problema reportado"
                        },
                        "priority": {
                            "type": "string",
                            "enum": ["low", "medium", "high"],
                            "description": "Prioridad del ticket"
                        }
                    },
                    "required": ["category", "description"]
                }
            }
        }
    ])
}

/* [T-9] Crear ticket de soporte: guarda como nota interna en la sesión de chat. */
async fn exec_create_support_ticket(
    pool: &PgPool,
    auth: Option<ToolAuthContext>,
    visitor_id: Option<&str>,
    args: &Value,
) -> ToolExecResult {
    let auth = match require_auth(auth) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    let category = args["category"].as_str().unwrap_or("general");
    let description = args["description"].as_str().unwrap_or("");
    let priority = args["priority"].as_str().unwrap_or("medium");

    if description.is_empty() {
        return tool_status("error", "Se necesita una descripción del problema");
    }

    let ticket_owner = visitor_id.unwrap_or("anónimo");

    /* Se almacena como nota en visitor_profile.context_summary para que el equipo lo vea.
     * En el futuro se puede crear una tabla dedicada de tickets. */
    if let Some(vid) = visitor_id {
        let ticket_json = json!({
            "type": "support_ticket",
            "category": category,
            "priority": priority,
            "description": description,
            "user_id": auth.user_id,
            "effective_role": auth.effective_role.to_string(),
            "created_at": chrono::Utc::now().to_rfc3339(),
        });
        let _ = ChatRepository::update_visitor_preferences(pool, vid, &ticket_json).await;
    }

    tracing::info!(
        user_id = %auth.user_id,
        visitor_id = ticket_owner,
        category,
        priority,
        "Ticket de soporte creado desde chatbot"
    );

    ToolExecResult {
        tool_result_json: json!({
            "status": "ok",
            "message": "Ticket de soporte creado exitosamente. El equipo lo revisará pronto.",
            "category": category,
            "priority": priority,
            "user_id": auth.user_id,
        })
        .to_string(),
        rich_message: Some(RichMessage {
            content: format!("📋 Ticket de soporte creado — {category} ({priority})"),
            message_type: "support_ticket".to_string(),
            metadata: json!({
                "category": category,
                "priority": priority,
                "description": description,
            }),
        }),
    }
}

/* [214A-5] Unit tests para tool_definitions y execute_tool (unknown tool).
 * Valida estructura JSON, conteo de tools, campos obligatorios por tool,
 * y el dispatch de tools desconocidas. */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definitions_returns_valid_json_array() {
        let defs = tool_definitions();
        assert!(
            defs.is_array(),
            "tool_definitions debe retornar un JSON array"
        );
    }

    #[test]
    fn tool_definitions_has_sixteen_tools() {
        let defs = tool_definitions();
        let arr = defs.as_array().unwrap();
        /* 2 service (create_invoice, request_human_assistance)
         * + 3 hosting (list_hosting_plans, list_my_hostings, create_hosting_checkout)
         * + 3 VPS (list_vps_plans, list_my_vps, create_vps_checkout)
         * + 2 visitor (capture_email, save_client_info)
         * + 6 registered/account = 16 */
        assert_eq!(
            arr.len(),
            16,
            "Se esperan 16 tools, encontradas {}",
            arr.len()
        );
    }

    #[test]
    fn tool_definitions_each_has_required_structure() {
        let defs = tool_definitions();
        let arr = defs.as_array().unwrap();
        for tool in arr {
            assert_eq!(tool["type"], "function", "type debe ser 'function'");
            let func = &tool["function"];
            assert!(func["name"].is_string(), "function.name debe ser string");
            assert!(
                func["description"].is_string(),
                "function.description debe ser string"
            );
            assert!(
                func["parameters"].is_object(),
                "function.parameters debe ser object"
            );
            assert!(
                !func["name"].as_str().unwrap().is_empty(),
                "function.name no debe estar vacío"
            );
            assert!(
                !func["description"].as_str().unwrap().is_empty(),
                "function.description no vacía"
            );
        }
    }

    #[test]
    fn tool_definitions_expected_names() {
        let defs = tool_definitions();
        let names: Vec<&str> = defs
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"create_invoice"));
        assert!(names.contains(&"list_hosting_plans"));
        assert!(names.contains(&"list_my_hostings"));
        assert!(names.contains(&"create_hosting_checkout"));
        assert!(names.contains(&"list_vps_plans"));
        assert!(names.contains(&"list_my_vps"));
        assert!(names.contains(&"create_vps_checkout"));
        assert!(names.contains(&"request_human_assistance"));
        assert!(names.contains(&"capture_email"));
        assert!(names.contains(&"save_client_info"));
        assert!(names.contains(&"list_my_orders"));
        assert!(names.contains(&"list_my_payments"));
        assert!(names.contains(&"list_my_reports"));
        assert!(names.contains(&"create_order_report"));
        assert!(names.contains(&"admin_operational_summary"));
        assert!(names.contains(&"create_support_ticket"));
    }

    #[test]
    fn tool_definitions_create_invoice_has_required_params() {
        let defs = tool_definitions();
        let invoice_tool = defs
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["function"]["name"] == "create_invoice")
            .expect("create_invoice debe existir");
        let params = &invoice_tool["function"]["parameters"];
        let required = params["required"].as_array().unwrap();
        let req_strs: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(req_strs.contains(&"amount_cents"));
        assert!(req_strs.contains(&"description"));
        assert!(req_strs.contains(&"client_email"));
    }

    #[test]
    fn tool_definitions_no_duplicate_names() {
        let defs = tool_definitions();
        let names: Vec<&str> = defs
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap())
            .collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(names.len(), unique.len(), "No debe haber tools duplicadas");
    }

    #[tokio::test]
    async fn execute_tool_unknown_returns_error() {
        let pool = PgPool::connect_lazy("postgres://invalid@localhost/test").unwrap();
        let http = reqwest::Client::new();
        let result = execute_tool(
            ToolExecutionContext {
                pool: &pool,
                http_client: &http,
                stripe_key: None,
                visitor_id: None,
                auth: None,
                session_id: uuid::Uuid::nil(),
            },
            "nonexistent_tool",
            &json!({}),
        )
        .await;
        let parsed: Value = serde_json::from_str(&result.tool_result_json).unwrap();
        assert_eq!(parsed["status"], "error");
        assert!(result.rich_message.is_none());
    }

    #[tokio::test]
    async fn create_hosting_checkout_requires_login() {
        let pool = PgPool::connect_lazy("postgres://invalid@localhost/test").unwrap();
        let http = reqwest::Client::new();
        let result = execute_tool(
            ToolExecutionContext {
                pool: &pool,
                http_client: &http,
                stripe_key: Some("sk_test_fake"),
                visitor_id: None,
                auth: None,
                session_id: uuid::Uuid::nil(),
            },
            "create_hosting_checkout",
            &json!({"plan": "basico"}),
        )
        .await;
        let parsed: Value = serde_json::from_str(&result.tool_result_json).unwrap();
        assert_eq!(parsed["status"], "requires_login");
        assert!(result.rich_message.is_none());
    }

    #[tokio::test]
async fn create_hosting_checkout_requires_stripe_key() {
    /* Asegurar que checkout_bypass no está configurado para que el test
     * realmente verifique el check de Stripe. Sin esto, otros tests que
     * seteen GLORY_TEST_CHECKOUT_EMAILS en paralelo pueden hacer que
     * checkout_bypass_is_configured() retorne true y el flujo se salte
     * el Stripe check, llegando a un error de DB en vez de "Stripe". */
    std::env::remove_var("GLORY_TEST_CHECKOUT_EMAILS");

    let pool = PgPool::connect_lazy("postgres://invalid@localhost/test").unwrap();
    let http = reqwest::Client::new();
    let result = execute_tool(
            ToolExecutionContext {
                pool: &pool,
                http_client: &http,
                stripe_key: None,
                visitor_id: None,
                auth: Some(ToolAuthContext::new(
                    uuid::Uuid::nil(),
                    UserRole::Client,
                    UserRole::Client,
                    None,
                )),
                session_id: uuid::Uuid::nil(),
            },
            "create_hosting_checkout",
            &json!({"plan": "basico"}),
        )
        .await;
        let parsed: Value = serde_json::from_str(&result.tool_result_json).unwrap();
        assert!(parsed["message"]
            .as_str()
            .is_some_and(|error| error.contains("Stripe")));
        assert!(result.rich_message.is_none());
    }

    #[tokio::test]
    async fn create_vps_checkout_requires_login() {
        let pool = PgPool::connect_lazy("postgres://invalid@localhost/test").unwrap();
        let http = reqwest::Client::new();
        let result = execute_tool(
            ToolExecutionContext {
                pool: &pool,
                http_client: &http,
                stripe_key: Some("sk_test_fake"),
                visitor_id: None,
                auth: None,
                session_id: uuid::Uuid::nil(),
            },
            "create_vps_checkout",
            &json!({"tier": "vps1"}),
        )
        .await;
        let parsed: Value = serde_json::from_str(&result.tool_result_json).unwrap();
        assert_eq!(parsed["status"], "requires_login");
        assert!(result.rich_message.is_none());
    }

    #[tokio::test]
    async fn list_my_orders_requires_login() {
        let pool = PgPool::connect_lazy("postgres://invalid@localhost/test").unwrap();
        let http = reqwest::Client::new();
        let result = execute_tool(
            ToolExecutionContext {
                pool: &pool,
                http_client: &http,
                stripe_key: None,
                visitor_id: None,
                auth: None,
                session_id: uuid::Uuid::nil(),
            },
            "list_my_orders",
            &json!({}),
        )
        .await;
        let parsed: Value = serde_json::from_str(&result.tool_result_json).unwrap();
        assert_eq!(parsed["status"], "requires_login");
    }

    #[tokio::test]
    async fn admin_summary_forbidden_for_client() {
        let pool = PgPool::connect_lazy("postgres://invalid@localhost/test").unwrap();
        let http = reqwest::Client::new();
        let result = execute_tool(
            ToolExecutionContext {
                pool: &pool,
                http_client: &http,
                stripe_key: None,
                visitor_id: None,
                auth: Some(ToolAuthContext::new(
                    uuid::Uuid::nil(),
                    UserRole::Client,
                    UserRole::Client,
                    None,
                )),
                session_id: uuid::Uuid::nil(),
            },
            "admin_operational_summary",
            &json!({}),
        )
        .await;
        let parsed: Value = serde_json::from_str(&result.tool_result_json).unwrap();
        assert_eq!(parsed["status"], "forbidden");
    }

    #[tokio::test]
    async fn create_order_report_requires_order_identifier() {
        let pool = PgPool::connect_lazy("postgres://invalid@localhost/test").unwrap();
        let http = reqwest::Client::new();
        let result = execute_tool(
            ToolExecutionContext {
                pool: &pool,
                http_client: &http,
                stripe_key: None,
                visitor_id: None,
                auth: Some(ToolAuthContext::new(
                    uuid::Uuid::nil(),
                    UserRole::Client,
                    UserRole::Client,
                    None,
                )),
                session_id: uuid::Uuid::nil(),
            },
            "create_order_report",
            &json!({"reason": "El entregable no corresponde a lo acordado"}),
        )
        .await;
        let parsed: Value = serde_json::from_str(&result.tool_result_json).unwrap();
        assert_eq!(parsed["status"], "error");
    }
}
