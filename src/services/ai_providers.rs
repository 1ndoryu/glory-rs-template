/* [174A-2] Proveedores de API AI (DeepSeek, Groq, Gemini). Extraído de ai_chat.rs para SRP.
 * Contiene: llamadas HTTP a APIs, retry multi-proveedor, construcción de body. */

use serde_json::Value;

use super::ai_chat::AiChatConfig;

#[derive(Clone, Copy)]
pub(crate) struct ChatApiOptions {
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: f32,
    pub timeout_secs: u64,
}

impl ChatApiOptions {
    pub(crate) const fn standard() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 800,
            top_p: 0.9,
            timeout_secs: 30,
        }
    }

    pub(crate) const fn terse(max_tokens: u32) -> Self {
        Self {
            temperature: 0.2,
            max_tokens,
            top_p: 0.8,
            timeout_secs: 15,
        }
    }
}

/* [095A-10] Llamar a API AI con retry multi-proveedor.
 * Flujo: DeepSeek → Groq (modelos × keys) → Gemini. */
/* [096A-9] call_ai_api ahora acepta un reqwest::Client compartido.
 * Crear Client por llamada hace DNS+TLS síncrono, bloqueando tokio workers.
 * Si no se provee client, crea uno nuevo (fallback para compatibilidad). */
pub(crate) async fn call_ai_api(
    config: &AiChatConfig,
    messages: &[Value],
    tools: Option<&Value>,
    client: Option<&reqwest::Client>,
) -> Result<Value, String> {
    call_ai_api_with_options(config, messages, tools, ChatApiOptions::standard(), client).await
}

pub(crate) async fn call_ai_api_with_options(
    config: &AiChatConfig,
    messages: &[Value],
    tools: Option<&Value>,
    options: ChatApiOptions,
    client: Option<&reqwest::Client>,
) -> Result<Value, String> {
    let default_client;
    /* [259A-1] if-let-else en vez de match de un solo patron (clippy single_match_else). */
    let client = if let Some(c) = client {
        c
    } else {
        default_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(options.timeout_secs))
            .build()
            .unwrap_or_default();
        &default_client
    };
    let mut last_error = String::new();

    if config.deepseek_key.is_some() {
        match try_deepseek_provider(config, client, messages, tools, options).await {
            Ok(json) => return Ok(json),
            Err(e) => last_error = e,
        }
    }

    if !config.api_keys.is_empty() {
        match try_groq_provider(config, client, messages, tools, options).await {
            Ok(json) => return Ok(json),
            Err(e) => last_error = e,
        }
    }

    if config.gemini_key.is_some() {
        match try_gemini_provider(config, client, messages, tools, options).await {
            Ok(json) => return Ok(json),
            Err(e) => last_error = e,
        }
    }

    Err(format!(
        "AI: todos los proveedores fallaron. Último error: {last_error}"
    ))
}

/* [095A-10] DeepSeek usa formato OpenAI-compatible y soporta tool calls.
 * Se intenta primero porque es el proveedor primario pedido para el chatbot. */
async fn try_deepseek_provider(
    config: &AiChatConfig,
    client: &reqwest::Client,
    messages: &[Value],
    tools: Option<&Value>,
    options: ChatApiOptions,
) -> Result<Value, String> {
    let deepseek_key = config
        .deepseek_key
        .as_deref()
        .ok_or("DeepSeek no configurado")?;
    let key_hint = &deepseek_key[..deepseek_key.len().min(8)];
    let body = build_chat_body(&config.deepseek_model, messages, tools, options);

    let resp = client
        .post(&config.deepseek_url)
        .timeout(std::time::Duration::from_secs(options.timeout_secs))
        .header("Authorization", format!("Bearer {deepseek_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;

    match resp {
        Err(e) => {
            tracing::error!(
                "AI DeepSeek [{}] key {key_hint}...: error red: {e}",
                config.deepseek_model
            );
            Err(format!("DeepSeek error de red: {e}"))
        }
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                let json: Value = response
                    .json()
                    .await
                    .map_err(|e| format!("AI parse error: {e}"))?;
                tracing::info!(
                    "AI OK: proveedor=DeepSeek, modelo={}, key={key_hint}...",
                    config.deepseek_model
                );
                return Ok(json);
            }
            let text = response.text().await.unwrap_or_default();
            tracing::error!(
                "AI DeepSeek [{}] key {key_hint}...: HTTP {status}: {text}",
                config.deepseek_model
            );
            Err(format!("DeepSeek HTTP {status}"))
        }
    }
}

/* Intenta Groq con rotación de modelos (fallback_chain) × keys (round-robin). */
async fn try_groq_provider(
    config: &AiChatConfig,
    client: &reqwest::Client,
    messages: &[Value],
    tools: Option<&Value>,
    options: ChatApiOptions,
) -> Result<Value, String> {
    let fallback_chain = config.model_fallback_chain();
    let num_keys = config.api_keys.len();
    let mut last_error = String::new();

    for model in &fallback_chain {
        let body = build_chat_body(model, messages, tools, options);
        let mut all_rate_limited = true;

        for attempt in 0..num_keys {
            let Some(api_key) = config.next_key() else {
                break;
            };
            let key_hint = &api_key[..api_key.len().min(8)];

            let resp = client
                .post(&config.api_url)
                .timeout(std::time::Duration::from_secs(options.timeout_secs))
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await;

            match resp {
                Err(e) => {
                    tracing::error!(
                        "AI Groq [{model}] key {key_hint}... intento {}/{num_keys}: error red: {e}",
                        attempt + 1
                    );
                    last_error = format!("Groq error de red: {e}");
                    all_rate_limited = false;
                }
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        let json: Value = response
                            .json()
                            .await
                            .map_err(|e| format!("AI parse error: {e}"))?;
                        if model == &fallback_chain[0] {
                            tracing::info!("AI OK: proveedor=Groq, modelo={model}, key={key_hint}..., intento={}", attempt + 1);
                        } else {
                            tracing::info!(
                                "AI fallback Groq: modelo={model}, key={key_hint}..., intento={}",
                                attempt + 1
                            );
                        }
                        return Ok(json);
                    }
                    let text = response.text().await.unwrap_or_default();
                    tracing::error!("AI Groq [{model}] key {key_hint}... intento {}/{num_keys}: HTTP {status}: {text}", attempt + 1);
                    last_error = format!("Groq HTTP {status}");

                    if status.as_u16() != 429 {
                        all_rate_limited = false;
                    }
                }
            }
        }

        if !all_rate_limited {
            break;
        }
        tracing::warn!("AI Groq modelo {model}: todas las keys con rate limit (429), probando siguiente modelo");
    }

    Err(last_error)
}

/* [084A-37] Intenta Google Gemini como proveedor secundario OpenAI-compatible. */
async fn try_gemini_provider(
    config: &AiChatConfig,
    client: &reqwest::Client,
    messages: &[Value],
    tools: Option<&Value>,
    options: ChatApiOptions,
) -> Result<Value, String> {
    let gemini_key = config
        .gemini_key
        .as_deref()
        .ok_or("Gemini no configurado")?;
    let gemini_models = [
        "gemini-2.5-flash",
        "gemini-2.5-flash-lite",
        "gemini-2.5-pro",
        "gemini-3-flash-preview",
        "gemini-3.1-pro-preview",
        "gemini-3.1-flash-lite-preview",
    ];
    let key_hint = &gemini_key[..gemini_key.len().min(8)];
    let mut last_error = String::new();

    for model in &gemini_models {
        let body = build_chat_body(model, messages, tools, options);

        let resp = client
            .post(&config.gemini_url)
            .timeout(std::time::Duration::from_secs(options.timeout_secs))
            .header("Authorization", format!("Bearer {gemini_key}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        match resp {
            Err(e) => {
                tracing::error!("AI Gemini [{model}] key {key_hint}...: error red: {e}");
                last_error = format!("Gemini error de red: {e}");
            }
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    let json: Value = response
                        .json()
                        .await
                        .map_err(|e| format!("AI parse error: {e}"))?;
                    tracing::info!("AI fallback Gemini: modelo={model}, key={key_hint}...");
                    return Ok(json);
                }
                let text = response.text().await.unwrap_or_default();
                tracing::error!("AI Gemini [{model}] key {key_hint}...: HTTP {status}: {text}");
                last_error = format!("Gemini HTTP {status}");

                if status.as_u16() != 429 {
                    break;
                }
            }
        }
    }

    Err(last_error)
}

/* Construye el body JSON para una llamada OpenAI-compatible. */
fn build_chat_body(
    model: &str,
    messages: &[Value],
    tools: Option<&Value>,
    options: ChatApiOptions,
) -> Value {
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "temperature": options.temperature,
        "max_tokens": options.max_tokens,
        "top_p": options.top_p
    });
    if let Some(t) = tools {
        body["tools"] = t.clone();
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_body_uses_requested_model_and_limits() {
        let messages = vec![serde_json::json!({"role": "user", "content": "hola"})];
        let body = build_chat_body(
            "deepseek-v4-flash",
            &messages,
            None,
            ChatApiOptions::standard(),
        );
        assert_eq!(body["model"], "deepseek-v4-flash");
        assert_eq!(body["max_tokens"], 800);
        assert!(body["tools"].is_null());
    }

    #[test]
    fn terse_body_keeps_small_output_budget() {
        let messages = vec![serde_json::json!({"role": "user", "content": "clasifica"})];
        let body = build_chat_body(
            "deepseek-v4-flash",
            &messages,
            None,
            ChatApiOptions::terse(5),
        );
        assert_eq!(body["max_tokens"], 5);
        let temperature = body["temperature"].as_f64().unwrap_or_default();
        assert!((temperature - 0.2).abs() < 0.000_001);
    }
}
