/* [01AA-4-f3q] Factura Stripe desde el chat (extraido de ai_tools_misc.rs).
 * Flujo: create customer → create invoice → add line item → finalize → URL.
 * [124A-INV] session_id en metadata para detectar pago via webhook invoice.paid. */

use serde_json::{json, Value};

use super::ai_tools::{tool_json, tool_status, RichMessage, ToolExecResult};

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

/* ============================================================
STRIPE HELPERS (invoice flow)
=========================================================== */

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
