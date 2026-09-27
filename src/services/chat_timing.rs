/* [T-1] Chat Timing Service: anti-spam, rate limiting y timing inteligente.
 * Gestiona una máquina de estados por sesión que bufferea mensajes del
 * visitante y decide cuándo generar respuesta IA. Evita respuestas
 * instantáneas a cada mensaje, emulando comportamiento humano.
 *
 * Máquina de estados:
 *   IDLE → recibe mensaje → WAITING (buffer=[msg], timer=4s)
 *   WAITING → timer expira → RESPONDING → genera respuesta → IDLE
 *   WAITING → nuevo mensaje → WAITING (buffer.push, timer reset)
 *   WAITING → typing start → LISTENING (timer=8s desde último typing)
 *   LISTENING → typing stop + no mensaje 3s → RESPONDING
 *   LISTENING → nuevo mensaje → LISTENING (buffer.push, timer reset)
 *   RESPONDING → en progreso → ignora triggers
 *
 * Rate limiting: max 10 msgs/min por visitor_id, cooldown progresivo.
 * Relevancia: filtro opcional para spam/off-topic; desactivado por defecto por falsos positivos. */

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{mpsc, Semaphore};
use uuid::Uuid;

use crate::services::{AiChatConfig, ChatHub, NotificationHub};

use super::chat_timing_loop::session_timing_loop;

/* [259A-4d] Hub de timing: tipos, constantes, registro de sesiones y tests.
 * Rate limiting en chat_timing_rate, FSM en chat_timing_loop, respuesta IA en
 * chat_timing_response, escalacion/resumen en chat_timing_escalation.
 * Misma crate: los metodos `impl ChatTimingService` se ven entre modulos;
 * las fns libres cruzadas son `pub(crate)`; los campos del struct `pub(crate)`. */

/* Dependencias agrupadas para evitar exceso de argumentos en register_session */
pub struct TimingSessionDeps {
    pub pool: PgPool,
    pub ai_config: AiChatConfig,
    pub chat_timing: ChatTimingService,
    pub hub: ChatHub,
    pub notification_hub: NotificationHub,
    pub http_client: reqwest::Client,
    pub stripe_key: Option<String>,
    pub visitor_id: String,
    pub client_ip: Option<String>,
    /* [T-9] user_id del cliente autenticado (None para visitantes anónimos) */
    pub user_id: Option<uuid::Uuid>,
    /* [095A-20] Rol real/operativo firmado para autorizar tools sensibles. */
    pub auth: Option<crate::services::ai_tools::ToolAuthContext>,
    /* [084A-28] Contexto de origen: "hosting:{uuid}", "service:{slug}", etc. */
    pub context: Option<String>,
    /* [114A-8] Config SMTP para email de escalación (None si SMTP no configurado) */
    pub email_config: Option<crate::services::EmailConfig>,
}

/* Constantes de timing configurables */
/* [084A-46] Reducido de 4s a 1s: el usuario percibía demasiada latencia */
pub(crate) const WAIT_TIMEOUT: Duration = Duration::from_secs(1);
pub(crate) const LISTEN_TIMEOUT: Duration = Duration::from_secs(8);
pub(crate) const TYPING_COOLDOWN: Duration = Duration::from_secs(3);
pub(crate) const MAX_ACCUMULATION: Duration = Duration::from_secs(30);

/* Rate limiting */
pub(crate) const RATE_LIMIT_PER_MIN: u32 = 10;
pub(crate) const RATE_WINDOW: Duration = Duration::from_mins(1);

/* [084A-42] Anti-bot: limites por IP (más altos porque IPs pueden ser compartidas) */
pub(crate) const IP_RATE_LIMIT_PER_MIN: u32 = 30;
pub(crate) const MAX_WS_CONNECTIONS_PER_IP: u32 = 10;
pub(crate) const MSG_MAX_LENGTH: usize = 2000;
pub(crate) const AI_MAX_COMBINED_CHARS: usize = 6000;
pub(crate) const AI_TOKEN_BUDGET_WINDOW: Duration = Duration::from_hours(1);
pub(crate) const AI_VISITOR_TOKEN_BUDGET_PER_HOUR: usize = 24_000;
pub(crate) const AI_IP_TOKEN_BUDGET_PER_HOUR: usize = 80_000;
pub(crate) const AI_REQUEST_OVERHEAD_TOKENS: usize = 1_500;

/* Relevancia */
pub(crate) const MAX_IRRELEVANT_STREAK: u32 = 3;

/* [096A-7] Concurrencia máxima de peticiones IA simultáneas.
 * Protege el pool DB (max=10) y APIs externas de saturación.
 * Cada petición IA retiene 1 conexión DB + 1 request HTTP hasta 90s. */
const MAX_CONCURRENT_AI_REQUESTS: usize = 3;

/* [096A-7] Timeout absoluto de seguridad para session_timing_loop (10 min).
 * Si por cualquier razón el loop no recibe Disconnect, este timeout lo mata.
 * El timeout normal de inactividad (300s en process_visitor_messages) cierra antes. */
const TIMING_LOOP_MAX_LIFETIME: Duration = Duration::from_secs(600);

/// Eventos que el handler WS envía al timing service
#[derive(Debug)]
pub enum TimingEvent {
    Message(String),
    TypingStart,
    TypingStop,
    Disconnect,
}

/// Resultado del rate limiter
pub enum RateCheckResult {
    Ok,
    Warning,
    Muted,
    Closed,
}

/* Estado de rate limiting por visitor_id */
pub(crate) struct RateState {
    pub(crate) count: u32,
    pub(crate) window_start: Instant,
    pub(crate) cooldown_level: u8,
    pub(crate) mute_until: Option<Instant>,
}

pub(crate) struct BudgetState {
    pub(crate) tokens: usize,
    pub(crate) window_start: Instant,
}

impl Default for BudgetState {
    fn default() -> Self {
        Self {
            tokens: 0,
            window_start: Instant::now(),
        }
    }
}

impl Default for RateState {
    fn default() -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
            cooldown_level: 0,
            mute_until: None,
        }
    }
}

/* [259A-4d] En el hub para que lo usen chat_timing_response y los tests.
 * Opt-in por env: desactivado por defecto por falsos positivos ([095A-13]). */
pub(crate) fn ai_relevance_enabled(raw: Option<&str>) -> bool {
    raw.is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

/// Servicio de timing: gestiona sesiones activas y rate limiting global
#[derive(Clone)]
pub struct ChatTimingService {
    pub(crate) sessions: Arc<DashMap<Uuid, mpsc::Sender<TimingEvent>>>,
    pub(crate) rate_limits: Arc<DashMap<String, RateState>>,
    /* [084A-42] Rate limiting por IP y contador de conexiones concurrentes */
    pub(crate) ip_rate_limits: Arc<DashMap<String, RateState>>,
    pub(crate) ip_connections: Arc<DashMap<String, u32>>,
    /* [095A-11] Presupuesto por tokens estimados: evita quemar saldo aunque el atacante
     * respete los limites de cantidad de mensajes. Ventana en memoria por hora. */
    pub(crate) ai_token_budgets: Arc<DashMap<String, BudgetState>>,
    /* [096A-7] Semáforo para limitar peticiones IA concurrentes.
     * Sin esto, N visitors simultáneos saturan el pool DB y APIs externas. */
    pub(crate) ai_semaphore: Arc<Semaphore>,
    /* [096A-7] Contador de timing loops activos para health endpoint. */
    pub active_timing_loops: Arc<AtomicU64>,
}

impl Default for ChatTimingService {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatTimingService {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            rate_limits: Arc::new(DashMap::new()),
            ip_rate_limits: Arc::new(DashMap::new()),
            ip_connections: Arc::new(DashMap::new()),
            ai_token_budgets: Arc::new(DashMap::new()),
            ai_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_AI_REQUESTS)),
            active_timing_loops: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Registra una sesión en el timing service. Devuelve el sender para
    /// enviar eventos desde el handler WS. Spawna la tarea de timing.
    /// [T-4] Si la sesión ya está registrada (otra pestaña/dispositivo), retorna
    /// el tx existente sin crear nueva tarea de timing.
    #[must_use]
    pub fn register_session(
        &self,
        session_id: Uuid,
        visitor_name: Option<String>,
        deps: TimingSessionDeps,
    ) -> mpsc::Sender<TimingEvent> {
        /* [T-4] Reutilizar timing loop existente si otra conexión ya registró la sesión */
        if let Some(existing) = self.sessions.get(&session_id) {
            return existing.clone();
        }

        let (tx, rx) = mpsc::channel::<TimingEvent>(64);
        self.sessions.insert(session_id, tx.clone());

        /* [096A-7] Pasar semáforo y contador al loop. Wrap en timeout global de seguridad. */
        let ai_sem = self.ai_semaphore.clone();
        let loop_counter = self.active_timing_loops.clone();
        let counter_ref = loop_counter.clone();
        let timing_service = self.clone();
        tokio::spawn(async move {
            counter_ref.fetch_add(1, Ordering::Relaxed);
            let result = tokio::time::timeout(
                TIMING_LOOP_MAX_LIFETIME,
                session_timing_loop(session_id, visitor_name, rx, deps, ai_sem),
            )
            .await;
            if result.is_err() {
                tracing::warn!(
                    "session_timing_loop {session_id} killed by global timeout ({TIMING_LOOP_MAX_LIFETIME:?})"
                );
            }
            /* [257A-5] El loop sobrevive desconexiones WS transitorias para no
             * perder mensajes ya persistidos que aún esperan respuesta. Por eso
             * su sender se elimina aquí al terminar realmente el loop; de otro
             * modo, el timeout dejaría una entrada cerrada y las reconexiones
             * reutilizarían un canal incapaz de recibir eventos. */
            timing_service.unregister_session(session_id);
            counter_ref.fetch_sub(1, Ordering::Relaxed);
        });

        tx
    }

    /// Elimina la sesión del tracking
    pub fn unregister_session(&self, session_id: Uuid) {
        self.sessions.remove(&session_id);
    }

    /// Enviar evento a una sesión existente (fallback si se registró antes)
    pub async fn send_event(
        &self,
        session_id: Uuid,
        event: TimingEvent,
    ) -> Result<(), &'static str> {
        /* [096A-8] Clonar sender fuera del Ref guard de DashMap.
         * DashMap::get() retiene el lock del shard; retenerlo a través de .await
         * bloquea register_session/unregister_session en ese shard. */
        let tx = self.sessions.get(&session_id).map(|guard| guard.clone());
        if let Some(tx) = tx {
            tx.send(event).await.map_err(|_| "channel closed")
        } else {
            Err("session not registered")
        }
    }

    /* [096A-7] Métricas internas para health endpoint.
     * Retorna (timing_loops_activos, sesiones_registradas, permits_ai_disponibles). */
    #[must_use]
    pub fn metrics(&self) -> (u64, usize, usize) {
        (
            self.active_timing_loops.load(Ordering::Relaxed),
            self.sessions.len(),
            self.ai_semaphore.available_permits(),
        )
    }
}

/* [214A-5] Unit tests para ChatTimingService: rate limiting, IP rate limiting,
 * conexiones WS, constantes de seguridad, y max_message_length.
 * No se testea session_timing_loop (async FSM con muchas dependencias). */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_rate_allows_under_limit() {
        let svc = ChatTimingService::new();
        for i in 1..=RATE_LIMIT_PER_MIN {
            let (result, msg) = svc.check_rate("visitor-1");
            assert!(
                matches!(result, RateCheckResult::Ok),
                "Mensaje {i} debería ser Ok"
            );
            assert!(msg.is_none());
        }
    }

    #[test]
    fn check_rate_warns_on_first_excess() {
        let svc = ChatTimingService::new();
        for _ in 0..RATE_LIMIT_PER_MIN {
            let _ = svc.check_rate("visitor-2");
        }
        let (result, msg) = svc.check_rate("visitor-2");
        assert!(matches!(result, RateCheckResult::Warning));
        assert!(msg.is_some());
    }

    #[test]
    fn check_rate_mutes_on_second_excess() {
        let svc = ChatTimingService::new();
        /* 10 ok + 1 warning = 11 */
        for _ in 0..=RATE_LIMIT_PER_MIN {
            let _ = svc.check_rate("visitor-3");
        }
        /* Siguiente es mute */
        let (result, msg) = svc.check_rate("visitor-3");
        assert!(matches!(result, RateCheckResult::Muted));
        assert!(msg.unwrap().contains("30 segundos"));
    }

    #[test]
    fn check_rate_closes_on_third_excess() {
        let svc = ChatTimingService::new();
        /* 10 ok + 1 warning (cooldown_level=1) */
        for _ in 0..=RATE_LIMIT_PER_MIN {
            let _ = svc.check_rate("visitor-4");
        }
        /* +1 muted (cooldown_level=2, mute_until=30s) */
        let (r2, _) = svc.check_rate("visitor-4");
        assert!(matches!(r2, RateCheckResult::Muted));
        /* Forzar que el mute ya expiró y resetear count para provocar nuevo exceso */
        {
            let mut entry = svc.rate_limits.get_mut("visitor-4").unwrap();
            entry.mute_until = Some(
                Instant::now()
                    .checked_sub(Duration::from_secs(1))
                    /* [259A-1] checked_sub en vez de resta directa (clippy unchecked_time_subtraction);
                     * en test: unwrap porque 1s siempre es restable. */
                    .unwrap(),
            );
            entry.count = RATE_LIMIT_PER_MIN;
        }
        /* Siguiente exceso → cooldown_level=3 → Closed */
        let (result, msg) = svc.check_rate("visitor-4");
        assert!(matches!(result, RateCheckResult::Closed));
        assert!(msg.is_some());
    }

    #[test]
    fn check_rate_muted_stays_muted() {
        let svc = ChatTimingService::new();
        /* Llegar a muted (10 ok + 1 warning + 1 muted) */
        for _ in 0..RATE_LIMIT_PER_MIN + 2 {
            let _ = svc.check_rate("visitor-5");
        }
        /* Mientras está muteado, sigue retornando Muted sin escalar */
        let (result, _) = svc.check_rate("visitor-5");
        /* mute_until está activo → se retorna Muted antes del match de cooldown */
        assert!(matches!(
            result,
            RateCheckResult::Muted | RateCheckResult::Closed
        ));
    }

    #[test]
    fn check_rate_independent_per_visitor() {
        let svc = ChatTimingService::new();
        for _ in 0..RATE_LIMIT_PER_MIN {
            let _ = svc.check_rate("visitor-a");
        }
        /* visitor-b debe empezar limpio */
        let (result, _) = svc.check_rate("visitor-b");
        assert!(matches!(result, RateCheckResult::Ok));
    }

    /* --- IP rate limiting --- */

    #[test]
    fn check_ip_rate_allows_under_limit() {
        let svc = ChatTimingService::new();
        for i in 1..=IP_RATE_LIMIT_PER_MIN {
            let (result, _) = svc.check_ip_rate("192.168.1.1");
            assert!(
                matches!(result, RateCheckResult::Ok),
                "IP msg {i} debería ser Ok"
            );
        }
    }

    #[test]
    fn check_ip_rate_warns_on_excess() {
        let svc = ChatTimingService::new();
        for _ in 0..IP_RATE_LIMIT_PER_MIN {
            let _ = svc.check_ip_rate("10.0.0.1");
        }
        let (result, _) = svc.check_ip_rate("10.0.0.1");
        assert!(matches!(result, RateCheckResult::Warning));
    }

    #[test]
    fn check_ip_rate_mutes_60s_on_second_excess() {
        let svc = ChatTimingService::new();
        for _ in 0..=IP_RATE_LIMIT_PER_MIN {
            let _ = svc.check_ip_rate("10.0.0.2");
        }
        let (result, msg) = svc.check_ip_rate("10.0.0.2");
        assert!(matches!(result, RateCheckResult::Muted));
        assert!(msg.unwrap().contains("un minuto"));
    }

    /* --- Conexiones WS por IP --- */

    #[test]
    fn track_ip_connect_allows_under_max() {
        let svc = ChatTimingService::new();
        for _ in 0..MAX_WS_CONNECTIONS_PER_IP {
            assert!(svc.track_ip_connect("1.2.3.4"));
        }
    }

    #[test]
    fn track_ip_connect_rejects_over_max() {
        let svc = ChatTimingService::new();
        for _ in 0..MAX_WS_CONNECTIONS_PER_IP {
            let _ = svc.track_ip_connect("5.6.7.8");
        }
        assert!(!svc.track_ip_connect("5.6.7.8"));
    }

    #[test]
    fn track_ip_disconnect_frees_slot() {
        let svc = ChatTimingService::new();
        for _ in 0..MAX_WS_CONNECTIONS_PER_IP {
            let _ = svc.track_ip_connect("9.8.7.6");
        }
        assert!(!svc.track_ip_connect("9.8.7.6"));
        svc.track_ip_disconnect("9.8.7.6");
        assert!(svc.track_ip_connect("9.8.7.6"));
    }

    #[test]
    fn track_ip_disconnect_saturating() {
        let svc = ChatTimingService::new();
        /* Disconnect sin connect previo no debe panic */
        svc.track_ip_disconnect("never-connected");
        /* También funciona con una sola conexión */
        let _ = svc.track_ip_connect("once");
        svc.track_ip_disconnect("once");
        svc.track_ip_disconnect("once"); /* doble disconnect no causa underflow */
    }

    #[test]
    fn track_ip_independent_per_ip() {
        let svc = ChatTimingService::new();
        for _ in 0..MAX_WS_CONNECTIONS_PER_IP {
            let _ = svc.track_ip_connect("ip-a");
        }
        assert!(!svc.track_ip_connect("ip-a"));
        assert!(svc.track_ip_connect("ip-b"));
    }

    /* --- Constantes de seguridad --- */

    #[test]
    fn max_message_length_is_2000() {
        assert_eq!(ChatTimingService::max_message_length(), 2000);
    }

    #[test]
    fn max_ai_combined_chars_limits_bursts() {
        assert_eq!(ChatTimingService::max_ai_combined_chars(), 6000);
    }

    #[test]
    fn estimate_ai_request_tokens_includes_overhead() {
        let estimate = ChatTimingService::estimate_ai_request_tokens("a".repeat(400).as_str());
        assert_eq!(estimate, AI_REQUEST_OVERHEAD_TOKENS + 101);
    }

    #[test]
    fn visitor_ai_budget_blocks_excessive_estimated_tokens() {
        let svc = ChatTimingService::new();
        let large = "a".repeat((AI_VISITOR_TOKEN_BUDGET_PER_HOUR + 1) * 4);
        let (result, msg) = svc.check_visitor_ai_budget("visitor-budget", &large);
        assert!(matches!(result, RateCheckResult::Muted));
        assert!(msg.is_some());
    }

    #[test]
    fn ip_ai_budget_tracks_independently_from_visitor_budget() {
        let svc = ChatTimingService::new();
        let content = "hola";
        let (visitor_result, _) = svc.check_visitor_ai_budget("visitor-budget-ok", content);
        let (ip_result, _) = svc.check_ip_ai_budget("203.0.113.10", content);
        assert!(matches!(visitor_result, RateCheckResult::Ok));
        assert!(matches!(ip_result, RateCheckResult::Ok));
    }

    #[test]
    fn ai_relevance_is_opt_in() {
        assert!(!ai_relevance_enabled(None));
        assert!(!ai_relevance_enabled(Some("false")));
        assert!(ai_relevance_enabled(Some("true")));
        assert!(ai_relevance_enabled(Some("TRUE")));
    }

    #[test]
    fn rate_constants_sane() {
        /* [259A-1] assert sobre constantes en bloque const (clippy assertions_on_constants). */
        const {
            assert!(RATE_LIMIT_PER_MIN > 0);
            assert!(IP_RATE_LIMIT_PER_MIN > RATE_LIMIT_PER_MIN);
            assert!(MAX_WS_CONNECTIONS_PER_IP > 0);
            assert!(AI_IP_TOKEN_BUDGET_PER_HOUR > AI_VISITOR_TOKEN_BUDGET_PER_HOUR);
        }
    }
}
