/* [267A-5] Tests de integración de servicios BDP contra simulador + PostgreSQL.
 *
 * Estos tests verifican los servicios de negocio completos (sync_venta,
 * add_order_payment, invoice_order) no solo el cliente HTTP.
 *
 * Usa #[sqlx::test(migrations = "./migrations")] para BD aislada por test,
 * y el simulador Python compartido para las llamadas HTTP a BDP.
 *
 * Requisitos: PostgreSQL local + Python 3 con pytest.
 * Ejecutar: cargo test --test bdp_service_integration -- --include-ignored
 */

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use chrono::{NaiveTime, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use glory_backend::models::{ConfiguracionRestaurante, Venta};
use glory_backend::services::BdpSyncService;

const SIMULATOR_PORT: u16 = 18766;
const SIMULATOR_URL: &str = "http://127.0.0.1:18766";
const ADMIN_KEY: &str = "test-clave-admin-segura";

static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static SIMULATOR: Mutex<Option<Child>> = Mutex::new(None);
static SIM_READY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/* ═══════════════════════════════════════════════════════════════════
 * HELPERS: Simulador
 * ═══════════════════════════════════════════════════════════════════ */

async fn ensure_simulator() -> bool {
    if let Some(&ready) = SIM_READY.get() {
        return ready;
    }
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
    {
        let mut guard = SIMULATOR.lock().unwrap();
        *guard = Some(child);
    }
    tokio::time::sleep(Duration::from_millis(1000)).await;
    let ready = client
        .post(format!("{SIMULATOR_URL}/Service/Health"))
        .json(&json!({}))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    let _ = SIM_READY.set(ready);
    ready
}

macro_rules! skip_if_no_simulator {
    ($guard:ident) => {
        if !ensure_simulator().await {
            eprintln!("SKIP: Python o simulador no disponible");
            return;
        }
        let $guard = TEST_MUTEX.lock().await;
        admin_reset().await;
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
}

async fn admin_reset() {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{SIMULATOR_URL}/__simulator/reset"))
        .header("X-Simulator-Key", ADMIN_KEY)
        .json(&json!({}))
        .send()
        .await;
}

/* ═══════════════════════════════════════════════════════════════════
 * HELPERS: Base de datos
 * ═══════════════════════════════════════════════════════════════════ */

async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("svc-test-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("create_test_user failed");
    id
}

async fn seed_config(pool: &PgPool, user_id: Uuid) -> ConfiguracionRestaurante {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO configuracion_restaurante
          (id, user_id, bdp_base_url, bdp_login, bdp_password, bdp_integrator_code,
           bdp_sync_enabled, bdp_sync_mode, bdp_auto_backup_before_write,
           bdp_pos_id, bdp_employee_id, bdp_items_profile_id,
           bdp_default_article_code, bdp_default_article_name,
           bdp_tender_map, bdp_order_type_map, bdp_poll_interval_secs,
           iva_por_defecto, ff_bdp_auto_arm, ff_bdp_partial_payments,
           ff_bdp_cancel_order, ff_bdp_purchase_notes_read,
           ff_bdp_purchase_notes_draft, ff_bdp_purchase_notes_receive)
          VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24)"#,
    )
    .bind(id).bind(user_id).bind(SIMULATOR_URL).bind("local").bind("secret").bind("SIM")
    .bind(true).bind("unidirectional").bind(true)
    .bind(1_i32).bind(1_i32).bind(1_i32)
    .bind("1001").bind("Servicio")
    .bind(json!({"efectivo":"1","tarjeta":"2"})).bind(json!({"comedor":"0","barra":"0"}))
    .bind(60_i32).bind(Decimal::new(10, 0))
    .bind(true).bind(true).bind(false).bind(false).bind(false).bind(false)
    .execute(pool).await.expect("seed_config failed");

    ConfiguracionRestaurante {
        id, user_id,
        reserva_email_obligatorio: false, reserva_telefono_obligatorio: true,
        reserva_nombre_obligatorio: true, reserva_apellidos_obligatorio: false,
        iva_por_defecto: Decimal::new(10, 0), nombre_restaurante: "Test Svc".to_string(),
        groq_api_key: None, auto_venta_reserva: false,
        hora_desayuno_inicio: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        hora_desayuno_fin: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
        hora_comida_inicio: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        hora_comida_fin: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
        hora_cena_inicio: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
        hora_cena_fin: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
        url_haddock: String::new(), haddock_api_token: String::new(),
        haddock_sync_enabled: false,
        bdp_base_url: SIMULATOR_URL.to_string(), bdp_login: "local".to_string(),
        bdp_password: "secret".to_string(), bdp_integrator_code: "SIM".to_string(),
        bdp_sync_enabled: true, bdp_pos_id: 1, bdp_employee_id: 1, bdp_items_profile_id: 1,
        bdp_default_article_code: "1001".into(), bdp_default_article_name: "Servicio".into(),
        bdp_tender_map: json!({"efectivo": "1", "tarjeta": "2"}),
        bdp_order_type_map: json!({"comedor": "0", "barra": "0"}),
        bdp_default_customer_code: String::new(), bdp_poll_interval_secs: 60,
        bdp_poll_enabled: false, bdp_auto_sync_customers: false,
        bdp_sync_mode: "unidirectional".into(), bdp_backup_retention_days: 30,
        bdp_auto_backup_before_write: true, bdp_env_bootstrap_applied_at: None,
        ff_bdp_auto_arm: true, ff_bdp_partial_payments: true, ff_bdp_cancel_order: false,
        ff_bdp_purchase_notes_read: false, ff_bdp_purchase_notes_draft: false,
        ff_bdp_purchase_notes_receive: false,
        google_review_url: String::new(), telefono_restaurante: String::new(),
        url_reservas: String::new(), created_at: Utc::now(), updated_at: Utc::now(),
    }
}

async fn seed_venta(pool: &PgPool, user_id: Uuid) -> Venta {
    let id = Uuid::new_v4();
    let importe_base = Decimal::new(2500, 2);
    let importe_iva = Decimal::new(250, 2);
    sqlx::query(
        r#"INSERT INTO ventas
          (id, user_id, fecha, descripcion, iva_porcentaje, turno, canal,
           metodo_pago, importe_base, importe_iva)
          VALUES ($1, $2, CURRENT_DATE, 'Test venta BDP', 10, 'mediodia',
                  'comedor', 'efectivo', $3, $4)"#,
    )
    .bind(id).bind(user_id).bind(importe_base).bind(importe_iva)
    .execute(pool).await.expect("seed_venta failed");

    Venta {
        id, user_id, fecha: Utc::now().date_naive(), comensales: Some(2),
        descripcion: "Test venta BDP".to_string(), iva_porcentaje: Decimal::new(10, 0),
        turno: "mediodia".into(), canal: "comedor".into(), metodo_pago: "efectivo".into(),
        importe_base, importe_iva, reserva_id: None, cliente_id: None,
        created_at: Utc::now(), updated_at: Utc::now(),
        haddock_synced: false, haddock_synced_at: None, haddock_sync_error: None,
        bdp_synced: false, bdp_synced_at: None, bdp_sync_error: None,
        bdp_order_id: None, bdp_order_status: None, bdp_invoiced: false,
    }
}

/// Siembra un armado vigente en bdp_write_arming para que authorize() no bloquee.
/// authorize() requiere snapshot_id IS NOT NULL, así que creamos un snapshot dummy.
async fn seed_arming(pool: &PgPool, config: &ConfiguracionRestaurante, venta_id: Uuid, scope: &str) {
    let fingerprint = glory_backend::services::BdpBackupService::connection_fingerprint(config)
        .unwrap_or_else(|_| "test-fingerprint".to_string());
    let target = glory_backend::services::BdpBackupService::canonical_target(config)
        .unwrap_or_else(|_| SIMULATOR_URL.to_string());

    /* Crear snapshot dummy (FK constraint requiere que exista en bdp_snapshots) */
    let snapshot_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO bdp_snapshots
          (user_id, tipo, direccion, trigger_tipo, datos, target_base_url, connection_fingerprint)
          VALUES ($1, 'pre_write_order', 'bdp', 'test', '{}', $2, $3)
          RETURNING id"#,
    )
    .bind(config.user_id).bind(&target).bind(&fingerprint)
    .fetch_one(pool).await.expect("seed_snapshot failed");

    sqlx::query(
        r#"INSERT INTO bdp_write_arming
          (user_id, base_url, scopes, target_entity_type, target_entity_id,
           reason, expires_at, remaining_operations, snapshot_id, connection_fingerprint)
          VALUES ($1, $2, ARRAY[$3], 'venta', $4, 'test', NOW() + INTERVAL '5 minutes', 1, $6, $5)
          ON CONFLICT (user_id) DO UPDATE SET
            base_url = EXCLUDED.base_url, scopes = EXCLUDED.scopes,
            target_entity_type = EXCLUDED.target_entity_type,
            target_entity_id = EXCLUDED.target_entity_id,
            reason = EXCLUDED.reason, expires_at = EXCLUDED.expires_at,
            remaining_operations = EXCLUDED.remaining_operations,
            snapshot_id = EXCLUDED.snapshot_id,
            connection_fingerprint = EXCLUDED.connection_fingerprint,
            created_at = NOW()"#,
    )
    .bind(config.user_id).bind(&target).bind(scope).bind(venta_id).bind(&fingerprint).bind(snapshot_id)
    .execute(pool).await.expect("seed_arming failed");
}

/* ═══════════════════════════════════════════════════════════════════
 * 1. GUARD TESTS — rechazos sin llamar a BDP (sin simulador)
 * ═══════════════════════════════════════════════════════════════════ */

#[sqlx::test(migrations = "./migrations")]
async fn svc_sync_venta_read_only_does_nothing(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let mut config = seed_config(&pool, user_id).await;
    config.bdp_sync_mode = "read_only".into();
    let venta = seed_venta(&pool, user_id).await;
    BdpSyncService::sync_venta(&pool, &venta, &config, false, None).await;
    let (synced,): (bool,) = sqlx::query_as("SELECT bdp_synced FROM ventas WHERE id = $1")
        .bind(venta.id).fetch_one(&pool).await.unwrap();
    assert!(!synced);
}

#[sqlx::test(migrations = "./migrations")]
async fn svc_sync_venta_disabled_does_nothing(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let mut config = seed_config(&pool, user_id).await;
    config.bdp_sync_enabled = false;
    let venta = seed_venta(&pool, user_id).await;
    BdpSyncService::sync_venta(&pool, &venta, &config, false, None).await;
    let (synced,): (bool,) = sqlx::query_as("SELECT bdp_synced FROM ventas WHERE id = $1")
        .bind(venta.id).fetch_one(&pool).await.unwrap();
    assert!(!synced);
}

#[sqlx::test(migrations = "./migrations")]
async fn svc_add_payment_read_only_returns_error(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let mut config = seed_config(&pool, user_id).await;
    config.bdp_sync_mode = "read_only".into();
    let mut venta = seed_venta(&pool, user_id).await;
    venta.bdp_order_id = Some(12345);
    let result = BdpSyncService::add_order_payment(
        &pool, &venta, &config, Decimal::new(2500, 2), 1, None,
    ).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("solo lectura"));
}

#[sqlx::test(migrations = "./migrations")]
async fn svc_add_payment_no_order_id_returns_error(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let config = seed_config(&pool, user_id).await;
    let venta = seed_venta(&pool, user_id).await;
    let result = BdpSyncService::add_order_payment(
        &pool, &venta, &config, Decimal::new(2500, 2), 1, None,
    ).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("bdp_order_id"));
}

#[sqlx::test(migrations = "./migrations")]
async fn svc_add_payment_zero_amount_returns_error(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let config = seed_config(&pool, user_id).await;
    let mut venta = seed_venta(&pool, user_id).await;
    venta.bdp_order_id = Some(12345);
    let result = BdpSyncService::add_order_payment(
        &pool, &venta, &config, Decimal::ZERO, 1, None,
    ).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("importe"));
}

#[sqlx::test(migrations = "./migrations")]
async fn svc_invoice_read_only_returns_error(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let mut config = seed_config(&pool, user_id).await;
    config.bdp_sync_mode = "read_only".into();
    let mut venta = seed_venta(&pool, user_id).await;
    venta.bdp_order_id = Some(12345);
    let result = BdpSyncService::invoice_order(&pool, &venta, &config, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("solo lectura"));
}

#[sqlx::test(migrations = "./migrations")]
async fn svc_invoice_no_order_id_returns_error(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let config = seed_config(&pool, user_id).await;
    let venta = seed_venta(&pool, user_id).await;
    let result = BdpSyncService::invoice_order(&pool, &venta, &config, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("bdp_order_id"));
}

/* ═══════════════════════════════════════════════════════════════════
 * 2. HELPER FUNCTION TESTS
 * ═══════════════════════════════════════════════════════════════════ */

#[test]
fn svc_marketplace_order_id_format() {
    let venta_id = Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap();
    let mkt_id = BdpSyncService::marketplace_order_id(venta_id);
    assert!(mkt_id.starts_with('G'));
    assert!(mkt_id.len() <= 15, "MarketplaceOrderId max 15 chars: {mkt_id}");
}

/* ═══════════════════════════════════════════════════════════════════
 * 3. E2E TESTS — servicio completo contra simulador + PostgreSQL
 * ═══════════════════════════════════════════════════════════════════ */

/// Helper HTTP: crea una comanda en el simulador y devuelve order_id.
async fn simulator_create_order(token: &str) -> i64 {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{SIMULATOR_URL}/API/Orders/Create"))
        .bearer_auth(token)
        .json(&json!({
            "EmployeeId": 1, "ItemsProfileId": 1,
            "OrderEndType": 0, "OrderOperationType": 0, "Invoice": false,
            "Order": {
                "MarketId": 9900, "MarketplaceOrderId": format!("GSVC{}", Uuid::new_v4().simple().to_string()[..10].to_string()),
                "PosId": 1, "Total": 27.50,
                "Items": [{"Id": 1001, "Name": "Test", "Units": 1, "Price": 27.50, "Total": 27.50}]
            }
        }))
        .send().await.unwrap().json().await.unwrap();
    resp["OrderId"].as_i64().expect("OrderId missing")
}

async fn simulator_login() -> String {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{SIMULATOR_URL}/Auth/Login"))
        .json(&json!({"Login":"local","Password":"secret","TiempoSession":59,"CodigoIntegrador":"SIM"}))
        .send().await.unwrap().json().await.unwrap();
    resp["AuthSession"]["Token"].as_str().unwrap().to_string()
}

#[sqlx::test(migrations = "./migrations")]
#[ignore]
async fn svc_sync_venta_creates_order_and_updates_db(pool: PgPool) {
    skip_if_no_simulator!(_g);
    let user_id = create_test_user(&pool).await;
    let config = seed_config(&pool, user_id).await;
    let venta = seed_venta(&pool, user_id).await;

    /* Sembrar armado para que authorize() no bloquee */
    seed_arming(&pool, &config, venta.id, "create_order").await;

    BdpSyncService::sync_venta(&pool, &venta, &config, false, None).await;

    let (synced, order_id, sync_error): (bool, Option<i64>, Option<String>) =
        sqlx::query_as("SELECT bdp_synced, bdp_order_id, bdp_sync_error FROM ventas WHERE id = $1")
            .bind(venta.id).fetch_one(&pool).await.unwrap();

    assert!(synced, "Venta debe estar sincronizada. Error: {sync_error:?}");
    assert!(order_id.is_some(), "bdp_order_id debe estar seteado");
    assert!(order_id.unwrap() > 0, "order_id debe ser positivo");
}

#[sqlx::test(migrations = "./migrations")]
#[ignore]
async fn svc_add_payment_full_flow(pool: PgPool) {
    skip_if_no_simulator!(_g);
    let user_id = create_test_user(&pool).await;
    let config = seed_config(&pool, user_id).await;

    let token = simulator_login().await;
    let order_id = simulator_create_order(&token).await;

    let venta_id = Uuid::new_v4();
    let importe_base = Decimal::new(2500, 2);
    let importe_iva = Decimal::new(250, 2);
    sqlx::query(
        r#"INSERT INTO ventas (id, user_id, fecha, descripcion, iva_porcentaje, turno, canal,
           metodo_pago, importe_base, importe_iva, bdp_synced, bdp_order_id)
          VALUES ($1, $2, CURRENT_DATE, 'Test pago', 10, 'mediodia',
                  'comedor', 'efectivo', $3, $4, true, $5)"#,
    )
    .bind(venta_id).bind(user_id).bind(importe_base).bind(importe_iva).bind(order_id)
    .execute(&pool).await.unwrap();

    let venta = Venta {
        id: venta_id, user_id, fecha: Utc::now().date_naive(), comensales: Some(2),
        descripcion: "Test pago".to_string(), iva_porcentaje: Decimal::new(10, 0),
        turno: "mediodia".into(), canal: "comedor".into(), metodo_pago: "efectivo".into(),
        importe_base, importe_iva, reserva_id: None, cliente_id: None,
        created_at: Utc::now(), updated_at: Utc::now(),
        haddock_synced: false, haddock_synced_at: None, haddock_sync_error: None,
        bdp_synced: true, bdp_synced_at: Some(Utc::now()), bdp_sync_error: None,
        bdp_order_id: Some(order_id), bdp_order_status: None, bdp_invoiced: false,
    };

    seed_arming(&pool, &config, venta_id, "add_payment").await;

    let result = BdpSyncService::add_order_payment(
        &pool, &venta, &config, Decimal::new(2750, 2), 1, Some("svc-test-pay-001"),
    ).await;
    assert!(result.is_ok(), "Pago debe ser exitoso: {:?}", result.err());

    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM bdp_pagos WHERE venta_id = $1 AND resultado = 'exito'",
    )
    .bind(venta_id).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1, "Debe haber 1 pago en el ledger");
}

#[sqlx::test(migrations = "./migrations")]
#[ignore]
async fn svc_invoice_order_full_flow(pool: PgPool) {
    skip_if_no_simulator!(_g);
    let user_id = create_test_user(&pool).await;
    let config = seed_config(&pool, user_id).await;

    let token = simulator_login().await;
    let order_id = simulator_create_order(&token).await;

    /* Pagar la orden directamente en simulador */
    let client = reqwest::Client::new();
    let pay_resp: serde_json::Value = client
        .post(format!("{SIMULATOR_URL}/API/Orders/Payment/Add"))
        .bearer_auth(&token)
        .json(&json!({
            "OrderIdentifier": {"OrderId": order_id},
            "Payment": {"TenderId": 1, "Amount": 27.50, "PaymentId": "PSVCINV01"}
        }))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(pay_resp["ErrorMessage"].as_str().unwrap_or(""), "");

    let venta_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO ventas (id, user_id, fecha, descripcion, iva_porcentaje, turno, canal,
           metodo_pago, importe_base, importe_iva, bdp_synced, bdp_order_id)
          VALUES ($1, $2, CURRENT_DATE, 'Test factura', 10, 'noche',
                  'comedor', 'tarjeta', 25.00, 2.50, true, $3)"#,
    )
    .bind(venta_id).bind(user_id).bind(order_id)
    .execute(&pool).await.unwrap();

    /* Registrar pago en ledger local */
    sqlx::query(
        r#"INSERT INTO bdp_pagos (venta_id, amount, tender_id, idempotency_key, bdp_order_id, resultado)
          VALUES ($1, 27.50, 1, 'svc-inv-pay-001', $2, 'exito')"#,
    )
    .bind(venta_id).bind(order_id)
    .execute(&pool).await.unwrap();

    let venta = Venta {
        id: venta_id, user_id, fecha: Utc::now().date_naive(), comensales: Some(1),
        descripcion: "Test factura".to_string(), iva_porcentaje: Decimal::new(10, 0),
        turno: "noche".into(), canal: "comedor".into(), metodo_pago: "tarjeta".into(),
        importe_base: Decimal::new(2500, 2), importe_iva: Decimal::new(250, 2),
        reserva_id: None, cliente_id: None,
        created_at: Utc::now(), updated_at: Utc::now(),
        haddock_synced: false, haddock_synced_at: None, haddock_sync_error: None,
        bdp_synced: true, bdp_synced_at: Some(Utc::now()), bdp_sync_error: None,
        bdp_order_id: Some(order_id), bdp_order_status: None, bdp_invoiced: false,
    };

    seed_arming(&pool, &config, venta_id, "invoice").await;

    let result = BdpSyncService::invoice_order(&pool, &venta, &config, Some("svc-test-inv-001")).await;
    assert!(result.is_ok(), "Facturación debe ser exitosa: {:?}", result.err());
    let invoice_number = result.unwrap();
    assert!(invoice_number.starts_with("SIM-"), "InvoiceNumber formato SIM-: {invoice_number}");

    let (invoiced, order_status): (bool, Option<String>) =
        sqlx::query_as("SELECT bdp_invoiced, bdp_order_status FROM ventas WHERE id = $1")
            .bind(venta_id).fetch_one(&pool).await.unwrap();
    assert!(invoiced, "Venta debe estar facturada");
    assert_eq!(order_status.as_deref(), Some("invoiced"));
}
