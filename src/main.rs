/* [01AA-4-F3r] Entrypoint delgado (~20 efectivas): todo el bootstrap vive en
 * `glory_backend::bootstrap` (fixtures = pool+fixtures+seed, background =
 * tareas de fondo, server = listener+hyper+watchdog). Antes: main.rs 766L
 * (god-object 566 + limite-lineas 566 + main() 149L funcion-larga-rs).
 * Sin cambio de comportamiento. */

use glory_backend::bootstrap;
use glory_backend::config::AppConfig;
use glory_backend::handlers;

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new(
                    "glory_backend=debug,glory_rs=debug,tower_http=debug,hyper=debug,h2=debug",
                )
            }),
        )
        .init();

    let config = AppConfig::from_env()?;

    let pool = bootstrap::fixtures::connect_pool(&config.database_url).await?;

    /* [250A-1] Fixtures + background tasks en helpers para mantener
     * main() por debajo de 100 líneas (regla funcion-larga-rs). */
    bootstrap::fixtures::setup_and_run_fixtures(&pool).await?;
    bootstrap::background::spawn_background_services(&pool);

    let server_port = config.port;
    let addr = format!("{}:{}", config.host, server_port);
    tracing::info!("Servidor iniciando en {addr}");
    tracing::info!("Swagger UI disponible en http://{addr}/swagger-ui/");

    let app = handlers::create_app(pool, config);

    bootstrap::server::run(app, &addr, server_port).await
}
