use std::net::SocketAddr;

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

    /* [297A-8] Limpiar sesiones expiradas al arrancar */
    let cleaned = glory_backend::services::SessionService::cleanup_expired(&pool)
        .await
        .unwrap_or(0);
    if cleaned > 0 {
        tracing::info!("Limpiadas {cleaned} sesiones expiradas al arrancar");
    }

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Servidor iniciando en {addr}");
    tracing::info!("Swagger UI disponible en http://{addr}/swagger-ui/");

    let app = handlers::create_router(pool, config);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    /* [297A-8] ConnectInfo necesario para extraer IP del cliente en rate limit */
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
