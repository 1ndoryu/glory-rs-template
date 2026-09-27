/* [259A-4d] Rate limiting de chat_timing: limites por visitor/IP, conexiones WS,
 * presupuestos de tokens IA. Mismo patron hub + `impl` inherente: los metodos
 * `pub` se resuelven como `ChatTimingService::...` desde toda la crate. */

use std::time::{Duration, Instant};

use super::chat_timing::{
    ChatTimingService, RateCheckResult, AI_IP_TOKEN_BUDGET_PER_HOUR, AI_MAX_COMBINED_CHARS,
    AI_REQUEST_OVERHEAD_TOKENS, AI_TOKEN_BUDGET_WINDOW, AI_VISITOR_TOKEN_BUDGET_PER_HOUR,
    IP_RATE_LIMIT_PER_MIN, MAX_WS_CONNECTIONS_PER_IP, MSG_MAX_LENGTH, RATE_LIMIT_PER_MIN,
    RATE_WINDOW,
};

impl ChatTimingService {
    /// Verifica rate limit para un `visitor_id`. Retorna el resultado y
    /// opcionalmente un mensaje de advertencia para enviar al visitante.
    #[must_use]
    pub fn check_rate(&self, visitor_id: &str) -> (RateCheckResult, Option<String>) {
        let mut entry = self.rate_limits.entry(visitor_id.to_string()).or_default();
        let state = entry.value_mut();

        /* Reset ventana si expiró */
        if state.window_start.elapsed() >= RATE_WINDOW {
            state.count = 0;
            state.window_start = Instant::now();
        }

        /* Verificar si está muteado actualmente */
        if let Some(until) = state.mute_until {
            if Instant::now() < until {
                return (
                    RateCheckResult::Muted,
                    Some("Has enviado demasiados mensajes. Espera un momento.".to_string()),
                );
            }
            state.mute_until = None;
        }

        state.count += 1;

        if state.count <= RATE_LIMIT_PER_MIN {
            return (RateCheckResult::Ok, None);
        }

        /* Exceso detectado — cooldown progresivo */
        state.cooldown_level += 1;
        match state.cooldown_level {
            1 => (
                RateCheckResult::Warning,
                Some("Por favor, escribe con calma. Estoy leyendo cada mensaje.".to_string()),
            ),
            2 => {
                state.mute_until = Some(Instant::now() + Duration::from_secs(30));
                (
                    RateCheckResult::Muted,
                    Some("Has enviado demasiados mensajes. Espera 30 segundos.".to_string()),
                )
            }
            _ => (
                RateCheckResult::Closed,
                Some(
                    "La sesión se ha cerrado por exceso de mensajes. Puedes volver más tarde."
                        .to_string(),
                ),
            ),
        }
    }

    /* [084A-42] Rate limit por IP: mismo mecanismo pero con umbral más alto.
     * Previene que un bot rote visitor_ids para evadir el rate limit por visitor. */
    #[must_use]
    pub fn check_ip_rate(&self, ip: &str) -> (RateCheckResult, Option<String>) {
        let mut entry = self.ip_rate_limits.entry(ip.to_string()).or_default();
        let state = entry.value_mut();

        if state.window_start.elapsed() >= RATE_WINDOW {
            state.count = 0;
            state.window_start = Instant::now();
        }

        if let Some(until) = state.mute_until {
            if Instant::now() < until {
                return (
                    RateCheckResult::Muted,
                    Some("Demasiadas conexiones desde tu red. Intenta más tarde.".into()),
                );
            }
            state.mute_until = None;
        }

        state.count += 1;

        if state.count <= IP_RATE_LIMIT_PER_MIN {
            return (RateCheckResult::Ok, None);
        }

        state.cooldown_level += 1;
        match state.cooldown_level {
            1 => (RateCheckResult::Warning, None),
            2 => {
                state.mute_until = Some(Instant::now() + Duration::from_mins(1));
                (
                    RateCheckResult::Muted,
                    Some("Demasiados mensajes desde tu red. Espera un minuto.".into()),
                )
            }
            _ => (
                RateCheckResult::Closed,
                Some("Sesión cerrada por exceso de actividad desde tu red.".into()),
            ),
        }
    }

    /* [084A-42] Tracker de conexiones WS concurrentes por IP.
     * Retorna false si la IP excede el máximo de conexiones. */
    #[must_use]
    pub fn track_ip_connect(&self, ip: &str) -> bool {
        let mut count = self.ip_connections.entry(ip.to_string()).or_insert(0);
        if *count >= MAX_WS_CONNECTIONS_PER_IP {
            tracing::warn!("Anti-bot: IP {ip} excede {MAX_WS_CONNECTIONS_PER_IP} conexiones WS");
            return false;
        }
        *count += 1;
        true
    }

    /* [084A-42] Decrementar contador al desconectar */
    pub fn track_ip_disconnect(&self, ip: &str) {
        if let Some(mut count) = self.ip_connections.get_mut(ip) {
            *count = count.saturating_sub(1);
        }
    }

    /* [084A-42] Longitud máxima de mensajes para prevenir token drain */
    #[must_use]
    pub const fn max_message_length() -> usize {
        MSG_MAX_LENGTH
    }

    #[must_use]
    pub const fn max_ai_combined_chars() -> usize {
        AI_MAX_COMBINED_CHARS
    }

    #[must_use]
    pub fn estimate_ai_request_tokens(content: &str) -> usize {
        content.len() / 4 + 1 + AI_REQUEST_OVERHEAD_TOKENS
    }

    #[must_use]
    pub fn check_visitor_ai_budget(
        &self,
        visitor_id: &str,
        content: &str,
    ) -> (RateCheckResult, Option<String>) {
        self.check_ai_budget(
            &format!("visitor:{visitor_id}"),
            Self::estimate_ai_request_tokens(content),
            AI_VISITOR_TOKEN_BUDGET_PER_HOUR,
            "Llegaste al límite temporal de uso del asistente. Te puede continuar atendiendo una persona del equipo.",
        )
    }

    #[must_use]
    pub fn check_ip_ai_budget(&self, ip: &str, content: &str) -> (RateCheckResult, Option<String>) {
        self.check_ai_budget(
            &format!("ip:{ip}"),
            Self::estimate_ai_request_tokens(content),
            AI_IP_TOKEN_BUDGET_PER_HOUR,
            "Detecté demasiado uso del asistente desde tu red. Intenta más tarde o espera a una persona del equipo.",
        )
    }

    fn check_ai_budget(
        &self,
        key: &str,
        estimated_tokens: usize,
        limit: usize,
        message: &str,
    ) -> (RateCheckResult, Option<String>) {
        let mut entry = self.ai_token_budgets.entry(key.to_string()).or_default();
        let state = entry.value_mut();
        if state.window_start.elapsed() >= AI_TOKEN_BUDGET_WINDOW {
            state.tokens = 0;
            state.window_start = Instant::now();
        }
        if state.tokens.saturating_add(estimated_tokens) > limit {
            tracing::warn!(
                "AI budget bloqueado para {key}: usados={}, intento={}, limite={limit}",
                state.tokens,
                estimated_tokens
            );
            return (RateCheckResult::Muted, Some(message.to_string()));
        }
        state.tokens += estimated_tokens;
        (RateCheckResult::Ok, None)
    }
}
