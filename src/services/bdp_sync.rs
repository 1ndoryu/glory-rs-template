/* [065A-5] Servicio de sincronización Glory → BDP WebLink REST API.
 * Crea comandas reales en el TPV cuando se registra una venta en Glory.
 * Usa exclusión local/distribuida y una única escritura con reconciliación;
 * nunca reintenta CreateOrder a ciegas.
 *
 * Flujo: VentaService::create/update → spawn_bdp_sync → BdpSyncService::sync_venta
 *   1. Login a BDP WebLink
 *   2. Construir Order (Type=0 Barra, OrderEndType=1 pendiente)
 *   3. CreateOrder (OperationType=0 escritura real)
 *   4. Actualizar bdp_synced / bdp_order_id en la venta
 *
 * Mapeo Glory → BDP: usa bdp_default_article_code configurado.
 * Glory ventas son monolíticas (1 descripción + total), BDP requiere líneas de artículos.
 * Por defecto, toda venta se envía como 1 artículo genérico configurable.
 *
 * Gotchas documentados:
 * - Type=0 (Barra) es el único que pasa validación en POS 31. Type=1 falla 300008, Type=2 falla 300009.
 * - OrderEndType=1 crea comanda pendiente (no facturada, no impresa). El TPV la muestra en autocomanda.
 * - MarketplaceOrderId max 15 chars (error 301011).
 * - AlreadyInvoiced e Invoice son campos REQUERIDOS dentro de Order.
 * - CancelOrder devuelve "Subscripción no activada" — no se puede cancelar vía API.
 * - Serie 00031TI (IVA incluido) configurada en POS 31 desde 2026-06-07. */

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex as StdMutex};

use chrono::Utc;
use rust_decimal::prelude::{Decimal, FromPrimitive};
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::sync::Mutex as TokioMutex;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::{ConfiguracionRestaurante, Venta, VentaLinea};
use crate::repositories::{
    BdpArticleMapRepository, BdpPagoRepository, ClienteRepository, VentaLineaRepository,
    VentaRepository,
};
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{
    BdpAddOrderPaymentRequest, BdpCatalogSyncResult, BdpCreateOrderRequest, BdpGetArticleRequest,
    BdpGetOrderRequest, BdpGetPosArticlesRequest, BdpGetPricesArticlesRequest,
    BdpGetPricesArticlesResponse, BdpGetRoomsTablesRequest, BdpGetRoomsTablesResponse,
    BdpInvoiceOrderRequest, BdpOrderIdentifier, BdpOrderPayment,
};

pub(crate) const BDP_SYNC_MARKET_ID: i32 = 9_900;
/* [247A-9] Tolerancia para comparaciones de saldo pendiente en pagos BDP. */
const BDP_PAYMENT_TOLERANCE: f64 = 0.005;

/* [F3.1] Contexto resuelto para construir el pedido BDP.
 * Se resuelve en sync_venta() y se pasa a build_order() para no hacer
 * lookups dentro de la función de construcción del payload. */
struct OrderContext {
    tender_id: Option<i32>,
    order_type: i32,
    customer_code: Option<String>,
    customer_name: Option<String>,
    customer_phone: Option<String>,
}

static SYNC_LOCKS: LazyLock<StdMutex<HashMap<uuid::Uuid, Arc<TokioMutex<()>>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

pub struct BdpSyncService;

/* [R14] Guard RAII que limpia el lock de SYNC_LOCKS al salir del scope.
 * Evita llamadas manuales a cleanup_lock en cada camino de retorno. */
struct SyncLockGuard {
    venta_id: Uuid,
}

impl SyncLockGuard {
    fn new(venta_id: Uuid) -> Self {
        Self { venta_id }
    }
}

impl Drop for SyncLockGuard {
    fn drop(&mut self) {
        BdpSyncService::cleanup_lock(self.venta_id);
    }
}

impl BdpSyncService {
    /// Orquesta el flujo completo Glory → BDP para una venta.
    /// `idempotency_key` se guarda en el audit log si se proporciona (C1).
    #[allow(clippy::too_many_lines)]
    pub async fn sync_venta(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        is_update: bool,
        idempotency_key: Option<&str>,
    ) {
        if !config.bdp_sync_enabled || !crate::services::bdp_sync_preflight::bdp_configurado(config)
        {
            return;
        }

        /* [F3] Gate: en modo read_only, no enviar ventas a BDP */
        if config.bdp_sync_mode != "unidirectional" {
            info!(
                "[F3] BDP en modo read_only — sync_venta omitida para venta {}",
                venta.id
            );
            return;
        }
        if !config.bdp_auto_backup_before_write {
            warn!(
                "[F2] Escritura BDP bloqueada para venta {}: auto-backup desactivado",
                venta.id
            );
            return;
        }

        let lock = {
            let mut map = SYNC_LOCKS
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            map.entry(venta.id)
                .or_insert_with(|| Arc::new(TokioMutex::new(())))
                .clone()
        };
        let Ok(_guard) = lock.try_lock() else {
            info!(
                "[065A-5] BDP sync ya en progreso para venta {}, saltando",
                venta.id
            );
            return;
        };
        let _sync_lock_guard = SyncLockGuard::new(venta.id);

        /* Guard: si ya sincronizada y es create (no update), saltar */
        if !is_update {
            match VentaRepository::find_by_id(pool, venta.id, venta.user_id).await {
                Ok(Some(fresh)) if fresh.bdp_synced => {
                    info!(
                        "[065A-5] Venta {} ya sincronizada con BDP, saltando",
                        venta.id
                    );
                    Self::cleanup_lock(venta.id);
                    return;
                }
                Ok(None) => {
                    warn!("[065A-5] Venta {} no encontrada en BD", venta.id);
                    Self::cleanup_lock(venta.id);
                    return;
                }
                Err(e) => {
                    warn!("[065A-5] Error leyendo venta {} para guard: {e}", venta.id);
                }
                _ => {}
            }
        }

        /* Lock distribuido: el mutex anterior solo protege este proceso. La
         * transacción mantiene un advisory lock durante toda la operación para
         * impedir que otra instancia envíe la misma venta simultáneamente. */
        let mut distributed_lock = match pool.begin().await {
            Ok(transaction) => transaction,
            Err(error) => {
                warn!(
                    "[BDP-SAFE] No se pudo iniciar lock distribuido para venta {}: {error}",
                    venta.id
                );
                Self::cleanup_lock(venta.id);
                return;
            }
        };
        let acquired = sqlx::query_scalar::<_, bool>(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))",
        )
        .bind(format!("bdp-order:{}", venta.id))
        .fetch_one(&mut *distributed_lock)
        .await
        .unwrap_or(false);
        if !acquired {
            info!(
                "[BDP-SAFE] Otra instancia procesa la venta {}; escritura omitida",
                venta.id
            );
            Self::cleanup_lock(venta.id);
            return;
        }

        /* [R2] Liberar la conexión de Postgres inmediatamente: el lock
         * transaccional ya cumplió su función de evitar que otra instancia
         * adquiera el mismo advisory lock en este instante. Mantener la tx
         * abierta durante las llamadas HTTP a BDP retendría una conexión del
         * pool innecesariamente y podría agotarlo bajo carga. La exclusión
         * dentro de este proceso sigue garantizada por SYNC_LOCKS; el lock
         * distribuido solo actúa como cortina de humo inicial. */
        if let Err(error) = distributed_lock.commit().await {
            warn!(
                "[BDP-SAFE] No se pudo liberar el lock distribuido para venta {}: {error}",
                venta.id
            );
            Self::cleanup_lock(venta.id);
            return;
        }

        if let Err(error) = crate::services::BdpWriteGuard::ensure_no_unresolved(
            pool,
            venta.user_id,
            "venta_id",
            venta.id,
            &["create_order", "update_order"],
        )
        .await
        {
            warn!("[BDP-SAFE] {error}");
            Self::cleanup_lock(venta.id);
            return;
        }

        /* [187A-1] Preparación fail-closed: la autorización, la intención y el
         * retorno a solo lectura se confirman atómicamente antes del HTTP. */
        let operacion = "create_order";
        let datos_enviados = serde_json::json!({
            "venta_id": venta.id,
            "importe_base": venta.importe_base,
            "is_update": is_update
        });
        let snapshot_pre_id = match crate::services::BdpBackupService::preparar_snapshot_escritura(
            pool,
            venta.user_id,
            operacion,
            config,
            None,
        )
        .await
        {
            Ok(id) => id,
            Err(e) => {
                let msg = format!("Pre-write audit BDP falló; escritura bloqueada: {e}");
                warn!("[F2] {msg} (venta {})", venta.id);
                let _ = VentaRepository::update_bdp_status(pool, venta.id, false, Some(&msg), None)
                    .await;
                Self::cleanup_lock(venta.id);
                return;
            }
        };

        let audit_id = match crate::services::BdpWriteGuard::authorize(
            pool,
            venta.user_id,
            config,
            "create_order",
            "venta",
            venta.id,
            "venta_id",
            &datos_enviados,
            snapshot_pre_id,
            idempotency_key,
        )
        .await
        {
            Ok(id) => id,
            Err(error) => {
                warn!("[BDP-SAFE] {error}");
                Self::cleanup_lock(venta.id);
                return;
            }
        };

        /* [R5] Timeout global de 45s para la fase HTTP a BDP. */
        let http_result = timeout(
            Duration::from_secs(45),
            Self::run_http_phase(pool, venta, config),
        )
        .await;

        match http_result {
            Ok(Ok(order_id)) => {
                info!(
                    "[065A-5] Venta {} sincronizada con BDP → OrderId={order_id}",
                    venta.id
                );
                /* [AUDIT-2.11] Envolver marca local + auditoría en transacción
                 * atómica para que, si el proceso muere después del HTTP, no
                 * quede bdp_synced=true sin auditoría cerrada ni viceversa. */
                let respuesta = serde_json::json!({"order_id": order_id});
                let commit_result = async {
                    let mut tx = pool
                        .begin()
                        .await
                        .map_err(|e| format!("Error iniciando tx post-create_order: {e}"))?;

                    sqlx::query(
                        "UPDATE ventas SET bdp_synced = true, bdp_synced_at = NOW(), bdp_order_id = $2, bdp_sync_error = NULL WHERE id = $1",
                    )
                    .bind(venta.id)
                    .bind(order_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!(
                        "BDP confirmó OrderId={order_id}, pero no se pudo persistir localmente: {e}"
                    ))?;

                    sqlx::query(
                        r"UPDATE bdp_audit_log
                        SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
                        WHERE id = $1",
                    )
                    .bind(audit_id)
                    .bind(Some(&respuesta))
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Venta confirmada, pero falló el cierre de auditoría: {e}"))?;

                    tx.commit()
                        .await
                        .map_err(|e| format!("Error confirmando tx post-create_order: {e}"))
                }
                .await;

                if let Err(e) = commit_result {
                    /* La tx falló pero BDP ya creó la comanda → auditoría ambigua. */
                    warn!("[BDP-SAFE] {e} (venta {})", venta.id);
                    let _ = crate::services::BdpBackupService::actualizar_resultado(
                        pool,
                        audit_id,
                        "ambiguo",
                        Some(&respuesta),
                        Some(&e),
                    )
                    .await;
                }
            }
            Ok(Err(OrderSendFailure::Rejected(msg))) => {
                let safe_msg = Self::sanitize_error(&msg);
                if let Err(error) = crate::services::BdpBackupService::actualizar_resultado(
                    pool,
                    audit_id,
                    "error",
                    None,
                    Some(&msg),
                )
                .await
                {
                    warn!(
                        "[BDP-SAFE] No se pudo cerrar auditoría fallida de venta {}: {error}",
                        venta.id
                    );
                }
                warn!(
                    "[BDP-SAFE] Escritura BDP error para venta {}: {safe_msg}; no se reintenta a ciegas",
                    venta.id
                );
                if let Err(e) =
                    VentaRepository::update_bdp_status(pool, venta.id, false, Some(&safe_msg), None)
                        .await
                {
                    warn!(
                        "[065A-5] Error guardando error BDP de venta {}: {e}",
                        venta.id
                    );
                }
            }
            Ok(Err(OrderSendFailure::Ambiguous(msg))) => {
                if let Err(error) = crate::services::BdpBackupService::actualizar_resultado(
                    pool,
                    audit_id,
                    "ambiguo",
                    None,
                    Some(&msg),
                )
                .await
                {
                    warn!(
                        "[BDP-SAFE] No se pudo cerrar auditoría ambigua de venta {}: {error}",
                        venta.id
                    );
                }
                warn!(
                    "[BDP-SAFE] Escritura BDP ambigua para venta {}: {msg}; no se reintenta a ciegas",
                    venta.id
                );
                if let Err(e) =
                    VentaRepository::update_bdp_status(pool, venta.id, false, Some(&msg), None)
                        .await
                {
                    warn!(
                        "[065A-5] Error guardando error BDP de venta {}: {e}",
                        venta.id
                    );
                }
            }
            Err(_) => {
                let msg = "Timeout esperando respuesta de BDP (45s)".to_string();
                warn!("[BDP-SAFE] {msg} (venta {})", venta.id);
                let _ = crate::services::BdpBackupService::actualizar_resultado(
                    pool,
                    audit_id,
                    "ambiguo",
                    None,
                    Some(&msg),
                )
                .await;
                if let Err(e) =
                    VentaRepository::update_bdp_status(pool, venta.id, false, Some(&msg), None)
                        .await
                {
                    warn!(
                        "[065A-5] Error guardando timeout BDP de venta {}: {e}",
                        venta.id
                    );
                }
            }
        }
    }

    /// Envía una sola vez. Ante un fallo de transporte intenta reconciliar por
    /// `MarketplaceOrderId`; nunca repite `CreateOrder` a ciegas.
    async fn retry_send_order(
        client: &BdpWeblinkClient<'_>,
        config: &ConfiguracionRestaurante,
        venta: &Venta,
        article: &ResolvedArticle,
        lineas: Option<&[VentaLinea]>,
        line_article_ids: Option<&[i64]>,
        order_ctx: &OrderContext,
    ) -> Result<i64, OrderSendFailure> {
        match Self::send_order(
            client,
            config,
            venta,
            article,
            lineas,
            line_article_ids,
            order_ctx,
        )
        .await
        {
            Ok(order_id) => Ok(order_id),
            Err(BdpSyncError::Rejected(msg)) => Err(OrderSendFailure::Rejected(msg)),
            Err(BdpSyncError::AmbiguousTransport(msg)) => {
                let marketplace_id = Self::marketplace_order_id(venta.id);
                let request = BdpGetOrderRequest {
                    order_identifier: BdpOrderIdentifier::by_market(
                        BDP_SYNC_MARKET_ID,
                        marketplace_id.clone(),
                    ),
                };
                match client.get_order(&request).await {
                    Ok(response) => {
                        let order_id = response
                            .get("OrderId")
                            .and_then(Value::as_i64)
                            .or_else(|| response.get("Order")?.get("OrderId")?.as_i64());
                        order_id.filter(|id| *id > 0).ok_or_else(|| {
                            OrderSendFailure::Ambiguous(format!(
                                "{msg}; reconciliación sin OrderId para {marketplace_id}"
                            ))
                        })
                    }
                    Err(error) => Err(OrderSendFailure::Ambiguous(format!(
                        "{msg}; reconciliación falló para {marketplace_id}: {error}"
                    ))),
                }
            }
        }
    }

    /// Construye y envía una comanda a BDP para la venta dada.
    async fn send_order(
        client: &BdpWeblinkClient<'_>,
        config: &ConfiguracionRestaurante,
        venta: &Venta,
        article: &ResolvedArticle,
        lineas: Option<&[VentaLinea]>,
        line_article_ids: Option<&[i64]>,
        order_ctx: &OrderContext,
    ) -> Result<i64, BdpSyncError> {
        let order = Self::build_order(config, venta, article, lineas, line_article_ids, order_ctx);
        let response = client
            .create_order(&order)
            .await
            .map_err(|error| match error {
                crate::services::bdp_weblink::BdpWeblinkError::Remote(message) => {
                    BdpSyncError::Rejected(format!("BDP: {message}"))
                }
                crate::services::bdp_weblink::BdpWeblinkError::NotConfigured => {
                    BdpSyncError::Rejected("BDP no está configurado".to_string())
                }
                crate::services::bdp_weblink::BdpWeblinkError::InvalidBaseUrl(url) => {
                    BdpSyncError::Rejected(format!("URL BDP inválida: {url}"))
                }
                crate::services::bdp_weblink::BdpWeblinkError::WriteTargetDenied(url) => {
                    BdpSyncError::Rejected(format!("destino de escritura BDP no autorizado: {url}"))
                }
                crate::services::bdp_weblink::BdpWeblinkError::Http(message) => {
                    BdpSyncError::AmbiguousTransport(format!("error de transporte BDP: {message}"))
                }
                crate::services::bdp_weblink::BdpWeblinkError::Api { status, body } => {
                    BdpSyncError::AmbiguousTransport(format!("BDP respondió HTTP {status}: {body}"))
                }
                crate::services::bdp_weblink::BdpWeblinkError::Throttled(message) => {
                    /* [R3] Throttling es un rechazo temporal del TPV local; si lo
                     * tratamos como permanente perdemos comandas. Lo marcamos
                     * ambiguo para que el operador/operación de reconciliación
                     * lo vuelva a intentar. */
                    BdpSyncError::AmbiguousTransport(format!("BDP throttled: {message}"))
                }
            })?;

        /* Extraer OrderId y ErrorMessage de la respuesta */
        let order_id = response.get("OrderId").and_then(Value::as_i64).unwrap_or(0);
        let error_msg = response
            .get("ErrorMessage")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        if error_msg.is_empty() && order_id > 0 {
            Ok(order_id)
        } else if error_msg.is_empty() {
            Err(BdpSyncError::Rejected(
                "BDP devolvió OrderId=0 sin error".to_string(),
            ))
        } else {
            Err(BdpSyncError::Rejected(format!("BDP: {error_msg}")))
        }
    }

    pub fn marketplace_order_id(venta_id: Uuid) -> String {
        let venta_hex = venta_id.simple().to_string();
        format!("G{}", &venta_hex[..14])
    }

    /// Construye el payload BDP `CreateOrder` desde una venta Glory.
    /// [F2.7] Si hay líneas, genera un pedido multi-item. Si no, usa fallback legacy (1 artículo).
    /// `line_article_ids`: paralelo a `lineas`, con el ID BDP de cada artículo resuelto.
    /// [F3.1] `order_ctx`: tender, order type y customer resueltos.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::too_many_lines
    )]
    fn build_order(
        config: &ConfiguracionRestaurante,
        venta: &Venta,
        article: &ResolvedArticle,
        lineas: Option<&[VentaLinea]>,
        line_article_ids: Option<&[i64]>,
        order_ctx: &OrderContext,
    ) -> BdpCreateOrderRequest {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        /* MarketplaceOrderId: estable por venta y max 15 chars.
         * Debe mantenerse idéntico entre reintentos para que BDP pueda deduplicar
         * una escritura que sí se aplicó pero cuya respuesta se perdió. */
        let marketplace_order_id = Self::marketplace_order_id(venta.id);

        let total =
            Self::decimal_to_f64(&venta.importe_base) + Self::decimal_to_f64(&venta.importe_iva);

        /* [F2.7] Construir Items array — multi-item si hay líneas, fallback si no */
        let items: Vec<Value> = if let Some(lineas) = lineas {
            lineas
                .iter()
                .enumerate()
                .map(|(i, linea)| {
                    let precio = Self::decimal_to_f64(&linea.precio_unitario);
                    let cantidad = Self::decimal_to_f64(&linea.cantidad);
                    let descuento = Self::decimal_to_f64(&linea.descuento);
                    /* [AUDIT-2.3] Validar que precio y cantidad sean positivos. */
                    if precio < 0.0 || cantidad <= 0.0 {
                        warn!(
                            "[BDP-SAFE] Línea '{}' tiene precio={} o cantidad={} inválidos; se envía a BDP tal cual",
                            linea.descripcion, precio, cantidad
                        );
                    }
                    let linea_total = (precio * cantidad) - descuento;
                    /* [F2.8] Usar artículo BDP mapeado por línea, o fallback al default */
                    let line_article_id = line_article_ids
                        .and_then(|ids| ids.get(i))
                        .copied()
                        .unwrap_or(article.id);
                    json!({
                        "Lin": (i + 1) as i32,
                        "Id": line_article_id,
                        "Name": linea.descripcion,
                        "Units": cantidad,
                        "Price": precio,
                        "Supplement": 0.0,
                        "Discount": descuento,
                        "DiscountPct": false,
                        "Total": linea_total,
                        "VatPct": Self::decimal_to_f64(&linea.iva_pct),
                        "Comments": [],
                        "Supplements": [],
                        "OrderItemType": 0,
                        "OrderItemTypeMetaInfo": "",
                        "TyC_D1": 0,
                        "TyC_D2": 0,
                        "TyC_D3": 0,
                        "OnSale": false
                    })
                })
                .collect()
        } else {
            /* Fallback legacy: 1 artículo genérico con el total de la venta */
            let description = if venta.descripcion.is_empty() {
                article.name.clone()
            } else {
                venta.descripcion.clone()
            };
            vec![json!({
                "Lin": 1,
                "Id": article.id,
                "Name": description,
                "Units": 1.0,
                "Price": total,
                "Supplement": 0.0,
                "Discount": 0.0,
                "DiscountPct": false,
                "Total": total,
                "VatPct": Self::decimal_to_f64(&venta.iva_porcentaje),
                "Comments": [],
                "Supplements": [],
                "OrderItemType": 0,
                "OrderItemTypeMetaInfo": "",
                "TyC_D1": 0,
                "TyC_D2": 0,
                "TyC_D3": 0,
                "OnSale": false
            })]
        };

        BdpCreateOrderRequest {
            employee_id: config.bdp_employee_id,
            items_profile_id: config.bdp_items_profile_id,
            order_end_type: 1, /* Pendiente de validación — no factura, no imprime ticket */
            order_operation_type: 0, /* Escritura real (0=CheckAndCreate) */
            invoice: Some(false),
            order: {
                /* [F3.1-3.3] Construir order JSON con campos opcionales */
                let mut order = json!({
                    "MarketplaceOrderId": &marketplace_order_id[..15.min(marketplace_order_id.len())],
                    "MarketId": BDP_SYNC_MARKET_ID,
                    "MarketName": "Glory",
                    "PreparationTime": now,
                    "OrderId": 0,
                    "PosId": config.bdp_pos_id,
                    "Type": order_ctx.order_type,
                    "RoomNumber": 0,
                    "TableNumber": 0,
                    "Items": items,
                    "Discount": 0.0,
                    "DiscountPct": false,
                    "Total": total,
                    "ExecutionTime": now,
                    "Status": 0,
                    "AlreadyInvoiced": false,
                    "Comments": format!("Glory venta {}", venta.id)
                });
                /* [F3.2] TenderId — mapeo de método de pago */
                if let Some(tender_id) = order_ctx.tender_id {
                    order["TenderId"] = json!(tender_id);
                }
                /* [F3.1] Customer — datos del cliente si existe */
                if let Some(ref name) = order_ctx.customer_name {
                    let mut customer = json!({ "Name": name });
                    if let Some(ref phone) = order_ctx.customer_phone {
                        if !phone.is_empty() {
                            customer["Phone"] = json!(phone);
                        }
                    }
                    if let Some(ref code) = order_ctx.customer_code {
                        if !code.is_empty() {
                            customer["Code"] = json!(code);
                        }
                    }
                    order["Customer"] = customer;
                }
                order
            },
        }
    }

    /// Resuelve qué artículo BDP usar. Intenta:
    /// 1. Si `bdp_default_article_code` es numérico, enriquecer con `GetArticle`
    /// 2. Si no, buscar el primer artículo del perfil
    /// 3. Fallback: artículo genérico con código 0
    async fn resolve_article(
        client: &BdpWeblinkClient<'_>,
        config: &ConfiguracionRestaurante,
    ) -> ResolvedArticle {
        /* Intento 1: código configurado como número → intentar GetArticle para datos reales */
        if let Ok(code) = config.bdp_default_article_code.trim().parse::<i64>() {
            if code > 0 {
                /* [157A-9] F9.2: GetArticle enriquece nombre, precio e IVA desde BDP */
                match client
                    .get_article(&BdpGetArticleRequest { art_code: code })
                    .await
                {
                    Ok(value) => {
                        let article_data = value.get("ArticleData").unwrap_or(&value);
                        let name = article_data
                            .get("ArtDescription")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&config.bdp_default_article_name)
                            .to_string();
                        #[allow(clippy::cast_precision_loss)]
                        let price = article_data
                            .get("Price1")
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0);
                        let vat_pct = article_data
                            .get("TAVPer")
                            .and_then(Value::as_f64)
                            .unwrap_or(10.0);
                        return ResolvedArticle {
                            id: code,
                            name,
                            price,
                            vat_pct,
                        };
                    }
                    Err(e) => {
                        warn!("[157A-9] GetArticle falló para código {code}, usando config: {e}");
                        return ResolvedArticle {
                            id: code,
                            name: config.bdp_default_article_name.clone(),
                            price: 0.0,
                            /* [R12] Usar iva_por_defecto de config en vez de 10.0 hardcodeado */
                            vat_pct: Self::decimal_to_f64(&config.iva_por_defecto),
                        };
                    }
                }
            }
        }

        /* Intento 2: buscar primer artículo del perfil */
        match client
            .get_pos_articles(&BdpGetPosArticlesRequest::first_page(
                config.bdp_items_profile_id,
                1,
            ))
            .await
        {
            Ok(value) => {
                let default_iva = Self::decimal_to_f64(&config.iva_por_defecto);
                if let Some(article) = Self::extract_first_article(&value, default_iva) {
                    return article;
                }
            }
            Err(e) => {
                warn!("[065A-5] Error buscando artículo BDP: {e}");
            }
        }

        /* Fallback: genérico */
        ResolvedArticle {
            id: 0,
            name: config.bdp_default_article_name.clone(),
            price: 0.0,
            /* [R12] Usar iva_por_defecto de config en vez de 10.0 hardcodeado */
            vat_pct: Self::decimal_to_f64(&config.iva_por_defecto),
        }
    }

    /// [F2.8] Resuelve el artículo BDP para cada línea de venta consultando `bdp_article_map`.
    /// Devuelve un Vec<i64> paralelo a `lineas` con el ID del artículo BDP.
    /// Si una línea no tiene mapeo, usa `default_article_id`.
    async fn resolve_line_articles(
        pool: &PgPool,
        user_id: uuid::Uuid,
        lineas: &[VentaLinea],
        default_article_id: i64,
    ) -> Vec<i64> {
        let mut ids = Vec::with_capacity(lineas.len());
        for linea in lineas {
            let resolved = if linea.articulo_codigo.is_empty() {
                default_article_id
            } else {
                match crate::repositories::BdpArticleMapRepository::buscar_por_codigo(
                    pool,
                    user_id,
                    &linea.articulo_codigo,
                )
                .await
                {
                    Ok(Some(map)) => {
                        /* El código BDP puede ser numérico (ID directo) o texto (requeriría lookup).
                         * Por ahora solo soportamos códigos numéricos. */
                        match map.articulo_bdp_codigo.trim().parse::<i64>() {
                            Ok(code) if code > 0 => code,
                            _ => {
                                info!(
                                    "[F2.8] Código BDP '{}' no numérico para línea '{}', usando default",
                                    map.articulo_bdp_codigo, linea.descripcion
                                );
                                default_article_id
                            }
                        }
                    }
                    Ok(None) => {
                        /* Sin mapeo — usar artículo default */
                        default_article_id
                    }
                    Err(e) => {
                        warn!(
                            "[F2.8] Error buscando mapeo BDP para código '{}': {e}",
                            linea.articulo_codigo
                        );
                        default_article_id
                    }
                }
            };
            ids.push(resolved);
        }
        ids
    }

    /// [F3.1-3.3] Resuelve el contexto del pedido: tender, order type y datos del cliente.
    /// Se ejecuta una sola vez por `sync_venta` y el resultado se reutiliza en `build_order`.
    async fn resolve_order_context(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> OrderContext {
        /* [F3.2] TenderId: mapear metodo_pago → ID BDP desde config.bdp_tender_map */
        let tender_id = Self::resolve_tender_id(venta, config);

        /* [F3.3] Order type: mapear canal → tipo BDP desde config.bdp_order_type_map */
        let order_type = Self::resolve_order_type(venta, config);

        /* [F3.1] Customer: lookup cliente si existe */
        let (customer_code, customer_name, customer_phone) =
            Self::resolve_customer(pool, venta, config).await;

        OrderContext {
            tender_id,
            order_type,
            customer_code,
            customer_name,
            customer_phone,
        }
    }

    /// [F3.2] Resuelve `TenderId` buscando `venta.metodo_pago` en `bdp_tender_map`.
    /// Ej: `{"efectivo": "1", "tarjeta": "2"}` → `metodo_pago`="Efectivo" → `tender_id=Some(1)`.
    fn resolve_tender_id(venta: &Venta, config: &ConfiguracionRestaurante) -> Option<i32> {
        let map = config.bdp_tender_map.as_object()?;
        let key = venta.metodo_pago.to_lowercase();
        let value = map.get(&key)?;
        let id = value
            .as_i64()
            .and_then(|id| i32::try_from(id).ok())
            .or_else(|| value.as_str()?.trim().parse::<i32>().ok())?;
        if id > 0 {
            Some(id)
        } else {
            None
        }
    }

    /// [F3.3] Resuelve el order type buscando venta.canal en `bdp_order_type_map`.
    /// Default: 0 (Barra/Takeaway) si no hay mapeo o el canal no está configurado.
    fn resolve_order_type(venta: &Venta, config: &ConfiguracionRestaurante) -> i32 {
        let Some(map) = config.bdp_order_type_map.as_object() else {
            return 0;
        };
        let key = venta.canal.to_lowercase();
        let Some(value) = map.get(&key) else {
            return 0;
        };
        match value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .or_else(|| value.as_str().and_then(|s| s.trim().parse::<i32>().ok()))
        {
            Some(t) if t >= 0 => t,
            _ => 0,
        }
    }

    /// [F3.1] Resuelve datos del cliente si la venta tiene `cliente_id`.
    /// Devuelve (code, name, phone) — cada uno puede ser None.
    async fn resolve_customer(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let Some(cliente_id) = venta.cliente_id else {
            /* Sin cliente — usar default_customer_code si existe */
            let code = &config.bdp_default_customer_code;
            if code.is_empty() {
                return (None, None, None);
            }
            return (Some(code.clone()), None, None);
        };

        let user_id = venta.user_id;

        match ClienteRepository::find_by_id(pool, cliente_id, user_id).await {
            Ok(Some(cliente)) => {
                let name = {
                    let full = format!("{} {}", cliente.nombre, cliente.apellidos)
                        .trim()
                        .to_string();
                    if full.is_empty() {
                        None
                    } else {
                        Some(full)
                    }
                };
                let phone = if cliente.telefono.is_empty() {
                    None
                } else {
                    Some(cliente.telefono.clone())
                };
                /* [F7.5] Priorizar bdp_customer_code del cliente sobre default config.
                 * Si el cliente ya fue sincronizado con BDP, usamos su código real.
                 * Si no, fallback al código genérico de config. */
                let code = if let Some(bdp_code) = cliente.bdp_customer_code {
                    Some(bdp_code.to_string())
                } else {
                    let default = config.bdp_default_customer_code.clone();
                    if default.is_empty() {
                        None
                    } else {
                        Some(default)
                    }
                };
                (code, name, phone)
            }
            Ok(None) => {
                info!(
                    "[F3.1] Cliente {} no encontrado para venta {}",
                    cliente_id, venta.id
                );
                (None, None, None)
            }
            Err(e) => {
                warn!("[F3.1] Error buscando cliente {}: {e}", cliente_id);
                (None, None, None)
            }
        }
    }

    /* [R12] Parámetro default_iva_pct: IVA fallback del config, no 10.0 hardcodeado. */
    fn extract_first_article(value: &Value, default_iva_pct: f64) -> Option<ResolvedArticle> {
        let items = value
            .get("ArticlesListData")
            .or_else(|| value.get("ArticleListData"))
            .or_else(|| value.get("Articles"))
            .and_then(|v| v.as_array())?;

        let item = items.first()?;
        #[allow(clippy::cast_possible_truncation)]
        let id = item
            .get("ArtCode")
            .or_else(|| item.get("Id"))
            .or_else(|| item.get("Code"))
            .and_then(Value::as_i64)
            .or_else(|| {
                item.get("ArtCode")
                    .or_else(|| item.get("Id"))
                    .and_then(Value::as_f64)
                    .map(|f| f as i64)
            })?;

        let name = item
            .get("ArtDescription")
            .or_else(|| item.get("Description"))
            .or_else(|| item.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Artículo BDP")
            .to_string();

        #[allow(clippy::cast_precision_loss)]
        let price = item
            .get("Price1")
            .or_else(|| item.get("Price"))
            .and_then(Value::as_f64)
            .or_else(|| item.get("Price1").and_then(Value::as_i64).map(|i| i as f64))
            .unwrap_or(0.0);

        /* [R12] Usar default_iva_pct de config en vez de 10.0 hardcodeado */
        let vat_pct = item
            .get("TAVPer")
            .or_else(|| item.get("VatPct"))
            .and_then(Value::as_f64)
            .unwrap_or(default_iva_pct);

        (id > 0).then_some(ResolvedArticle {
            id,
            name,
            price,
            vat_pct,
        })
    }

    /* [R5] Fase HTTP de sync_venta: resolve artículo, contexto, líneas y envío.
     * Se extrae a una función para poder aplicar timeout global con
     * `tokio::time::timeout` sin perder claridad. */
    async fn run_http_phase(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
    ) -> Result<i64, OrderSendFailure> {
        let client = BdpWeblinkClient::new(config);
        let article = Self::resolve_article(&client, config).await;

        /* [F7.5] Si la política exige cliente BDP, la comanda solo continúa
         * cuando el cliente ya tiene un código local confirmado. */
        if config.bdp_auto_sync_customers {
            if let Some(cliente_id) = venta.cliente_id {
                if let Some(bdp_code) =
                    Self::ensure_cliente_bdp_synced(pool, cliente_id, venta.user_id, config).await
                {
                    info!(
                        "[F7.5] Cliente {} auto-sincronizado con BDP (code={bdp_code}) para venta {}",
                        cliente_id, venta.id
                    );
                } else {
                    let msg = if config.bdp_default_customer_code.is_empty() {
                        "Cliente sin código BDP confirmado. Asigne un código BDP al cliente o configure un cliente por defecto en Configuración → BDP.".to_string()
                    } else {
                        "Cliente sin código BDP confirmado. Se usará el cliente por defecto configurado.".to_string()
                    };
                    warn!(
                        "[BDP-SAFE] {msg} (cliente {cliente_id}, venta {})",
                        venta.id
                    );
                    return Err(OrderSendFailure::Rejected(msg));
                }
            }
        }

        /* [F3.1] Resolver contexto del pedido: tender, order type, customer. */
        let order_ctx = Self::resolve_order_context(pool, venta, config).await;

        /* [F2.6] Obtener líneas de venta para multi-item.
         * Si la venta tiene líneas en BD, se usan para construir un pedido multi-item.
         * Si no, se usa el comportamiento legacy (1 artículo genérico). */
        let lineas = match VentaLineaRepository::listar_por_venta(pool, venta.id).await {
            Ok(l) if !l.is_empty() => Some(l),
            Ok(_) => None,
            Err(e) => {
                warn!("[F2.6] Error obteniendo líneas de venta {}: {e}", venta.id);
                None
            }
        };

        /* [F2.8] Resolver artículo BDP por línea usando bdp_article_map. */
        let line_article_ids: Option<Vec<i64>> = if let Some(ref lineas) = lineas {
            Some(Self::resolve_line_articles(pool, venta.user_id, lineas, article.id).await)
        } else {
            None
        };

        Self::retry_send_order(
            &client,
            config,
            venta,
            &article,
            lineas.as_deref(),
            line_article_ids.as_deref(),
            &order_ctx,
        )
        .await
    }

    /* [R16] Conversión Decimal → f64 para serialización JSON a BDP.
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

    fn sanitize_error(raw: &str) -> String {
        if raw.contains("401") || raw.contains("403") {
            "Error de autenticación con BDP (401/403)".to_string()
        } else if raw.contains("300035") {
            "BDP: serie no válida (300035)".to_string()
        } else if raw.contains("300008") {
            "BDP: salón/mesa no válidos (300008)".to_string()
        } else if raw.contains("300009") {
            "BDP: delivery no soportado en este POS (300009)".to_string()
        } else if raw.contains("301011") {
            "BDP: MarketplaceOrderId demasiado largo (301011)".to_string()
        } else if raw.contains("301400") {
            "BDP: caja cerrada (301400)".to_string()
        } else {
            let truncated: String = raw.chars().take(200).collect();
            format!("Error BDP: {truncated}")
        }
    }

    fn cleanup_lock(venta_id: uuid::Uuid) {
        let mut map = SYNC_LOCKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = map.get(&venta_id) {
            if Arc::strong_count(entry) <= 2 {
                map.remove(&venta_id);
            }
        }
        /* [AUDIT-N3] Sweep periódico: eliminar entradas huérfanas cuyo Arc
         * solo vive en el HashMap (strong_count == 1). Esto previene leak
         * de memoria cuando cleanup_lock no se llama por panic o early return. */
        if map.len() > 100 {
            map.retain(|_, arc| Arc::strong_count(arc) > 1);
        }
    }

    /// [F7.5] Auto-sync de cliente Glory → BDP.
    /// Devuelve únicamente un código BDP previamente confirmado. La creación
    /// automática quedó deliberadamente deshabilitada: `max + 1` y hashes no
    /// pueden garantizar ausencia de colisiones con otros escritores del TPV.
    pub async fn ensure_cliente_bdp_synced(
        pool: &PgPool,
        cliente_id: uuid::Uuid,
        user_id: uuid::Uuid,
        _config: &ConfiguracionRestaurante,
    ) -> Option<i32> {
        let Ok(Some(cliente)) = ClienteRepository::find_by_id(pool, cliente_id, user_id).await
        else {
            return None;
        };
        if let Some(code) = cliente.bdp_customer_code {
            return (code > 0).then_some(code);
        }
        let msg = "Creación automática BDP deshabilitada: asigne y verifique un código explícito desde la sincronización manual";
        let _ = ClienteRepository::update_bdp_sync(pool, cliente_id, None, false, Some(msg)).await;
        None
    }

    /* ===== FASE 8: AddOrderPayment + InvoiceOrder ===== */

    /* [F8.1] Registrar pago contra una orden BDP existente.
     * Ahora soporta pagos parciales controlados por el feature flag
     * ff_bdp_partial_payments. Cada pago se registra en el ledger local
     * bdp_pagos para evitar sobrepagos y mantener historial.
     *
     * ️ REQUIERE AUTORIZACIÓN DEL USUARIO para llamadas reales a BDP. */
    #[allow(clippy::too_many_lines)]
    pub async fn add_order_payment(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        amount: Decimal,
        tender_id: i32,
        idempotency_key: Option<&str>,
    ) -> Result<Option<String>, String> {
        if !config.bdp_sync_enabled || !crate::services::bdp_sync_preflight::bdp_configurado(config)
        {
            return Err("BDP no está habilitado o configurado".into());
        }

        /* [F3] Gate: en modo read_only, no registrar pagos en BDP */
        if config.bdp_sync_mode != "unidirectional" {
            return Err(
                "BDP en modo solo lectura. Cambia el modo en configuración para registrar pagos."
                    .into(),
            );
        }
        if !config.bdp_auto_backup_before_write {
            return Err("Escritura BDP bloqueada: auto-backup pre-write desactivado".into());
        }

        let order_id = venta
            .bdp_order_id
            .ok_or_else(|| format!("Venta {} no tiene bdp_order_id", venta.id))?;

        if amount <= Decimal::ZERO || tender_id <= 0 {
            return Err("Pago BDP bloqueado: importe o tender inválido".into());
        }

        /* [247A-9] Exclusión por venta para evitar pagos concurrentes que
         * podrían sobrepasar el saldo pendiente. */
        let lock = {
            let mut map = SYNC_LOCKS
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            map.entry(venta.id)
                .or_insert_with(|| Arc::new(TokioMutex::new(())))
                .clone()
        };
        let Ok(_guard) = lock.try_lock() else {
            return Err("Ya hay un pago en proceso para esta venta; inténtalo de nuevo".into());
        };
        let _sync_lock_guard = SyncLockGuard::new(venta.id);

        /* [247A-9] Idempotencia: si la clave ya existe con éxito, no repetir.
         * La clave debe pertenecer a la misma venta y tener mismos amount/tender_id
         * para considerarse un reintento legítimo; de lo contrario es un error de
         * cliente o un intento de reuso malicioso de clave. */
        if let Some(key) = idempotency_key {
            if let Ok(Some(pago)) = BdpPagoRepository::obtener_por_idempotency_key(pool, key).await
            {
                if pago.venta_id != venta.id {
                    return Err("idempotencia_duplicada:ledger:otra_venta".into());
                }
                if pago.amount != amount || pago.tender_id != tender_id {
                    return Err("idempotencia_duplicada:ledger:campos_distintos".into());
                }
                if pago.resultado == "exito" {
                    info!(
                        "[247A-9] Pago con idempotencia {} ya existía; no se reenvía a BDP",
                        key
                    );
                    return Ok(None);
                }
            }
        }

        let client = BdpWeblinkClient::new(config);
        client
            .login()
            .await
            .map_err(|e| format!("Error login BDP: {e}"))?;

        crate::services::BdpWriteGuard::ensure_no_unresolved(
            pool,
            venta.user_id,
            "venta_id",
            venta.id,
            &["add_payment"],
        )
        .await?;

        let current = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|e| {
                format!("Pago bloqueado: no se pudo reconciliar la orden antes de escribir: {e}")
            })?;
        let order = current
            .get("Order")
            .ok_or_else(|| "Pago bloqueado: GetOrder no devolvió el objeto Order".to_string())?;
        let status = order
            .get("Status")
            .and_then(Value::as_i64)
            .ok_or_else(|| "Pago bloqueado: GetOrder no devolvió Status".to_string())?;
        if matches!(status, 2 | 3) {
            return Err("Pago bloqueado: la orden está cancelada o facturada".into());
        }
        let total = order
            .get("Total")
            .and_then(Value::as_f64)
            .ok_or_else(|| "Pago bloqueado: GetOrder no devolvió Total".to_string())?;
        let requested = Self::decimal_to_f64(&amount);

        /* [247A-9] Saldo pendiente basado en ledger local. BDP también puede
         * tener pagos hechos fuera de Glory; si el total pagado local es menor
         * que el de BDP, aceptamos el menor saldo pendiente para no bloquear
         * pagos legítimos. */
        let local_paid = BdpPagoRepository::total_pagado(pool, venta.id)
            .await
            .map_err(|e| format!("Pago bloqueado: no se pudo calcular saldo local: {e}"))?;
        let local_paid_f64 = Self::decimal_to_f64(&local_paid);
        let pending = total - local_paid_f64;

        let is_partial = (requested - pending).abs() > BDP_PAYMENT_TOLERANCE;
        if is_partial && !config.ff_bdp_partial_payments {
            return Err(format!(
                "Pago bloqueado: pagos parciales desactivados. Saldo={pending:.2}, solicitado={requested:.2}"
            ));
        }
        if requested > pending + BDP_PAYMENT_TOLERANCE {
            return Err(format!(
                "Pago bloqueado: el importe {requested:.2} excede el saldo pendiente {pending:.2}"
            ));
        }

        /* [187A-1] El snapshot remoto debe completarse antes de consumir el
         * armado; authorize registra la intención y cierra el modo escritura. */
        let datos_pago = serde_json::json!({
            "venta_id": venta.id,
            "order_id": order_id,
            "amount": amount,
            "tender_id": tender_id,
            "is_partial": is_partial
        });
        let snapshot_pre_id = crate::services::BdpBackupService::preparar_snapshot_escritura(
            pool,
            venta.user_id,
            "add_payment",
            config,
            Some(order_id),
        )
        .await
        .map_err(|e| format!("Pre-write audit BDP falló; pago bloqueado: {e}"))?;

        let audit_id = crate::services::BdpWriteGuard::authorize(
            pool,
            venta.user_id,
            config,
            "add_payment",
            "venta",
            venta.id,
            "venta_id",
            &datos_pago,
            snapshot_pre_id,
            idempotency_key,
        )
        .await?;

        /* [247A-9] ID de pago determinístico a partir de la idempotencia para
         * que los reintentos con la misma clave reutilicen el mismo identificador
         * en BDP y no se creen pagos duplicados. */
        let venta_prefix = venta.id.simple().to_string();
        let venta_prefix = &venta_prefix[..14];
        let payment_id = idempotency_key.map_or_else(
            || {
                let uuid_short = Uuid::new_v4().simple().to_string();
                let uuid_short = &uuid_short[..8];
                format!("P{venta_prefix}-{uuid_short}")
            },
            |k| format!("P{venta_prefix}-{k}"),
        );

        let request = BdpAddOrderPaymentRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            payment: BdpOrderPayment {
                tender_id,
                amount,
                payment_id: payment_id.clone(),
            },
            invoice: None,
            pos_id: Some(config.bdp_pos_id),
            employee_id: Some(config.bdp_employee_id),
            invoice_parameters: None,
        };

        let response = match client.add_order_payment(&request).await {
            Ok(response) => response,
            Err(e) => {
                let msg = format!("Error AddOrderPayment: {e}");
                let resultado = match e {
                    crate::services::bdp_weblink::BdpWeblinkError::Http(_)
                    | crate::services::bdp_weblink::BdpWeblinkError::Api { .. }
                    | crate::services::bdp_weblink::BdpWeblinkError::Throttled(_) => "ambiguo",
                    _ => "error",
                };
                let _ = crate::services::BdpBackupService::actualizar_resultado(
                    pool,
                    audit_id,
                    resultado,
                    None,
                    Some(&msg),
                )
                .await;
                return Err(msg);
            }
        };

        let invoice_number = response
            .get("InvoiceNumber")
            .and_then(|v| v.as_str())
            .map(String::from);

        /* [207A-2] S7-H2: Envolver marca local + auditoría + ledger en
         * transacción para consistencia. */
        let commit_result = async {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| format!("Error iniciando tx post-pago: {e}"))?;

            if let Some(ref inv) = invoice_number {
                /* [F8.3] Si BDP devolvió InvoiceNumber, marcar venta como facturada. */
                info!(
                    "[F8.1] Pago registrado en BDP para venta {} → InvoiceNumber={inv}",
                    venta.id
                );
                sqlx::query(
                    "UPDATE ventas SET bdp_invoiced = true, bdp_order_status = 'invoiced', updated_at = NOW() WHERE id = $1",
                )
                .bind(venta.id)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("BDP confirmó pago y factura {inv}, pero no se pudo persistir localmente: {e}"))?;
            } else {
                info!(
                    "[F8.1] Pago registrado en BDP para venta {} (sin InvoiceNumber)",
                    venta.id
                );
            }

            /* Cerrar auditoría dentro de la misma transacción. */
            sqlx::query(
                r"UPDATE bdp_audit_log
                SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
                WHERE id = $1",
            )
            .bind(audit_id)
            .bind(Some(&response))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Pago confirmado, pero falló el cierre de auditoría: {e}"))?;

            /* [247A-9] Registrar pago local en el mismo commit atómico. */
            let idempotency_for_insert = idempotency_key.unwrap_or(&payment_id).to_string();
            let pago_respuesta: Option<serde_json::Value> = Some(response.clone());
            sqlx::query(
                r"INSERT INTO bdp_pagos
                (venta_id, amount, tender_id, idempotency_key, bdp_order_id, bdp_payment_id, resultado, datos_respuesta)
                VALUES ($1, $2, $3, $4, $5, $6, 'exito', $7)
                ON CONFLICT (idempotency_key) DO UPDATE SET
                    updated_at = NOW(),
                    resultado = EXCLUDED.resultado,
                    datos_respuesta = EXCLUDED.datos_respuesta
                WHERE bdp_pagos.venta_id = EXCLUDED.venta_id
                  AND bdp_pagos.amount = EXCLUDED.amount
                  AND bdp_pagos.tender_id = EXCLUDED.tender_id
                RETURNING id",
            )
            .bind(venta.id)
            .bind(amount)
            .bind(tender_id)
            .bind(idempotency_for_insert)
            .bind(order_id)
            .bind(Some(&payment_id))
            .bind(pago_respuesta)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| format!("Pago confirmado, pero falló el registro en ledger: {e}"))?;

            tx.commit()
                .await
                .map_err(|e| format!("Error confirmando tx post-pago: {e}"))
        }
        .await;

        if let Err(e) = commit_result {
            /* La tx falló pero BDP ya procesó el pago → auditoría ambigua. */
            let _ = crate::services::BdpBackupService::actualizar_resultado(
                pool,
                audit_id,
                "ambiguo",
                Some(&response),
                Some(&e),
            )
            .await;
            return Err(e);
        }

        Ok(invoice_number)
    }

    /* [F8.2] Facturar una orden BDP existente.
     * Llama a `POST /API/Orders/Invoice` con el order_id de la venta.
     * Retorna el InvoiceNumber.
     *
     * ⚠️ REQUIERE AUTORIZACIÓN DEL USUARIO para llamadas reales a BDP. */
    /* [187A-1] La secuencia factura/preflight/snapshot/autorización/auditoría
     * se mantiene lineal para conservar una única frontera de escritura. */
    #[allow(clippy::too_many_lines)]
    pub async fn invoice_order(
        pool: &PgPool,
        venta: &Venta,
        config: &ConfiguracionRestaurante,
        idempotency_key: Option<&str>,
    ) -> Result<String, String> {
        if !config.bdp_sync_enabled || !crate::services::bdp_sync_preflight::bdp_configurado(config)
        {
            return Err("BDP no está habilitado o configurado".into());
        }

        /* [F3] Gate: en modo read_only, no facturar en BDP */
        if config.bdp_sync_mode != "unidirectional" {
            return Err(
                "BDP en modo solo lectura. Cambia el modo en configuración para facturar.".into(),
            );
        }
        if !config.bdp_auto_backup_before_write {
            return Err("Escritura BDP bloqueada: auto-backup pre-write desactivado".into());
        }

        let order_id = venta
            .bdp_order_id
            .ok_or_else(|| format!("Venta {} no tiene bdp_order_id", venta.id))?;

        let client = BdpWeblinkClient::new(config);
        client
            .login()
            .await
            .map_err(|e| format!("Error login BDP: {e}"))?;

        crate::services::BdpWriteGuard::ensure_no_unresolved(
            pool,
            venta.user_id,
            "venta_id",
            venta.id,
            &["invoice"],
        )
        .await?;

        let current = client
            .get_order(&BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            })
            .await
            .map_err(|e| {
                format!("Factura bloqueada: no se pudo reconciliar la orden antes de escribir: {e}")
            })?;
        let order = current
            .get("Order")
            .ok_or_else(|| "Factura bloqueada: GetOrder no devolvió el objeto Order".to_string())?;
        let status = order
            .get("Status")
            .and_then(Value::as_i64)
            .ok_or_else(|| "Factura bloqueada: GetOrder no devolvió Status".to_string())?;
        if status == 2 {
            return Err("Factura bloqueada: la orden está cancelada".into());
        }
        if status == 3 {
            let invoice_number = order
                .get("InvoiceNumber")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned)
                .ok_or_else(|| {
                    "Orden ya facturada pero sin InvoiceNumber reconciliable".to_string()
                })?;
            /* [AUDIT-N6] Envolver reconciliación en transacción por consistencia
             * con el path normal de facturación. */
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| format!("Error iniciando tx reconciliación factura: {e}"))?;
            sqlx::query(
                "UPDATE ventas SET bdp_invoiced = true, bdp_order_status = 'invoiced', updated_at = NOW() WHERE id = $1",
            )
            .bind(venta.id)
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("Factura BDP reconciliada, pero no se pudo persistir localmente: {error}"))?;
            tx.commit()
                .await
                .map_err(|e| format!("Error confirmando tx reconciliación factura: {e}"))?;
            return Ok(invoice_number);
        }
        /* [D9] Parsear Total como string directamente a Decimal para evitar
         * pérdida de precisión por conversión f64 intermedia. BDP puede
         * devolver tanto números como strings. */
        let total_decimal = match order.get("Total") {
            Some(serde_json::Value::Number(n)) => {
                n.as_f64()
                    .and_then(Decimal::from_f64)
                    .ok_or_else(|| "Factura bloqueada: Total de BDP no es convertible a Decimal".to_string())?
            }
            Some(serde_json::Value::String(s)) => {
                s.parse::<Decimal>()
                    .map_err(|_| format!("Factura bloqueada: Total '{s}' no es un Decimal válido"))?
            }
            _ => return Err("Factura bloqueada: GetOrder no devolvió Total".into()),
        };
        /* [D9] Parsear pagos de BDP sin unwrap_or(0.0) */
        let paid: Decimal = order
            .get("Payments")
            .and_then(Value::as_array)
            .ok_or_else(|| "Factura bloqueada: GetOrder no devolvió Payments".to_string())?
            .iter()
            .map(|payment| {
                payment
                    .get("Amount")
                    .and_then(Value::as_f64)
                    .and_then(Decimal::from_f64)
                    .unwrap_or(Decimal::ZERO)
            })
            .sum();
        let local_paid = BdpPagoRepository::total_pagado(pool, venta.id)
            .await
            .map_err(|e| format!("Factura bloqueada: no se pudo calcular saldo local: {e}"))?;
        let tolerance = Decimal::new(5, 3); /* 0.005 */
        if (total_decimal - local_paid).abs() > tolerance {
            return Err("Factura bloqueada: la orden conserva saldo pendiente local".into());
        }
        let bdp_pending = total_decimal - paid;
        if bdp_pending.abs() > tolerance {
            return Err("Factura bloqueada: la orden conserva saldo pendiente en BDP".into());
        }

        /* [187A-1] Snapshot obligatorio + autorización de un solo uso. */
        let datos_factura = serde_json::json!({
            "venta_id": venta.id,
            "order_id": order_id
        });
        let snapshot_pre_id = crate::services::BdpBackupService::preparar_snapshot_escritura(
            pool,
            venta.user_id,
            "invoice",
            config,
            Some(order_id),
        )
        .await
        .map_err(|e| format!("Pre-write audit BDP falló; facturación bloqueada: {e}"))?;

        let audit_id = crate::services::BdpWriteGuard::authorize(
            pool,
            venta.user_id,
            config,
            "invoice",
            "venta",
            venta.id,
            "venta_id",
            &datos_factura,
            snapshot_pre_id,
            idempotency_key,
        )
        .await?;

        let request = BdpInvoiceOrderRequest {
            pos_id: config.bdp_pos_id,
            employee_id: config.bdp_employee_id,
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            invoice_parameters: None,
        };

        let response = match client.invoice_order(&request).await {
            Ok(response) => response,
            Err(e) => {
                let msg = format!("Error InvoiceOrder: {e}");
                let resultado = match e {
                    crate::services::bdp_weblink::BdpWeblinkError::Http(_)
                    | crate::services::bdp_weblink::BdpWeblinkError::Api { .. }
                    | crate::services::bdp_weblink::BdpWeblinkError::Throttled(_) => "ambiguo",
                    _ => "error",
                };
                crate::services::BdpBackupService::actualizar_resultado(
                    pool,
                    audit_id,
                    resultado,
                    None,
                    Some(&msg),
                )
                .await
                .map_err(|audit_error| {
                    format!("{msg}; además falló el cierre de auditoría: {audit_error}")
                })?;
                return Err(msg);
            }
        };

        let invoice_number = response
            .get("InvoiceNumber")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if invoice_number.is_empty() {
            let msg = "BDP no devolvió InvoiceNumber; no se marcará la venta como facturada";
            crate::services::BdpBackupService::actualizar_resultado(
                pool,
                audit_id,
                "ambiguo",
                Some(&response),
                Some(msg),
            )
            .await
            .map_err(|audit_error| {
                format!("{msg}; además falló el cierre de auditoría: {audit_error}")
            })?;
            return Err(msg.to_string());
        }

        /* [207A-2] S7-H2: Envolver marca local + auditoría en transacción
         * para que, si el proceso muere después del HTTP, no quede
         * bdp_invoiced=true sin auditoría cerrada (o viceversa). */
        let commit_result = async {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| format!("Error iniciando tx post-factura: {e}"))?;

            /* [F8.3] Marcar venta como facturada. */
            sqlx::query(
                "UPDATE ventas SET bdp_invoiced = true, bdp_order_status = 'invoiced', updated_at = NOW() WHERE id = $1",
            )
            .bind(venta.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                format!("BDP confirmó InvoiceNumber={invoice_number}, pero no se pudo persistir localmente: {e}")
            })?;

            /* Cerrar auditoría dentro de la misma transacción. */
            sqlx::query(
                r"UPDATE bdp_audit_log
                SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
                WHERE id = $1",
            )
            .bind(audit_id)
            .bind(Some(&response))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Factura confirmada, pero falló el cierre de auditoría: {e}"))?;

            tx.commit()
                .await
                .map_err(|e| format!("Error confirmando tx post-factura: {e}"))
        }
        .await;

        if let Err(e) = commit_result {
            /* La tx falló pero BDP ya procesó la factura → auditoría ambigua. */
            let _ = crate::services::BdpBackupService::actualizar_resultado(
                pool,
                audit_id,
                "ambiguo",
                Some(&response),
                Some(&e),
            )
            .await;
            return Err(e);
        }

        info!(
            "[F8.2] Orden {} facturada en BDP → InvoiceNumber={invoice_number}",
            venta.id
        );

        Ok(invoice_number)
    }

    /* [157A-7] F9.1: sync_catalog — Sincroniza catálogo completo BDP → Glory.
     * Llama a ExportArticles, parsea respuesta tipada, hace upsert enriquecido
     * en bdp_article_map. Devuelve resumen de creados/actualizados/sin_cambios/errores.
     * NO requiere auth BDP en modo mock — se puede testear sin conexión. */
    pub async fn sync_catalog(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        user_id: Uuid,
        type_price: i32,
    ) -> Result<crate::services::bdp_weblink_catalog::BdpCatalogSyncResult, String> {
        use crate::repositories::BdpArticleMapRepository;
        use crate::services::bdp_weblink_catalog::{
            BdpCatalogSyncResult, BdpExportArticlesRequest, BdpExportArticlesResponse,
        };

        /* 1. Llamar ExportArticles */
        let articles_json = client
            .export_articles(&BdpExportArticlesRequest::all_web_articles(type_price))
            .await
            .map_err(|e| format!("Error ExportArticles: {e}"))?;

        /* 2. Parsear respuesta tipada */
        let response: BdpExportArticlesResponse = serde_json::from_value(articles_json)
            .map_err(|e| format!("Error parseando ExportArticles: {e}"))?;

        let articles = response.articles;
        let total_bdp = articles.len();
        let mut actualizados: u32 = 0;
        let mut sin_cambios: u32 = 0;
        let mut errores: u32 = 0;

        /* 3. Upsert cada artículo */
        for art in &articles {
            let Some(code) = art.art_code() else {
                errores += 1;
                continue;
            };

            let descripcion = art.description().to_string();
            let precio = art.price1.unwrap_or(Decimal::ZERO);
            let iva = art.tax1.unwrap_or(Decimal::ZERO);
            let dept = art.department.unwrap_or(0);
            let fam = art.family.unwrap_or(0);
            let subfam = art.subfamily.unwrap_or(0);
            let barcode = art.bar_code.as_deref().unwrap_or("");

            let upsert_data = crate::repositories::BdpArticleUpsertData {
                bdp_code: code,
                descripcion: &descripcion,
                precio_tarifa1: precio,
                iva_pct: iva,
                departamento: dept,
                familia: fam,
                subfamilia: subfam,
                activo: art.active,
                barcode,
                /* [237A-4] Stock actual del artículo — viene de PricesTableDataType
                 * en la respuesta de ExportArticles. Si el módulo de almacén no
                 * está activo, current_stock será None y queda en 0. */
                stock_actual: art.effective_stock().unwrap_or(Decimal::ZERO),
            };

            match BdpArticleMapRepository::upsert_from_bdp(pool, user_id, &upsert_data).await {
                Ok(true) => {
                    /* upsert_from_bdp returns true when row was created or changed */
                    /* Heuristic: if the row didn't exist before, it's "creado".
                     * We can't distinguish created vs updated from the SQL result alone,
                     * so we count all changes as "actualizados" (upsert semantics). */
                    actualizados += 1;
                    /* [247A-10/S2] Guardar también el stock por almacén (General por defecto).
                     * Se hace como operación separada; si falla no debe romper el sync
                     * del artículo, pero se loguea. */
                    if let Err(e) = BdpArticleMapRepository::upsert_stock(
                        pool,
                        user_id,
                        code,
                        upsert_data.stock_actual,
                    )
                    .await
                    {
                        warn!("[247A-10/S2] Error upsert stock por almacén {code}: {e}");
                    }
                }
                Ok(false) => {
                    sin_cambios += 1;
                    /* Aunque el artículo no haya cambiado, sincronizamos el stock
                     * para asegurar que existe la fila de almacén por defecto. */
                    if let Err(e) = BdpArticleMapRepository::upsert_stock(
                        pool,
                        user_id,
                        code,
                        upsert_data.stock_actual,
                    )
                    .await
                    {
                        warn!("[247A-10/S2] Error upsert stock por almacén {code}: {e}");
                    }
                }
                Err(e) => {
                    warn!("[157A-7] Error upsert artículo BDP {code}: {e}");
                    errores += 1;
                }
            }
        }

        /* [237A-4] Info si ningún artículo trajo stock — probablemente el módulo
         * de almacén de BDP no está activo. Se usa info! en vez de warn! para
         * evitar spam: es un estado esperado si el módulo no está contratado. */
        let stock_populado = articles.iter().any(|a| a.effective_stock().is_some());
        if !stock_populado && total_bdp > 0 {
            info!(
                "[237A-4] Ningún artículo de ExportArticles trajo CurrentStock. \
                 Si el módulo de almacén de BDP no está activo, la columna Stock \
                 mostrará 0 para todos los artículos."
            );
        }

        info!(
            "[157A-7] sync_catalog completado: {} artículos BDP → {actualizados} cambios, {sin_cambios} sin cambios, {errores} errores, stock_disponible={stock_populado}",
            total_bdp
        );

        Ok(BdpCatalogSyncResult {
            creados: 0,
            actualizados,
            sin_cambios,
            errores,
            total_bdp,
        })
    }

    /* [157A-9] F9.3: Refresh de precios de artículos ya mapeados.
     * Consulta GetPricesArticles para cada artículo mapeado y actualiza precio_tarifa1.
     * Devuelve conteo de actualizados/errores. */
    pub async fn sync_prices(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<BdpCatalogSyncResult, String> {
        let maps = BdpArticleMapRepository::listar(pool, user_id)
            .await
            .map_err(|e| format!("Error listando mapeos: {e}"))?;

        let total_bdp = maps.len();
        let mut actualizados: u32 = 0;
        let mut sin_cambios: u32 = 0;
        let mut errores: u32 = 0;

        for map in &maps {
            let code: i64 = if let Ok(c) = map.articulo_bdp_codigo.parse() {
                c
            } else {
                sin_cambios += 1;
                continue;
            };

            match client
                .get_prices_articles(&BdpGetPricesArticlesRequest { art_code: code })
                .await
            {
                Ok(value) => {
                    let resp: BdpGetPricesArticlesResponse = match serde_json::from_value(value) {
                        Ok(r) => r,
                        Err(e) => {
                            warn!("[157A-9] Error parseando precios BDP para {code}: {e}");
                            errores += 1;
                            continue;
                        }
                    };

                    if !resp.error_message.is_empty() {
                        warn!(
                            "[157A-9] BDP error en precios para {code}: {}",
                            resp.error_message
                        );
                        errores += 1;
                        continue;
                    }

                    let new_price = resp.prices.first().copied().unwrap_or(Decimal::ZERO);
                    /* [AUDIT-9.1] No aplicar precios negativos. Precio 0 se permite
                     * (puede ser un artículo de cortesía o servicio gratuito). */
                    if new_price < Decimal::ZERO {
                        warn!(
                            "[157A-9] BDP devolvió precio negativo {} para artículo {code}; ignorando",
                            new_price
                        );
                        errores += 1;
                        continue;
                    }
                    if (new_price - map.precio_tarifa1).abs() > Decimal::new(1, 4) {
                        /* Precio cambió — actualizar directamente via SQL */
                        match sqlx::query(
                            "UPDATE bdp_article_map SET precio_tarifa1 = $1, ultima_sync_at = NOW(), updated_at = NOW() \
                             WHERE id = $2",
                        )
                        .bind(new_price)
                        .bind(map.id)
                        .execute(pool)
                        .await
                        {
                            Ok(_) => actualizados += 1,
                            Err(e) => {
                                warn!("[157A-9] Error actualizando precio de {code}: {e}");
                                errores += 1;
                            }
                        }
                    } else {
                        sin_cambios += 1;
                    }
                }
                Err(e) => {
                    warn!("[157A-9] Error GetPricesArticles para {code}: {e}");
                    errores += 1;
                }
            }
        }

        info!(
            "[157A-9] sync_prices completado: {total_bdp} artículos → {actualizados} precios actualizados, {sin_cambios} sin cambios, {errores} errores"
        );

        Ok(BdpCatalogSyncResult {
            creados: 0,
            actualizados,
            sin_cambios,
            errores,
            total_bdp,
        })
    }

    /* [157A-9] F9.4: Sincroniza salones/mesas de BDP al plano de sala de Glory.
     * Consulta GetRoomsTables → crea/actualiza ZonaSala por cada Room y Mesa por cada table.
     * Devuelve conteo de zonas y mesas procesadas. */
    pub async fn sync_tables(
        client: &BdpWeblinkClient<'_>,
        pool: &PgPool,
        user_id: Uuid,
        aplicar: bool,
    ) -> Result<SyncTablesResult, String> {
        let resp_value = client
            .get_rooms_tables(&BdpGetRoomsTablesRequest::default())
            .await
            .map_err(|e| format!("Error consultando salones BDP: {e}"))?;

        let resp: BdpGetRoomsTablesResponse = serde_json::from_value(resp_value)
            .map_err(|e| format!("Error parseando respuesta GetRoomsTables: {e}"))?;

        if !resp.error_message.is_empty() {
            return Err(format!("BDP error: {}", resp.error_message));
        }

        let mut zonas_creadas: u32 = 0;
        let mut mesas_creadas: u32 = 0;

        for room in &resp.rooms {
            /* Buscar o crear zona por nombre del salón */
            let zonas = crate::repositories::PlanoSalaRepository::listar_zonas(pool, user_id)
                .await
                .map_err(|e| format!("Error listando zonas: {e}"))?;

            let existing_zone = zonas.iter().find(|z| z.nombre == room.name).cloned();
            if existing_zone.is_none() && !aplicar {
                zonas_creadas += 1;
                mesas_creadas += u32::try_from(room.tables.len()).unwrap_or(u32::MAX);
                continue;
            }
            let zona = if let Some(existing) = existing_zone {
                existing
            } else {
                let created = crate::repositories::PlanoSalaRepository::crear_zona(
                    pool, user_id, &room.name, room.id, 800, 600,
                )
                .await
                .map_err(|e| format!("Error creando zona '{}': {e}", room.name))?;
                zonas_creadas += 1;
                created
            };

            /* Crear mesas que no existan aún en la zona */
            for &table_num in &room.tables {
                let existing =
                    crate::repositories::PlanoSalaRepository::buscar_mesa_por_zona_numero(
                        pool,
                        user_id,
                        &zona.nombre,
                        table_num,
                    )
                    .await
                    .map_err(|e| format!("Error buscando mesa {table_num}: {e}"))?;

                if existing.is_none() {
                    /* [157A-9] crear_mesa recibe CrearMesaRequest que incluye zona_id */
                    let mesa_index = i32::try_from(mesas_creadas).unwrap_or(i32::MAX);
                    let mesa_req = crate::models::CrearMesaRequest {
                        zona_id: zona.id,
                        numero: table_num,
                        pos_x: Some(20 + (mesa_index % 8) * 80),
                        pos_y: Some(20 + (mesa_index / 8) * 80),
                        ancho: Some(60),
                        alto: Some(60),
                        forma: Some("cuadrada".to_string()),
                        min_personas: Some(2),
                        max_personas: Some(4),
                    };
                    if aplicar {
                        crate::repositories::PlanoSalaRepository::crear_mesa(pool, &mesa_req)
                            .await
                            .map_err(|e| format!("Error creando mesa {table_num}: {e}"))?;
                    }
                    mesas_creadas += 1;
                }
            }
        }

        info!(
            "[157A-9] sync_tables completado: {} salones BDP → {zonas_creadas} zonas nuevas, {mesas_creadas} mesas nuevas",
            resp.rooms.len()
        );

        Ok(SyncTablesResult {
            salones_bdp: u32::try_from(resp.rooms.len()).unwrap_or(u32::MAX),
            zonas_creadas,
            mesas_creadas,
            applied: aplicar,
        })
    }
}

/// Resultado del sync de mesas BDP → Glory (F9.4).
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct SyncTablesResult {
    pub salones_bdp: u32,
    pub zonas_creadas: u32,
    pub mesas_creadas: u32,
    pub applied: bool,
}

/// Artículo BDP resuelto para el mapeo.
struct ResolvedArticle {
    id: i64,
    name: String,
    #[allow(dead_code)]
    price: f64,
    #[allow(dead_code)]
    vat_pct: f64,
}

/// Errores clasificados para decidir si reintentar.
enum BdpSyncError {
    /// Rechazo conocido: no se aplicó una operación válida y no se reintenta.
    Rejected(String),
    /// Timeout, HTTP anómalo o JSON inválido: BDP pudo haber aplicado la orden.
    AmbiguousTransport(String),
}

enum OrderSendFailure {
    Rejected(String),
    Ambiguous(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn test_config() -> ConfiguracionRestaurante {
        ConfiguracionRestaurante {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            bdp_base_url: "http://localhost:8068".into(),
            bdp_login: "admin".into(),
            bdp_password: "pass".into(),
            bdp_integrator_code: "TEST1234".into(),
            bdp_sync_enabled: true,
            bdp_pos_id: 31,
            bdp_employee_id: 1,
            bdp_items_profile_id: 1,
            bdp_default_article_code: "1001".into(),
            bdp_default_article_name: "CAFE BOMBON".into(),
            bdp_tender_map: serde_json::json!({"efectivo": "1", "tarjeta": "2"}),
            bdp_order_type_map: serde_json::json!({"comedor": "0", "barra": "0"}),
            bdp_default_customer_code: "GENERIC".into(),
            ..Default::default()
        }
    }

    fn test_order_ctx() -> OrderContext {
        OrderContext {
            tender_id: Some(1),
            order_type: 0,
            customer_code: None,
            customer_name: None,
            customer_phone: None,
        }
    }

    fn test_venta() -> Venta {
        Venta {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            fecha: chrono::NaiveDate::from_ymd_opt(2026, 6, 7).unwrap(),
            comensales: Some(2),
            descripcion: "Cena para 2".into(),
            iva_porcentaje: Decimal::from_str("10.0").unwrap(),
            turno: "cena".into(),
            canal: "comedor".into(),
            metodo_pago: "efectivo".into(),
            importe_base: Decimal::from_str("25.00").unwrap(),
            importe_iva: Decimal::from_str("2.50").unwrap(),
            reserva_id: None,
            cliente_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            haddock_synced: false,
            haddock_synced_at: None,
            haddock_sync_error: None,
            bdp_synced: false,
            bdp_synced_at: None,
            bdp_sync_error: None,
            bdp_order_id: None,
            bdp_order_status: None,
            bdp_invoiced: false,
        }
    }

    #[test]
    fn build_order_uses_venta_total() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let order =
            BdpSyncService::build_order(&config, &venta, &article, None, None, &test_order_ctx());
        let order_json = &order.order;

        /* El total debe ser importe_base + importe_iva = 27.50 */
        let total = order_json.get("Total").and_then(|v| v.as_f64()).unwrap();
        assert!((total - 27.5).abs() < 0.01, "Expected ~27.5, got {total}");

        /* Item[0].Price debe ser el total de la venta */
        let item_price = order_json
            .get("Items")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|i| i.get("Price"))
            .and_then(|v| v.as_f64())
            .unwrap();
        assert!((item_price - 27.5).abs() < 0.01);

        /* Type=0 (Barra) */
        let tipo = order_json.get("Type").and_then(|v| v.as_i64()).unwrap();
        assert_eq!(tipo, 0);

        /* OrderEndType=1 (pendiente) */
        assert_eq!(order.order_end_type, 1);

        /* OrderOperationType=0 (escritura real) */
        assert_eq!(order.order_operation_type, 0);

        /* MarketplaceOrderId <= 15 chars */
        let mid = order_json
            .get("MarketplaceOrderId")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(mid.len() <= 15, "MarketplaceOrderId too long: {mid}");
    }

    #[test]
    fn build_order_uses_configured_employee_and_pos() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let order =
            BdpSyncService::build_order(&config, &venta, &article, None, None, &test_order_ctx());
        assert_eq!(order.employee_id, 1);
        assert_eq!(order.items_profile_id, 1);

        let pos_id = order.order.get("PosId").and_then(|v| v.as_i64()).unwrap();
        assert_eq!(pos_id, 31);
    }

    #[test]
    fn build_order_uses_venta_description_as_item_name() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let order =
            BdpSyncService::build_order(&config, &venta, &article, None, None, &test_order_ctx());
        let item_name = order
            .order
            .get("Items")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|i| i.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(item_name, "Cena para 2");
    }

    #[test]
    fn build_order_multi_item_with_lineas() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let lineas = vec![
            VentaLinea {
                id: uuid::Uuid::new_v4(),
                venta_id: venta.id,
                articulo_codigo: "1001".into(),
                descripcion: "Café bombón".into(),
                cantidad: Decimal::from_str("2").unwrap(),
                precio_unitario: Decimal::from_str("5.00").unwrap(),
                iva_pct: Decimal::from_str("10.0").unwrap(),
                descuento: Decimal::from_str("0.00").unwrap(),
                created_at: Utc::now(),
            },
            VentaLinea {
                id: uuid::Uuid::new_v4(),
                venta_id: venta.id,
                articulo_codigo: "2002".into(),
                descripcion: "Tostada".into(),
                cantidad: Decimal::from_str("1").unwrap(),
                precio_unitario: Decimal::from_str("3.50").unwrap(),
                iva_pct: Decimal::from_str("10.0").unwrap(),
                descuento: Decimal::from_str("0.50").unwrap(),
                created_at: Utc::now(),
            },
        ];

        let order = BdpSyncService::build_order(
            &config,
            &venta,
            &article,
            Some(&lineas),
            Some(&[1001, 2002]),
            &test_order_ctx(),
        );
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();

        /* Debe tener 2 items */
        assert_eq!(items.len(), 2, "Expected 2 items, got {}", items.len());

        /* Primer item: Café bombón x2 */
        assert_eq!(items[0].get("Lin").unwrap(), 1);
        assert_eq!(items[0].get("Id").unwrap().as_i64().unwrap(), 1001);
        assert_eq!(
            items[0].get("Name").unwrap().as_str().unwrap(),
            "Café bombón"
        );
        assert!((items[0].get("Units").unwrap().as_f64().unwrap() - 2.0).abs() < 0.01);
        assert!((items[0].get("Price").unwrap().as_f64().unwrap() - 5.0).abs() < 0.01);
        assert!((items[0].get("Total").unwrap().as_f64().unwrap() - 10.0).abs() < 0.01);

        /* Segundo item: Tostada x1, descuento 0.50 → total = 3.50 - 0.50 = 3.00 */
        assert_eq!(items[1].get("Lin").unwrap(), 2);
        assert_eq!(items[1].get("Id").unwrap().as_i64().unwrap(), 2002);
        assert_eq!(items[1].get("Name").unwrap().as_str().unwrap(), "Tostada");
        assert!((items[1].get("Units").unwrap().as_f64().unwrap() - 1.0).abs() < 0.01);
        assert!((items[1].get("Discount").unwrap().as_f64().unwrap() - 0.50).abs() < 0.01);
        assert!((items[1].get("Total").unwrap().as_f64().unwrap() - 3.00).abs() < 0.01);

        /* VatPct por línea */
        assert!((items[0].get("VatPct").unwrap().as_f64().unwrap() - 10.0).abs() < 0.01);
        assert!((items[1].get("VatPct").unwrap().as_f64().unwrap() - 10.0).abs() < 0.01);

        /* MarketplaceOrderId <= 15 chars */
        let mid = order
            .order
            .get("MarketplaceOrderId")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(mid.len() <= 15, "MarketplaceOrderId too long: {mid}");
    }

    #[test]
    fn extract_first_article_parses_bdp_response() {
        let json = serde_json::json!({
            "ArticlesListData": [{
                "ArtCode": 1001,
                "ArtDescription": "CAFE BOMBON",
                "Price1": 5.0,
                "TAVPer": 10.0
            }],
            "ErrorMessage": ""
        });

        let article = BdpSyncService::extract_first_article(&json, 10.0).unwrap();
        assert_eq!(article.id, 1001);
        assert_eq!(article.name, "CAFE BOMBON");
        assert!((article.price - 5.0).abs() < 0.01);
    }

    /* [R12] Test: extract_first_article usa default_iva_pct cuando BDP no devuelve TAVPer */
    #[test]
    fn extract_first_article_uses_default_iva_when_missing() {
        let json = serde_json::json!({
            "ArticlesListData": [{
                "ArtCode": 1002,
                "ArtDescription": "SIN IVA",
                "Price1": 3.0
            }]
        });

        let article = BdpSyncService::extract_first_article(&json, 21.0).unwrap();
        assert_eq!(article.vat_pct, 21.0, "Debe usar default_iva_pct pasado como parámetro");
    }

    #[test]
    fn sanitize_error_classifies_known_codes() {
        assert!(BdpSyncService::sanitize_error("300035 series").contains("serie"));
        assert!(BdpSyncService::sanitize_error("300008 salón").contains("salón"));
        assert!(BdpSyncService::sanitize_error("301011 too long").contains("MarketplaceOrderId"));
        assert!(BdpSyncService::sanitize_error("301400 caja").contains("caja"));
        assert!(BdpSyncService::sanitize_error("401 Unauthorized").contains("autenticación"));
    }

    /* [F3.2] Test: TenderId se mapea desde metodo_pago via bdp_tender_map */
    #[test]
    fn build_order_maps_tender_id_from_metodo_pago() {
        let config = test_config();
        let venta = test_venta(); /* metodo_pago = "efectivo" */
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let ctx = OrderContext {
            tender_id: Some(1), /* efectivo → 1 */
            order_type: 0,
            customer_code: None,
            customer_name: None,
            customer_phone: None,
        };
        let order = BdpSyncService::build_order(&config, &venta, &article, None, None, &ctx);
        let tender = order
            .order
            .get("TenderId")
            .and_then(|v| v.as_i64())
            .unwrap();
        assert_eq!(tender, 1, "TenderId should be 1 for efectivo");
    }

    /* [F3.2] Test: TenderId no se incluye si es None */
    #[test]
    fn build_order_no_tender_when_none() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let ctx = OrderContext {
            tender_id: None,
            order_type: 0,
            customer_code: None,
            customer_name: None,
            customer_phone: None,
        };
        let order = BdpSyncService::build_order(&config, &venta, &article, None, None, &ctx);
        assert!(
            order.order.get("TenderId").is_none(),
            "TenderId should not be present"
        );
    }

    /* [F3.3] Test: Order type se mapea desde canal */
    #[test]
    fn build_order_uses_order_type_from_canal() {
        let config = test_config();
        let venta = test_venta(); /* canal = "comedor" */
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let ctx = OrderContext {
            tender_id: None,
            order_type: 0, /* comedor → 0 */
            customer_code: None,
            customer_name: None,
            customer_phone: None,
        };
        let order = BdpSyncService::build_order(&config, &venta, &article, None, None, &ctx);
        let tipo = order.order.get("Type").and_then(|v| v.as_i64()).unwrap();
        assert_eq!(tipo, 0);
    }

    /* [F3.1] Test: Customer se incluye cuando hay nombre */
    #[test]
    fn build_order_includes_customer_when_present() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let ctx = OrderContext {
            tender_id: Some(1),
            order_type: 0,
            customer_code: Some("GENERIC".into()),
            customer_name: Some("Juan Pérez".into()),
            customer_phone: Some("600123456".into()),
        };
        let order = BdpSyncService::build_order(&config, &venta, &article, None, None, &ctx);
        let customer = order.order.get("Customer").unwrap();
        assert_eq!(
            customer.get("Name").unwrap().as_str().unwrap(),
            "Juan Pérez"
        );
        assert_eq!(
            customer.get("Phone").unwrap().as_str().unwrap(),
            "600123456"
        );
        assert_eq!(customer.get("Code").unwrap().as_str().unwrap(), "GENERIC");
    }

    /* [F3.1] Test: Customer NO se incluye cuando nombre es None */
    #[test]
    fn build_order_no_customer_when_name_none() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let ctx = OrderContext {
            tender_id: Some(1),
            order_type: 0,
            customer_code: None,
            customer_name: None,
            customer_phone: None,
        };
        let order = BdpSyncService::build_order(&config, &venta, &article, None, None, &ctx);
        assert!(
            order.order.get("Customer").is_none(),
            "Customer should not be present"
        );
    }

    /* [F3.2-3.3] Test: resolve_tender_id y resolve_order_type helpers */
    #[test]
    fn resolve_tender_id_maps_metodo_pago() {
        let config = test_config();
        let mut venta = test_venta();

        venta.metodo_pago = "efectivo".into();
        assert_eq!(BdpSyncService::resolve_tender_id(&venta, &config), Some(1));

        venta.metodo_pago = "tarjeta".into();
        assert_eq!(BdpSyncService::resolve_tender_id(&venta, &config), Some(2));

        venta.metodo_pago = "bizum".into(); /* no está en el map */
        assert_eq!(BdpSyncService::resolve_tender_id(&venta, &config), None);
    }

    #[test]
    fn resolve_order_type_maps_canal() {
        let config = test_config();
        let mut venta = test_venta();

        venta.canal = "comedor".into();
        assert_eq!(BdpSyncService::resolve_order_type(&venta, &config), 0);

        venta.canal = "barra".into();
        assert_eq!(BdpSyncService::resolve_order_type(&venta, &config), 0);

        venta.canal = "delivery".into(); /* no configurado, default 0 */
        assert_eq!(BdpSyncService::resolve_order_type(&venta, &config), 0);
    }

    /* [BDP-TEST-A] Tests adicionales: edge cases de build_order */

    /* build_order con Some(&[]) (vec vacío) debe producir Items vacío,
     * NO caer al fallback legacy. Esto valida que el codegen distingue
     * Some(vec![]) de None correctamente. */
    #[test]
    fn build_order_con_0_lineas_produce_items_vacio() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let lineas_vacias: Vec<VentaLinea> = vec![];
        let order = BdpSyncService::build_order(
            &config,
            &venta,
            &article,
            Some(&lineas_vacias),
            Some(&[]),
            &test_order_ctx(),
        );
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(
            items.len(),
            0,
            "Some(vec![]) should produce empty Items array"
        );
    }

    /* build_order con None cae al fallback legacy: 1 item genérico */
    #[test]
    fn build_order_con_none_produce_fallback_legacy() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let order =
            BdpSyncService::build_order(&config, &venta, &article, None, None, &test_order_ctx());
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(items.len(), 1, "None should produce 1 fallback item");
        assert_eq!(
            items[0].get("Name").unwrap().as_str().unwrap(),
            "Cena para 2",
            "Fallback item Name should be venta.descripcion"
        );
    }

    /* build_order con 1 línea explícita: un solo item custom */
    #[test]
    fn build_order_con_1_linea_explicita() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let lineas = vec![VentaLinea {
            id: uuid::Uuid::new_v4(),
            venta_id: venta.id,
            articulo_codigo: "5001".into(),
            descripcion: "Ensalada César".into(),
            cantidad: Decimal::from_str("1").unwrap(),
            precio_unitario: Decimal::from_str("8.50").unwrap(),
            iva_pct: Decimal::from_str("10.0").unwrap(),
            descuento: Decimal::from_str("0.00").unwrap(),
            created_at: Utc::now(),
        }];

        let order = BdpSyncService::build_order(
            &config,
            &venta,
            &article,
            Some(&lineas),
            Some(&[5001]),
            &test_order_ctx(),
        );
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get("Id").unwrap().as_i64().unwrap(), 5001);
        assert_eq!(
            items[0].get("Name").unwrap().as_str().unwrap(),
            "Ensalada César"
        );
        assert!((items[0].get("Total").unwrap().as_f64().unwrap() - 8.50).abs() < 0.01);
    }

    /* build_order con 3 líneas: valida Lin secuencial y totales individuales */
    #[test]
    fn build_order_con_3_lineas() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let lineas = vec![
            VentaLinea {
                id: uuid::Uuid::new_v4(),
                venta_id: venta.id,
                articulo_codigo: "1001".into(),
                descripcion: "Café bombón".into(),
                cantidad: Decimal::from_str("2").unwrap(),
                precio_unitario: Decimal::from_str("5.00").unwrap(),
                iva_pct: Decimal::from_str("10.0").unwrap(),
                descuento: Decimal::from_str("0.00").unwrap(),
                created_at: Utc::now(),
            },
            VentaLinea {
                id: uuid::Uuid::new_v4(),
                venta_id: venta.id,
                articulo_codigo: "2002".into(),
                descripcion: "Tostada".into(),
                cantidad: Decimal::from_str("1").unwrap(),
                precio_unitario: Decimal::from_str("3.50").unwrap(),
                iva_pct: Decimal::from_str("10.0").unwrap(),
                descuento: Decimal::from_str("0.50").unwrap(),
                created_at: Utc::now(),
            },
            VentaLinea {
                id: uuid::Uuid::new_v4(),
                venta_id: venta.id,
                articulo_codigo: "3003".into(),
                descripcion: "Zumo naranja".into(),
                cantidad: Decimal::from_str("3").unwrap(),
                precio_unitario: Decimal::from_str("2.00").unwrap(),
                iva_pct: Decimal::from_str("10.0").unwrap(),
                descuento: Decimal::from_str("0.00").unwrap(),
                created_at: Utc::now(),
            },
        ];

        let order = BdpSyncService::build_order(
            &config,
            &venta,
            &article,
            Some(&lineas),
            Some(&[1001, 2002, 3003]),
            &test_order_ctx(),
        );
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();

        assert_eq!(items.len(), 3, "Expected 3 items");

        /* Lin secuencial: 1, 2, 3 */
        assert_eq!(items[0].get("Lin").unwrap(), 1);
        assert_eq!(items[1].get("Lin").unwrap(), 2);
        assert_eq!(items[2].get("Lin").unwrap(), 3);

        /* Tercer item: Zumo x3 = 6.00 */
        assert_eq!(items[2].get("Id").unwrap().as_i64().unwrap(), 3003);
        assert_eq!(
            items[2].get("Name").unwrap().as_str().unwrap(),
            "Zumo naranja"
        );
        assert!((items[2].get("Units").unwrap().as_f64().unwrap() - 3.0).abs() < 0.01);
        assert!((items[2].get("Total").unwrap().as_f64().unwrap() - 6.00).abs() < 0.01);
    }

    /* build_order con descuento parcial: descuento se refleja en Total */
    #[test]
    fn build_order_linea_con_descuento_parcial() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 1001,
            name: "CAFE BOMBON".into(),
            price: 5.0,
            vat_pct: 10.0,
        };

        let lineas = vec![VentaLinea {
            id: uuid::Uuid::new_v4(),
            venta_id: venta.id,
            articulo_codigo: "9999".into(),
            descripcion: "Menú del día".into(),
            cantidad: Decimal::from_str("2").unwrap(),
            precio_unitario: Decimal::from_str("12.00").unwrap(),
            iva_pct: Decimal::from_str("10.0").unwrap(),
            descuento: Decimal::from_str("4.00").unwrap(),
            created_at: Utc::now(),
        }];

        let order = BdpSyncService::build_order(
            &config,
            &venta,
            &article,
            Some(&lineas),
            Some(&[9999]),
            &test_order_ctx(),
        );
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();

        /* Total = (12.00 * 2) - 4.00 = 20.00 */
        assert!((items[0].get("Total").unwrap().as_f64().unwrap() - 20.00).abs() < 0.01);
        assert!((items[0].get("Discount").unwrap().as_f64().unwrap() - 4.00).abs() < 0.01);
    }

    /* build_order sin line_article_ids usa article.id como fallback */
    #[test]
    fn build_order_linea_sin_article_ids_usa_fallback() {
        let config = test_config();
        let venta = test_venta();
        let article = ResolvedArticle {
            id: 7777,
            name: "DEFAULT".into(),
            price: 1.0,
            vat_pct: 10.0,
        };

        let lineas = vec![VentaLinea {
            id: uuid::Uuid::new_v4(),
            venta_id: venta.id,
            articulo_codigo: "XXXX".into(),
            descripcion: "Sin mapeo".into(),
            cantidad: Decimal::from_str("1").unwrap(),
            precio_unitario: Decimal::from_str("3.00").unwrap(),
            iva_pct: Decimal::from_str("10.0").unwrap(),
            descuento: Decimal::from_str("0.00").unwrap(),
            created_at: Utc::now(),
        }];

        /* line_article_ids = None → usa article.id (7777) */
        let order = BdpSyncService::build_order(
            &config,
            &venta,
            &article,
            Some(&lineas),
            None,
            &test_order_ctx(),
        );
        let items = order.order.get("Items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(
            items[0].get("Id").unwrap().as_i64().unwrap(),
            7777,
            "Should fallback to article.id when line_article_ids is None"
        );
    }
}
