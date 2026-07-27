/* [064A-5] Servicio de sincronización unidireccional con Haddock POS API.
 * Envía ventas a Haddock cuando se crean o actualizan en nuestra plataforma.
 * API: https://pos-api.haddock.app (Basic Auth, POST /orders/, POST /catalog/)
 * Limitaciones: sin DELETE, sin GET, sin webhooks — solo push.
 * [064A-7] Prevención de duplicados: mutex por venta + guard en create si ya sincronizada. */

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex as StdMutex};

use reqwest::Client;
use serde::Serialize;
use sqlx::PgPool;
use tokio::sync::Mutex as TokioMutex;
use tracing::{info, warn};

use crate::models::{ConfiguracionRestaurante, Venta};
use crate::repositories::VentaRepository;

const HADDOCK_API_BASE: &str = "https://pos-api.haddock.app";
const MAX_RETRIES: u32 = 3;

/* [064A-7] Mapa de locks por venta. Evita que dos syncs concurrentes
 * de la misma venta envíen duplicados a Haddock.
 * El StdMutex exterior solo protege el HashMap (lock brevísimo).
 * El TokioMutex interior se sostiene durante toda la operación HTTP. */
static SYNC_LOCKS: LazyLock<StdMutex<HashMap<uuid::Uuid, Arc<TokioMutex<()>>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

/* Estructuras que replica el schema OpenAPI de Haddock */

#[derive(Debug, Serialize)]
struct HaddockOrdersPayload {
    orders: Vec<HaddockOrder>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HaddockOrder {
    /* [064A-15] Haddock espera "externalID" (capital ID), no "externalId" (camelCase).
     * serde rename_all=camelCase produce "externalId" — override explícito obligatorio. */
    #[serde(rename = "externalID")]
    external_id: String,
    date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    seats: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<String>,
    payments: Vec<HaddockPayment>,
    items: Vec<HaddockItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HaddockPayment {
    method: String,
    base: f64,
    tax: f64,
    total: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HaddockItem {
    /* [064A-15] Mismo override que HaddockOrder: Haddock exige "externalID". */
    #[serde(rename = "externalID")]
    external_id: String,
    name: String,
    quantity: i32,
    total: f64,
    price_per_unit: f64,
}

pub struct HaddockService;

impl HaddockService {
    /// Sincroniza una venta con Haddock. Se ejecuta en background (`tokio::spawn`).
    /// Actualiza `haddock_synced`/`haddock_sync_error` en BD tras cada intento.
    /// `is_update`: true si la venta fue editada (siempre re-envía),
    ///              false si es creación (skip si ya sincronizada).
    pub async fn sync_order(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        is_update: bool,
    ) {
        let url = format!("{HADDOCK_API_BASE}/orders/");
        Self::sync_order_to_url(pool, venta, config, &url, is_update).await;
    }

    /* [064A-6] Orquesta sync + actualización de estado en BD.
     * [064A-7] Mutex por venta para evitar syncs concurrentes.
     *          Guard: en create, si la venta ya está synced en BD, skip.
     * Separa HTTP (send_order_http) de persistencia para testabilidad. */
    async fn sync_order_to_url(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        url: &str,
        is_update: bool,
    ) {
        if !config.haddock_sync_enabled || config.haddock_api_token.is_empty() {
            return;
        }

        /* [064A-7] Adquirir lock exclusivo por venta. Si otro sync está en progreso
         * para esta misma venta, skip silencioso — se evitan duplicados. */
        let lock = {
            let mut map = SYNC_LOCKS.lock().expect("SYNC_LOCKS poisoned");
            map.entry(venta.id)
                .or_insert_with(|| Arc::new(TokioMutex::new(())))
                .clone()
        };
        let Ok(_guard) = lock.try_lock() else {
            info!(
                "[064A-7] Sync ya en progreso para venta {}, saltando duplicado",
                venta.id
            );
            return;
        };

        /* [064A-7] Guard de duplicados en creación: re-leer venta de BD para verificar
         * si otro sync ya la marcó como sincronizada mientras esperábamos. */
        if !is_update {
            match VentaRepository::find_by_id(pool, venta.id, venta.user_id).await {
                Ok(Some(fresh)) if fresh.haddock_synced => {
                    info!(
                        "[064A-7] Venta {} ya sincronizada con Haddock, saltando create duplicado",
                        venta.id
                    );
                    Self::cleanup_lock(venta.id);
                    return;
                }
                Ok(None) => {
                    warn!(
                        "[064A-7] Venta {} no encontrada en BD, abortando sync",
                        venta.id
                    );
                    Self::cleanup_lock(venta.id);
                    return;
                }
                Err(e) => {
                    warn!("[064A-7] Error leyendo venta {} para guard: {e}", venta.id);
                    /* Continuar con sync — es mejor intentar que silenciar */
                }
                _ => {} /* venta existe, no synced → continuar */
            }
        }

        match Self::send_order_http(venta, config, url).await {
            Ok(()) => {
                if let Err(e) =
                    VentaRepository::update_haddock_status(pool, venta.id, true, None).await
                {
                    warn!(
                        "[064A-6] Error actualizando status sync de venta {}: {e}",
                        venta.id
                    );
                }
            }
            Err(error_msg) => {
                /* [064A-15] Sanitizar mensaje antes de persistir en BD.
                 * Los errores crudos de Haddock pueden contener info interna
                 * (nombres de merchant, tokens parciales). Solo guardamos
                 * el código HTTP o tipo de error genérico. */
                let safe_msg = Self::sanitize_error_msg(&error_msg);
                if let Err(e) =
                    VentaRepository::update_haddock_status(pool, venta.id, false, Some(&safe_msg))
                        .await
                {
                    warn!(
                        "[064A-6] Error actualizando status sync fallido de venta {}: {e}",
                        venta.id
                    );
                }
                /* El error completo solo vive en logs del servidor */
                warn!(
                    "[064A-15] Error completo de sync venta {}: {error_msg}",
                    venta.id
                );
            }
        }

        Self::cleanup_lock(venta.id);
    }

    /* [064A-7] Limpia la entrada del mapa de locks si no hay otros usuarios.
     * Previene memory leak en el HashMap estático. */
    fn cleanup_lock(venta_id: uuid::Uuid) {
        let mut map = SYNC_LOCKS.lock().expect("SYNC_LOCKS poisoned");
        if let Some(entry) = map.get(&venta_id) {
            if Arc::strong_count(entry) <= 2 {
                map.remove(&venta_id);
            }
        }
    }

    /* [064A-15] Sanitiza errores de Haddock antes de guardarlos en BD.
     * Extrae solo el tipo/código de error sin detalles internos de Haddock. */
    fn sanitize_error_msg(raw: &str) -> String {
        if raw.contains("401") {
            "Error de autenticación con Haddock (401)".to_string()
        } else if raw.contains("403") {
            "Acceso denegado por Haddock (403)".to_string()
        } else if raw.contains("400") {
            "Datos rechazados por Haddock (400)".to_string()
        } else if raw.contains("429") {
            "Límite de requests excedido en Haddock (429)".to_string()
        } else if raw.contains("500") || raw.contains("502") || raw.contains("503") {
            "Error interno de Haddock (servidor)".to_string()
        } else if raw.contains("Error de red") {
            "Error de conexión con Haddock".to_string()
        } else {
            /* Fallback genérico — truncar a 120 chars por seguridad */
            let truncated: String = raw.chars().take(120).collect();
            format!("Error de sincronización: {truncated}")
        }
    }

    /* [064A-6] Lógica HTTP pura: mapea, envía con retry, retorna Ok/Err.
     * No toca BD — testeable con wiremock sin PgPool. */
    async fn send_order_http(
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        url: &str,
    ) -> Result<(), String> {
        if !config.haddock_sync_enabled || config.haddock_api_token.is_empty() {
            return Ok(());
        }

        let order = Self::map_venta_to_order(venta);
        let payload = HaddockOrdersPayload {
            orders: vec![order],
        };

        let token = &config.haddock_api_token;
        let mut last_error = String::new();

        for attempt in 0..MAX_RETRIES {
            match Self::send_request(url, token, &payload).await {
                Ok(status) if status.is_success() => {
                    info!(
                        "[064A-6] Venta {} sincronizada con Haddock (intento {})",
                        venta.id,
                        attempt + 1
                    );
                    return Ok(());
                }
                /* [064A-15] 401/403 son fallos permanentes de autenticación.
                 * Reintentar no los va a resolver — abortar inmediatamente
                 * para no desperdiciar requests contra la API de Haddock. */
                Ok(status) if status.as_u16() == 401 || status.as_u16() == 403 => {
                    let msg = format!("Haddock rechazó credenciales (HTTP {status})");
                    warn!("[064A-15] {msg} para venta {} — no se reintenta", venta.id);
                    return Err(msg);
                }
                Ok(status) => {
                    last_error = format!("Haddock respondió HTTP {status}");
                    warn!(
                        "[064A-6] {last_error} para venta {} (intento {})",
                        venta.id,
                        attempt + 1
                    );
                }
                Err(e) => {
                    last_error = format!("Error de red: {e}");
                    warn!(
                        "[064A-6] {last_error} sincronizando venta {} (intento {})",
                        venta.id,
                        attempt + 1
                    );
                }
            }

            if attempt < MAX_RETRIES - 1 {
                let delay = std::time::Duration::from_secs(1 << attempt);
                tokio::time::sleep(delay).await;
            }
        }

        warn!(
            "[064A-6] Fallo definitivo sincronizando venta {} con Haddock después de {MAX_RETRIES} intentos",
            venta.id
        );
        Err(last_error)
    }

    /// Mapea una Venta de nuestra plataforma al formato que espera Haddock
    fn map_venta_to_order(venta: &Venta) -> HaddockOrder {
        let total =
            Self::decimal_to_f64(&venta.importe_base) + Self::decimal_to_f64(&venta.importe_iva);

        /* Hora estimada por turno (Haddock requiere ISO 8601 con hora) */
        let hora = match venta.turno.as_str() {
            "manana" => "09:00:00",
            "mediodia" => "14:00:00",
            _ => "21:00:00",
        };
        let date = format!("{}T{}Z", venta.fecha, hora);

        HaddockOrder {
            external_id: venta.id.to_string(),
            date,
            seats: venta.comensales,
            channel: Some(Self::map_canal(&venta.canal)),
            payments: vec![HaddockPayment {
                method: Self::map_metodo_pago(&venta.metodo_pago),
                base: Self::decimal_to_f64(&venta.importe_base),
                tax: Self::decimal_to_f64(&venta.importe_iva),
                total,
            }],
            /* items es obligatorio en Haddock — enviamos un ítem genérico
             * porque nuestra plataforma no desglosa artículos individuales */
            items: vec![HaddockItem {
                external_id: format!("venta-{}", venta.id),
                name: if venta.descripcion.is_empty() {
                    "Servicio de mesa".to_string()
                } else {
                    venta.descripcion.clone()
                },
                quantity: 1,
                total,
                price_per_unit: total,
            }],
        }
    }

    /* Mapeo de canales: nuestro enum → valores que acepta Haddock */
    fn map_canal(canal: &str) -> String {
        match canal {
            "comedor" => "dining-room",
            "barra" => "bar",
            "terraza" => "terrace",
            "delivery" => "delivery",
            "just_eat" => "justeat",
            "eventos" => "events",
            _ => "any",
        }
        .to_string()
    }

    /* Mapeo de métodos de pago: nuestro enum → valores que acepta Haddock */
    fn map_metodo_pago(metodo: &str) -> String {
        match metodo {
            "efectivo" => "cash",
            "tarjeta" => "card",
            "transferencia" => "transfer",
            _ => "any",
        }
        .to_string()
    }

    /* [R16] Conversión Decimal→f64 para serialización JSON a Haddock.
     * Se convierte vía string para máxima precisión; es el enfoque más fiable
     * para Decimal→f64. El redondeo monetario se aplica en los call-sites
     * que lo necesiten, no aquí (vat_pct es un porcentaje, no moneda). */
    fn decimal_to_f64(d: &rust_decimal::Decimal) -> f64 {
        use std::str::FromStr;
        match f64::from_str(&d.to_string()) {
            Ok(v) => v,
            Err(e) => {
                warn!("[R16] Error convirtiendo Decimal '{d}' a f64: {e}");
                0.0
            }
        }
    }

    /* [064A-15] Cliente HTTP reutilizable — LazyLock garantiza un solo Client
     * con connection pool persistente en vez de crear uno por cada request. */
    async fn send_request(
        url: &str,
        token: &str,
        payload: &HaddockOrdersPayload,
    ) -> Result<reqwest::StatusCode, reqwest::Error> {
        static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);
        let resp = HTTP_CLIENT
            .post(url)
            .header("Authorization", format!("Basic {token}"))
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .await?;
        Ok(resp.status())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime, Utc};
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use uuid::Uuid;
    use wiremock::{
        matchers::{body_json, header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    fn mock_venta() -> Venta {
        Venta {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            fecha: NaiveDate::from_ymd_opt(2026, 4, 6).unwrap(),
            comensales: Some(4),
            descripcion: "Menú del día".to_string(),
            iva_porcentaje: Decimal::from(10),
            turno: "mediodia".to_string(),
            canal: "comedor".to_string(),
            metodo_pago: "tarjeta".to_string(),
            importe_base: Decimal::from_str("45.00").unwrap(),
            importe_iva: Decimal::from_str("4.50").unwrap(),
            reserva_id: None,
            cliente_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            /* [064A-6] Campos de tracking Haddock */
            haddock_synced: false,
            haddock_synced_at: None,
            haddock_sync_error: None,
            /* [065A-5] Campos de tracking BDP */
            bdp_synced: false,
            bdp_synced_at: None,
            bdp_sync_error: None,
            bdp_order_id: None,
            /* [276A-4.1] Estado BDP polling */
            bdp_order_status: None,
            bdp_invoiced: false,
        }
    }

    fn mock_config(token: &str, enabled: bool) -> ConfiguracionRestaurante {
        ConfiguracionRestaurante {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            reserva_email_obligatorio: false,
            reserva_telefono_obligatorio: true,
            reserva_nombre_obligatorio: true,
            reserva_apellidos_obligatorio: false,
            iva_por_defecto: Decimal::from(10),
            nombre_restaurante: "Test".to_string(),
            groq_api_key: None,
            auto_venta_reserva: false,
            hora_desayuno_inicio: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            hora_desayuno_fin: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            hora_comida_inicio: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            hora_comida_fin: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
            hora_cena_inicio: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
            hora_cena_fin: NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
            url_haddock: String::new(),
            haddock_api_token: token.to_string(),
            haddock_sync_enabled: enabled,
            bdp_base_url: String::new(),
            bdp_login: String::new(),
            bdp_password: String::new(),
            bdp_integrator_code: String::new(),
            bdp_sync_enabled: false,
            bdp_pos_id: 1,
            bdp_employee_id: 1,
            bdp_items_profile_id: 1,
            bdp_default_article_code: String::new(),
            bdp_default_article_name: String::new(),
            /* [276A-4.2] Campos BDP nuevos */
            bdp_tender_map: serde_json::json!({}),
            bdp_order_type_map: serde_json::json!({}),
            bdp_default_customer_code: String::new(),
            bdp_poll_interval_secs: 60,
            bdp_poll_enabled: false,
            bdp_auto_sync_customers: false,
            bdp_sync_mode: "read_only".to_string(),
            bdp_backup_retention_days: 30,
            bdp_auto_backup_before_write: true,
            bdp_env_bootstrap_applied_at: None,
            /* [XT2-1] Feature flags BDP desactivados por defecto en tests */
            ff_bdp_auto_arm: false,
            ff_bdp_partial_payments: false,
            ff_bdp_cancel_order: false,
            ff_bdp_purchase_notes_read: false,
            ff_bdp_purchase_notes_draft: false,
            ff_bdp_purchase_notes_receive: false,
            google_review_url: String::new(),
            telefono_restaurante: String::new(),
            url_reservas: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /* ── Tests de mapeo de datos (unitarios) ────────────────────── */

    #[test]
    fn test_map_venta_to_order() {
        let venta = mock_venta();
        let order = HaddockService::map_venta_to_order(&venta);

        assert_eq!(order.external_id, venta.id.to_string());
        assert!(order.date.contains("14:00:00"));
        assert_eq!(order.seats, Some(4));
        assert_eq!(order.channel.as_deref(), Some("dining-room"));
        assert_eq!(order.payments.len(), 1);
        assert_eq!(order.payments[0].method, "card");
        assert!((order.payments[0].base - 45.0).abs() < 0.01);
        assert!((order.payments[0].tax - 4.50).abs() < 0.01);
        assert!((order.payments[0].total - 49.50).abs() < 0.01);
        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].name, "Menú del día");
    }

    #[test]
    fn test_map_canales() {
        assert_eq!(HaddockService::map_canal("comedor"), "dining-room");
        assert_eq!(HaddockService::map_canal("barra"), "bar");
        assert_eq!(HaddockService::map_canal("terraza"), "terrace");
        assert_eq!(HaddockService::map_canal("delivery"), "delivery");
        assert_eq!(HaddockService::map_canal("just_eat"), "justeat");
        assert_eq!(HaddockService::map_canal("eventos"), "events");
        assert_eq!(HaddockService::map_canal("desconocido"), "any");
    }

    #[test]
    fn test_map_metodos_pago() {
        assert_eq!(HaddockService::map_metodo_pago("efectivo"), "cash");
        assert_eq!(HaddockService::map_metodo_pago("tarjeta"), "card");
        assert_eq!(HaddockService::map_metodo_pago("transferencia"), "transfer");
        assert_eq!(HaddockService::map_metodo_pago("otro"), "any");
    }

    #[test]
    fn test_empty_description_fallback() {
        let mut venta = mock_venta();
        venta.descripcion = String::new();
        let order = HaddockService::map_venta_to_order(&venta);
        assert_eq!(order.items[0].name, "Servicio de mesa");
    }

    #[test]
    fn test_zero_amounts_auto_venta() {
        let mut venta = mock_venta();
        venta.importe_base = Decimal::ZERO;
        venta.importe_iva = Decimal::ZERO;
        let order = HaddockService::map_venta_to_order(&venta);
        assert!((order.payments[0].total).abs() < 0.001);
        assert!((order.items[0].total).abs() < 0.001);
    }

    #[test]
    fn test_turno_hora_mapping() {
        let mut venta = mock_venta();
        venta.turno = "manana".to_string();
        let order = HaddockService::map_venta_to_order(&venta);
        assert!(order.date.contains("09:00:00"));

        venta.turno = "noche".to_string();
        let order = HaddockService::map_venta_to_order(&venta);
        assert!(order.date.contains("21:00:00"));

        /* Turno desconocido → fallback a noche */
        venta.turno = "inventado".to_string();
        let order = HaddockService::map_venta_to_order(&venta);
        assert!(order.date.contains("21:00:00"));
    }

    #[test]
    fn test_date_format_iso8601() {
        let venta = mock_venta();
        let order = HaddockService::map_venta_to_order(&venta);
        /* Formato esperado: YYYY-MM-DDTHH:MM:SSZ */
        assert_eq!(order.date, "2026-04-06T14:00:00Z");
    }

    #[test]
    fn test_external_id_is_uuid() {
        let venta = mock_venta();
        let order = HaddockService::map_venta_to_order(&venta);
        /* Verificar que external_id es un UUID válido */
        assert!(Uuid::parse_str(&order.external_id).is_ok());
    }

    /* ── Tests de integración con mock server HTTP ──────────────── */

    #[tokio::test]
    async fn test_sync_disabled_no_request() {
        let server = MockServer::start().await;
        /* No registramos ningún mock — si se hace request, el test falla */
        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", false);
        let url = format!("{}/orders/", server.uri());

        let _ = HaddockService::send_order_http(&venta, &config, &url).await;

        /* Verificar que NO se hizo ningún request */
        let received = server.received_requests().await.unwrap();
        assert!(
            received.is_empty(),
            "No debería enviar requests con sync desactivado"
        );
    }

    #[tokio::test]
    async fn test_empty_token_no_request() {
        let server = MockServer::start().await;
        let venta = mock_venta();
        let config = mock_config("", true);
        let url = format!("{}/orders/", server.uri());

        let _ = HaddockService::send_order_http(&venta, &config, &url).await;

        let received = server.received_requests().await.unwrap();
        assert!(
            received.is_empty(),
            "No debería enviar requests con token vacío"
        );
    }

    #[tokio::test]
    async fn test_both_disabled_and_empty_no_request() {
        let server = MockServer::start().await;
        let venta = mock_venta();
        let config = mock_config("", false);
        let url = format!("{}/orders/", server.uri());

        let _ = HaddockService::send_order_http(&venta, &config, &url).await;

        let received = server.received_requests().await.unwrap();
        assert!(
            received.is_empty(),
            "No debería enviar requests con ambos desactivados"
        );
    }

    #[tokio::test]
    async fn test_sync_success_200() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .expect(1)
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", true);
        let url = format!("{}/orders/", server.uri());

        let _ = HaddockService::send_order_http(&venta, &config, &url).await;
        /* wiremock verifica automáticamente que se recibió exactamente 1 request */
    }

    #[tokio::test]
    async fn test_authorization_header_format() {
        let token = "dG9rZW46c2VjcmV0";
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .and(header("Authorization", &format!("Basic {token}")))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config(token, true);
        let url = format!("{}/orders/", server.uri());

        let _ = HaddockService::send_order_http(&venta, &config, &url).await;
    }

    #[tokio::test]
    async fn test_content_type_json() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .and(header("Content-Type", "application/json"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", true);
        let url = format!("{}/orders/", server.uri());

        let _ = HaddockService::send_order_http(&venta, &config, &url).await;
    }

    /* [064A-15] 401 ya NO reintenta — aborta de inmediato */
    #[tokio::test]
    async fn test_401_unauthorized_aborts_no_retry() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(
                ResponseTemplate::new(401).set_body_string(r#"{"error":"Invalid credentials"}"#),
            )
            .expect(1) /* [064A-15] 1 request, no 3 */
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config("token-malo", true);
        let url = format!("{}/orders/", server.uri());

        let result = HaddockService::send_order_http(&venta, &config, &url).await;
        assert!(result.is_err(), "401 debe retornar Err");
    }

    #[tokio::test]
    async fn test_500_server_error_retries_3_times() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .expect(3)
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", true);
        let url = format!("{}/orders/", server.uri());

        let _ = HaddockService::send_order_http(&venta, &config, &url).await;
    }

    #[tokio::test]
    async fn test_success_on_second_retry() {
        let server = MockServer::start().await;
        /* Primer intento falla con 500, segundo tiene éxito */
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", true);
        let url = format!("{}/orders/", server.uri());

        let _ = HaddockService::send_order_http(&venta, &config, &url).await;
        /* Total: 2 requests (1 fallo + 1 éxito) */
    }

    #[tokio::test]
    async fn test_network_error_no_panic() {
        /* Apuntar a un puerto cerrado — genera error de conexión */
        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", true);
        let url = "http://127.0.0.1:1/orders/";

        /* Esto NO debe hacer panic — solo loguear el error */
        let _ = HaddockService::send_order_http(&venta, &config, url).await;
    }

    #[tokio::test]
    async fn test_payload_json_structure() {
        let server = MockServer::start().await;
        let venta = mock_venta();
        let order = HaddockService::map_venta_to_order(&venta);
        let expected_payload = HaddockOrdersPayload {
            orders: vec![order],
        };

        Mock::given(method("POST"))
            .and(path("/orders/"))
            .and(body_json(&expected_payload))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let config = mock_config("dG9rZW46c2VjcmV0", true);
        let url = format!("{}/orders/", server.uri());

        let _ = HaddockService::send_order_http(&venta, &config, &url).await;
    }

    /* [064A-15] Verifica que el JSON usa los nombres exactos que espera Haddock.
     * "externalID" (capital ID) — NO "externalId" (camelCase genérico).
     * "pricePerUnit" (camelCase normal) — NO "price_per_unit" (snake_case). */
    #[tokio::test]
    async fn test_json_has_correct_haddock_field_names() {
        let venta = mock_venta();
        let order = HaddockService::map_venta_to_order(&venta);
        let payload = HaddockOrdersPayload {
            orders: vec![order],
        };
        let json = serde_json::to_string(&payload).unwrap();

        /* Haddock exige "externalID" con ID en mayúsculas */
        assert!(
            json.contains("externalID"),
            "Debe usar externalID (capital ID): {json}"
        );
        assert!(
            json.contains("pricePerUnit"),
            "Debe usar camelCase: pricePerUnit"
        );
        /* No debe contener las variantes incorrectas */
        assert!(
            !json.contains("\"externalId\""),
            "No debe usar externalId (lowercase d)"
        );
        assert!(!json.contains("external_id"), "No debe usar snake_case");
        assert!(!json.contains("price_per_unit"), "No debe usar snake_case");
    }

    #[tokio::test]
    async fn test_venta_sin_comensales() {
        let mut venta = mock_venta();
        venta.comensales = None;
        let order = HaddockService::map_venta_to_order(&venta);
        assert!(order.seats.is_none());

        /* Verificar que seats no aparece en JSON (skip_serializing_if) */
        let payload = HaddockOrdersPayload {
            orders: vec![order],
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(
            !json.contains("seats"),
            "seats=null no debería serializarse"
        );
    }

    #[tokio::test]
    async fn test_large_amounts() {
        let mut venta = mock_venta();
        venta.importe_base = Decimal::from_str("99999.99").unwrap();
        venta.importe_iva = Decimal::from_str("21000.00").unwrap();
        let order = HaddockService::map_venta_to_order(&venta);
        assert!((order.payments[0].base - 99_999.99).abs() < 0.01);
        assert!((order.payments[0].total - 120_999.99).abs() < 0.01);
    }

    /* ── [064A-11] Tests de flujo y robustez ───────────────────── */

    /* Test 1: send_order_http retorna Ok(()) cuando Haddock responde 200.
     * Verifica el contrato de retorno, no solo que no paniquee. */
    #[tokio::test]
    async fn test_sync_success_returns_ok() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", true);
        let url = format!("{}/orders/", server.uri());

        let result = HaddockService::send_order_http(&venta, &config, &url).await;
        assert!(result.is_ok(), "Sync exitoso debe retornar Ok(())");
    }

    /* Test 2: send_order_http retorna Err con mensaje descriptivo tras 3 reintentos con 500.
     * Simula: crear venta → sync falla 3x → error con HTTP status en mensaje. */
    #[tokio::test]
    async fn test_sync_failure_returns_error_with_status() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(500))
            .expect(3)
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", true);
        let url = format!("{}/orders/", server.uri());

        let result = HaddockService::send_order_http(&venta, &config, &url).await;
        assert!(result.is_err(), "3 fallos deben retornar Err");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("500"),
            "Mensaje de error debe incluir código HTTP: {msg}"
        );
    }

    /* Test 3: Token inválido → 401 → error con mensaje que incluye "401".
     * [064A-15] Actualizado: ya no reintenta en 401 — solo 1 request. */
    #[tokio::test]
    async fn test_invalid_token_401_returns_error_msg() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1) /* [064A-15] 1, no 3 — auth failures no se reintentan */
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config("token-invalido", true);
        let url = format!("{}/orders/", server.uri());

        let result = HaddockService::send_order_http(&venta, &config, &url).await;
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("401"),
            "Error de auth debe reportar 401: {msg}"
        );
    }

    /* Test 4: Error de red retorna Err con "Error de red" en el mensaje.
     * Cubre el caso donde Haddock está caído o hay problemas de conectividad. */
    #[tokio::test]
    async fn test_network_error_returns_error_msg() {
        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", true);

        let result =
            HaddockService::send_order_http(&venta, &config, "http://127.0.0.1:1/orders/").await;
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("Error de red"), "Debe ser error de red: {msg}");
    }

    /* Test 5: Guard de config deshabilitada retorna Ok (no Err).
     * Importante: no es un error, es un skip legítimo. */
    #[tokio::test]
    async fn test_guard_disabled_config_returns_ok_not_err() {
        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", false);

        let result =
            HaddockService::send_order_http(&venta, &config, "http://should-not-be-called/orders/")
                .await;
        assert!(result.is_ok(), "Config deshabilitada → Ok, no Err");
    }

    /* Test 6: Guard de token vacío retorna Ok (no Err).
     * El empty token es condición de skip, no error. */
    #[tokio::test]
    async fn test_guard_empty_token_returns_ok_not_err() {
        let venta = mock_venta();
        let config = mock_config("", true);

        let result =
            HaddockService::send_order_http(&venta, &config, "http://should-not-be-called/orders/")
                .await;
        assert!(result.is_ok(), "Token vacío → Ok, no Err");
    }

    /* Test 7: HTTP 403 Forbidden — [064A-15] ya NO se reintenta (error permanente). */
    #[tokio::test]
    async fn test_403_forbidden_aborts_no_retry() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(403))
            .expect(1) /* [064A-15] 1 request, no 3 */
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", true);
        let url = format!("{}/orders/", server.uri());

        let result = HaddockService::send_order_http(&venta, &config, &url).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("403"));
    }

    /* Test 8: Payload NO incluye campos internos de tracking (haddock_synced, etc.).
     * Estos campos son de BD interna, nunca deben filtrarse a la API de Haddock. */
    #[tokio::test]
    async fn test_payload_excludes_internal_tracking_fields() {
        let mut venta = mock_venta();
        venta.haddock_synced = true;
        venta.haddock_sync_error = Some("error previo".to_string());

        let order = HaddockService::map_venta_to_order(&venta);
        let payload = HaddockOrdersPayload {
            orders: vec![order],
        };
        let json = serde_json::to_string(&payload).unwrap();

        assert!(
            !json.contains("haddock_synced"),
            "haddock_synced no debe filtrarse al payload"
        );
        assert!(
            !json.contains("haddockSynced"),
            "haddockSynced no debe filtrarse al payload"
        );
        assert!(
            !json.contains("sync_error"),
            "sync_error no debe filtrarse al payload"
        );
        assert!(
            !json.contains("syncError"),
            "syncError no debe filtrarse al payload"
        );
        assert!(
            !json.contains("error previo"),
            "Contenido de sync_error no debe filtrarse"
        );
    }

    /* Test 9: Payload NO incluye reserva_id, cliente_id ni user_id.
     * Estos son IDs internos que no pertenecen al schema de Haddock. */
    #[tokio::test]
    async fn test_payload_excludes_internal_ids() {
        let mut venta = mock_venta();
        venta.reserva_id = Some(Uuid::new_v4());
        venta.cliente_id = Some(Uuid::new_v4());

        let order = HaddockService::map_venta_to_order(&venta);
        let payload = HaddockOrdersPayload {
            orders: vec![order],
        };
        let json = serde_json::to_string(&payload).unwrap();

        assert!(!json.contains("reserva_id"), "reserva_id no debe filtrarse");
        assert!(!json.contains("reservaId"), "reservaId no debe filtrarse");
        assert!(!json.contains("cliente_id"), "cliente_id no debe filtrarse");
        assert!(!json.contains("clienteId"), "clienteId no debe filtrarse");
        assert!(!json.contains("user_id"), "user_id no debe filtrarse");
        assert!(!json.contains("userId"), "userId no debe filtrarse");
    }

    /* Test 10: El lock estático SYNC_LOCKS se adquiere y limpia correctamente.
     * Verifica que cleanup_lock elimina la entrada cuando solo queda 1 referencia. */
    #[tokio::test]
    async fn test_sync_lock_acquire_and_cleanup() {
        let venta_id = Uuid::new_v4();

        /* Insertar lock manualmente */
        {
            let mut map = SYNC_LOCKS.lock().expect("SYNC_LOCKS poisoned");
            map.insert(venta_id, Arc::new(TokioMutex::new(())));
        }

        /* Verificar que existe */
        {
            let map = SYNC_LOCKS.lock().expect("SYNC_LOCKS poisoned");
            assert!(
                map.contains_key(&venta_id),
                "Lock debe existir tras inserción"
            );
        }

        /* Cleanup debe eliminarlo (strong_count = 1, solo el HashMap lo tiene) */
        HaddockService::cleanup_lock(venta_id);

        {
            let map = SYNC_LOCKS.lock().expect("SYNC_LOCKS poisoned");
            assert!(
                !map.contains_key(&venta_id),
                "Lock debe eliminarse tras cleanup con 1 referencia"
            );
        }
    }

    /* ── [064A-15] Tests de auditoría — correcciones críticas ──── */

    /* Test 064A-15-1: 401 no se reintenta — aborta inmediatamente.
     * Haddock devuelve 401 = credenciales inválidas, reintentar no sirve. */
    #[tokio::test]
    async fn test_401_aborts_immediately_no_retry() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1) /* Solo 1 request, no 3 */
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config("token-malo", true);
        let url = format!("{}/orders/", server.uri());

        let result = HaddockService::send_order_http(&venta, &config, &url).await;
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("401"), "Error debe contener código: {msg}");
        assert!(
            msg.contains("credenciales"),
            "Error debe mencionar credenciales: {msg}"
        );
    }

    /* Test 064A-15-2: 403 no se reintenta — aborta inmediatamente. */
    #[tokio::test]
    async fn test_403_aborts_immediately_no_retry() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(403))
            .expect(1) /* Solo 1 request, no 3 */
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", true);
        let url = format!("{}/orders/", server.uri());

        let result = HaddockService::send_order_http(&venta, &config, &url).await;
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("403"), "Error debe contener código: {msg}");
    }

    /* Test 064A-15-3: 500 SÍ se reintenta 3 veces (transient server error). */
    #[tokio::test]
    async fn test_500_still_retries_3_times() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(500))
            .expect(3)
            .mount(&server)
            .await;

        let venta = mock_venta();
        let config = mock_config("dG9rZW46c2VjcmV0", true);
        let url = format!("{}/orders/", server.uri());

        let result = HaddockService::send_order_http(&venta, &config, &url).await;
        assert!(result.is_err());
    }

    /* Test 064A-15-4: Sanitización de errores — 401 genera mensaje genérico. */
    #[test]
    fn test_sanitize_error_401() {
        let raw = "Haddock rechazó credenciales (HTTP 401 Unauthorized)";
        let sanitized = HaddockService::sanitize_error_msg(raw);
        assert!(sanitized.contains("401"), "Debe incluir código");
        assert!(sanitized.contains("autenticación"), "Debe ser genérico");
        assert!(
            !sanitized.contains("Unauthorized"),
            "No debe filtrar detalle HTTP"
        );
    }

    /* Test 064A-15-5: Sanitización de errores — 500 genera mensaje genérico. */
    #[test]
    fn test_sanitize_error_500() {
        let raw = "Haddock respondió HTTP 500 Internal Server Error con body: {\"detail\":\"merchant xyz\"}";
        let sanitized = HaddockService::sanitize_error_msg(raw);
        assert!(sanitized.contains("servidor"), "Debe ser genérico");
        assert!(
            !sanitized.contains("merchant"),
            "No debe filtrar info del merchant"
        );
    }

    /* Test 064A-15-6: Sanitización trunca errores desconocidos a 120 chars. */
    #[test]
    fn test_sanitize_error_truncates_unknown() {
        let raw = "X".repeat(300);
        let sanitized = HaddockService::sanitize_error_msg(&raw);
        assert!(
            sanitized.len() <= 150,
            "Debe estar truncado: {} chars",
            sanitized.len()
        );
    }

    /* ── [064A-13] Tests de flujo completo con BD real ─────────── */
    /* Estos tests usan #[sqlx::test] para crear una BD temporal con migraciones.
     * Necesitan acceso a sync_order_to_url (privado) para inyectar URL de wiremock.
     * Flujo: crear usuario → crear venta en BD → sync con wiremock → verificar BD. */

    async fn create_test_user(pool: &sqlx::PgPool) -> Uuid {
        let id = Uuid::new_v4();
        let email = format!("test-{id}@example.com");
        sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
            .bind(id)
            .bind(&email)
            .bind("argon2_hash_placeholder")
            .execute(pool)
            .await
            .expect("create_test_user falló");
        id
    }

    async fn create_test_config(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        token: &str,
        enabled: bool,
    ) -> ConfiguracionRestaurante {
        use crate::models::ActualizarConfiguracionRequest;
        use crate::repositories::ConfiguracionRepository;

        ConfiguracionRepository::obtener_o_crear(pool, user_id)
            .await
            .unwrap();
        let req = ActualizarConfiguracionRequest {
            haddock_sync_enabled: Some(enabled),
            haddock_api_token: Some(token.to_string()),
            url_haddock: None,
            bdp_base_url: None,
            bdp_login: None,
            bdp_password: None,
            bdp_integrator_code: None,
            bdp_sync_enabled: None,
            bdp_pos_id: None,
            bdp_employee_id: None,
            bdp_items_profile_id: None,
            bdp_tender_map: None,
            bdp_order_type_map: None,
            bdp_default_customer_code: None,
            bdp_default_article_code: None,
            bdp_default_article_name: None,
            bdp_poll_interval_secs: None,
            bdp_poll_enabled: None,
            google_review_url: None,
            telefono_restaurante: None,
            url_reservas: None,
            reserva_email_obligatorio: None,
            reserva_telefono_obligatorio: None,
            reserva_nombre_obligatorio: None,
            reserva_apellidos_obligatorio: None,
            iva_por_defecto: None,
            nombre_restaurante: None,
            groq_api_key: None,
            auto_venta_reserva: None,
            hora_desayuno_inicio: None,
            hora_desayuno_fin: None,
            hora_comida_inicio: None,
            hora_comida_fin: None,
            hora_cena_inicio: None,
            hora_cena_fin: None,
            bdp_auto_sync_customers: None,
            bdp_sync_mode: None,
            bdp_backup_retention_days: None,
            bdp_auto_backup_before_write: None,
            /* [XT2-1] Feature flags BDP */
            ff_bdp_auto_arm: None,
            ff_bdp_partial_payments: None,
            ff_bdp_cancel_order: None,
            ff_bdp_purchase_notes_read: None,
            ff_bdp_purchase_notes_draft: None,
            ff_bdp_purchase_notes_receive: None,
        };
        ConfiguracionRepository::actualizar(pool, user_id, &req)
            .await
            .unwrap()
    }

    async fn create_test_venta(pool: &sqlx::PgPool, user_id: Uuid) -> Venta {
        use crate::repositories::venta::{NuevaVenta, VentaRepository};

        let data = NuevaVenta {
            user_id,
            fecha: NaiveDate::from_ymd_opt(2026, 4, 6).unwrap(),
            comensales: Some(2),
            descripcion: "Test DB flow",
            iva_porcentaje: Decimal::from(10),
            turno: "mediodia",
            canal: "comedor",
            metodo_pago: "tarjeta",
            importe_base: Decimal::from_str("30.00").unwrap(),
            importe_iva: Decimal::from_str("3.00").unwrap(),
            reserva_id: None,
            cliente_id: None,
        };
        VentaRepository::create(pool, &data).await.unwrap()
    }

    /* Test DB-1: Flujo completo sync exitoso → haddock_synced=true en BD.
     * Crea venta real → wiremock responde 200 → verifica que update_haddock_status
     * persistió synced=true y timestamp en la BD real. */
    #[sqlx::test(migrations = "./migrations")]
    async fn test_db_sync_success_updates_haddock_status(pool: sqlx::PgPool) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let user_id = create_test_user(&pool).await;
        let config = create_test_config(&pool, user_id, "dG9rZW46c2VjcmV0", true).await;
        let venta = create_test_venta(&pool, user_id).await;
        let url = format!("{}/orders/", server.uri());

        HaddockService::sync_order_to_url(&pool, &venta, &config, &url, false).await;

        let updated = VentaRepository::find_by_id(&pool, venta.id, user_id)
            .await
            .unwrap()
            .unwrap();
        assert!(updated.haddock_synced, "sync exitoso → synced=true en BD");
        assert!(
            updated.haddock_synced_at.is_some(),
            "Debe persistir timestamp"
        );
        assert!(updated.haddock_sync_error.is_none(), "Sin error en BD");
    }

    /* Test DB-2: Flujo completo sync fallido → error registrado en BD.
     * Wiremock responde 500 3 veces → verifica que haddock_sync_error se persistió. */
    #[sqlx::test(migrations = "./migrations")]
    async fn test_db_sync_failure_records_error_in_db(pool: sqlx::PgPool) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(500))
            .expect(3)
            .mount(&server)
            .await;

        let user_id = create_test_user(&pool).await;
        let config = create_test_config(&pool, user_id, "dG9rZW46c2VjcmV0", true).await;
        let venta = create_test_venta(&pool, user_id).await;
        let url = format!("{}/orders/", server.uri());

        HaddockService::sync_order_to_url(&pool, &venta, &config, &url, false).await;

        let updated = VentaRepository::find_by_id(&pool, venta.id, user_id)
            .await
            .unwrap()
            .unwrap();
        assert!(!updated.haddock_synced, "sync fallido → synced=false");
        assert!(
            updated.haddock_sync_error.is_some(),
            "Debe registrar error en BD"
        );
        /* [064A-15] El error ahora está sanitizado — no contiene código HTTP crudo,
         * sino un mensaje genérico. Verificamos que menciona "servidor". */
        assert!(
            updated
                .haddock_sync_error
                .as_deref()
                .unwrap()
                .contains("servidor"),
            "Error sanitizado debe mencionar 'servidor': {:?}",
            updated.haddock_sync_error
        );
    }

    /* Test DB-3: Guard de duplicados — venta ya synced + is_update=false → no HTTP.
     * Verifica flujo real: BD marca synced → segundo sync con create → skip. */
    #[sqlx::test(migrations = "./migrations")]
    async fn test_db_dedup_guard_skips_already_synced(pool: sqlx::PgPool) {
        let server = MockServer::start().await;
        /* Si se hace algún request, el test falla (expect 0) */
        Mock::given(method("POST"))
            .and(path("/orders/"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        let user_id = create_test_user(&pool).await;
        let config = create_test_config(&pool, user_id, "dG9rZW46c2VjcmV0", true).await;
        let venta = create_test_venta(&pool, user_id).await;

        /* Marcar como synced manualmente */
        VentaRepository::update_haddock_status(&pool, venta.id, true, None)
            .await
            .unwrap();

        let url = format!("{}/orders/", server.uri());
        /* is_update=false → guard de duplicados lee BD y encuentra synced=true → skip */
        HaddockService::sync_order_to_url(&pool, &venta, &config, &url, false).await;
        /* wiremock verifica que no se hizo ningún request */
    }
}
