use super::super::email::html_escape;
use super::layout::{
    cta_button, email_layout, email_layout_no_footer, paragraph, paragraph_tight, section_title,
    simple_table, summary_table, table_row, table_row_highlight,
};

/// Escalación a humano — admin
pub fn render_escalation(visitor_name: &str, panel_link: &str) -> String {
    let content = format!(
        "{p1}\n{button}",
        p1 = paragraph_tight(&format!(
            "El visitante <strong>{}</strong> pidió hablar con un humano.",
            html_escape(visitor_name)
        )),
        button = cta_button("Abrir panel", panel_link),
    );
    email_layout(
        "#991b1b",
        "🚨 Escalación a humano",
        &content,
        "El asistente automático solicitó ayuda humana",
    )
}

/// Factura de chat pagada — cliente
pub fn render_chat_invoice_paid_client(
    client_name: &str,
    invoice_number: &str,
    amount_display: &str,
) -> String {
    let content = format!(
        "{title}\n{msg}\n{table}",
        title = section_title(&format!("¡Hola, {}!", html_escape(client_name))),
        msg = paragraph(&format!(
            "Hemos recibido el pago de tu factura <strong>{}</strong>. ¡Gracias por tu confianza!",
            html_escape(invoice_number)
        )),
        table = summary_table(&[("Monto pagado", &html_escape(amount_display))]),
    );
    email_layout(
        "#166534",
        "✅ Pago Recibido",
        &content,
        "Notificación automática de pago",
    )
}

/// Factura de chat pagada — admin
pub fn render_chat_invoice_paid_admin(
    client_name: &str,
    invoice_number: &str,
    amount_display: &str,
    panel_link: &str,
) -> String {
    let rows = format!(
        "{r1}\n{r2}\n{r3}",
        r1 = table_row("Factura", &html_escape(invoice_number)),
        r2 = table_row("Cliente", &html_escape(client_name)),
        r3 = table_row_highlight("Monto", &html_escape(amount_display)),
    );
    let content = format!(
        "{msg}\n{table}\n{button}",
        msg = paragraph_tight("Se ha pagado una factura generada desde el chat."),
        table = simple_table(&rows),
        button = cta_button("Ver factura", panel_link),
    );
    email_layout(
        "#166534",
        "💰 Factura Pagada",
        &content,
        "Notificación automática de pago",
    )
}

/// VPS pendiente — admin
pub fn render_vps_pending_approval(
    client_name: &str,
    plan_name: &str,
    panel_link: &str,
) -> String {
    let content = format!(
        "{msg}\n{table}\n{button}",
        msg = paragraph_tight(&format!(
            "El cliente <strong>{}</strong> solicitó un VPS que requiere aprobación.",
            html_escape(client_name)
        )),
        table = summary_table(&[("Plan solicitado", &html_escape(plan_name))]),
        button = cta_button("Revisar solicitud", panel_link),
    );
    email_layout(
        "#92400e",
        "🖥 Nueva Solicitud de VPS",
        &content,
        "Notificación automática de VPS",
    )
}

/// VPS aprobado — cliente (incluye credenciales iniciales)
pub fn render_vps_approved(
    client_name: &str,
    plan_name: &str,
    public_ip: &str,
    username: &str,
    password: &str,
) -> String {
    let content = format!(
        "{title}\n{msg}\n{table}\n{desc}",
        title = section_title(&format!("¡Hola, {}!", html_escape(client_name))),
        msg = paragraph("Tu solicitud de VPS ha sido aprobada. Ya puedes acceder desde tu panel."),
        table = summary_table(&[
            ("Plan", &html_escape(plan_name)),
            ("IP pública", &html_escape(public_ip)),
            ("Usuario", &html_escape(username)),
            ("Contraseña inicial", &html_escape(password)),
        ]),
        desc = paragraph("Cambia la contraseña en tu primera conexión y guarda estas credenciales en un gestor seguro."),
    );
    email_layout(
        "#166534",
        "✅ VPS Aprobado",
        &content,
        "Notificación automática de VPS",
    )
}

/// VPS rechazado — cliente
pub fn render_vps_rejected(client_name: &str, plan_name: &str, reason: &str) -> String {
    let content = format!(
        "{title}\n{msg}\n{table}\n{reason_block}",
        title = section_title(&format!("Hola, {}", html_escape(client_name))),
        msg = paragraph("Lamentablemente, tu solicitud de VPS no pudo ser aprobada."),
        table = summary_table(&[("Plan solicitado", &html_escape(plan_name))]),
        reason_block = paragraph(&format!("<strong>Motivo:</strong> {}", html_escape(reason))),
    );
    email_layout(
        "#991b1b",
        "❌ Solicitud de VPS Rechazada",
        &content,
        "Notificación automática de VPS",
    )
}

/// Email cambiado — notificación a nueva dirección
pub fn render_profile_email_changed_new(
    old_email: &str,
    new_email: &str,
    recipient_name: &str,
) -> String {
    let content = format!(
        "{greeting}\n{msg}\n{warn}",
        greeting = paragraph_tight(&format!("Hola, <strong>{}</strong>.", html_escape(recipient_name))),
        msg = paragraph(&format!(
            "Tu cuenta cambió el correo de acceso de <strong>{}</strong> a <strong>{}</strong>.",
            html_escape(old_email),
            html_escape(new_email)
        )),
        warn = paragraph("Desde ahora puedes iniciar sesión con este correo. Este cambio no requirió verificación por email."),
    );
    email_layout_no_footer("#1a1a1a", "Correo actualizado", &content)
}

/// Email cambiado — notificación a dirección anterior
pub fn render_profile_email_changed_old(
    old_email: &str,
    new_email: &str,
    recipient_name: &str,
) -> String {
    let content = format!(
        "{greeting}\n{msg}\n{warn}",
        greeting = paragraph_tight(&format!("Hola, <strong>{}</strong>.", html_escape(recipient_name))),
        msg = paragraph(&format!(
            "Tu cuenta dejó de usar <strong>{}</strong> y ahora usa <strong>{}</strong> para iniciar sesión.",
            html_escape(old_email),
            html_escape(new_email)
        )),
        warn = paragraph("Si no reconoces este cambio, responde a este correo o contacta a soporte de inmediato."),
    );
    email_layout_no_footer("#991b1b", "Cambio de correo detectado", &content)
}

/// Contraseña cambiada
pub fn render_profile_password_changed(recipient_name: &str) -> String {
    let content = format!(
        "{greeting}\n{msg}\n{warn}",
        greeting = paragraph_tight(&format!("Hola, <strong>{}</strong>.", html_escape(recipient_name))),
        msg = paragraph("La contraseña de tu cuenta fue cambiada correctamente desde la configuración de perfil."),
        warn = paragraph("Si no fuiste tú, cambia tu contraseña de nuevo de inmediato y contacta al equipo de soporte."),
    );
    email_layout_no_footer("#1a1a1a", "Contraseña actualizada", &content)
}

/// Nuevo usuario registrado — admin
pub fn render_new_user_registered_admin(
    user_name: &str,
    user_email: &str,
    panel_link: &str,
) -> String {
    let rows = format!(
        "{r1}\n{r2}",
        r1 = table_row("Nombre", &html_escape(user_name)),
        r2 = table_row("Email", &html_escape(user_email)),
    );
    let content = format!(
        "{msg}\n{table}\n{button}",
        msg = paragraph_tight("Se ha registrado un nuevo usuario en la plataforma."),
        table = simple_table(&rows),
        button = cta_button("Ver usuario", panel_link),
    );
    email_layout(
        "#1a1a1a",
        "👤 Nuevo Usuario",
        &content,
        "Notificación automática de registro",
    )
}

/* [237A-7d] Nuevo mensaje de cliente en chat — notificación a admin.
 * Se envía via outbox worker cuando un cliente/visitante envía un mensaje. */
pub fn render_chat_client_message_admin(
    sender_label: &str,
    preview: &str,
    panel_link: &str,
) -> String {
    let content = format!(
        "{p1}\n{table}\n{button}",
        p1 = paragraph_tight(&format!(
            "<strong>{}</strong> ha enviado un nuevo mensaje en el chat.",
            html_escape(sender_label)
        )),
        table = summary_table(&[
            ("Remitente", &html_escape(sender_label)),
            ("Mensaje", &html_escape(preview)),
        ]),
        button = cta_button("Abrir chat", panel_link),
    );
    email_layout(
        "#c9a84c",
        "💬 Nuevo Mensaje de Chat",
        &content,
        "Notificación automática de chat",
    )
}

/* [237A-7j] Email de continuación de conversación de chat.
 * Se envía cuando el visitante con email conocido se desconecta por más de 2 minutos.
 * Contiene un enlace firmado de un solo uso para reanudar la conversación. */
pub fn render_chat_continuation(visitor_name: &str, continuation_url: &str) -> String {
    let content = format!(
        "{title}\n{p1}\n{p2}\n{button}\n{p3}",
        title = section_title(&format!("Hola, {}", html_escape(visitor_name))),
        p1 = paragraph(
            "Notamos que te desconectaste de nuestra conversación. \
             Puedes continuar exactamente donde lo dejaste usando el botón de abajo."
        ),
        p2 = paragraph("Este enlace es personal, de un solo uso y expira en 7 días."),
        button = cta_button("Continuar conversación", continuation_url),
        p3 = paragraph("Si no solicitaste este enlace, puedes ignorar este correo con seguridad."),
    );
    email_layout(
        "#c9a84c",
        "💬 Continúa tu conversación",
        &content,
        "Enlace de continuación de chat",
    )
}
