use super::super::email::html_escape;
use super::layout::{cta_button, paragraph, section_title, summary_table};

/// Confirmación de pedido — cliente
pub fn render_order_confirmation(
    client_name: &str,
    order_number: i32,
    service_title: &str,
    total_display: &str,
) -> String {
    let content = format!(
        "{title}\n{msg}\n{table}\n{button}",
        title = section_title(&format!("¡Hola, {}!", html_escape(client_name))),
        msg = paragraph(&format!(
            "Hemos recibido tu pedido <strong>#{order_number}</strong>. Te avisaremos cuando empecemos a trabajar en él."
        )),
        table = summary_table(&[
            ("Servicio", &html_escape(service_title)),
            ("Total", &html_escape(total_display)),
        ]),
        button = cta_button("Ver mi pedido", "https://nakomi.studio/panel"),
    );
    super::layout::email_layout(
        "#c9a84c",
        "✅ Pedido Recibido",
        &content,
        "Notificación automática de cuenta",
    )
}

/// Pedido completado — cliente
pub fn render_order_completed_client(
    client_name: &str,
    order_number: i32,
    service_title: &str,
) -> String {
    let content = format!(
        "{title}\n{msg}\n{table}\n{button}",
        title = section_title(&format!("¡Hola, {}!", html_escape(client_name))),
        msg = paragraph(&format!(
            "Tu pedido <strong>#{order_number}</strong> ha sido completado exitosamente."
        )),
        table = summary_table(&[("Servicio", &html_escape(service_title))]),
        button = cta_button("Ver pedido", "https://nakomi.studio/panel"),
    );
    super::layout::email_layout(
        "#166534",
        "✅ Pedido Completado",
        &content,
        "Notificación automática de pedido",
    )
}

/// Pedido cancelado — cliente
pub fn render_order_cancelled_client(client_name: &str, order_number: i32, reason: &str) -> String {
    let content = format!(
        "{title}\n{msg}\n{table}\n{reason_block}",
        title = section_title(&format!("Hola, {}", html_escape(client_name))),
        msg = paragraph(&format!(
            "Tu pedido <strong>#{order_number}</strong> ha sido cancelado."
        )),
        table = summary_table(&[("Pedido", &format!("#{order_number}"))]),
        reason_block = if reason.is_empty() {
            String::new()
        } else {
            paragraph(&format!("<strong>Motivo:</strong> {}", html_escape(reason)))
        },
    );
    super::layout::email_layout(
        "#991b1b",
        "❌ Pedido Cancelado",
        &content,
        "Notificación automática de pedido",
    )
}

/// Fase entregada — cliente
pub fn render_phase_delivered_client(
    client_name: &str,
    order_number: i32,
    phase_name: &str,
    panel_link: &str,
) -> String {
    let content = format!(
        "{title}\n{msg}\n{table}\n{button}",
        title = section_title(&format!("¡Hola, {}!", html_escape(client_name))),
        msg = paragraph(&format!(
            "Se ha entregado una nueva fase de tu pedido <strong>#{order_number}</strong>."
        )),
        table = summary_table(&[("Fase", &html_escape(phase_name))]),
        button = cta_button("Revisar entrega", panel_link),
    );
    super::layout::email_layout(
        "#1a1a1a",
        "📦 Fase Entregada",
        &content,
        "Notificación automática de entrega",
    )
}

/// Problema reportado — cliente
pub fn render_problem_reported_client(
    client_name: &str,
    order_number: i32,
    problem_description: &str,
) -> String {
    let content = format!(
        "{title}\n{msg}\n{table}\n{desc}",
        title = section_title(&format!("Hola, {}", html_escape(client_name))),
        msg = paragraph(&format!(
            "Se ha reportado un problema con tu pedido <strong>#{order_number}</strong>."
        )),
        table = summary_table(&[("Pedido", &format!("#{order_number}"))]),
        desc = paragraph(&format!(
            "<strong>Descripción:</strong> {}",
            html_escape(problem_description)
        )),
    );
    super::layout::email_layout(
        "#92400e",
        "⚠ Problema Reportado",
        &content,
        "Notificación automática de pedido",
    )
}
