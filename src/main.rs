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
