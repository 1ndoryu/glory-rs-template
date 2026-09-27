/* [T-2] Herramientas de IA para el chatbot (tool use / function calling).
 * Define las tools que la IA puede invocar y ejecuta las llamadas.
 * Groq soporta tool use compatible con OpenAI en llama-3.3-70b-versatile.
 * Cada tool retorna un resultado JSON que se reenvía a la IA + opcionalmente
 * un mensaje rico para el WS del visitante. */

/* [259A-4e] Hub: tipos públicos (`ToolExecResult`, `RichMessage`,
 * `ToolExecutionContext`, `ToolAuthContext`), `tool_definitions`,
 * dispatcher `execute_tool`, helpers `pub(crate)` y tests.
 * Dominios hermanos: `ai_tools_hosting`, `ai_tools_vps`, `ai_tools_orders`,
 * `ai_tools_reports`, `ai_tools_misc`. Las fns libres cruzadas son
 * `pub(crate)`; las rutas externas (`ai_tools::...`) no cambian. */

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::UserRole;

use super::ai_tools_hosting::{
    exec_create_hosting_checkout, exec_list_hosting_plans, exec_list_my_hostings, hosting_tool_defs,
};
use super::ai_tools_misc::{
    exec_capture_email, exec_create_invoice, exec_create_support_ticket, exec_request_human,
    exec_save_client_info, registered_client_tool_defs, service_tool_defs, visitor_tool_defs,
};
use super::ai_tools_orders::{exec_list_my_orders, exec_list_my_payments};
use super::ai_tools_reports::{
    exec_admin_operational_summary, exec_create_order_report, exec_list_my_reports,
};
use super::ai_tools_vps::{
    exec_create_vps_checkout, exec_list_my_vps, exec_list_vps_plans, vps_tool_defs,
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

pub(crate) fn tool_json(payload: impl serde::Serialize) -> ToolExecResult {
    ToolExecResult {
        tool_result_json: serde_json::to_string(&payload).unwrap_or_else(|e| {
            tracing::error!("Error serializando resultado de AI tool: {e}");
            json!({"status": "error", "message": "No se pudo serializar el resultado"}).to_string()
        }),
        rich_message: None,
    }
}

pub(crate) fn tool_status(status: &str, message: &str) -> ToolExecResult {
    tool_json(json!({"status": status, "message": message}))
}

pub(crate) fn requires_login(message: &str) -> ToolExecResult {
    tool_status("requires_login", message)
}

pub(crate) fn forbidden(message: &str) -> ToolExecResult {
    tool_status("forbidden", message)
}

pub(crate) fn not_found(message: &str) -> ToolExecResult {
    tool_status("not_found", message)
}

pub(crate) fn require_auth(
    auth: Option<ToolAuthContext>,
) -> Result<ToolAuthContext, ToolExecResult> {
    auth.ok_or_else(|| {
        requires_login("Para consultar datos de tu cuenta necesitas iniciar sesión.")
    })
}

pub(crate) fn can_access_order(
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
