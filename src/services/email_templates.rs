/* [20CA-11] Plantillas HTML centralizadas para emails.
 * Elimina duplicación entre email.rs (envío) y email_preview.rs (previews).
 * Cada función render_* genera el HTML completo usando helpers de layout.
 * email_preview.rs reutiliza estas funciones con datos SAMPLE. */

use super::email::html_escape;

/* ============================================================
LAYOUT HELPERS — Componentes HTML reutilizables
============================================================ */

/// Layout completo del email: DOCTYPE + head + body wrapper + header + content + footer
fn email_layout(header_color: &str, title: &str, content: &str, footer_text: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"></head>
<body style="margin:0;padding:0;background:#f8f8f8;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;">
<div style="max-width:600px;margin:24px auto;background:#fff;border-radius:12px;overflow:hidden;box-shadow:0 2px 8px rgba(0,0,0,0.06);">
  <div style="background:{header_color};padding:24px;text-align:center;">
    <h1 style="margin:0;color:#fff;font-size:22px;font-weight:600;">{title}</h1>
  </div>
  <div style="padding:32px 24px;">
{content}
  </div>
  <div style="padding:16px 24px;border-top:1px solid #eee;text-align:center;">
    <p style="margin:0;color:#999;font-size:12px;">Nakomi Studio · {footer_text}</p>
  </div>
</div>
</body></html>"#
    )
}

/// Layout sin footer (para emails transaccionales cortos)
fn email_layout_no_footer(header_color: &str, title: &str, content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"></head>
<body style="margin:0;padding:0;background:#f8f8f8;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;">
<div style="max-width:600px;margin:24px auto;background:#fff;border-radius:12px;overflow:hidden;box-shadow:0 2px 8px rgba(0,0,0,0.06);">
  <div style="background:{header_color};padding:24px;text-align:center;">
    <h1 style="margin:0;color:#fff;font-size:22px;font-weight:600;">{title}</h1>
  </div>
  <div style="padding:32px 24px;">
{content}
  </div>
</div>
</body></html>"#
    )
}

/// Título de sección (h2)
fn section_title(text: &str) -> String {
    format!(
        r#"    <h2 style="margin:0 0 8px;color:#1a1a1a;font-size:20px;">{text}</h2>"#
    )
}

/// Párrafo estándar
fn paragraph(text: &str) -> String {
    format!(
        r#"    <p style="color:#555;font-size:15px;line-height:1.6;margin:0 0 24px;">{text}</p>"#
    )
}

/// Párrafo con margen inferior reducido
fn paragraph_tight(text: &str) -> String {
    format!(
        r#"    <p style="color:#333;font-size:15px;line-height:1.6;margin:0 0 16px;">{text}</p>"#
    )
}

/// Botón CTA
fn cta_button(text: &str, url: &str) -> String {
    format!(
        r#"    <a href="{url}" style="display:inline-block;background:#c9a84c;color:#fff;text-decoration:none;padding:12px 28px;border-radius:8px;font-weight:600;font-size:14px;">{text}</a>"#
    )
}

/// Tabla de resumen (label → value)
fn summary_table(rows: &[(&str, &str)]) -> String {
    let mut table = String::from(
        r#"    <div style="background:#f8f8f8;border-radius:8px;padding:20px;margin-bottom:24px;">
      <table style="width:100%;border-collapse:collapse;font-size:14px;color:#333;">"#
    );
    for (label, value) in rows {
        table.push_str(&format!(
            r#"
        <tr><td style="padding:6px 0;color:#888;">{label}</td><td style="padding:6px 0;font-weight:500;text-align:right;">{value}</td></tr>"#
        ));
    }
    table.push_str(
        r#"
      </table>
    </div>"#
    );
    table
}

/// Fila de tabla simple (sin background card)
fn table_row(label: &str, value: &str) -> String {
    format!(
        r#"      <tr><td style="padding:6px 0;color:#888;">{label}</td><td style="padding:6px 0;font-weight:500;text-align:right;">{value}</td></tr>"#
    )
}

/// Fila de tabla con valor destacado (color dorado)
fn table_row_highlight(label: &str, value: &str) -> String {
    format!(
        r#"      <tr><td style="padding:6px 0;color:#888;">{label}</td><td style="padding:6px 0;font-weight:600;text-align:right;color:#c9a84c;">{value}</td></tr>"#
    )
}

/// Tabla simple sin card background
fn simple_table(rows_html: &str) -> String {
    format!(
        r#"    <table style="width:100%;border-collapse:collapse;font-size:14px;color:#333;">
{rows_html}
    </table>"#
    )
}

/* ============================================================
TEMPLATES — Cada función genera el HTML completo de un email
============================================================ */

/// Confirmación de pedido al cliente
pub fn render_order_confirmation(
    client_name: &str,
    order_number: i32,
    service_title: &str,
    plan_name: &str,
    price_display: &str,
) -> String {
    let content = format!(
        "{title}\n{greeting}\n{table}\n{desc}\n{button}",
        title = section_title(&format!("¡Hola, {}!", html_escape(client_name))),
        greeting = paragraph(&format!(
            "Tu pedido <strong>#{order_number}</strong> ha sido recibido exitosamente. \
             Nuestro equipo lo revisará y será atendido dentro de las próximas <strong>48 horas</strong>."
        )),
        table = summary_table(&[
            ("Servicio", &html_escape(service_title)),
            ("Plan", &html_escape(plan_name)),
            ("Precio", &html_escape(price_display)),
        ]),
        desc = paragraph("Puedes seguir el progreso de tu pedido en tiempo real desde tu panel."),
        button = cta_button("Ver mi pedido", "https://nakomi.studio/panel"),
    );
    email_layout("#1a1a1a", "Nakomi Studio", &content, "Este email fue enviado porque realizaste un pedido.")
}

/// Nueva orden notificada a admins
pub fn render_new_order_admin(
    client_name: &str,
    client_email: &str,
    order_number: i32,
    service_title: &str,
    plan_name: &str,
    price_display: &str,
    payment_mode: &str,
    panel_link: &str,
) -> String {
    let rows = format!(
        "{r1}\n{r2}\n{r3}\n{r4}\n{r5}\n{r6}",
        r1 = table_row("Pedido", &format!("#{order_number}")),
        r2 = table_row("Cliente", &format!("{} ({})", html_escape(client_name), html_escape(client_email))),
        r3 = table_row("Servicio", &html_escape(service_title)),
        r4 = table_row("Plan", &html_escape(plan_name)),
        r5 = table_row("Modalidad", payment_mode),
        r6 = table_row_highlight("Precio", &html_escape(price_display)),
    );
    let content = format!(
        "{msg}\n{table}\n{button}",
        msg = paragraph_tight("Se ha creado un nuevo pedido en Nakomi Studio."),
        table = simple_table(&rows),
        button = cta_button("Revisar pedido", panel_link),
    );
    email_layout("#1a1a1a", "🆕 Nueva Orden", &content, "Notificación automática de nuevo pedido")
}

/// Escalación de chat a admins
pub fn render_escalation(visitor_name: &str, panel_link: &str) -> String {
    let content = format!(
        "{p1}\n{p2}\n{button}",
        p1 = paragraph_tight(&format!(
            "La IA detectó que <strong>{}</strong> necesita asistencia humana.",
            html_escape(visitor_name)
        )),
        p2 = paragraph("Por favor, revisa la sesión de chat lo antes posible para atender al visitante."),
        button = cta_button("Abrir sesión de chat", panel_link),
    );
    email_layout("#b91c1c", "⚠ Escalación de Chat", &content, "Notificación automática de escalación")
}

/// Pago recibido notificado a admins
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
        msg = paragraph_tight("Se ha recibido un pago confirmado."),
        table = simple_table(&rows),
        button = cta_button("Ver pedido", panel_link),
    );
    email_layout("#166534", "💰 Pago Recibido", &content, "Notificación automática de pago")
}

/// Factura de chat pagada — cliente
pub fn render_chat_invoice_paid_client(
    client_name: &str,
    amount_display: &str,
    session_id: &str,
) -> String {
    let content = format!(
        "{title}\n{msg}\n{table}\n{button}",
        title = section_title(&format!("¡Hola, {}!", html_escape(client_name))),
        msg = paragraph("Tu pago por la sesión de chat ha sido procesado exitosamente."),
        table = summary_table(&[
            ("Monto", &html_escape(amount_display)),
            ("Sesión", session_id),
        ]),
        button = cta_button("Ver chat", "https://nakomi.studio/panel"),
    );
    email_layout("#166534", "✅ Pago Confirmado", &content, "Notificación automática de pago")
}

/// Factura de chat pagada — admin
pub fn render_chat_invoice_paid_admin(
    client_name: &str,
    amount_display: &str,
    session_id: &str,
    panel_link: &str,
) -> String {
    let rows = format!(
        "{r1}\n{r2}\n{r3}",
        r1 = table_row("Cliente", &html_escape(client_name)),
        r2 = table_row_highlight("Monto", &html_escape(amount_display)),
        r3 = table_row("Sesión", session_id),
    );
    let content = format!(
        "{msg}\n{table}\n{button}",
        msg = paragraph_tight("Se ha recibido el pago de una factura de chat."),
        table = simple_table(&rows),
        button = cta_button("Ver sesión", panel_link),
    );
    email_layout("#166534", "💰 Factura de Chat Pagada", &content, "Notificación automática de pago")
}

/// VPS pendiente de aprobación — admin
pub fn render_vps_pending_approval(
    client_name: &str,
    plan_name: &str,
    domain: &str,
    panel_link: &str,
) -> String {
    let rows = format!(
        "{r1}\n{r2}\n{r3}",
        r1 = table_row("Cliente", &html_escape(client_name)),
        r2 = table_row("Plan", &html_escape(plan_name)),
        r3 = table_row("Dominio", &html_escape(domain)),
    );
    let content = format!(
        "{msg}\n{table}\n{button}",
        msg = paragraph_tight("Hay una nueva solicitud de VPS pendiente de aprobación."),
        table = simple_table(&rows),
        button = cta_button("Revisar solicitud", panel_link),
    );
    email_layout("#92400e", "🖥 Solicitud de VPS", &content, "Notificación automática de VPS")
}

/// VPS aprobado — cliente
pub fn render_vps_approved(
    client_name: &str,
    plan_name: &str,
    public_ip: &str,
    username: &str,
    password: &str,
) -> String {
    let _rows = format!(
        "{r1}\n{r2}\n{r3}\n{r4}",
        r1 = table_row("IP pública", &html_escape(public_ip)),
        r2 = table_row("Plan", &html_escape(plan_name)),
        r3 = table_row("Usuario", &html_escape(username)),
        r4 = table_row("Contraseña inicial", &html_escape(password)),
    );
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
    email_layout("#166534", "✅ VPS Aprobado", &content, "Notificación automática de VPS")
}

/// VPS rechazado — cliente
pub fn render_vps_rejected(
    client_name: &str,
    plan_name: &str,
    reason: &str,
) -> String {
    let content = format!(
        "{title}\n{msg}\n{table}\n{reason_block}",
        title = section_title(&format!("Hola, {}", html_escape(client_name))),
        msg = paragraph("Lamentablemente, tu solicitud de VPS no pudo ser aprobada."),
        table = summary_table(&[("Plan solicitado", &html_escape(plan_name))]),
        reason_block = paragraph(&format!(
            "<strong>Motivo:</strong> {}",
            html_escape(reason)
        )),
    );
    email_layout("#991b1b", "❌ Solicitud de VPS Rechazada", &content, "Notificación automática de VPS")
}

/// Email cambiado — notificación a nueva dirección
pub fn render_profile_email_changed_new(old_email: &str, new_email: &str, recipient_name: &str) -> String {
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
pub fn render_profile_email_changed_old(old_email: &str, new_email: &str, recipient_name: &str) -> String {
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
    email_layout("#166534", "✅ Pedido Completado", &content, "Notificación automática de pedido")
}

/// Pedido cancelado — cliente
pub fn render_order_cancelled_client(
    client_name: &str,
    order_number: i32,
    reason: &str,
) -> String {
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
    email_layout("#991b1b", "❌ Pedido Cancelado", &content, "Notificación automática de pedido")
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
    email_layout("#1a1a1a", "📦 Fase Entregada", &content, "Notificación automática de entrega")
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
    email_layout("#92400e", "⚠ Problema Reportado", &content, "Notificación automática de pedido")
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
    email_layout("#166534", "✅ Pedido Completado", &content, "Notificación automática de pedido")
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
    email_layout("#991b1b", "❌ Pedido Cancelado", &content, "Notificación automática de pedido")
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
    email_layout("#92400e", "⚠ Problema Reportado", &content, "Notificación automática de pedido")
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
    email_layout("#92400e", "💸 Solicitud de Reembolso", &content, "Notificación automática de pago")
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
    email_layout("#1a1a1a", "👤 Nuevo Usuario", &content, "Notificación automática de registro")
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
    email_layout("#c9a84c", "💬 Nuevo Mensaje de Chat", &content, "Notificación automática de chat")
}

/* [237A-7j] Email de continuación de conversación de chat.
 * Se envía cuando el visitante con email conocido se desconecta por más de 2 minutos.
 * Contiene un enlace firmado de un solo uso para reanudar la conversación. */
pub fn render_chat_continuation(
    visitor_name: &str,
    continuation_url: &str,
) -> String {
    let content = format!(
        "{title}\n{p1}\n{p2}\n{button}\n{p3}",
        title = section_title(&format!("Hola, {}", html_escape(visitor_name))),
        p1 = paragraph("Notamos que te desconectaste de nuestra conversación. \
             Puedes continuar exactamente donde lo dejaste usando el botón de abajo."),
        p2 = paragraph("Este enlace es personal, de un solo uso y expira en 7 días."),
        button = cta_button("Continuar conversación", continuation_url),
        p3 = paragraph("Si no solicitaste este enlace, puedes ignorar este correo con seguridad."),
    );
    email_layout("#c9a84c", "💬 Continúa tu conversación", &content, "Enlace de continuación de chat")
}
