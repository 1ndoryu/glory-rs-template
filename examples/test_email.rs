/* [311A-1] Prueba de envío SMTP — verifica que la configuración de email funciona.
 * Envía un correo de prueba a la dirección configurada en SMTP_BCC (o a andoryyu@gmail.com).
 * Ejecutar: cargo run --example test_email
 * Requiere SMTP_HOST/SMTP_USER/SMTP_PASSWORD en .env o variables de entorno. */

fn main() {
    dotenvy::dotenv().ok();

    let Some(config) = glory_backend::services::EmailConfig::from_env() else {
        eprintln!("❌ SMTP no configurado. Verifica SMTP_HOST, SMTP_USER y SMTP_PASSWORD en .env");
        std::process::exit(1);
    };

    let to = config
        .bcc_email
        .clone()
        .unwrap_or_else(|| "andoryyu@gmail.com".to_string());
    let subject = "🧪 Prueba SMTP — Nakomi Studio".to_string();
    let html = r#"<!DOCTYPE html>
<html lang="es">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"></head>
<body style="margin:0;padding:0;background:#f8f8f8;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;">
<div style="max-width:600px;margin:24px auto;background:#fff;border-radius:12px;overflow:hidden;box-shadow:0 2px 8px rgba(0,0,0,0.06);">
  <div style="background:#166534;padding:24px;text-align:center;">
    <h1 style="margin:0;color:#fff;font-size:20px;font-weight:600;">✅ Prueba SMTP</h1>
  </div>
  <div style="padding:32px 24px;">
    <p style="color:#333;font-size:15px;line-height:1.6;margin:0 0 16px;">
      Este es un correo de prueba para verificar que la configuración SMTP de Nakomi Studio funciona correctamente.
    </p>
    <p style="color:#555;font-size:14px;line-height:1.6;margin:0 0 16px;">
      Si estás viendo esto, el envío de correos desde la plataforma está operativo. 🎉
    </p>
    <table style="width:100%;border-collapse:collapse;font-size:14px;color:#333;background:#f8f8f8;border-radius:8px;padding:20px;">
      <tr><td style="padding:6px 0;color:#888;">Servidor</td><td style="padding:6px 0;font-weight:500;text-align:right;">{host}</td></tr>
      <tr><td style="padding:6px 0;color:#888;">Puerto</td><td style="padding:6px 0;font-weight:500;text-align:right;">{port}</td></tr>
      <tr><td style="padding:6px 0;color:#888;">Usuario</td><td style="padding:6px 0;font-weight:500;text-align:right;">{user}</td></tr>
      <tr><td style="padding:6px 0;color:#888;">From</td><td style="padding:6px 0;font-weight:500;text-align:right;">{from}</td></tr>
    </table>
  </div>
  <div style="padding:16px 24px;border-top:1px solid #eee;text-align:center;">
    <p style="margin:0;color:#999;font-size:12px;">Nakomi Studio · Correo de prueba generado el {date}</p>
  </div>
</div>
</body></html>"#;

    println!("📧 Enviando correo de prueba a: {to}");
    println!("   Host: {}", config.host);
    println!("   Port: {}", config.port);
    println!("   User: {}", config.user);
    println!("   From: {} <{}>", config.from_name, config.from_email);
    println!("   BCC:  {:?}", config.bcc_email);

    let rt = tokio::runtime::Runtime::new().expect("Error creando runtime tokio");
    let start = std::time::Instant::now();

    let html = html
        .replace("{host}", &config.host)
        .replace("{port}", &config.port.to_string())
        .replace("{user}", &config.user)
        .replace(
            "{from}",
            &format!("{} <{}>", config.from_name, config.from_email),
        )
        .replace(
            "{date}",
            &chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string(),
        );

    match rt.block_on(glory_backend::services::EmailService::send(
        &config, &to, &subject, &html,
    )) {
        Ok(()) => {
            let elapsed = start.elapsed();
            println!(
                "✅ Correo enviado exitosamente en {}.{:03}s",
                elapsed.as_secs(),
                elapsed.subsec_millis()
            );
            println!("   Revisa tu bandeja de entrada (y spam) en: {to}");
        }
        Err(e) => {
            eprintln!("❌ Error enviando correo: {e}");
            std::process::exit(1);
        }
    }
}
