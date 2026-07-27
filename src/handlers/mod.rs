#![allow(clippy::needless_for_each)] // Generado por utoipa OpenApi derive

mod admin;
mod api_keys;
mod auth;
mod bdp_article_map;
mod bdp_backup;
mod bdp_customer_sync;
mod bdp_purchase_note;

pub use bdp_purchase_note::{
    conciliar_purchase_note, listar_purchase_notes, marcar_borrador_purchase_note,
    sincronizar_purchase_notes,
};
mod campanas;
mod canales_reserva;
mod chatbot;
mod clientes;
mod configuracion;
mod dashboard;
mod errores;
mod etiquetas;
mod gastos;
mod health;
mod inactividad;
mod notificaciones;
mod plano_sala;
mod plantillas_whatsapp;
mod recordatorios;
mod resenas;
mod reservas;
mod trabajadores;
mod ventas;

use axum::Router;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::errors::ErrorResponse;

use crate::AppState;

/* [S16-H2] Límite máximo del body de peticiones: 2 MB.
 * Previene ataques de denegación de servicio mediante payloads excesivos. */
const MAX_BODY_SIZE: usize = 2 * 1024 * 1024;

/// Define el esquema de seguridad Bearer para Swagger UI
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        /* components existe porque el derive ya registra schemas */
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
                    ),
                ),
            );
            /* [283A-2] Esquema de seguridad para API keys de chatbot */
            components.add_security_scheme(
                "api_key_auth",
                utoipa::openapi::security::SecurityScheme::ApiKey(
                    utoipa::openapi::security::ApiKey::Header(
                        utoipa::openapi::security::ApiKeyValue::new("X-API-Key"),
                    ),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health_check,
        auth::register,
        auth::login,
        auth::forgot_password,
        auth::reset_password,
        ventas::crear_venta,
        ventas::obtener_venta,
        ventas::obtener_lineas_venta,
        ventas::listar_ventas,
        ventas::actualizar_venta,
        ventas::eliminar_venta,
        ventas::reintentar_sync_haddock,
        ventas::reintentar_sync_bdp,
        ventas::obtener_bdp_status,
        ventas::bdp_poll,
        ventas::bdp_invoice,
        ventas::bdp_payment,
        ventas::listar_bdp_payments,
        gastos::crear_gasto,
        gastos::obtener_gasto,
        gastos::listar_gastos,
        gastos::actualizar_gasto,
        gastos::eliminar_gasto,
        gastos::listar_categorias,
        gastos::digitalizar_documento,
        gastos::listar_proveedores,
        reservas::crear_reserva,
        reservas::obtener_reserva,
        reservas::listar_reservas,
        reservas::actualizar_reserva,
        reservas::eliminar_reserva,
        reservas::conteo_reservas,
        reservas::resumen_mensual,
        reservas::no_show_stats,
        clientes::crear_cliente,
        clientes::obtener_cliente,
        clientes::listar_clientes,
        clientes::actualizar_cliente,
        clientes::eliminar_cliente,
        clientes::merge_clientes,
        etiquetas::listar_categorias,
        etiquetas::crear_categoria,
        etiquetas::listar_etiquetas,
        etiquetas::crear_etiqueta,
        etiquetas::eliminar_etiqueta,
        etiquetas::asignar_etiqueta_cliente,
        etiquetas::desasignar_etiqueta_cliente,
        etiquetas::obtener_etiquetas_cliente,
        etiquetas::asignar_etiqueta_reserva,
        etiquetas::desasignar_etiqueta_reserva,
        etiquetas::obtener_etiquetas_reserva,
        canales_reserva::listar_canales,
        canales_reserva::crear_canal,
        canales_reserva::eliminar_canal,
        dashboard::resumen,
        dashboard::dashboard_reservas,
        plano_sala::obtener_plano,
        plano_sala::crear_zona,
        plano_sala::actualizar_zona,
        plano_sala::eliminar_zona,
        plano_sala::crear_mesa,
        plano_sala::actualizar_mesa,
        plano_sala::eliminar_mesa,
        plano_sala::actualizar_posiciones,
        plano_sala::crear_combinacion,
        plano_sala::eliminar_combinacion,
        plano_sala::exportar_plano,
        plano_sala::importar_plano,
        plano_sala::obtener_ocupacion,
        plano_sala::crear_pared,
        plano_sala::actualizar_pared,
        plano_sala::eliminar_pared,
        plano_sala::actualizar_posiciones_paredes,
        configuracion::obtener_configuracion,
        configuracion::actualizar_configuracion,
        configuracion::obtener_integraciones,
        configuracion::actualizar_integraciones,
        configuracion::diagnosticar_bdp,
        configuracion::diagnosticar_bdp_sync_dry_run,
        configuracion::cambiar_bdp_sync_mode,
        campanas::crear_campana,
        campanas::obtener_campana,
        campanas::listar_campanas,
        campanas::actualizar_campana,
        campanas::eliminar_campana,
        campanas::preview_segmento,
        campanas::enviar_campana,
        plantillas_whatsapp::crear_plantilla,
        plantillas_whatsapp::listar_plantillas,
        plantillas_whatsapp::obtener_plantilla,
        plantillas_whatsapp::actualizar_plantilla,
        plantillas_whatsapp::eliminar_plantilla,
        plantillas_whatsapp::enviar_a_meta,
        recordatorios::crear_regla,
        recordatorios::listar_reglas,
        recordatorios::obtener_regla,
        recordatorios::actualizar_regla,
        recordatorios::eliminar_regla,
        recordatorios::historial_recordatorios,
        chatbot::disponibilidad,
        chatbot::restaurante_info,
        chatbot::crear_reserva,
        chatbot::buscar_reservas,
        chatbot::obtener_reserva,
        chatbot::cancelar_reserva,
        api_keys::crear_api_key,
        api_keys::listar_api_keys,
        api_keys::revocar_api_key,
        notificaciones::listar_notificaciones,
        notificaciones::contar_no_leidas,
        notificaciones::marcar_leida,
        notificaciones::marcar_todas_leidas,
        notificaciones::stream_notificaciones,
        errores::reportar_error,
        admin::ejecutar_seed,
        admin::eliminar_datos,
        admin::ejecutar_seed,
        admin::eliminar_datos,
        trabajadores::listar,
        bdp_article_map::listar_article_maps,
        bdp_article_map::crear_article_map,
        bdp_article_map::actualizar_article_map,
        bdp_article_map::eliminar_article_map,
        bdp_article_map::importar_catalogo,
        bdp_article_map::sync_catalog,
        bdp_article_map::sync_prices,
        bdp_article_map::sync_tables,
        bdp_article_map::get_menu_definition,
        bdp_article_map::get_fastfood_definition,
        bdp_article_map::get_pack_definition,
        bdp_purchase_note::listar_purchase_notes,
        bdp_purchase_note::sincronizar_purchase_notes,
        bdp_purchase_note::marcar_borrador_purchase_note,
        bdp_purchase_note::conciliar_purchase_note,
        bdp_customer_sync::importar_clientes_bdp,
        bdp_customer_sync::sincronizar_cliente_bdp,
        bdp_backup::explorar_bdp,
        bdp_backup::snapshot_completo,
        bdp_backup::snapshot_parcial,
        bdp_backup::snapshot_glory,
        bdp_backup::listar_snapshots,
        bdp_backup::obtener_snapshot,
        bdp_backup::eliminar_snapshot,
        bdp_backup::restaurar_glory,
        bdp_backup::listar_audit,
        trabajadores::crear,
        trabajadores::actualizar,
        trabajadores::eliminar,
        trabajadores::login_trabajador,
        trabajadores::listar_secciones,
        resenas::listar_resenas,
        resenas::solicitar_resena,
        resenas::obtener_resena_publica,
        resenas::responder_resena,
        inactividad::listar,
        inactividad::crear,
        inactividad::actualizar,
        inactividad::eliminar,
    ),
    components(schemas(
        health::HealthResponse,
        crate::models::RegisterRequest,
        crate::models::LoginRequest,
        crate::models::AuthResponse,
        crate::models::ForgotPasswordRequest,
        crate::models::ResetPasswordRequest,
        crate::models::MessageResponse,
        crate::models::Venta,
        crate::models::CrearVentaRequest,
        crate::models::ActualizarVentaRequest,
        crate::models::VentasPaginadas,
        crate::models::VentaConCliente,
        crate::models::Gasto,
        crate::models::CrearGastoRequest,
        crate::models::ActualizarGastoRequest,
        crate::models::GastosPaginados,
        crate::models::CategoriaGasto,
        crate::models::Reserva,
        crate::models::CrearReservaRequest,
        crate::models::ActualizarReservaRequest,
        crate::models::ReservasPaginadas,
        crate::models::ReservasConteo,
        crate::models::ResumenDiario,
        crate::models::NoShowStats,
        crate::models::NoShowPorCanal,
        crate::models::CanalReserva,
        crate::models::CrearCanalReservaRequest,
        crate::models::Cliente,
        crate::models::CrearClienteRequest,
        crate::models::ActualizarClienteRequest,
        crate::models::ClientesPaginados,
        crate::models::MergeClientesRequest,
        crate::models::MergeClientesResponse,
        crate::models::CategoriaEtiqueta,
        crate::models::Etiqueta,
        crate::models::EtiquetaConCategoria,
        crate::models::CrearEtiquetaRequest,
        crate::models::CrearCategoriaEtiquetaRequest,
        etiquetas::TagAssignBody,
        crate::models::ResumenEconomico,
        crate::models::DashboardReservas,
        crate::models::ResumenReservas,
        crate::models::OcupacionReservas,
        crate::models::AnalisisReservas,
        crate::models::AgrupacionFecha,
        crate::models::AgrupacionDiaSemana,
        crate::models::AgrupacionCanal,
        crate::models::AgrupacionHora,
        crate::models::AgrupacionTurno,
        crate::models::ZonaSala,
        crate::models::Mesa,
        crate::models::CombinacionMesas,
        crate::models::PlanoSala,
        crate::models::ZonaConMesas,
        crate::models::CombinacionConMesas,
        crate::models::CrearZonaRequest,
        crate::models::ActualizarZonaRequest,
        crate::models::CrearMesaRequest,
        crate::models::ActualizarMesaRequest,
        crate::models::ActualizarPosicionesRequest,
        crate::models::PosicionMesa,
        crate::models::CrearCombinacionRequest,
        crate::models::ParedSala,
        crate::models::CrearParedRequest,
        crate::models::ActualizarParedRequest,
        crate::models::ActualizarPosicionesParedesRequest,
        crate::models::PosicionPared,
        crate::models::PlanoExport,
        crate::models::ZonaExport,
        crate::models::MesaExport,
        crate::models::CombinacionExport,
        crate::models::ParedExport,
        crate::models::PlanoOcupacion,
        crate::models::ZonaOcupacion,
        crate::models::MesaOcupacion,
        crate::models::ReservaMesa,
        crate::models::ConfiguracionRestaurante,
        crate::models::ActualizarConfiguracionRequest,
        configuracion::BdpDiagnosticoResponse,
        configuracion::CambiarBdpSyncModeRequest,
        crate::services::BdpSyncDryRunResponse,
        crate::services::BdpSyncDryRunCheck,
        crate::models::IntegracionMarketingPublica,
        crate::models::ActualizarIntegracionesRequest,
        crate::models::Campana,
        crate::models::CrearCampanaRequest,
        crate::models::ActualizarCampanaRequest,
        crate::models::CampanasPaginadas,
        crate::models::CampanaDestinatario,
        crate::models::SegmentoPreview,
        crate::models::PlantillaWhatsapp,
        crate::models::CrearPlantillaRequest,
        crate::models::ActualizarPlantillaRequest,
        crate::models::PlantillasPaginadas,
        crate::models::ReglaRecordatorio,
        crate::models::CrearReglaRequest,
        crate::models::ActualizarReglaRequest,
        crate::models::ReglasPaginadas,
        crate::models::RecordatorioEnviadoDetalle,
        crate::models::HistorialRecordatorios,
        crate::models::Turno,
        crate::models::CanalVenta,
        crate::models::MetodoPago,
        crate::models::TipoDocumento,
        crate::models::EstadoReserva,
        crate::models::ApiKeyResponse,
        crate::models::ApiKeyCreatedResponse,
        crate::models::CrearApiKeyRequest,
        crate::models::DisponibilidadResponse,
        crate::models::FranjaDisponibilidad,
        crate::models::RestauranteInfoResponse,
        crate::models::CamposObligatorios,
        crate::models::ZonaResumen,
        crate::models::ChatbotCrearReservaRequest,
        crate::models::ChatbotReservaResponse,
        crate::models::ChatbotBuscarReservasQuery,
        crate::models::DigitalizarDocumentoRequest,
        crate::models::DatosDocumentoExtraidos,
        crate::models::Notificacion,
        crate::models::TrabajadorResponse,
        crate::models::CrearTrabajadorRequest,
        crate::models::ActualizarTrabajadorRequest,
        crate::models::LoginTrabajadorRequest,
        crate::models::TrabajadorAuthResponse,
        crate::models::PermisoSeccion,
        crate::models::ResenaPublicaResponse,
        crate::models::ResponderResenaRequest,
        crate::models::ResponderResenaResponse,
        crate::models::ResenasPaginadas,
        crate::models::ResenaAdmin,
        resenas::SolicitarResponse,
        crate::models::ReglaInactividad,
        crate::models::CrearReglaInactividadRequest,
        crate::models::ActualizarReglaInactividadRequest,
        notificaciones::ConteoNoLeidas,
        errores::ReportarErrorRequest,
        errores::ReportarErrorResponse,
        admin::AdminResult,
        crate::models::BdpArticleMap,
        crate::models::BdpArticleStock,
        crate::models::CrearBdpArticleMapRequest,
        crate::models::ActualizarBdpArticleMapRequest,
        crate::models::BdpPurchaseNote,
        crate::models::BdpPurchaseNoteDraftRequest,
        crate::models::BdpPurchaseNoteEstado,
        crate::models::BdpPurchaseNoteListParams,
        crate::models::BdpPurchaseNoteReconcileRequest,
        crate::models::BdpPurchaseNoteReconcileResult,
        crate::models::BdpPurchaseNoteSyncRequest,
        crate::models::BdpPurchaseNoteSyncResult,
        crate::services::BdpCatalogSyncResult,
        crate::services::SyncTablesResult,
        bdp_article_map::SyncTablesRequest,
        crate::services::BdpExploracionResultado,
        crate::services::ExploracionCategoria,
        crate::services::BdpSnapshot,
        crate::services::BdpAuditEntry,
        crate::services::RestoreResult,
        crate::models::VentaLinea,
        crate::models::CrearVentaLineaRequest,
        ventas::BdpOrderStatusResponse,
        ventas::BdpInvoiceRequest,
        ventas::BdpPaymentRequest,
        ventas::BdpPaymentResponse,
        ventas::BdpPaymentHistoryItem,
        ventas::BdpPaymentsResponse,
        ventas::BdpInvoiceResponse,
        ventas::BdpPollResponse,
        bdp_customer_sync::BdpCustomerSyncRequest,
        bdp_customer_sync::BdpCustomerImportRequest,
        ErrorResponse,
    )),
    modifiers(&SecurityAddon),
    info(
        title = "Gestion Restaurante API",
        version = "0.1.0",
        description = "API para gestion de restaurantes - Ventas, Gastos, Reservas, Dashboard"
    )
)]
#[allow(clippy::needless_for_each)]
pub struct ApiDoc;

/// Crea el router principal con CORS, tracing, Swagger UI y todas las rutas
pub fn create_router(pool: sqlx::PgPool, config: crate::config::AppConfig) -> Router {
    /* [283A-20] Canal broadcast para notificaciones SSE — 256 mensajes en buffer */
    let (notif_tx, _) = tokio::sync::broadcast::channel(256);

    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret.clone(),
        config,
        notif_tx,
    };

    /* [303A-2] CORS: restringir orígenes en producción.
     * CORS_ORIGINS env var define orígenes permitidos separados por coma.
     * Si no se define (dev local), se permite todo para no romper desarrollo.
     * En producción, definir CORS_ORIGINS=http://restaurante.wandori.us */
    let cors = match std::env::var("CORS_ORIGINS") {
        Ok(origins) if !origins.is_empty() => {
            let allowed: Vec<_> = origins
                .split(',')
                .filter_map(|o| o.trim().parse().ok())
                .collect();
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(allowed))
                .allow_methods(Any)
                .allow_headers(Any)
        }
        _ => CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    };

    /* [263A-20] En produccion, servir el frontend SPA desde ./static.
     * El fallback_service reenvía rutas no-API al index.html para client-side routing. */
    let spa_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "static".to_string());
    let spa_fallback =
        ServeDir::new(&spa_dir).fallback(ServeFile::new(format!("{spa_dir}/index.html")));

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api", api_routes())
        .fallback_service(spa_fallback)
        .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE)) /* [S16-H2] */
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(auth::routes())
        .merge(ventas::routes())
        .merge(gastos::routes())
        .merge(reservas::routes())
        .merge(clientes::routes())
        .merge(etiquetas::routes())
        .merge(canales_reserva::routes())
        .merge(dashboard::routes())
        .merge(plano_sala::routes())
        .merge(configuracion::routes())
        .merge(bdp_article_map::routes())
        .merge(bdp_backup::routes())
        .merge(bdp_customer_sync::routes())
        .merge(bdp_purchase_note::routes())
        .merge(campanas::routes())
        .merge(plantillas_whatsapp::routes())
        .merge(recordatorios::routes())
        .merge(chatbot::routes())
        .merge(api_keys::routes())
        .merge(notificaciones::routes())
        .merge(errores::routes())
        .merge(admin::routes())
        .merge(trabajadores::routes())
        .merge(resenas::routes())
        .merge(resenas::public_routes())
        .merge(inactividad::routes())
}
