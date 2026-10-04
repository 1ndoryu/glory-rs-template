/* [01AA-4-f3q] Fachada de tools IA misceláneas (extraido de ai_tools_misc.rs).
 * defs = definiciones JSON; invoice = factura Stripe; escalation = humano + CTA;
 * visitor_memory = email + info del visitante; support_ticket = tickets. */

mod defs;
mod escalation;
mod invoice;
mod support_ticket;
mod visitor_memory;

pub(crate) use defs::{registered_client_tool_defs, service_tool_defs, visitor_tool_defs};
pub(crate) use escalation::exec_request_human;
pub(crate) use invoice::exec_create_invoice;
pub(crate) use support_ticket::exec_create_support_ticket;
pub(crate) use visitor_memory::{exec_capture_email, exec_save_client_info};
