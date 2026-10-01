use super::super::email::html_escape;
use super::layout::{
    cta_button, email_layout, paragraph_tight, simple_table, table_row, table_row_highlight,
};

/// Nuevo pedido — admin
pub fn render_new_order_admin(
    client_name: &str,
    order_number: i32,
    service_title: &str,
    total_display: &str,
    panel_link: &str,
) -> String {
    let rows = format!(
        "{r1}\n{r2}\n{r3}\n{r4}",
        r1 = table_row("Pedido", &format!("#{order_number}")),
        r2 = table_row("Cliente", &html_escape(client_name)),
        r3 = table_row("Servicio", &html_escape(service_title)),
        r4 = table_row_highlight("Total", &html_escape(total_display)),
    );
    let content = format!(
        "{msg}\n{table}\n{button}",
        msg = paragraph_tight("Se ha recibido un nuevo pedido en la plataforma."),
        table = simple_table(&rows),
        button = cta_button("Ver pedido", panel_link),
    );
    email_layout(
        "#c9a84c",
        "🛒 Nuevo Pedido",
        &content,
        "Notificación automática de pedido",
    )
}

/// Pago recibido — admin
pub fn render_payment_received_admin(
    client_name: &str,
    order_number: i32,
    amount_display: &str,
    panel_link: &str,
) -> String {
    let rows = format!(
        "{r1}\n{r2}\n{r3}",
        r1 = table_row("Pedido", &format!("#{order_number}")),
        r2 = table_row("Cliente", &html_escape(client_name)),
        r3 = table_row_highlight("Monto", &html_escape(amount_display)),
    );
    let content = format!(
        "{msg}\n{table}\n{button}",
        msg = paragraph_tight("Se ha confirmado un pago para un pedido."),
        table = simple_table(&rows),
        button = cta_button("Ver pedido", panel_link),
    );
    email_layout(
        "#166534",
        "💰 Pago Recibido",
        &content,
        "Notificación automática de pago",
    )
}

/// Pedido completado — admin
pub fn render_order_completed_admin(
    client_name: &str,
    order_number: i32,
    service_title: &str,
    panel_link: &str,
) -> String {
    let rows = format!(
        "{r1}\n{r2}\n{r3}",
        r1 = table_row("Pedido", &format!("#{order_number}")),
        r2 = table_row("Cliente", &html_escape(client_name)),
        r3 = table_row("Servicio", &html_escape(service_title)),
    );
    let content = format!(
        "{msg}\n{table}\n{button}",
        msg = paragraph_tight("Un pedido ha sido marcado como completado."),
        table = simple_table(&rows),
        button = cta_button("Ver pedido", panel_link),
    );
    email_layout(
        "#166534",
        "✅ Pedido Completado",
        &content,
        "Notificación automática de pedido",
    )
}

/// Pedido cancelado — admin
pub fn render_order_cancelled_admin(
    client_name: &str,
    order_number: i32,
    reason: &str,
    panel_link: &str,
) -> String {
    let rows = format!(
        "{r1}\n{r2}",
        r1 = table_row("Pedido", &format!("#{order_number}")),
        r2 = table_row("Cliente", &html_escape(client_name)),
    );
    let reason_block = if reason.is_empty() {
        String::new()
    } else {
        paragraph_tight(&format!("<strong>Motivo:</strong> {}", html_escape(reason)))
    };
    let content = format!(
        "{msg}\n{table}\n{reason}\n{button}",
        msg = paragraph_tight("Un pedido ha sido cancelado."),
        table = simple_table(&rows),
        reason = reason_block,
        button = cta_button("Ver pedido", panel_link),
    );
    email_layout(
        "#991b1b",
        "❌ Pedido Cancelado",
        &content,
        "Notificación automática de pedido",
    )
}

/// Problema reportado — admin
pub fn render_problem_reported_admin(
    client_name: &str,
    order_number: i32,
    problem_description: &str,
    panel_link: &str,
) -> String {
    let rows = format!(
        "{r1}\n{r2}",
        r1 = table_row("Pedido", &format!("#{order_number}")),
        r2 = table_row("Cliente", &html_escape(client_name)),
    );
    let content = format!(
        "{msg}\n{table}\n{desc}\n{button}",
        msg = paragraph_tight("Se ha reportado un nuevo problema en un pedido."),
        table = simple_table(&rows),
        desc = paragraph_tight(&format!(
            "<strong>Descripción:</strong> {}",
            html_escape(problem_description)
        )),
        button = cta_button("Ver problema", panel_link),
    );
    email_layout(
        "#92400e",
        "⚠ Problema Reportado",
        &content,
        "Notificación automática de pedido",
    )
}

/// Solicitud de reembolso — admin
pub fn render_refund_requested_admin(
    client_name: &str,
    order_number: i32,
    amount_display: &str,
    reason: &str,
    panel_link: &str,
) -> String {
    let rows = format!(
        "{r1}\n{r2}\n{r3}",
        r1 = table_row("Pedido", &format!("#{order_number}")),
        r2 = table_row("Cliente", &html_escape(client_name)),
        r3 = table_row_highlight("Monto", &html_escape(amount_display)),
    );
    let content = format!(
        "{msg}\n{table}\n{reason}\n{button}",
        msg = paragraph_tight("Un cliente ha solicitado un reembolso."),
        table = simple_table(&rows),
        reason = paragraph_tight(&format!("<strong>Motivo:</strong> {}", html_escape(reason))),
        button = cta_button("Ver solicitud", panel_link),
    );
    email_layout(
        "#92400e",
        "💸 Solicitud de Reembolso",
        &content,
        "Notificación automática de pago",
    )
}
