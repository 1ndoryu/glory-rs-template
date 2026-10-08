mod alerta_whatsapp;
mod auth;
mod canal_resolver;
mod inmueble;
pub mod marketplace;
mod marketplace_vuelo; // [08AA-7] Singleflight en su dominio (re-exportado arriba)
mod note;
mod outbox_idempotency;
pub mod politica;
pub mod sesion;
mod solicitud;
mod suscriptor;
mod tope_uso;
pub mod transporte;
pub mod triage;
pub mod turno;

pub use alerta_whatsapp::vigilar as vigilar_alertas_whatsapp;
pub use auth::{AuthService, Claims};
pub use canal_resolver::{modo_por_canal, CanalResolver};
pub use inmueble::InmuebleService;
pub use note::NoteService;
pub use outbox_idempotency::{
    clave_idempotencia, corte_cubre, debe_usar_clave, encolar as encolar_outbox_idem,
    marcar as marcar_outbox, purgar_resueltos, reencolar_fallidos, Encolado,
};
pub use politica::{
    codigo_trato, leer_autorizados, prefijo_contexto, resolver, texto_para_turno, DecisionPolitica,
    Rol, CLAVE_AUTORIZADOS, REGLA_FRONTERA,
};
pub use solicitud::SolicitudService;
pub use suscriptor::SuscriptorService;
pub use tope_uso::{revisar_tope, vigilar as vigilar_tope_uso};
pub use triage::{
    decidir, decidir_para, evaluar_trato, Contenido, Decision, Duplicidad, EntradaSinResolver,
    EvaluacionTrato, EventoTriage, MarcaTransporte, Motivo, Procedencia, RegistroDuplicados, Trato,
    VENTANA_RETRASO_MIN_DEFAULT,
};
