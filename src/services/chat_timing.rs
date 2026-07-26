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

use std::fmt::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{mpsc, Semaphore};
use uuid::Uuid;

use crate::models::{CreateNotification, NOTIF_ESCALATION_NEEDED};
use crate::repositories::UserRepository;
use crate::services::{AiChatConfig, AiChatService, AiResponse, ChatHub, NotificationHub};

use super::ai_providers::{call_ai_api_with_options, ChatApiOptions};

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
const WAIT_TIMEOUT: Duration = Duration::from_secs(1);
const LISTEN_TIMEOUT: Duration = Duration::from_secs(8);
const TYPING_COOLDOWN: Duration = Duration::from_secs(3);
const MAX_ACCUMULATION: Duration = Duration::from_secs(30);

/* Rate limiting */
const RATE_LIMIT_PER_MIN: u32 = 10;
const RATE_WINDOW: Duration = Duration::from_mins(1);

/* [084A-42] Anti-bot: limites por IP (más altos porque IPs pueden ser compartidas) */
const IP_RATE_LIMIT_PER_MIN: u32 = 30;
const MAX_WS_CONNECTIONS_PER_IP: u32 = 10;
const MSG_MAX_LENGTH: usize = 2000;
const AI_MAX_COMBINED_CHARS: usize = 6000;
const AI_TOKEN_BUDGET_WINDOW: Duration = Duration::from_hours(1);
const AI_VISITOR_TOKEN_BUDGET_PER_HOUR: usize = 24_000;
const AI_IP_TOKEN_BUDGET_PER_HOUR: usize = 80_000;
const AI_REQUEST_OVERHEAD_TOKENS: usize = 1_500;

/* Relevancia */
const MAX_IRRELEVANT_STREAK: u32 = 3;

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
struct RateState {
    count: u32,
    window_start: Instant,
    cooldown_level: u8,
    mute_until: Option<Instant>,
}

struct BudgetState {
    tokens: usize,
    window_start: Instant,
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

/// Servicio de timing: gestiona sesiones activas y rate limiting global
#[derive(Clone)]
pub struct ChatTimingService {
    sessions: Arc<DashMap<Uuid, mpsc::Sender<TimingEvent>>>,
    rate_limits: Arc<DashMap<String, RateState>>,
    /* [084A-42] Rate limiting por IP y contador de conexiones concurrentes */
    ip_rate_limits: Arc<DashMap<String, RateState>>,
    ip_connections: Arc<DashMap<String, u32>>,
    /* [095A-11] Presupuesto por tokens estimados: evita quemar saldo aunque el atacante
     * respete los limites de cantidad de mensajes. Ventana en memoria por hora. */
    ai_token_budgets: Arc<DashMap<String, BudgetState>>,
    /* [096A-7] Semáforo para limitar peticiones IA concurrentes.
     * Sin esto, N visitors simultáneos saturan el pool DB y APIs externas. */
    ai_semaphore: Arc<Semaphore>,
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

/* Máquina de estados del timing por sesión.
 * Recibe eventos, bufferea mensajes, decide cuándo responder.
 * Se ejecuta como tokio::spawn por cada sesión activa. */
async fn session_timing_loop(
    session_id: Uuid,
    visitor_name: Option<String>,
    mut rx: mpsc::Receiver<TimingEvent>,
    deps: TimingSessionDeps,
    ai_semaphore: Arc<Semaphore>,
) {
    let mut buffer: Vec<String> = Vec::new();
    let mut is_typing = false;
    let mut irrelevant_count: u32 = 0;

    tracing::info!(%session_id, "session_timing_loop: iniciado");
    loop {
        /* Estado IDLE: esperar primer mensaje */
        let Some(event) = rx.recv().await else {
            tracing::info!(%session_id, "session_timing_loop: channel cerrado, saliendo");
            break; /* channel cerrado, sesión terminó */
        };

        tracing::info!(%session_id, ?event, "session_timing_loop: evento recibido");

        match event {
            TimingEvent::Disconnect => break,
            TimingEvent::TypingStart => {
                is_typing = true;
                continue;
            }
            TimingEvent::TypingStop => {
                is_typing = false;
                continue;
            }
            TimingEvent::Message(content) => {
                buffer.push(content);
            }
        }

        /* Acumular mensajes hasta timeout */
        let first_msg = Instant::now();
        accumulate_messages(&mut rx, &mut buffer, &mut is_typing, first_msg).await;

        if buffer.is_empty() {
            continue;
        }

        /* Estado RESPONDING */
        let combined = buffer.join("\n");
        buffer.clear();

        tracing::info!(%session_id, chars = combined.len(), "Iniciando respuesta IA");
        /* [096A-7] Adquirir permit del semáforo antes de llamar a IA.
         * Limita peticiones IA concurrentes para proteger pool DB y APIs. */
        /* [096A-8] Timeout en semaphore: si 3 permits ocupados >30s, abortar en vez de bloquear.
         * Sin timeout, una 4ta sesión espera indefinidamente → canal mpsc se llena → WS se congela. */
        let _permit =
            match tokio::time::timeout(Duration::from_secs(30), ai_semaphore.acquire()).await {
                Ok(permit) => permit.expect("semaphore closed"),
                Err(_) => {
                    tracing::warn!("Timeout 30s esperando semáforo IA para sesión {session_id}");
                    continue;
                }
            };
        irrelevant_count = generate_ai_response(
            session_id,
            visitor_name.as_deref(),
            &combined,
            irrelevant_count,
            &deps,
        )
        .await;
        drop(_permit);
    }

    /* [T-3] Al cerrar sesión, generar resumen de contexto para futuras conversaciones.
     * Usa modelo pequeño para resumir el historial y lo guarda en visitor_profiles.
     * [096A-7] Wrap en timeout 60s: fire-and-forget no debe vivir indefinidamente. */
    let summary_pool = deps.pool.clone();
    let summary_config = deps.ai_config.clone();
    let summary_http = deps.http_client.clone();
    tokio::spawn(async move {
        let _ = tokio::time::timeout(
            Duration::from_secs(60),
            generate_context_summary(
                summary_pool,
                summary_config,
                summary_http,
                session_id,
                deps.visitor_id,
            ),
        )
        .await;
    });
}

/* Acumula mensajes del buffer hasta que expira el timeout.
 * Gestiona transiciones WAITING ↔ LISTENING según typing indicators. */
async fn accumulate_messages(
    rx: &mut mpsc::Receiver<TimingEvent>,
    buffer: &mut Vec<String>,
    is_typing: &mut bool,
    first_msg: Instant,
) {
    loop {
        let deadline = if *is_typing {
            LISTEN_TIMEOUT
        } else {
            WAIT_TIMEOUT
        };

        let max_remaining = MAX_ACCUMULATION
            .checked_sub(first_msg.elapsed())
            .unwrap_or(Duration::ZERO);

        let effective_timeout = deadline.min(max_remaining);

        tokio::select! {
            () = tokio::time::sleep(effective_timeout) => {
                /* Timeout expirado. Si typing activo, esperar cooldown extra */
                if *is_typing {
                    tokio::time::sleep(TYPING_COOLDOWN).await;
                    drain_pending(rx, buffer, is_typing);
                }
                return; /* listo para responder */
            }
            Some(evt) = rx.recv() => {
                match evt {
                    TimingEvent::Message(c) => { buffer.push(c); }
                    TimingEvent::TypingStart => { *is_typing = true; }
                    TimingEvent::TypingStop => { *is_typing = false; }
                    TimingEvent::Disconnect => { buffer.clear(); return; }
                }
                /* Continuar acumulando */
            }
        }
    }
}

/* Drena eventos pendientes sin bloquear (para el cooldown post-typing) */
fn drain_pending(
    rx: &mut mpsc::Receiver<TimingEvent>,
    buffer: &mut Vec<String>,
    is_typing: &mut bool,
) {
    while let Ok(evt) = rx.try_recv() {
        match evt {
            TimingEvent::Message(c) => buffer.push(c),
            TimingEvent::TypingStart => *is_typing = true,
            TimingEvent::TypingStop => *is_typing = false,
            TimingEvent::Disconnect => {
                buffer.clear();
                return;
            }
        }
    }
}

/* [257A-9] Revalida en PostgreSQL justo antes de publicar. Esto evita que una
 * respuesta generada durante 90 s aparezca después de que el humano contestó
 * o pulsó detener IA, incluso si ocurrió en otro worker o tras una reconexión. */
async fn ai_generation_is_current(
    pool: &sqlx::PgPool,
    session_id: Uuid,
    expected_epoch: i64,
) -> bool {
    match crate::repositories::ChatRepository::find_session_by_id(pool, session_id).await {
        Ok(Some(session)) => {
            session.ai_enabled
                && session.assigned_staff_id.is_none()
                && session.ai_mode != "manual_pause"
                && session.ai_generation_epoch == expected_epoch
        }
        _ => false,
    }
}

/* Genera respuesta IA con el buffer combinado.
 * Verifica sesión activa, clasifica relevancia, genera respuesta y escala si necesario.
 * [T-2] Envía rich_messages (service_cards, invoices) como mensajes separados.
 * Retorna el nuevo valor de irrelevant_count. */
async fn generate_ai_response(
    session_id: Uuid,
    visitor_name: Option<&str>,
    combined: &str,
    mut irrelevant_count: u32,
    deps: &TimingSessionDeps,
) -> u32 {
    /* Verificar que la sesión sigue con IA activa.
     * [237A-9] Respetar ai_mode: manual_pause bloquea IA completamente;
     * human_priority con ciclo waiting también bloquea (el worker genera fallback). */
    let session =
        match crate::repositories::ChatRepository::find_session_by_id(&deps.pool, session_id).await
        {
            Ok(Some(s)) => s,
            _ => {
                tracing::info!(%session_id, "generate_ai_response: sesión no encontrada");
                return irrelevant_count;
            }
        };

    if !session.ai_enabled || session.assigned_staff_id.is_some() {
        tracing::info!(%session_id, "generate_ai_response: sesión no activa, saltando IA");
        return irrelevant_count;
    }

    /* [237A-9] manual_pause: IA desactivada por staff explícitamente */
    if session.ai_mode == "manual_pause" {
        tracing::debug!(%session_id, "generate_ai_response: ai_mode=manual_pause, saltando IA");
        return irrelevant_count;
    }

    /* [237A-9] human_priority con ciclo waiting: dejar que el humano responda.
     * El worker de response cycles generará fallback si expira el deadline. */
    if session.ai_mode == "human_priority" {
        if let Ok(true) =
            crate::repositories::ResponseCycleRepository::is_in_human_window(&deps.pool, session_id)
                .await
        {
            tracing::debug!(%session_id, "generate_ai_response: human_priority + ciclo waiting, saltando IA");
            return irrelevant_count;
        }
    }

    /* [257A-9] Capturar la versión antes de cualquier llamada lenta. Un toggle
     * o mensaje humano la incrementa en BD y vuelve obsoleta esta generación. */
    let generation_epoch = session.ai_generation_epoch;

    if !ensure_ai_request_allowed(session_id, combined, deps).await {
        return irrelevant_count;
    }

    /* Clasificador de relevancia: filtrar off-topic con modelo pequeño */
    if let Ok(false) =
        check_relevance(&deps.pool, &deps.ai_config, combined, &deps.http_client).await
    {
        irrelevant_count += 1;
        let msg = if irrelevant_count >= MAX_IRRELEVANT_STREAK {
            irrelevant_count = 0;
            "Parece que tus consultas no están relacionadas con nuestros \
             servicios. Si necesitas ayuda con diseño web, desarrollo de apps, \
             branding o agentes IA, estoy aquí para ayudarte."
        } else {
            "Interesante. ¿Hay algo relacionado con nuestros servicios \
             en lo que pueda ayudarte? Ofrecemos diseño web, desarrollo \
             de aplicaciones, branding y agentes IA."
        };
        if ai_generation_is_current(&deps.pool, session_id, generation_epoch).await {
            let _ = deps
                .hub
                .send_message(session_id, "ai", Some("ai"), msg)
                .await;
        }
        return irrelevant_count;
    }

    /* Relevante: reset streak y generar respuesta principal */
    irrelevant_count = 0;

    /* [114A-6] Timeout global 90s para toda la cadena de retries de IA.
     * Sin esto, la cadena Groq (3 keys × 3 modelos) + Gemini (6 modelos)
     * podría bloquear hasta 7+ minutos reteniendo conexión DB. */
    tracing::info!(%session_id, "Llamando a IA para generar respuesta...");
    let ai_result = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        AiChatService::generate_response(
            &deps.pool,
            &deps.ai_config,
            &deps.http_client,
            deps.stripe_key.as_deref(),
            crate::services::AiSessionContext {
                session_id,
                visitor_id: Some(&deps.visitor_id),
                auth: deps.auth,
                user_id: deps.user_id,
                context: deps.context.as_deref(),
            },
            combined,
        ),
    )
    .await;

    let ai_resp = if let Ok(result) = ai_result {
        result.unwrap_or_else(|e| {
            tracing::warn!(%session_id, error = %e, "Error en respuesta IA");
            AiResponse {
                text: format!("Error IA: {e}"),
                needs_escalation: true,
                rich_messages: Vec::new(),
            }
        })
    } else {
        tracing::error!("AI response timeout (90s) para sesión {session_id}");
        AiResponse {
            text: "Disculpa, estoy tardando más de lo normal. Un miembro del equipo \
                   te asistirá en breve."
                .to_string(),
            needs_escalation: true,
            rich_messages: Vec::new(),
        }
    };

    tracing::info!(%session_id, has_escalation = ai_resp.needs_escalation, rich_count = ai_resp.rich_messages.len(), "Respuesta IA recibida, enviando...");

    if !ai_generation_is_current(&deps.pool, session_id, generation_epoch).await {
        tracing::info!(%session_id, generation_epoch, "Respuesta IA descartada por intervención humana");
        return irrelevant_count;
    }

    /* [T-2] Enviar rich messages (service_cards, invoices) antes del texto */
    for rm in &ai_resp.rich_messages {
        if !ai_generation_is_current(&deps.pool, session_id, generation_epoch).await {
            tracing::info!(%session_id, generation_epoch, "Rich messages IA interrumpidos por intervención humana");
            return irrelevant_count;
        }
        let _ = deps
            .hub
            .send_rich_message(
                session_id,
                "ai",
                Some("ai"),
                &rm.content,
                &rm.message_type,
                &rm.metadata,
            )
            .await;
    }

    if ai_generation_is_current(&deps.pool, session_id, generation_epoch).await {
        let _ = deps
            .hub
            .send_message(session_id, "ai", Some("ai"), &ai_resp.text)
            .await;
    }

    if ai_resp.needs_escalation {
        send_escalation(
            &deps.pool,
            &deps.notification_hub,
            session_id,
            visitor_name,
            deps.email_config.as_ref(),
        )
        .await;
    }

    irrelevant_count
}

async fn ensure_ai_request_allowed(
    session_id: Uuid,
    combined: &str,
    deps: &TimingSessionDeps,
) -> bool {
    if combined.len() > ChatTimingService::max_ai_combined_chars() {
        tracing::warn!(
            "AI bloqueada por input combinado excesivo: session={session_id}, chars={}",
            combined.len()
        );
        send_ai_budget_message(
            deps,
            session_id,
            "Recibí demasiado texto de golpe. Envíame un resumen más corto o espera a una persona del equipo.",
        )
        .await;
        return false;
    }

    let (budget_result, budget_msg) = deps
        .chat_timing
        .check_visitor_ai_budget(&deps.visitor_id, combined);
    if !matches!(budget_result, RateCheckResult::Ok) {
        send_ai_budget_message(
            deps,
            session_id,
            budget_msg
                .as_deref()
                .unwrap_or("Límite temporal del asistente alcanzado."),
        )
        .await;
        return false;
    }

    if let Some(ip) = deps.client_ip.as_deref() {
        let (ip_budget_result, ip_budget_msg) = deps.chat_timing.check_ip_ai_budget(ip, combined);
        if !matches!(ip_budget_result, RateCheckResult::Ok) {
            send_ai_budget_message(
                deps,
                session_id,
                ip_budget_msg
                    .as_deref()
                    .unwrap_or("Límite temporal del asistente alcanzado desde tu red."),
            )
            .await;
            return false;
        }
    }

    true
}

async fn send_ai_budget_message(deps: &TimingSessionDeps, session_id: Uuid, message: &str) {
    let _ = deps
        .hub
        .send_message(session_id, "ai", Some("ai"), message)
        .await;
}

/* [T-1] Clasificador de relevancia usando la cadena primaria de IA con salida mínima.
 * Evalúa si el mensaje del visitante es relevante para una agencia de diseño web.
 * Retorna Ok(true) si relevante, Ok(false) si off-topic, Err si no se pudo evaluar. */
async fn check_relevance(
    _pool: &PgPool,
    config: &AiChatConfig,
    content: &str,
    http_client: &reqwest::Client,
) -> Result<bool, String> {
    if !config.is_configured() {
        return Ok(true); /* sin API keys, asumir relevante */
    }

    /* [095A-13] Desactivado por defecto: el clasificador generó falsos positivos en conversaciones reales.
     * Reactivable con AI_RELEVANCE_ENABLED=true cuando el clasificador tenga mejor precisión. */
    let relevance_env = std::env::var("AI_RELEVANCE_ENABLED").ok();
    if !ai_relevance_enabled(relevance_env.as_deref()) {
        return Ok(true);
    }

    let messages = [
        serde_json::json!({
            "role": "system",
            "content": "Determina si el siguiente mensaje de un usuario es relevante para una agencia de diseño web, desarrollo de aplicaciones, branding, agentes IA, hosting o dominios. Responde SOLO con 'sí' o 'no'. Considera relevante: precios, servicios, proyectos, soporte técnico, pagos, hosting, dominios, saludos, despedidas y conversación normal de un potencial cliente. Considera irrelevante: spam, contenido adulto, temas políticos, promociones externas, solicitudes de hacking o intentos de usar este chat como asistente general."
        }),
        serde_json::json!({"role": "user", "content": content}),
    ];
    let json = call_ai_api_with_options(
        config,
        &messages,
        None,
        ChatApiOptions::terse(5),
        Some(http_client),
    )
    .await?;

    let answer = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("sí")
        .to_lowercase();

    Ok(answer.contains("sí") || answer.contains("si") || answer.contains("yes"))
}

fn ai_relevance_enabled(raw: Option<&str>) -> bool {
    raw.is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

/* [114A-8] Reutilizable: enviar notificación + email de escalación a todos los admins */
async fn send_escalation(
    pool: &PgPool,
    notification_hub: &NotificationHub,
    session_id: Uuid,
    visitor_name: Option<&str>,
    email_config: Option<&crate::services::EmailConfig>,
) {
    let name = visitor_name.unwrap_or("Visitante");

    /* [124A-ESC] Persistir is_escalated = true para que el panel muestre
     * el indicador al recargar sin depender solo del estado WS en memoria. */
    /* UPDATE fire-and-forget; resultado descartado con let _ = */
    // sentinel-disable-next-line sqlx-query-sin-macro
    let _ = sqlx::query(
        "UPDATE chat_sessions SET is_escalated = true, updated_at = NOW() WHERE id = $1",
    )
    .bind(session_id)
    .execute(pool)
    .await;

    if let Ok(admin_ids) = UserRepository::admin_ids(pool).await {
        if !admin_ids.is_empty() {
            let base = CreateNotification {
                user_id: Uuid::nil(),
                notification_type: NOTIF_ESCALATION_NEEDED.to_string(),
                title: format!("Escalación: {name} necesita ayuda"),
                body: Some(
                    "La IA detectó que se requiere intervención humana en la sesión de chat."
                        .to_string(),
                ),
                link: Some(format!("/admin/chat?session={session_id}")),
                reference_type: Some("chat_session".to_string()),
                reference_id: Some(session_id),
            };
            let _ = notification_hub.notify_many(&admin_ids, &base).await;
            tracing::info!(
                "Escalación enviada a {} admins para sesión {session_id}",
                admin_ids.len()
            );
        }
    }

    /* [114A-8] Email de escalación a admins */
    if let Some(cfg) = email_config {
        if let Ok(emails) = UserRepository::admin_emails(pool).await {
            if !emails.is_empty() {
                let site_url = std::env::var("SITE_URL")
                    .unwrap_or_else(|_| "https://nakomi.studio".to_string());
                crate::services::EmailService::send_escalation_emails(
                    cfg, pool, &emails, name, session_id, &site_url,
                )
                .await;
            }
        }
    }
}

/* [T-3] Genera resumen de la conversación al cerrar sesión.
 * Usa modelo pequeño (llama-3.1-8b-instant) para resumir el historial
 * del visitante y lo guarda en visitor_profiles.context_summary.
 * Se ejecuta como tokio::spawn para no bloquear el cierre de WS. */
async fn generate_context_summary(
    pool: PgPool,
    config: AiChatConfig,
    http_client: reqwest::Client,
    session_id: Uuid,
    visitor_id: String,
) {
    if !config.is_configured() {
        return;
    }

    /* Obtener historial de la sesión */
    let messages =
        match crate::repositories::ChatRepository::list_messages(&pool, session_id, 50, 0).await {
            Ok(msgs) if !msgs.is_empty() => msgs,
            _ => return,
        };

    /* Construir transcript compacto */
    let mut transcript = String::new();
    for msg in &messages {
        /* [084A-46] Agente renombrado a Claudia */
        let role = if msg.sender_type == "ai" {
            "Claudia"
        } else {
            "Cliente"
        };
        let _ = writeln!(transcript, "{role}: {}", msg.content);
    }

    if transcript.len() < 100 {
        return;
    }

    let transcript_truncated = if transcript.len() > 3000 {
        &transcript[..3000]
    } else {
        &transcript
    };

    let Some(summary) = call_summary_api(&config, transcript_truncated, &http_client).await else {
        return;
    };

    /* Cargar resumen previo y concatenar (max 2000 chars total) */
    let existing =
        match crate::repositories::ChatRepository::find_visitor_profile(&pool, &visitor_id).await {
            Ok(Some(p)) => p.context_summary.unwrap_or_default(),
            _ => String::new(),
        };

    let final_summary = if existing.is_empty() {
        summary
    } else {
        let combined = format!("{existing}\n---\n{summary}");
        if combined.len() > 2000 {
            combined[combined.len() - 2000..].to_string()
        } else {
            combined
        }
    };

    if let Err(e) = crate::repositories::ChatRepository::update_context_summary(
        &pool,
        &visitor_id,
        &final_summary,
    )
    .await
    {
        tracing::warn!("Error guardando context summary para {visitor_id}: {e}");
    } else {
        tracing::info!("Context summary actualizado para visitor {visitor_id}");
    }
}

/* [T-3] Llama a la API de Groq con modelo ligero para generar resumen de sesión.
 * Retorna None si la API falla o el resumen está vacío. */
async fn call_summary_api(
    config: &AiChatConfig,
    transcript: &str,
    http_client: &reqwest::Client,
) -> Option<String> {
    let messages = [
        serde_json::json!({
            "role": "system",
            "content": "Genera un resumen conciso (máximo 500 caracteres) de esta conversación \
                 de chat de soporte. Incluye: qué necesitaba el cliente, qué servicios le interesaron, \
                 si se capturó email, si se generó factura, si quedó algo pendiente, y cualquier \
                 preferencia o dato relevante del cliente. Solo el resumen, sin formato especial."
        }),
        serde_json::json!({"role": "user", "content": transcript}),
    ];

    match call_ai_api_with_options(
        config,
        &messages,
        None,
        ChatApiOptions::terse(200),
        Some(http_client),
    )
    .await
    {
        Ok(json) => {
            let s = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        Err(e) => {
            tracing::warn!("Context summary API error: {e}");
            None
        }
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
            entry.mute_until = Some(Instant::now() - Duration::from_secs(1));
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
        assert!(RATE_LIMIT_PER_MIN > 0);
        assert!(IP_RATE_LIMIT_PER_MIN > RATE_LIMIT_PER_MIN);
        assert!(MAX_WS_CONNECTIONS_PER_IP > 0);
        assert!(AI_IP_TOKEN_BUDGET_PER_HOUR > AI_VISITOR_TOKEN_BUDGET_PER_HOUR);
    }
}
