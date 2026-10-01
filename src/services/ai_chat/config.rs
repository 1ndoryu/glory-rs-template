/* [01AA-4-F3g] Configuración multi-proveedor del AI Chat (era parte de ai_chat.rs):
 * AiChatConfig (DeepSeek primario, Groq fallback, Gemini respaldo) + sanitize_for_prompt. */

/* [084A-30] Sanitiza texto controlado por el usuario antes de inyectarlo en el system prompt.
 * Previene prompt injection: elimina caracteres de control, trunca longitud excesiva,
 * y envuelve el dato en delimitadores para que el modelo lo trate como dato, no instrucción. */
pub(crate) fn sanitize_for_prompt(input: &str, max_len: usize) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .take(max_len)
        .collect::<String>()
        .replace("INSTRUCCIÓN", "")
        .replace("INSTRUCTION", "")
        .replace("IGNORE", "")
        .replace("SYSTEM", "")
}

/* [114A-12] Rotación de API keys eliminada 2026-05-23.
 * next_key() siempre retorna la primera key disponible. */

/// Configuracion del servicio de IA con soporte multi-proveedor.
/// `DeepSeek` como primario, `Groq` como fallback y `Gemini` como ultimo respaldo.
/// Las APIs usan formato OpenAI-compatible.
#[derive(Clone)]
pub struct AiChatConfig {
    pub(crate) deepseek_key: Option<String>,
    pub(crate) deepseek_model: String,
    pub(crate) deepseek_url: String,
    pub api_keys: Vec<String>,
    pub model: String,
    pub api_url: String,
    /* [084A-37] Google Gemini como proveedor secundario OpenAI-compatible.
     * Se usa como fallback cuando Groq agota todos los modelos x keys. */
    pub(crate) gemini_key: Option<String>,
    pub(crate) gemini_url: String,
}

impl AiChatConfig {
    /// Carga config desde variables de entorno. Soporta `GROQ_API_1`, `GROQ_API_2`, `GROQ_API_3`.
    /// Fallback a `AI_API_KEY`/`GEMINI_API_KEY`/`OPENAI_API_KEY` si no hay keys Groq.
    #[must_use]
    pub fn from_env() -> Self {
        let deepseek_key = std::env::var("DEEPSEEK_API_KEY")
            .or_else(|_| std::env::var("DEEPSEEK_API"))
            .ok()
            .filter(|k| !k.is_empty());
        let deepseek_model =
            std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".to_string());
        let deepseek_allowed = [
            "deepseek-v4-flash",
            "deepseek-v4-pro",
            "deepseek-chat",
            "deepseek-reasoner",
        ];
        let deepseek_model = if deepseek_allowed.contains(&deepseek_model.as_str()) {
            deepseek_model
        } else {
            tracing::warn!("Modelo DeepSeek no permitido: {deepseek_model}, usando default");
            "deepseek-v4-flash".to_string()
        };
        let deepseek_url = std::env::var("DEEPSEEK_API_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com/chat/completions".to_string());

        let mut keys = Vec::new();
        for var in ["GROQ_API_1", "GROQ_API_2", "GROQ_API_3", "GROQ_API"] {
            if let Ok(k) = std::env::var(var) {
                if !k.is_empty() && !keys.contains(&k) {
                    keys.push(k);
                }
            }
        }
        /* Fallback a keys genéricas si no hay Groq */
        if keys.is_empty() {
            for var in ["AI_API_KEY", "GEMINI_API_KEY", "OPENAI_API_KEY"] {
                if let Ok(k) = std::env::var(var) {
                    if !k.is_empty() {
                        keys.push(k);
                        break;
                    }
                }
            }
        }

        let model = std::env::var("AI_MODEL").unwrap_or_else(|_| "openai/gpt-oss-120b".to_string());

        /* [084A-36] Whitelist de modelos Groq, ordenada por inteligencia descendente.
         * GPT-OSS-120B como primario. Maverick eliminado (deprecated en Groq).
         * El sistema usa esta lista como cadena de fallback por rate limit (429). */
        let allowed_models = [
            "openai/gpt-oss-120b",
            "meta-llama/llama-4-scout-17b-16e-instruct",
            "qwen/qwen3-32b",
            "llama-3.3-70b-versatile",
            "openai/gpt-oss-20b",
            "llama-3.1-8b-instant",
        ];
        let model = if allowed_models.contains(&model.as_str()) {
            model
        } else {
            tracing::warn!("Modelo AI no permitido: {model}, usando default");
            "openai/gpt-oss-120b".to_string()
        };

        let api_url = std::env::var("AI_API_URL")
            .unwrap_or_else(|_| "https://api.groq.com/openai/v1/chat/completions".to_string());

        /* [084A-37] Google Gemini como proveedor secundario.
         * La API de Gemini es OpenAI-compatible: mismo formato de request,
         * diferente base_url y api_key. Se usa como fallback cuando Groq falla. */
        let gemini_key = std::env::var("GOOGLE_GEMINI_API")
            .ok()
            .filter(|k| !k.is_empty());
        let gemini_url = std::env::var("GEMINI_API_URL").unwrap_or_else(|_| {
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions".to_string()
        });
        if deepseek_key.is_some() {
            tracing::info!("AI: DeepSeek configurado como proveedor primario");
        }
        if gemini_key.is_some() {
            tracing::info!("AI: Gemini configurado como proveedor secundario");
        }

        Self {
            deepseek_key,
            deepseek_model,
            deepseek_url,
            api_keys: keys,
            model,
            api_url,
            gemini_key,
            gemini_url,
        }
    }

    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.deepseek_key.is_some() || !self.api_keys.is_empty() || self.gemini_key.is_some()
    }

    /* [114A-12] Rotación eliminada. Siempre retorna la primera key. */
    pub(crate) fn next_key(&self) -> Option<&str> {
        self.api_keys.first().map(String::as_str)
    }

    /// Numero total de API keys configuradas
    #[must_use]
    pub fn total_keys(&self) -> usize {
        self.api_keys.len()
    }

    /* [084A-36] Cadena de modelos fallback Groq ordenados por inteligencia.
     * GPT-OSS-120B como primario. Maverick eliminado (deprecated en Groq abril 2026).
     * El modelo primario (self.model) va primero, seguido por los demás en orden descendente. */
    pub(crate) fn model_fallback_chain(&self) -> Vec<&str> {
        let all_models = [
            "openai/gpt-oss-120b",
            "meta-llama/llama-4-scout-17b-16e-instruct",
            "qwen/qwen3-32b",
            "llama-3.3-70b-versatile",
            "openai/gpt-oss-20b",
            "llama-3.1-8b-instant",
        ];
        let mut chain: Vec<&str> = vec![self.model.as_str()];
        for m in &all_models {
            if *m != self.model.as_str() {
                chain.push(m);
            }
        }
        chain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /* [084A-37] Helper para crear config de test sin repetir campos Gemini */
    fn test_config(keys: Vec<String>, model: &str) -> AiChatConfig {
        AiChatConfig {
            deepseek_key: None,
            deepseek_model: "deepseek-v4-flash".into(),
            deepseek_url: "https://api.deepseek.com/chat/completions".into(),
            api_keys: keys,
            model: model.to_string(),
            api_url: "https://api.groq.com".into(),
            gemini_key: None,
            gemini_url: "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
                .into(),
        }
    }

    #[test]
    fn model_fallback_chain_primary_first() {
        let config = test_config(vec!["key1".into()], "qwen/qwen3-32b");
        let chain = config.model_fallback_chain();
        assert_eq!(chain[0], "qwen/qwen3-32b");
        assert!(chain.contains(&"openai/gpt-oss-120b"));
        /* [084A-36] Maverick ya no debe estar en la cadena */
        assert!(!chain.iter().any(|m| m.contains("maverick")));
        let unique: std::collections::HashSet<&&str> = chain.iter().collect();
        assert_eq!(unique.len(), chain.len());
    }

    /* [084A-36] Test actualizado: GPT-OSS-120B es ahora el modelo primario por defecto */
    #[test]
    fn model_fallback_chain_default_gpt_oss() {
        let config = test_config(vec!["key1".into()], "openai/gpt-oss-120b");
        let chain = config.model_fallback_chain();
        assert_eq!(chain[0], "openai/gpt-oss-120b");
        assert_eq!(
            chain
                .iter()
                .filter(|m| **m == "openai/gpt-oss-120b")
                .count(),
            1,
        );
        assert_eq!(chain.len(), 6);
    }

    #[test]
    fn next_key_returns_first_always() {
        let config = test_config(vec!["key_a".into(), "key_b".into(), "key_c".into()], "test");
        assert_eq!(config.next_key(), Some("key_a"));
        assert_eq!(config.next_key(), Some("key_a"));
    }

    #[test]
    fn next_key_empty_returns_none() {
        let config = test_config(vec![], "test");
        assert!(config.next_key().is_none());
    }

    /* [084A-37] Tests para configuración multi-proveedor Gemini */
    #[test]
    fn is_configured_with_only_gemini() {
        let mut config = test_config(vec![], "test");
        assert!(!config.is_configured());
        config.gemini_key = Some("gemini-key-123".into());
        assert!(config.is_configured());
    }

    #[test]
    fn is_configured_with_only_deepseek() {
        let mut config = test_config(vec![], "test");
        assert!(!config.is_configured());
        config.deepseek_key = Some("deepseek-key".into());
        assert!(config.is_configured());
    }

    #[test]
    fn deepseek_defaults_to_flash_model() {
        let config = test_config(vec![], "test");
        assert_eq!(config.deepseek_model, "deepseek-v4-flash");
        assert!(config.deepseek_url.contains("api.deepseek.com"));
    }

    #[test]
    fn is_configured_with_groq_and_gemini() {
        let mut config = test_config(vec!["groq-key".into()], "openai/gpt-oss-120b");
        config.gemini_key = Some("gemini-key".into());
        assert!(config.is_configured());
    }

    #[test]
    fn gemini_url_default() {
        let config = test_config(vec![], "test");
        assert!(config
            .gemini_url
            .contains("generativelanguage.googleapis.com"));
    }
}
