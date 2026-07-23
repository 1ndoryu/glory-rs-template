/* [237A-7d] Cliente del gateway WhatsApp firmado (HMAC-SHA256).
 * Envía alertas al gateway interno de glorytemplate que a su vez
 * reenvía por wacli al administrativo. El secreto se comparte via
 * GLORY_INTERNAL_ALERT_SECRET y es distinto de JWT/SMTP/Stripe.
 *
 * Canonical string:
 *   POST\n/wp-json/glory/v1/internal/alerts\n<timestamp>\n<nonce>\n<sha256_body>
 */

use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::{Digest, Sha256};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Payload que se envía al gateway.
#[derive(Debug, serde::Serialize)]
pub struct GatewayPayload {
    pub event: String,
    #[serde(rename = "messageId")]
    pub message_id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "visitorLabel")]
    pub visitor_label: String,
    pub preview: String,
    #[serde(rename = "panelUrl")]
    pub panel_url: String,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "idempotencyKey")]
    pub idempotency_key: String,
}

/// Resultado de un envío al gateway.
#[derive(Debug)]
pub enum GatewayResult {
    /// 202 Accepted — encolado correctamente.
    Accepted,
    /// Error temporal — reintentable.
    Retryable(String),
    /// Error de contrato/firma — dead.
    Fatal(String),
}

/// Envía una alerta al gateway de glorytemplate con firma HMAC.
pub async fn send_alert(
    http_client: &Client,
    gateway_url: &str,
    shared_secret: &str,
    payload: &GatewayPayload,
) -> GatewayResult {
    let body = match serde_json::to_vec(payload) {
        Ok(b) => b,
        Err(e) => return GatewayResult::Fatal(format!("Error serializando payload: {e}")),
    };

    let timestamp = chrono::Utc::now().timestamp();
    let nonce = Uuid::new_v4().to_string();
    let body_hash = {
        let mut hasher = Sha256::new();
        hasher.update(&body);
        hex::encode(hasher.finalize())
    };

    /* Canonical string */
    let canonical = format!(
        "POST\n/wp-json/glory/v1/internal/alerts\n{timestamp}\n{nonce}\n{body_hash}"
    );

    let signature = {
        let mut mac =
            HmacSha256::new_from_slice(shared_secret.as_bytes()).expect("HMAC accepts any key size");
        mac.update(canonical.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };

    let url = format!("{gateway_url}/wp-json/glory/v1/internal/alerts");

    let response = match http_client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("X-Glory-Timestamp", timestamp.to_string())
        .header("X-Glory-Nonce", &nonce)
        .header(
            "X-Glory-Idempotency-Key",
            &payload.idempotency_key,
        )
        .header("X-Glory-Signature", &signature)
        .body(body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            if e.is_timeout() {
                return GatewayResult::Retryable("Timeout gateway (10s)".into());
            }
            return GatewayResult::Retryable(format!("Error de red al gateway: {e}"));
        }
    };

    let status = response.status();
    if status == reqwest::StatusCode::ACCEPTED || status == reqwest::StatusCode::OK {
        GatewayResult::Accepted
    } else if status.is_client_error() && status != reqwest::StatusCode::TOO_MANY_REQUESTS
        && status != reqwest::StatusCode::REQUEST_TIMEOUT
    {
        GatewayResult::Fatal(format!("Gateway rechazó: HTTP {status}"))
    } else {
        GatewayResult::Retryable(format!("Gateway HTTP {status}"))
    }
}
