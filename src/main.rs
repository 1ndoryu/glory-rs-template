use glory_backend::config::AppConfig;
use glory_backend::handlers;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("glory_backend=debug,tower_http=debug")
            }),
        )
        .init();

    let config = AppConfig::from_env()?;

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    /* [03AA-3 M4] Purga de caché mp al arrancar (siempre, fail-open) + pg_cron
     * diario solo con `DB_24H=true` (07:00 UTC = 03:00 Caracas). Sin pg_cron
     * en el servidor se avisa y se sigue: la purga al arrancar ya cubre. */
    match glory_backend::services::marketplace::purgar_cache(&pool).await {
        Ok(n) => tracing::info!("mp caché: {n} vencidas purgadas al arrancar"),
        Err(e) => tracing::warn!("mp caché: purga inicial falló ({e}), sigue sin purgar"),
    }
    if std::env::var("DB_24H").as_deref() == Ok("true") {
        match glory_backend::services::marketplace::programar_purga_diaria(&pool).await {
            Ok(()) => tracing::info!("mp caché: purga diaria pg_cron 07:00 UTC"),
            Err(e) => {
                tracing::warn!("mp caché: pg_cron no programado ({e}); solo purga al arrancar")
            }
        }
    }

    tokio::fs::create_dir_all(&config.upload_dir).await?;
    tracing::info!("Uploads en {}", config.upload_dir);

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Servidor iniciando en {addr}");
    tracing::info!("Swagger UI disponible en http://{addr}/swagger-ui/");

    let app = handlers::create_router(pool.clone(), config);
    /* [169A-4] Worker de avisos WhatsApp (outbox kind='whatsapp').
     * Sin `GLORY_ALERT_GATEWAY_URL` avisa en logs y no itera: los avisos
     * quedan 'pending' visibles en el panel (nunca silencio). */
    tokio::spawn(glory_backend::services::vigilar_alertas_whatsapp(
        pool.clone(),
        std::env::var("GLORY_ALERT_GATEWAY_URL").ok(),
    ));
    /* [279A-2] Tope diario de tokens LLM: solo alerta (nunca apaga). */
    tokio::spawn(glory_backend::services::vigilar_tope_uso(pool));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
