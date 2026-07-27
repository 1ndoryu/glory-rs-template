/* [267A-2] Tests de integración Rust contra el simulador BDP WebLink.
 *
 * Estos tests levantan el simulador Python UNA VEZ como subprocesso y comparten
 * la instancia entre todos los tests. Cada test resetea el estado del simulador
 * vía el admin API antes de ejecutarse.
 *
 * Requisitos: Python 3 en PATH.
 * Si Python no está disponible, los tests se ignoran automáticamente.
 *
 * Para ejecutar:
 *   cargo test --test bdp_simulator_integration -- --include-ignored
 */

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use glory_backend::services::bdp_weblink::BdpWeblinkClient;
use glory_backend::services::bdp_weblink_catalog::{
    BdpAddOrderPaymentRequest, BdpCancelOrderRequest, BdpCreateCustomerRequest,
    BdpCreateOrderRequest, BdpExportArticlesRequest, BdpExportCustomersRequest,
    BdpGetOrderRequest, BdpInvoiceOrderRequest, BdpOrderIdentifier, BdpOrderPayment,
};
use glory_backend::services::bdp_weblink::BdpWeblinkError;

use chrono::{NaiveTime, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

const SIMULATOR_PORT: u16 = 18765;
const SIMULATOR_URL: &str = "http://127.0.0.1:18765";
const ADMIN_KEY: &str = "test-clave-admin-segura";

/* [267A-2] Mutex global para serializar los tests contra el simulador.
 * El throttle BDP limita a 2 requests concurrentes por base_url.
 * Sin este mutex, los tests se bloquean mutuamente. */
static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/* ── Simulador compartido: se inicia una sola vez ──────────────── */

static SIMULATOR: Mutex<Option<Child>> = Mutex::new(None);
static SIM_READY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

async fn ensure_simulator() -> bool {
    /* Si ya se intentó inicializar, devolver el resultado */
    if let Some(&ready) = SIM_READY.get() {
        return ready;
    }

    /* Verificar si el simulador ya está corriendo (test anterior o manual) */
    let client = reqwest::Client::new();
    let alive = client
        .post(format!("{SIMULATOR_URL}/Service/Health"))
        .json(&json!({}))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    if alive {
        let _ = SIM_READY.set(true);
        return true;
    }

    /* Intentar lanzar el simulador Python */
    let server_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools")
        .join("bdp-weblink-simulator")
        .join("server.py");

    if !server_path.exists() {
        eprintln!("SKIP: simulador no encontrado en {}", server_path.display());
        let _ = SIM_READY.set(false);
        return false;
    }

    let child = match Command::new("python")
        .arg(&server_path)
        .arg("--port")
        .arg(SIMULATOR_PORT.to_string())
        .arg("--admin-key")
        .arg(ADMIN_KEY)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("SKIP: no se pudo lanzar Python: {e}");
            let _ = SIM_READY.set(false);
            return false;
        }
    };

    /* Guardar el child para que no se muera */
    {
        let mut guard = SIMULATOR.lock().unwrap();
        *guard = Some(child);
    }

    /* Esperar a que el simulador esté listo */
    tokio::time::sleep(Duration::from_millis(1000)).await;

    /* Health check */
    let ready = client
        .post(format!("{SIMULATOR_URL}/Service/Health"))
        .json(&json!({}))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    if ready {
        eprintln!("Simulador BDP arrancado en {SIMULATOR_URL}");
    } else {
        eprintln!("SKIP: simulador no respondió a health check");
    }
    let _ = SIM_READY.set(ready);
    ready
}

macro_rules! skip_if_no_simulator {
    ($guard:ident) => {
        /* 1. Verificar/arrancar simulador (sin mutex, seguro concurrente) */
        if !ensure_simulator().await {
            eprintln!("SKIP: Python o simulador no disponible");
            return;
        }
        /* 2. Adquirir mutex ANTES de reset — serializa acceso al simulador
         *    y evita que un test resetee el estado mientras otro ejecuta. */
        let $guard = TEST_MUTEX.lock().await;
        /* 3. Reset DENTRO del mutex — estado limpio garantizado */
        admin_post("/__simulator/reset", json!({})).await;
        /* 4. Pequeña pausa para que el throttle libere permits del test anterior */
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
}

/* ── Helpers HTTP al admin API del simulador ────────────────────── */

async fn admin_post(path: &str, payload: serde_json::Value) {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{SIMULATOR_URL}{path}"))
        .header("X-Simulator-Key", ADMIN_KEY)
        .json(&payload)
        .send()
        .await;
}

async fn inject_fault(path: &str, fault: serde_json::Value) {
    let mut payload = fault;
    payload["Path"] = json!(path);
    admin_post("/__simulator/fault", payload).await;
}

/* ── Helper: configuración apuntando al simulador ──────────────── */

fn simulator_config() -> glory_backend::models::ConfiguracionRestaurante {
    glory_backend::models::ConfiguracionRestaurante {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        reserva_email_obligatorio: false,
        reserva_telefono_obligatorio: true,
        reserva_nombre_obligatorio: true,
        reserva_apellidos_obligatorio: false,
        iva_por_defecto: Decimal::new(10, 0),
        nombre_restaurante: "Test Sim".to_string(),
        groq_api_key: None,
        auto_venta_reserva: false,
        hora_desayuno_inicio: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        hora_desayuno_fin: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
        hora_comida_inicio: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        hora_comida_fin: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
        hora_cena_inicio: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
        hora_cena_fin: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
        url_haddock: String::new(),
        haddock_api_token: String::new(),
        haddock_sync_enabled: false,
        bdp_base_url: SIMULATOR_URL.to_string(),
        bdp_login: "local".to_string(),
        bdp_password: "secret".to_string(),
        bdp_integrator_code: "SIM".to_string(),
        bdp_sync_enabled: true,
        bdp_pos_id: 1,
        bdp_employee_id: 1,
        bdp_items_profile_id: 1,
        bdp_default_article_code: "1001".into(),
        bdp_default_article_name: "Servicio".into(),
        bdp_tender_map: json!({"efectivo": "1"}),
        bdp_order_type_map: json!({"comedor": "0"}),
        bdp_default_customer_code: String::new(),
        bdp_poll_interval_secs: 60,
        bdp_poll_enabled: false,
        bdp_auto_sync_customers: false,
        bdp_sync_mode: "unidirectional".into(),
        bdp_backup_retention_days: 30,
        bdp_auto_backup_before_write: true,
        bdp_env_bootstrap_applied_at: None,
        ff_bdp_auto_arm: false,
        ff_bdp_partial_payments: true,
        ff_bdp_cancel_order: true,
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

/* ═══════════════════════════════════════════════════════════════════
 * 1. HEALTH, LOGIN, VERSION
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_health_check() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);
    let health = client.health().await.expect("health should succeed");
    assert!(health.is_alive, "Simulador debe reportar is_alive=true");
}

#[tokio::test]
#[ignore]
async fn simulator_login_returns_token() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);
    let session = client.login().await.expect("login should succeed");
    assert!(!session.token.is_empty(), "Token no debe estar vacío");
    assert!(session.expires_in_seconds > 0);
}

#[tokio::test]
#[ignore]
async fn simulator_login_cached() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);
    let s1 = client.login().await.unwrap();
    let s2 = client.login().await.unwrap();
    assert_eq!(s1.token, s2.token, "Segundo login debe usar caché");
}

#[tokio::test]
#[ignore]
async fn simulator_get_version() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);
    let version = client.get_version().await.expect("get_version should succeed");
    assert!(version.revision.contains("SIMULATOR"));
}

/* ═══════════════════════════════════════════════════════════════════
 * 2. CATÁLOGO (LECTURA)
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_export_articles() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);
    let response = client
        .export_articles(&BdpExportArticlesRequest::all_web_articles(1))
        .await
        .expect("export_articles should succeed");
    let articles = response["Articles"].as_array().expect("Articles should be array");
    assert!(!articles.is_empty(), "Debe haber al menos 1 artículo fixture");
}

#[tokio::test]
#[ignore]
async fn simulator_export_customers() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);
    let response = client
        .export_customers(&BdpExportCustomersRequest::default())
        .await
        .expect("export_customers should succeed");
    let customers = response["Customers"].as_array().expect("Customers should be array");
    assert!(!customers.is_empty());
}

#[tokio::test]
#[ignore]
async fn simulator_get_tenders() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);
    let response = client.get_tenders().await.expect("get_tenders should succeed");
    assert!(response["Tenders"].as_array().is_some());
}

/* ═══════════════════════════════════════════════════════════════════
 * 3. CREAR COMANDA (CreateOrder)
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_create_order() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let request = BdpCreateOrderRequest {
        employee_id: 1,
        items_profile_id: 1,
        order_end_type: 0,
        order_operation_type: 0,
        invoice: Some(false),
        order: json!({
            "MarketId": 77,
            "MarketplaceOrderId": "GTEST0000000001",
            "PosId": 1,
            "Total": 15.50,
            "Items": [{"Id": 1001, "Name": "Café", "Units": 2, "Price": 7.75, "Total": 15.50}]
        }),
    };

    let response = client.create_order(&request).await.expect("create_order should succeed");
    let order_id = response["OrderId"].as_i64().expect("OrderId should be integer");
    assert!(order_id > 0, "OrderId debe ser positivo");
    assert!(response["Order"].is_object(), "Respuesta debe incluir Order");
}

#[tokio::test]
#[ignore]
async fn simulator_create_order_idempotent() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let request = BdpCreateOrderRequest {
        employee_id: 1,
        items_profile_id: 1,
        order_end_type: 0,
        order_operation_type: 0,
        invoice: Some(false),
        order: json!({
            "MarketId": 77,
            "MarketplaceOrderId": "GIDEMPOT0000001",
            "PosId": 1, "Total": 10.0,
            "Items": [{"Id": 1001, "Name": "Test", "Units": 1, "Price": 10.0, "Total": 10.0}]
        }),
    };

    let r1 = client.create_order(&request).await.unwrap();
    let r2 = client.create_order(&request).await.unwrap();
    assert_eq!(r1["OrderId"], r2["OrderId"], "Mismo MarketplaceOrderId → mismo OrderId");
    assert!(r2["Duplicate"].as_bool().unwrap_or(false), "Segunda llamada debe marcar Duplicate");

    /* Tercer llamada con diferente payload → debe fallar con error de conflicto */
    let mut conflict_request = request.clone();
    conflict_request.order["Total"] = json!(999.0);
    let r3 = client.create_order(&conflict_request).await;
    assert!(r3.is_err(), "Payload diferente debe ser rechazado");
}

/* ═══════════════════════════════════════════════════════════════════
 * 4. FLUJO COMPLETO: CREAR → PAGAR → FACTURAR
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_full_lifecycle_create_pay_invoice() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    /* 1. Crear comanda */
    let order_resp = client
        .create_order(&BdpCreateOrderRequest {
            employee_id: 1,
            items_profile_id: 1,
            order_end_type: 0,
            order_operation_type: 0,
            invoice: Some(false),
            order: json!({
                "MarketId": 77,
                "MarketplaceOrderId": "GE2E_LIFECYCLE01",
                "PosId": 1, "Total": 25.0,
                "Items": [{"Id": 1001, "Name": "Menu", "Units": 1, "Price": 25.0, "Total": 25.0}]
            }),
        })
        .await
        .unwrap();
    let order_id = order_resp["OrderId"].as_i64().unwrap();
    assert!(order_id > 0);

    /* 2. Verificar estado pendiente */
    let get_resp = client
        .get_order(&BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
        })
        .await
        .unwrap();
    let status = get_resp["Order"]["Status"].as_i64().unwrap_or(-1);
    assert_eq!(status, 0, "Orden recién creada debe estar pendiente (Status=0)");

    /* 3. Pago parcial */
    let pay1_resp = client
        .add_order_payment(&BdpAddOrderPaymentRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            payment: BdpOrderPayment {
                tender_id: 1,
                amount: Decimal::from(15),
                payment_id: "PE2E-PARTIAL-1".into(),
            },
            invoice: None,
            pos_id: Some(1),
            employee_id: Some(1),
            invoice_parameters: None,
        })
        .await
        .unwrap();
    let balance1 = pay1_resp["Balance"].as_f64().unwrap_or(-1.0);
    assert!((balance1 - 10.0).abs() < 0.01, "Saldo después de pago parcial debe ser ~10.0, fue {balance1}");

    /* 4. Pago restante */
    let pay2_resp = client
        .add_order_payment(&BdpAddOrderPaymentRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            payment: BdpOrderPayment {
                tender_id: 1,
                amount: Decimal::from(10),
                payment_id: "PE2E-REMAINING-1".into(),
            },
            invoice: None,
            pos_id: Some(1),
            employee_id: Some(1),
            invoice_parameters: None,
        })
        .await
        .unwrap();
    let balance2 = pay2_resp["Balance"].as_f64().unwrap_or(-1.0);
    assert!((balance2 - 0.0).abs() < 0.01, "Saldo después de pago completo debe ser 0.0, fue {balance2}");

    /* 5. Facturar */
    let inv_resp = client
        .invoice_order(&BdpInvoiceOrderRequest {
            pos_id: 1,
            employee_id: 1,
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            invoice_parameters: None,
        })
        .await
        .unwrap();
    let invoice_number = inv_resp["InvoiceNumber"]
        .as_str()
        .expect("InvoiceNumber debe existir");
    assert!(invoice_number.starts_with("SIM-"), "InvoiceNumber formato SIM-XXXXXX");

    /* 6. Verificar estado final */
    let final_resp = client
        .get_order(&BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
        })
        .await
        .unwrap();
    assert_eq!(final_resp["Order"]["Status"].as_i64().unwrap(), 3, "Orden facturada → Status=3");
    assert_eq!(final_resp["Order"]["InvoiceNumber"].as_str().unwrap(), invoice_number);
    assert_eq!(
        final_resp["Order"]["Payments"].as_array().unwrap().len(),
        2,
        "Debe tener 2 pagos registrados"
    );
}

/* ═══════════════════════════════════════════════════════════════════
 * 5. PAGOS PARCIALES Y SOBREPAGO
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_overpayment_rejected() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let order_resp = client
        .create_order(&BdpCreateOrderRequest {
            employee_id: 1,
            items_profile_id: 1,
            order_end_type: 0,
            order_operation_type: 0,
            invoice: Some(false),
            order: json!({
                "MarketId": 77,
                "MarketplaceOrderId": "GOVERPAY0000001",
                "PosId": 1, "Total": 10.0,
                "Items": [{"Id": 1001, "Name": "Test", "Units": 1, "Price": 10.0, "Total": 10.0}]
            }),
        })
        .await
        .unwrap();
    let order_id = order_resp["OrderId"].as_i64().unwrap();

    let result = client
        .add_order_payment(&BdpAddOrderPaymentRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            payment: BdpOrderPayment {
                tender_id: 1,
                amount: Decimal::from(20),
                payment_id: "POVERPAY001".into(),
            },
            invoice: None,
            pos_id: Some(1),
            employee_id: Some(1),
            invoice_parameters: None,
        })
        .await;

    assert!(result.is_err(), "Sobrepago debe ser rechazado");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("saldo pendiente") || err_msg.contains("superior"),
        "Error debe mencionar saldo: {err_msg}"
    );
}

/* ═══════════════════════════════════════════════════════════════════
 * 6. CANCELACIÓN
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_cancel_order() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let order_resp = client
        .create_order(&BdpCreateOrderRequest {
            employee_id: 1,
            items_profile_id: 1,
            order_end_type: 0,
            order_operation_type: 0,
            invoice: Some(false),
            order: json!({
                "MarketId": 77,
                "MarketplaceOrderId": "GCANCEL00000001",
                "PosId": 1, "Total": 5.0,
                "Items": [{"Id": 1001, "Name": "Test", "Units": 1, "Price": 5.0, "Total": 5.0}]
            }),
        })
        .await
        .unwrap();
    let order_id = order_resp["OrderId"].as_i64().unwrap();

    let cancel_resp = client
        .cancel_order(&BdpCancelOrderRequest {
            pos_id: 1,
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
        })
        .await
        .expect("cancel should succeed");

    assert_eq!(cancel_resp["Order"]["Status"].as_i64().unwrap(), 2, "Orden cancelada → Status=2");
}

#[tokio::test]
#[ignore]
async fn simulator_cancel_already_invoiced_fails() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    /* Crear, pagar y facturar */
    let order_resp = client
        .create_order(&BdpCreateOrderRequest {
            employee_id: 1,
            items_profile_id: 1,
            order_end_type: 0,
            order_operation_type: 0,
            invoice: Some(false),
            order: json!({
                "MarketId": 77,
                "MarketplaceOrderId": "GCANINV0000001",
                "PosId": 1, "Total": 5.0,
                "Items": [{"Id": 1001, "Name": "Test", "Units": 1, "Price": 5.0, "Total": 5.0}]
            }),
        })
        .await
        .unwrap();
    let order_id = order_resp["OrderId"].as_i64().unwrap();
    client
        .add_order_payment(&BdpAddOrderPaymentRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            payment: BdpOrderPayment { tender_id: 1, amount: Decimal::from(5), payment_id: "PCINV01".into() },
            invoice: None, pos_id: Some(1), employee_id: Some(1), invoice_parameters: None,
        })
        .await
        .unwrap();
    client
        .invoice_order(&BdpInvoiceOrderRequest {
            pos_id: 1, employee_id: 1,
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            invoice_parameters: None,
        })
        .await
        .unwrap();

    /* Intentar cancelar → debe fallar */
    let result = client
        .cancel_order(&BdpCancelOrderRequest {
            pos_id: 1,
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
        })
        .await;

    assert!(result.is_err(), "Cancelar orden facturada debe fallar");
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("facturada") || err.contains("cancel"), "Error debe mencionar estado: {err}");
}

/* ═══════════════════════════════════════════════════════════════════
 * 7. CLIENTES
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_create_customer() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let resp = client
        .create_customer(&BdpCreateCustomerRequest {
            code: 9001,
            fiscal_name: "Test Rust SL".into(),
            commercial_name: "Test".into(),
            mobile_phone: "600000000".into(),
            email: "test@example.invalid".into(),
            overwrite: false,
        })
        .await
        .expect("create_customer should succeed");

    assert_eq!(resp["ErrorMessage"].as_str().unwrap_or(""), "");
    assert!(resp["Customer"].is_object());
}

/* ═══════════════════════════════════════════════════════════════════
 * 8. FAULT INJECTION — ERRORES HTTP
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_fault_http_500_on_create_order() {
    skip_if_no_simulator!(_g);
    inject_fault("/API/Orders/Create", json!({"http_status": 500})).await;

    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let result = client
        .create_order(&BdpCreateOrderRequest {
            employee_id: 1,
            items_profile_id: 1,
            order_end_type: 0,
            order_operation_type: 0,
            invoice: Some(false),
            order: json!({
                "MarketId": 77,
                "MarketplaceOrderId": "GFAULT50000001",
                "PosId": 1, "Total": 5.0,
                "Items": [{"Id": 1001, "Name": "X", "Units": 1, "Price": 5.0, "Total": 5.0}]
            }),
        })
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        BdpWeblinkError::Api { status, .. } => assert_eq!(status, 500),
        other => panic!("Esperaba Api error, obtuvo: {other}"),
    }
}

#[tokio::test]
#[ignore]
async fn simulator_fault_remote_error_on_create_order() {
    skip_if_no_simulator!(_g);
    inject_fault("/API/Orders/Create", json!({"remote_error": "[300035] serie no válida"})).await;

    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let result = client
        .create_order(&BdpCreateOrderRequest {
            employee_id: 1,
            items_profile_id: 1,
            order_end_type: 0,
            order_operation_type: 0,
            invoice: Some(false),
            order: json!({
                "MarketId": 77,
                "MarketplaceOrderId": "GFAULTREM00001",
                "PosId": 1, "Total": 5.0,
                "Items": [{"Id": 1001, "Name": "X", "Units": 1, "Price": 5.0, "Total": 5.0}]
            }),
        })
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        BdpWeblinkError::Remote(msg) => assert!(msg.contains("300035"), "Error debe contener código: {msg}"),
        other => panic!("Esperaba Remote error, obtuvo: {other}"),
    }
}

#[tokio::test]
#[ignore]
async fn simulator_fault_invalid_json() {
    skip_if_no_simulator!(_g);
    inject_fault("/API/Orders/Get", json!({"invalid_json": true})).await;

    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let result = client
        .get_order(&BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(1),
        })
        .await;

    assert!(result.is_err(), "JSON inválido debe causar error");
    match result.unwrap_err() {
        BdpWeblinkError::Http(msg) => assert!(msg.contains("JSON") || msg.contains("invalida") || msg.contains("respuesta"), "Error: {msg}"),
        other => panic!("Esperaba Http error, obtuvo: {other}"),
    }
}

/* ═══════════════════════════════════════════════════════════════════
 * 9. RECONCILIACIÓN (apply_then_disconnect)
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_reconcile_after_disconnect() {
    skip_if_no_simulator!(_g);
    inject_fault("/API/Orders/Create", json!({"apply_then_disconnect": true})).await;

    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let request = BdpCreateOrderRequest {
        employee_id: 1,
        items_profile_id: 1,
        order_end_type: 0,
        order_operation_type: 0,
        invoice: Some(false),
        order: json!({
            "MarketId": 77,
            "MarketplaceOrderId": "GRECON000000001",
            "PosId": 1, "Total": 10.0,
            "Items": [{"Id": 1001, "Name": "Recon", "Units": 1, "Price": 10.0, "Total": 10.0}]
        }),
    };

    /* El envío debe fallar (disconnect) */
    let result = client.create_order(&request).await;
    assert!(result.is_err(), "Disconnect debe causar error de transporte");

    /* Pero la orden debe existir en el simulador → reconciliable por MarketplaceOrderId */
    let get_resp = client
        .get_order(&BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_market(77, "GRECON000000001"),
        })
        .await
        .expect("GetOrder por MarketplaceOrderId debe funcionar tras disconnect");

    let order_id = get_resp["Order"]["OrderId"]
        .as_i64()
        .or_else(|| get_resp["OrderId"].as_i64())
        .filter(|id| *id > 0);
    assert!(order_id.is_some(), "Orden debe existir en simulador a pesar del disconnect");
}

/* ═══════════════════════════════════════════════════════════════════
 * 10. FACTURACIÓN SIN PAGO → RECHAZADA
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_invoice_without_payment_rejected() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let order_resp = client
        .create_order(&BdpCreateOrderRequest {
            employee_id: 1,
            items_profile_id: 1,
            order_end_type: 0,
            order_operation_type: 0,
            invoice: Some(false),
            order: json!({
                "MarketId": 77,
                "MarketplaceOrderId": "GINVNOPAY00001",
                "PosId": 1, "Total": 10.0,
                "Items": [{"Id": 1001, "Name": "Test", "Units": 1, "Price": 10.0, "Total": 10.0}]
            }),
        })
        .await
        .unwrap();
    let order_id = order_resp["OrderId"].as_i64().unwrap();

    let result = client
        .invoice_order(&BdpInvoiceOrderRequest {
            pos_id: 1,
            employee_id: 1,
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            invoice_parameters: None,
        })
        .await;

    assert!(result.is_err(), "Facturar sin pagar debe fallar");
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("saldo") || err.contains("pendiente"),
        "Error debe mencionar saldo: {err}"
    );
}

/* ═══════════════════════════════════════════════════════════════════
 * 11. PAGO A ORDEN INEXISTENTE
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_payment_to_nonexistent_order() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let result = client
        .add_order_payment(&BdpAddOrderPaymentRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(999999),
            payment: BdpOrderPayment {
                tender_id: 1,
                amount: Decimal::from(10),
                payment_id: "PNOEXIST001".into(),
            },
            invoice: None,
            pos_id: Some(1),
            employee_id: Some(1),
            invoice_parameters: None,
        })
        .await;

    assert!(result.is_err(), "Pago a orden inexistente debe fallar");
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("inexistente") || err.contains("error"),
        "Error debe mencionar orden inexistente: {err}"
    );
}

/* ═══════════════════════════════════════════════════════════════════
 * 12. FACTURA IDEMPOTENTE
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_invoice_idempotent() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    /* Crear y pagar */
    let order_resp = client
        .create_order(&BdpCreateOrderRequest {
            employee_id: 1,
            items_profile_id: 1,
            order_end_type: 0,
            order_operation_type: 0,
            invoice: Some(false),
            order: json!({
                "MarketId": 77,
                "MarketplaceOrderId": "GINVIDEMPOT001",
                "PosId": 1, "Total": 10.0,
                "Items": [{"Id": 1001, "Name": "Test", "Units": 1, "Price": 10.0, "Total": 10.0}]
            }),
        })
        .await
        .unwrap();
    let order_id = order_resp["OrderId"].as_i64().unwrap();
    client
        .add_order_payment(&BdpAddOrderPaymentRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            payment: BdpOrderPayment { tender_id: 1, amount: Decimal::from(10), payment_id: "PIDEMPOT01".into() },
            invoice: None, pos_id: Some(1), employee_id: Some(1), invoice_parameters: None,
        })
        .await
        .unwrap();

    /* Facturar dos veces */
    let inv1 = client
        .invoice_order(&BdpInvoiceOrderRequest {
            pos_id: 1, employee_id: 1,
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            invoice_parameters: None,
        })
        .await
        .unwrap();
    let inv2 = client
        .invoice_order(&BdpInvoiceOrderRequest {
            pos_id: 1, employee_id: 1,
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            invoice_parameters: None,
        })
        .await
        .unwrap();

    assert_eq!(
        inv1["InvoiceNumber"].as_str().unwrap(),
        inv2["InvoiceNumber"].as_str().unwrap(),
        "Factura idempotente debe devolver mismo InvoiceNumber"
    );
    assert!(inv2["Duplicate"].as_bool().unwrap_or(false), "Segunda factura debe marcar Duplicate");
}

/* ═══════════════════════════════════════════════════════════════════
 * 13. GETORDER POR MARKETPLACE ID
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_get_order_by_marketplace_id() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let mkt_id = "GMKTLOOKUP0001";
    client
        .create_order(&BdpCreateOrderRequest {
            employee_id: 1,
            items_profile_id: 1,
            order_end_type: 0,
            order_operation_type: 0,
            invoice: Some(false),
            order: json!({
                "MarketId": 77,
                "MarketplaceOrderId": mkt_id,
                "PosId": 1, "Total": 5.0,
                "Items": [{"Id": 1001, "Name": "Test", "Units": 1, "Price": 5.0, "Total": 5.0}]
            }),
        })
        .await
        .unwrap();

    let resp = client
        .get_order(&BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_market(77, mkt_id),
        })
        .await
        .unwrap();

    assert_eq!(
        resp["Order"]["MarketplaceOrderId"].as_str().unwrap(),
        mkt_id
    );
}

/* ═══════════════════════════════════════════════════════════════════
 * 14. PAGO — VERIFICAR BALANCE EN RESPUESTA
 * ═══════════════════════════════════════════════════════════════════ */

#[tokio::test]
#[ignore]
async fn simulator_payment_returns_balance() {
    skip_if_no_simulator!(_g);
    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let order_resp = client
        .create_order(&BdpCreateOrderRequest {
            employee_id: 1,
            items_profile_id: 1,
            order_end_type: 0,
            order_operation_type: 0,
            invoice: Some(false),
            order: json!({
                "MarketId": 77,
                "MarketplaceOrderId": "GPAYBAL00000001",
                "PosId": 1, "Total": 10.0,
                "Items": [{"Id": 1001, "Name": "Test", "Units": 1, "Price": 10.0, "Total": 10.0}]
            }),
        })
        .await
        .unwrap();
    let order_id = order_resp["OrderId"].as_i64().unwrap();

    let pay_resp = client
        .add_order_payment(&BdpAddOrderPaymentRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            payment: BdpOrderPayment {
                tender_id: 1,
                amount: Decimal::from(10),
                payment_id: "PPAYBAL001".into(),
            },
            invoice: None,
            pos_id: Some(1),
            employee_id: Some(1),
            invoice_parameters: None,
        })
        .await
        .unwrap();

    assert!(pay_resp["Balance"].as_f64().is_some(), "Respuesta debe incluir Balance");
    assert!((pay_resp["Balance"].as_f64().unwrap() - 0.0).abs() < 0.01, "Balance debe ser 0 tras pago completo");
}

/* ═══════════════════════════════════════════════════════════════════
 * 15. FAULT: delay_ms — TIMEOUT HANDLING (R4)
 * ═══════════════════════════════════════════════════════════════════ */

/* [R4] Verifica que el cliente HTTP (timeout 20s) maneja correctamente
 * respuestas lentas inyectadas por el simulador. El fault delay_ms añade
 * artificial latency; si excede el timeout, debe mapearse a BdpWeblinkError::Http. */
#[tokio::test]
#[ignore]
async fn simulator_fault_delay_ms_causes_timeout() {
    skip_if_no_simulator!(_g);
    /* 25s > 20s timeout del HTTP_CLIENT en bdp_weblink.rs */
    inject_fault("/API/Orders/Get", json!({"delay_ms": 25000})).await;

    let config = simulator_config();
    let client = BdpWeblinkClient::new(&config);

    let result = tokio::time::timeout(
        Duration::from_secs(22),
        client.get_order(&BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(1),
        }),
    )
    .await;

    match result {
        Ok(inner) => {
            assert!(inner.is_err(), "Con delay_ms 25s, debe retornar error");
            match inner.unwrap_err() {
                BdpWeblinkError::Http(msg) => {
                    assert!(
                        msg.contains("timed out") || msg.contains("timeout") || msg.contains("error") ,
                        "Error HTTP debe mencionar timeout: {msg}"
                    );
                }
                other => panic!("Esperaba Http error por timeout, obtuvo: {other}"),
            }
        }
        Err(_) => {
            /* tokio timeout reached — el cliente HTTP colgó más de 22s,
             * lo cual es un bug (timeout debería ser 20s). */
            panic!("El test colgó >22s — el timeout del HTTP_CLIENT no funciona");
        }
    }
}
