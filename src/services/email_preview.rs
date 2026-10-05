/* [311A-INV] Módulo de previsualización de plantillas email.
 * Genera HTML de muestra para cada plantilla usando datos ficticios,
 * sin enviar correos reales ni requerir conexión SMTP/BD.
 *
 * [20CA-11] Refactorizado: delega a `email_templates::render_*()` con datos SAMPLE,
 * eliminando toda duplicación de HTML con email.rs. */

use crate::services::email::EmailConfig;

/// Metadatos de una plantilla de email
#[derive(Debug, Clone, serde::Serialize)]
pub struct TemplateMeta {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub recipients: &'static str,
}

/* [01AA-4-f3s] Catálogo estático de plantillas (dato, cero ramas): el vec!
 * monolítico (150 efectivas) se hoista a static y la fn solo lo clona. */
static TEMPLATES: &[TemplateMeta] = &[
        TemplateMeta {
            id: "order_confirmation",
            label: "Confirmación al cliente",
            description: "Se envía al cliente tras crear un pedido",
            category: "orders",
            recipients: "cliente",
        },
        TemplateMeta {
            id: "new_order_admin",
            label: "Nueva orden (admin)",
            description: "Notifica a los admins cuando se crea un pedido",
            category: "orders",
            recipients: "admin",
        },
        TemplateMeta {
            id: "payment_received_admin",
            label: "Pago recibido (admin)",
            description: "Notifica a los admins cuando un pago se recibe",
            category: "payments",
            recipients: "admin",
        },
        TemplateMeta {
            id: "order_completed_client",
            label: "Orden completada (cliente)",
            description: "Se envía al cliente cuando su orden se completa",
            category: "orders",
            recipients: "cliente",
        },
        TemplateMeta {
            id: "order_completed_admin",
            label: "Orden completada (admin)",
            description: "Notifica a admins cuando una orden se completa",
            category: "orders",
            recipients: "admin",
        },
        TemplateMeta {
            id: "order_cancelled_client",
            label: "Orden cancelada (cliente)",
            description: "Se envía al cliente cuando su orden se cancela",
            category: "orders",
            recipients: "cliente",
        },
        TemplateMeta {
            id: "order_cancelled_admin",
            label: "Orden cancelada (admin)",
            description: "Notifica a admins cuando una orden se cancela",
            category: "orders",
            recipients: "admin",
        },
        TemplateMeta {
            id: "phase_delivered_client",
            label: "Fase entregada (cliente)",
            description: "Notifica al cliente que una fase fue entregada",
            category: "orders",
            recipients: "cliente",
        },
        TemplateMeta {
            id: "problem_reported_client",
            label: "Problema reportado (cliente)",
            description: "Se envía al cliente cuando se reporta un problema",
            category: "orders",
            recipients: "cliente",
        },
        TemplateMeta {
            id: "problem_reported_admin",
            label: "Problema reportado (admin)",
            description: "Notifica a admins cuando se reporta un problema",
            category: "orders",
            recipients: "admin",
        },
        TemplateMeta {
            id: "refund_requested_admin",
            label: "Reembolso solicitado (admin)",
            description: "Notifica a admins cuando un cliente solicita reembolso",
            category: "orders",
            recipients: "admin",
        },
        TemplateMeta {
            id: "escalation",
            label: "Escalación de chat",
            description: "Notifica a admins cuando la IA escala un chat",
            category: "chat",
            recipients: "admin",
        },
        TemplateMeta {
            id: "chat_invoice_paid_client",
            label: "Factura chat pagada (cliente)",
            description: "Confirma al cliente el pago de una factura de chat",
            category: "chat",
            recipients: "cliente",
        },
        TemplateMeta {
            id: "chat_invoice_paid_admin",
            label: "Factura chat pagada (admin)",
            description: "Notifica a admins del pago de una factura de chat",
            category: "chat",
            recipients: "admin",
        },
        TemplateMeta {
            id: "vps_pending_approval",
            label: "VPS pendiente (admin)",
            description: "Notifica a admins de una suscripción VPS pendiente",
            category: "vps",
            recipients: "admin",
        },
        TemplateMeta {
            id: "vps_approved",
            label: "VPS aprobado (cliente)",
            description: "Se envía al cliente cuando su VPS es aprobado",
            category: "vps",
            recipients: "cliente",
        },
        TemplateMeta {
            id: "vps_rejected",
            label: "VPS rechazado (cliente)",
            description: "Se envía al cliente cuando su VPS es rechazado",
            category: "vps",
            recipients: "cliente",
        },
        TemplateMeta {
            id: "new_user_registered_admin",
            label: "Nuevo usuario registrado (admin)",
            description: "Notifica a admins cuando un nuevo usuario se registra",
            category: "profile",
            recipients: "admin",
        },
        TemplateMeta {
            id: "profile_email_changed_new",
            label: "Email cambiado (nuevo)",
            description: "Confirma al nuevo correo el cambio de email",
            category: "profile",
            recipients: "cliente",
        },
        TemplateMeta {
            id: "profile_email_changed_old",
            label: "Email cambiado (anterior)",
            description: "Alerta al correo anterior sobre el cambio",
            category: "profile",
            recipients: "cliente",
        },
        TemplateMeta {
            id: "profile_password_changed",
            label: "Contraseña cambiada",
            description: "Notifica al usuario que su contraseña fue cambiada",
            category: "profile",
            recipients: "cliente",
        },
];

/// Lista completa de plantillas disponibles
pub fn list_templates() -> Vec<TemplateMeta> {
    TEMPLATES.to_vec()
}

pub fn render_preview(_config: &EmailConfig, template: &str) -> Result<String, String> {
    use super::email_templates as t;

    const NAME: &str = "Juan Pérez";
    const EMAIL: &str = "cliente@ejemplo.com";
    const ORDER: i32 = 12345;
    const SERVICE: &str = "Diseño Web Profesional";
    const PLAN: &str = "Pro";
    const PRICE: &str = "$499.00 USD";
    const PAYMENT_MODE: &str = "transferencia";
    const REASON: &str = "Fondos insuficientes en el método de pago";
    const PHASE: &str = "Maquetación responsive";
    const PROBLEM_DESC: &str = "El formulario de contacto no envía los datos correctamente. Ya verificamos la configuración SMTP y el problema persiste.";
    const VISITOR: &str = "María García";
    const TIER: &str = "VPS Avanzado";
    const PUBLIC_IP: &str = "203.0.113.42";
    const USERNAME: &str = "root";
    const PASSWORD: &str = "s3gur4-P@ss!";
    const OLD_EMAIL: &str = "anterior@ejemplo.com";
    const NEW_EMAIL: &str = "nuevo@ejemplo.com";
    const SITE: &str = "https://nakomi.studio";
    const UUID: &str = "123e4567-e89b-12d3-a456-426614174000";

    let panel = format!("{SITE}/panel?seccion=ordenes&id={UUID}");
    let chat_panel = format!("{SITE}/panel/chat?session={UUID}");

    match template {
        "order_confirmation" => Ok(t::render_order_confirmation(
            NAME, ORDER, SERVICE, PLAN, PRICE,
        )),
        "new_order_admin" => Ok(t::render_new_order_admin(
            NAME,
            EMAIL,
            ORDER,
            SERVICE,
            PLAN,
            PRICE,
            PAYMENT_MODE,
            &panel,
        )),
        "payment_received_admin" => {
            Ok(t::render_payment_received_admin(NAME, ORDER, PRICE, &panel))
        }
        "order_completed_client" => Ok(t::render_order_completed_client(NAME, ORDER, SERVICE)),
        "order_completed_admin" => Ok(t::render_order_completed_admin(
            NAME, ORDER, SERVICE, &panel,
        )),
        "order_cancelled_client" => Ok(t::render_order_cancelled_client(NAME, ORDER, REASON)),
        "order_cancelled_admin" => Ok(t::render_order_cancelled_admin(NAME, ORDER, REASON, &panel)),
        "phase_delivered_client" => {
            Ok(t::render_phase_delivered_client(NAME, ORDER, PHASE, &panel))
        }
        "problem_reported_client" => {
            Ok(t::render_problem_reported_client(NAME, ORDER, PROBLEM_DESC))
        }
        "problem_reported_admin" => Ok(t::render_problem_reported_admin(
            NAME,
            ORDER,
            PROBLEM_DESC,
            &panel,
        )),
        "refund_requested_admin" => Ok(t::render_refund_requested_admin(
            NAME, ORDER, PRICE, REASON, &panel,
        )),
        "new_user_registered_admin" => Ok(t::render_new_user_registered_admin(NAME, EMAIL, &panel)),
        "escalation" => Ok(t::render_escalation(VISITOR, &chat_panel)),
        "chat_invoice_paid_client" => Ok(t::render_chat_invoice_paid_client(NAME, PRICE, UUID)),
        "chat_invoice_paid_admin" => Ok(t::render_chat_invoice_paid_admin(
            NAME,
            PRICE,
            UUID,
            &chat_panel,
        )),
        "vps_pending_approval" => Ok(t::render_vps_pending_approval(
            NAME,
            TIER,
            "mi-dominio.com",
            &panel,
        )),
        "vps_approved" => Ok(t::render_vps_approved(
            NAME, TIER, PUBLIC_IP, USERNAME, PASSWORD,
        )),
        "vps_rejected" => Ok(t::render_vps_rejected(NAME, TIER, REASON)),
        "profile_email_changed_new" => Ok(t::render_profile_email_changed_new(
            OLD_EMAIL, NEW_EMAIL, NAME,
        )),
        "profile_email_changed_old" => Ok(t::render_profile_email_changed_old(
            OLD_EMAIL, NEW_EMAIL, NAME,
        )),
        "profile_password_changed" => Ok(t::render_profile_password_changed(NAME)),
        _ => Err(format!("Plantilla desconocida: {template}")),
    }
}
