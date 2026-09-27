/* [259A-4e] Tools IA misceláneas: `service_tool_defs`, `visitor_tool_defs`,
 * `registered_client_tool_defs`, factura Stripe (`exec_create_invoice`),
 * escalación (`exec_request_human`), memoria de visitante
 * (`exec_capture_email`, `exec_save_client_info`) y tickets de soporte. */

/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: query runtime intencional.
 * `update_session_visitor_name` usa `sqlx::query` sin macro para no depender de
 * caché offline en una actualización puntual de sesión (259A-4e). */

use serde_json::{json, Value};
use sqlx::PgPool;

use crate::repositories::ChatRepository;

use super::ai_tools::{
    require_auth, tool_json, tool_status, RichMessage, ToolAuthContext, ToolExecResult,
};

/* [084A-51] show_service y list_services removidos — las service cards son antinaturales
 * en un chat conversacional. Solo se mantienen tools de factura, escalación y captura. */
pub(crate) fn service_tool_defs() -> Value {
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
pub(crate) fn visitor_tool_defs() -> Value {
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

/* [084A-51] exec_show_service y exec_list_services eliminados — herramientas desactivadas.
 * Las service cards son antinaturales en el chat conversacional. Si se reactivan en el futuro,
 * recuperar implementación del git history (commit anterior a 084A-51). */

/* create_invoice: crea factura en Stripe con link de pago.
 * Flujo: create customer → create invoice → add line item → finalize → get URL.
 * El resultado es un mensaje de tipo "invoice" con el link de pago.
 * [124A-INV] session_id se guarda en metadata de Stripe para detectar pago
 * via webhook invoice.paid y notificar al admin/cliente. */
pub(crate) async fn exec_create_invoice(
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
pub(crate) fn exec_request_human(args: &Value) -> ToolExecResult {
    let reason = args["reason"].as_str().unwrap_or("Sin motivo especificado");

    /* Construir CTA de WhatsApp si el número público está configurado */
    let support_whatsapp = std::env::var("PUBLIC_SUPPORT_WHATSAPP")
        .ok()
        .filter(|s| !s.is_empty());

    let rich_message = support_whatsapp.and_then(|raw_number| {
        /* Normalizar a dígitos para wa.me — requiere al menos 7 dígitos (número real) */
        let digits: String = raw_number.chars().filter(char::is_ascii_digit).collect();
        if digits.len() < 7 {
            tracing::warn!("PUBLIC_SUPPORT_WHATSAPP no tiene dígitos suficientes: {raw_number}");
            return None;
        }
        let prefill = "Hola, quiero conversar más a fondo sobre mi proyecto con Nakomi Studio.";
        let href = format!(
            "https://wa.me/{digits}?text={}",
            urlencoding::encode(prefill)
        );

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

pub(crate) async fn exec_capture_email(
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
        return tool_status(
            "error",
            "El email proporcionado no tiene un formato válido.",
        );
    }

    let email_normalized = raw_email.trim().to_lowercase();
    let display_name = args["display_name"].as_str();

    /* Si la IA nos da el nombre junto con el email, actualizar visitor_name en la sesión. */
    if let Some(name) = display_name {
        if let Err(error) = update_session_visitor_name(pool, session_id, name).await {
            tracing::error!(%session_id, "Error actualizando nombre de sesión: {error}");
            return tool_status("error", "Error guardando el nombre del visitante");
        }
    }

    match ChatRepository::capture_visitor_email(pool, vid, &email_normalized, display_name).await {
        Ok(profile) => {
            /* [267A-2] No registrar PII: basta identificar al visitante y el resultado. */
            tracing::info!(visitor_id = %vid, "Email de visitante capturado");
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
pub(crate) async fn exec_save_client_info(
    pool: &PgPool,
    visitor_id: Option<&str>,
    session_id: uuid::Uuid,
    args: &Value,
) -> ToolExecResult {
    let Some(vid) = visitor_id else {
        return tool_status("error", "visitor_id no disponible");
    };

    /* [267A-2] Nombre y email tienen escrituras independientes: guardar el nombre
     * nunca debe convertir un email previamente capturado en cadena vacía. */
    if let Some(name) = args["name"].as_str() {
        if !name.trim().is_empty() {
            let normalized_name = name.trim();
            if let Err(error) =
                ChatRepository::update_visitor_display_name(pool, vid, normalized_name).await
            {
                tracing::error!(visitor_id = %vid, "Error guardando nombre del visitante: {error}");
                return tool_status("error", "Error guardando el nombre del visitante");
            }
            if let Err(error) = update_session_visitor_name(pool, session_id, normalized_name).await
            {
                tracing::error!(%session_id, "Error actualizando nombre de sesión: {error}");
                return tool_status("error", "Error guardando el nombre en la conversación");
            }
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
pub(crate) fn registered_client_tool_defs() -> Value {
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
pub(crate) async fn exec_create_support_ticket(
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
