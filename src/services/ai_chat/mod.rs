/* [064A-29] AI Chat Service: integración con Groq API (OpenAI-compatible).
 * [114A-12] Rotación de API keys eliminada 2026-05-23. Siempre usa la primera key.
 * System prompt dinámico: pre-venta (servicios, precios) vs soporte de orden
 * (contexto de orden, fase actual, historial). Usa reqwest HTTP client.
 * [01AA-4-F3g] Partido por responsabilidad (era god-object 643): config =
 * configuración multi-proveedor + sanitize; responses = loop de respuesta +
 * tool calls; context = ensamblado de mensajes + helpers puros. Misma API pública. */

mod config;
mod context;
mod responses;

pub(crate) use config::sanitize_for_prompt;
pub use config::AiChatConfig;
pub use responses::{AiChatService, AiResponse, AiSessionContext};
