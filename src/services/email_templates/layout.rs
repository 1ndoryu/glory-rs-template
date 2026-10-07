use std::fmt::Write as _;

pub(crate) fn email_layout(
    accent: &str,
    title: &str,
    content_html: &str,
    preheader: &str,
) -> String {
    format!(
        "<!DOCTYPE html><html lang=\"es\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{title}</title></head>\
         <body style=\"margin:0;padding:0;background:#f4f4f5;font-family:Arial,Helvetica,sans-serif;\">\
         <div style=\"display:none;max-height:0;overflow:hidden;opacity:0;\">{preheader}</div>\
         <table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\">\
         <tr><td align=\"center\" style=\"padding:32px 16px;\">\
         <table role=\"presentation\" width=\"600\" cellpadding=\"0\" cellspacing=\"0\" \
         style=\"background:#ffffff;border-radius:12px;overflow:hidden;\
         border-top:6px solid {accent};\">\
         <tr><td style=\"padding:32px 36px;font-size:15px;line-height:1.6;color:#27272a;\">\
         <h1 style=\"font-size:22px;margin:0 0 16px;color:#18181b;\">{title}</h1>\
         {content_html}\
         <hr style=\"border:none;border-top:1px solid #e4e4e7;margin:24px 0 12px;\">\
         <p style=\"font-size:12px;color:#71717a;margin:0;\">\
         Este es un correo automático, por favor no respondas a esta dirección.</p>\
         </td></tr></table></td></tr></table></body></html>"
    )
}

pub(crate) fn section_title(text: &str) -> String {
    format!("<h2 style=\"font-size:18px;margin:0 0 12px;color:#18181b;\">{text}</h2>")
}

pub(crate) fn paragraph(text: &str) -> String {
    format!("<p style=\"font-size:15px;line-height:1.6;color:#27272a;margin:0 0 12px;\">{text}</p>")
}

pub(crate) fn paragraph_tight(text: &str) -> String {
    format!("<p style=\"font-size:14px;line-height:1.5;color:#27272a;margin:0 0 8px;\">{text}</p>")
}

pub(crate) fn summary_table(rows: &[(&str, &str)]) -> String {
    let mut body = String::new();
    for (label, value) in rows {
        let _ = write!(
            body,
            "<tr><td style=\"padding:8px 12px;font-size:14px;color:#52525b;border-bottom:1px solid #f4f4f5;\">\
             {label}</td>\
             <td style=\"padding:8px 12px;font-size:14px;color:#18181b;font-weight:bold;\
             border-bottom:1px solid #f4f4f5;text-align:right;\">{value}</td></tr>"
        );
    }
    format!(
        "<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" \
         style=\"background:#fafafa;border:1px solid #e4e4e7;border-radius:8px;\
         margin:0 0 16px;\">{body}</table>"
    )
}

pub(crate) fn cta_button(label: &str, url: &str) -> String {
    format!(
        "<p style=\"margin:16px 0 4px;\">\
         <a href=\"{url}\" style=\"display:inline-block;background:#c9a84c;color:#18181b;\
         font-size:15px;font-weight:bold;text-decoration:none;padding:12px 28px;\
         border-radius:8px;\">{label}</a></p>"
    )
}

pub(crate) fn table_row(label: &str, value: &str) -> String {
    format!(
        "<tr><td style=\"padding:8px 12px;font-size:14px;color:#52525b;\
         border-bottom:1px solid #f4f4f5;\">{label}</td>\
         <td style=\"padding:8px 12px;font-size:14px;color:#18181b;\
         border-bottom:1px solid #f4f4f5;text-align:right;\">{value}</td></tr>"
    )
}

pub(crate) fn table_row_highlight(label: &str, value: &str) -> String {
    format!(
        "<tr><td style=\"padding:8px 12px;font-size:14px;color:#52525b;\
         border-bottom:1px solid #f4f4f5;\">{label}</td>\
         <td style=\"padding:8px 12px;font-size:15px;color:#166534;font-weight:bold;\
         border-bottom:1px solid #f4f4f5;text-align:right;\">{value}</td></tr>"
    )
}

pub(crate) fn simple_table(rows_html: &str) -> String {
    format!(
        "<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" \
         style=\"background:#fafafa;border:1px solid #e4e4e7;border-radius:8px;\
         margin:0 0 16px;\">{rows_html}</table>"
    )
}

pub(crate) fn email_layout_no_footer(accent: &str, title: &str, content_html: &str) -> String {
    format!(
        "<!DOCTYPE html><html lang=\"es\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{title}</title></head>\
         <body style=\"margin:0;padding:0;background:#f4f4f5;font-family:Arial,Helvetica,sans-serif;\">\
         <table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\">\
         <tr><td align=\"center\" style=\"padding:32px 16px;\">\
         <table role=\"presentation\" width=\"600\" cellpadding=\"0\" cellspacing=\"0\" \
         style=\"background:#ffffff;border-radius:12px;overflow:hidden;\
         border-top:6px solid {accent};\">\
         <tr><td style=\"padding:32px 36px;font-size:15px;line-height:1.6;color:#27272a;\">\
         <h1 style=\"font-size:22px;margin:0 0 16px;color:#18181b;\">{title}</h1>\
         {content_html}\
         </td></tr></table></td></tr></table></body></html>"
    )
}
