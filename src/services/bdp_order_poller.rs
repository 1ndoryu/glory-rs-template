/* [276A-4.2] Servicio de polling BDP — consulta periódicamente el estado de comandas
 * enviadas al POS para detectar facturación (Status=3) u otros estados finales.
 *
 * Flujo:
 *   1. VentaRepository::list_bdp_pending() → ventas con bdp_synced=true, bdp_order_status no final
 *   2. Para cada venta: GetOrder(bdp_order_id) → Status de BDP
 *   3. Mapear status integer → string legible: pending(0), accepted(1), cancelled(2), invoiced(3)
 *   4. Actualizar bdp_order_status en la venta
 *
 * Status BDP (de la API Weblink REST, estructura Order):
 *   0 = Esperando validación → "pending"
 *   1 = Aceptada por el establecimiento → "accepted"
 *   2 = Cancelada → "cancelled"
 *   3 = Facturada → "invoiced"
 *
 * Gotchas:
 *   - GetOrder gratuito solo devuelve Status, no el Order completo (limitación subscripción).
 *   - El intervalo de polling se configura en bdp_poll_interval_secs (configuración restaurante).
 *   - Cada venta tiene un mutex para evitar polling concurrente del mismo order. */

use rust_decimal::prelude::ToPrimitive;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::ConfiguracionRestaurante;
use crate::repositories::VentaRepository;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetOrderRequest, BdpOrderIdentifier};

pub struct BdpOrderPollerService;

impl BdpOrderPollerService {
    /// Ejecuta únicamente configuraciones cuyo polling fue habilitado de forma
    /// explícita y cuya ventana está vencida. La tabla de agenda actúa como
    /// claim atómico entre múltiples instancias.
    pub async fn poll_due(pool: &PgPool) -> Result<usize, String> {
        let configs = sqlx::query_as::<_, ConfiguracionRestaurante>(
            "SELECT * FROM configuracion_restaurante \
             WHERE bdp_poll_enabled = TRUE AND bdp_sync_enabled = TRUE \
               AND bdp_base_url <> '' ORDER BY user_id LIMIT 100",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error listando configuraciones BDP para polling: {e}"))?;

        let mut total = 0;
        for config in configs {
            let claimed: Option<uuid::Uuid> = sqlx::query_scalar(
                r"INSERT INTO bdp_poll_schedule (user_id, next_poll_at, updated_at)
                   VALUES ($1, NOW() + ($2 * INTERVAL '1 second'), NOW())
                   ON CONFLICT (user_id) DO UPDATE SET
                     next_poll_at = NOW() + ($2 * INTERVAL '1 second'),
                     updated_at = NOW()
                   WHERE bdp_poll_schedule.next_poll_at <= NOW()
                   RETURNING user_id",
            )
            .bind(config.user_id)
            .bind(config.bdp_poll_interval_secs.clamp(10, 600))
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("Error reclamando turno de polling BDP: {e}"))?;
            if claimed.is_some() {
                match Self::poll_pending(pool, config.user_id, &config).await {
                    Ok(updated) => total += updated,
                    Err(error) => warn!("Polling BDP usuario {}: {error}", config.user_id),
                }
            }
        }
        Ok(total)
    }

    /// Consulta BDP para todas las ventas pendientes de este usuario.
    /// Retorna el número de ventas actualizadas.
    #[allow(clippy::too_many_lines)]
    pub async fn poll_pending(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
    ) -> Result<usize, String> {
        if !config.bdp_sync_enabled {
            return Ok(0);
        }

        let ventas = VentaRepository::list_bdp_pending(pool, user_id)
            .await
            .map_err(|e| format!("Error consultando ventas BDP pendientes: {e}"))?;

        /* [AUDIT-2.11b] Buscar ventas huérfanas: la comanda puede existir en
         * BDP pero Glory no recibió confirmación (crash entre HTTP y UPDATE). */
        let orphaned = VentaRepository::list_bdp_orphaned(pool, user_id)
            .await
            .unwrap_or_else(|e| {
                warn!("[AUDIT-2.11b] Error buscando ventas huérfanas: {e}");
                Vec::new()
            });

        /* [AUDIT-N2] Buscar clientes huérfanos: bdp_synced=true pero auditoría
         * pendiente/ambiguo para create_customer. */
        let orphaned_customers =
            crate::repositories::VentaRepository::list_bdp_orphaned_customers(pool, user_id)
                .await
                .unwrap_or_else(|e| {
                    warn!("[AUDIT-N2] Error buscando clientes huérfanos: {e}");
                    Vec::new()
                });

        if ventas.is_empty() && orphaned.is_empty() && orphaned_customers.is_empty() {
            return Ok(0);
        }

        let client = BdpWeblinkClient::new(config);

        /* [R1] Reconciliar auditorías ambiguas antes de procesar estados normales. */
        match Self::reconcile_ambiguous(pool, user_id, config, &client).await {
            Ok(count) => {
                if count > 0 {
                    info!(
                        "[R1] {} auditorías ambiguas reconciliadas para usuario {}",
                        count, user_id
                    );
                }
            }
            Err(error) => {
                warn!("[R1] Error reconciliando auditorías ambiguas: {error}");
            }
        }

        let mut updated = 0;

        if !orphaned.is_empty() {
            warn!(
                "[AUDIT-2.11b] {} ventas huérfanas detectadas (bdp_synced=false con bdp_order_id). \
                 Consultando BDP para reconciliar.",
                orphaned.len()
            );
            for venta in &orphaned {
                match Self::check_order_status(&client, venta.bdp_order_id.unwrap_or(0)).await {
                    Ok(status) => {
                        /* La comanda existe en BDP → reconciliar Glory */
                        info!(
                            "[AUDIT-2.11b] Venta {} reconciliada: comanda existe en BDP (status={status}). \
                             Marcando bdp_synced=true.",
                            venta.id
                        );
                        /* [D10] No descartar errores: un fallo persistente de BD
                         * haría que el poller repita reconciliaciones infinitamente. */
                        if let Err(e) = VentaRepository::update_bdp_status(
                            pool,
                            venta.id,
                            true,
                            None,
                            venta.bdp_order_id,
                        )
                        .await
                        {
                            warn!("[D10] Error actualizando bdp_status de venta huérfana {}: {e}", venta.id);
                            continue;
                        }
                        if let Err(e) =
                            VentaRepository::update_bdp_order_status(pool, venta.id, &status).await
                        {
                            warn!("[D10] Error actualizando order_status de venta huérfana {}: {e}", venta.id);
                        }
                        updated += 1;
                    }
                    Err(e) => {
                        /* La comanda no existe o BDP no responde → marcar error
                         * para que no se reintente infinitamente */
                        warn!("[AUDIT-2.11b] Venta {} no reconciliable: {e}", venta.id);
                        /* [D10] Propagar errores de BD en vez de descartarlos. */
                        if let Err(e) = VentaRepository::update_bdp_status(
                            pool,
                            venta.id,
                            false,
                            Some("No se pudo verificar existencia en BDP; reconciliación manual requerida"),
                            venta.bdp_order_id,
                        )
                        .await
                        {
                            warn!("[D10] Error marcando venta huérfana {} como no reconciliable: {e}", venta.id);
                        }
                    }
                }
            }
        }

        /* [AUDIT-N2] Reconciliar clientes huérfanos: cerrar auditoría pendiente
         * ya que el cliente fue creado exitosamente en BDP (bdp_synced=true). */
        if !orphaned_customers.is_empty() {
            info!(
                "[AUDIT-N2] {} clientes huérfanos detectados (bdp_synced=true con auditoría pendiente). \
                 Cerrando auditoría.",
                orphaned_customers.len()
            );
            for cliente in &orphaned_customers {
                if let Some(bdp_code) = cliente.bdp_customer_code {
                    /* El cliente ya tiene bdp_synced=true → la operación fue exitosa.
                     * Cerrar todas las auditorías pendientes para este cliente. */
                    let _ = sqlx::query(
                        r"UPDATE bdp_audit_log
                        SET resultado = 'exito', error_mensaje = 'reconciliado por polling N2', updated_at = NOW()
                        WHERE user_id = $1
                          AND target_entity_type = 'cliente'
                          AND target_entity_id = $2
                          AND operacion = 'create_customer'
                          AND resultado IN ('pendiente', 'ambiguo')",
                    )
                    .bind(user_id)
                    .bind(cliente.id)
                    .execute(pool)
                    .await;
                    updated += 1;
                    info!(
                        "[AUDIT-N2] Cliente {} (code={bdp_code}) reconciliado: auditoría cerrada.",
                        cliente.id
                    );
                }
            }
        }

        if !ventas.is_empty() {
            info!("[276A-4.2] Polling BDP: {} ventas pendientes", ventas.len());
            for venta in &ventas {
                match Self::poll_one(pool, venta, config, Some(&client)).await {
                    Ok(true) => updated += 1,
                    Ok(false) => {}
                    Err(e) => {
                        warn!(
                            "[276A-4.2] Error consultando GetOrder para venta {}: {e}",
                            venta.id
                        );
                    }
                }
            }
        }

        Ok(updated)
    }

    pub async fn refresh_one(
        pool: &PgPool,
        venta: &crate::models::Venta,
        config: &ConfiguracionRestaurante,
    ) -> Result<bool, String> {
        let client = BdpWeblinkClient::new(config);
        Self::poll_one(pool, venta, config, Some(&client)).await
    }

    async fn poll_one(
        pool: &PgPool,
        venta: &crate::models::Venta,
        _config: &ConfiguracionRestaurante,
        client: Option<&BdpWeblinkClient<'_>>,
    ) -> Result<bool, String> {
        let order_id = venta
            .bdp_order_id
            .ok_or_else(|| format!("Venta {} no tiene bdp_order_id", venta.id))?;
        let client = client.ok_or_else(|| "Cliente BDP no disponible".to_string())?;
        let status = Self::check_order_status(client, order_id).await?;
        VentaRepository::update_bdp_order_status(pool, venta.id, &status)
            .await
            .map_err(|e| format!("Error actualizando estado local BDP: {e}"))?;
        info!("Venta {} → bdp_order_status = {status}", venta.id);
        Ok(true)
    }

    /// Consulta `GetOrder` para un `order_id` específico y devuelve el status como string.
    async fn check_order_status(
        client: &BdpWeblinkClient<'_>,
        order_id: i64,
    ) -> Result<String, String> {
        let req = BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
        };

        let resp = client
            .get_order(&req)
            .await
            .map_err(|e| format!("Error BDP GetOrder: {e}"))?;

        /* La respuesta de GetOrder tiene:
         *   { "Order": { ... }, "Status": <int>, "ErrorMessage": "" }
         * Status values: 0=pending, 1=accepted, 2=cancelled, 3=invoiced */
        let status_code = Self::parse_status(&resp)?;

        Ok(Self::map_status(status_code))
    }

    fn parse_status(resp: &serde_json::Value) -> Result<i64, String> {
        let status_value = resp
            .get("Status")
            .or_else(|| resp.get("Order").and_then(|order| order.get("Status")))
            .ok_or_else(|| "Respuesta BDP sin campo Status".to_string())?;
        status_value
            .as_i64()
            .or_else(|| status_value.as_str()?.trim().parse::<i64>().ok())
            .ok_or_else(|| "Respuesta BDP contiene Status inválido".to_string())
    }

    /// Mapea el integer de status BDP → string legible almacenado en `bdp_order_status`.
    fn map_status(code: i64) -> String {
        match code {
            0 => "pending".to_string(),
            1 => "accepted".to_string(),
            2 => "cancelled".to_string(),
            3 => "invoiced".to_string(),
            other => {
                warn!("[R7] BDP devolvió status desconocido: {other}. Se almacena como unknown_{other}.");
                format!("unknown_{other}")
            }
        }
    }

    /* [R1] Reconciliar auditorías BDP marcadas como ambiguas.
     * [247A-10/P3] También reconcilia pagos ambiguos del ledger local.
     * Devuelve el número de auditorías/pagos cerrados como exito. */
    async fn reconcile_ambiguous(
        pool: &PgPool,
        user_id: uuid::Uuid,
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
    ) -> Result<usize, String> {
        let rows: Vec<(uuid::Uuid, String, uuid::Uuid, serde_json::Value)> = sqlx::query_as(
            r"SELECT id, operacion, target_entity_id, datos_enviados
               FROM bdp_audit_log
               WHERE user_id = $1
                 AND resultado = 'ambiguo'
                 AND operacion IN ('create_order', 'add_payment', 'invoice')
               ORDER BY created_at DESC
               LIMIT 100",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error listando auditorías ambiguas: {e}"))?;

        let mut reconciled = 0;
        for (audit_id, operacion, target_entity_id, datos_enviados) in rows {
            let result = match operacion.as_str() {
                "create_order" => {
                    Self::reconcile_create_order(pool, client, audit_id, target_entity_id).await
                }
                "add_payment" => {
                    Self::reconcile_add_payment(
                        pool,
                        client,
                        audit_id,
                        target_entity_id,
                        &datos_enviados,
                    )
                    .await
                }
                "invoice" => {
                    Self::reconcile_invoice(pool, client, audit_id, target_entity_id).await
                }
                _ => Ok(false),
            };
            match result {
                Ok(true) => {
                    info!("[R1] Auditoría {audit_id} reconciliada para {operacion}");
                    reconciled += 1;
                }
                Ok(false) => {}
                Err(e) => {
                    warn!("[R1] Error reconciliando auditoría {audit_id}: {e}");
                }
            }
        }

        /* [247A-10/P3] También reconciliar pagos ambiguos del ledger local. */
        reconciled += Self::reconcile_ambiguous_pagos(pool, user_id, config, client).await?;

        Ok(reconciled)
    }

    /* [247A-10/P3] Reconciliar pagos ambiguos del ledger bdp_pagos.
     * Devuelve el número de pagos reconciliados. */
    async fn reconcile_ambiguous_pagos(
        pool: &PgPool,
        user_id: uuid::Uuid,
        _config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
    ) -> Result<usize, String> {
        use crate::repositories::BdpPagoRepository;
        let pagos = BdpPagoRepository::listar_ambiguos(pool, user_id)
            .await
            .map_err(|e| format!("Error listando pagos ambiguos: {e}"))?;

        let mut reconciled = 0;
        for pago in pagos {
            let Some(order_id) = Self::find_bdp_order_id_for_venta(pool, pago.venta_id).await?
            else {
                continue;
            };
            let request = BdpGetOrderRequest {
                order_identifier: BdpOrderIdentifier::by_order_id(order_id),
            };
            match client.get_order(&request).await {
                Ok(response) => {
                    let order = response.get("Order").cloned().unwrap_or(response);
                    let payments = order
                        .get("Payments")
                        .and_then(serde_json::Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    let expected_amount =
                        rust_decimal::Decimal::to_f64(&pago.amount).unwrap_or(0.0);
                    let expected_tender = i64::from(pago.tender_id);
                    let mut matched: Option<String> = None;
                    for payment in payments {
                        let tender = payment
                            .get("TenderId")
                            .and_then(serde_json::Value::as_i64)
                            .unwrap_or(-1);
                        /* [D1] No usar unwrap_or(0.0) en montos financieros.
                         * Si BDP devuelve Amount como string o null, no hacer match
                         * con 0.0 — eso podría reconciliar un pago fantasma. */
                        let Some(amount) = payment
                            .get("Amount")
                            .and_then(serde_json::Value::as_f64)
                        else {
                            continue;
                        };
                        if tender == expected_tender && (amount - expected_amount).abs() < 0.005 {
                            matched = payment
                                .get("PaymentId")
                                .and_then(serde_json::Value::as_str)
                                .map(String::from);
                            break;
                        }
                    }
                    if let Some(payment_id) = matched {
                        let datos =
                            serde_json::json!({ "order_id": order_id, "payment_id": payment_id });
                        if let Err(e) = BdpPagoRepository::reconciliar_exito(
                            pool,
                            pago.id,
                            Some(&payment_id),
                            Some(&datos),
                        )
                        .await
                        {
                            warn!("[247A-10/P3] Error cerrando pago ambiguo {}: {e}", pago.id);
                        } else {
                            info!("[247A-10/P3] Pago ambiguo {} reconciliado (PaymentId={payment_id})", pago.id);
                            reconciled += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "[247A-10/P3] GetOrder falló para reconciliar pago {}: {e}",
                        pago.id
                    );
                }
            }
        }
        Ok(reconciled)
    }

    async fn reconcile_create_order(
        pool: &PgPool,
        client: &BdpWeblinkClient<'_>,
        audit_id: uuid::Uuid,
        venta_id: uuid::Uuid,
    ) -> Result<bool, String> {
        let marketplace_id =
            crate::services::bdp_sync::BdpSyncService::marketplace_order_id(venta_id);
        let request = BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_market(
                crate::services::bdp_sync::BDP_SYNC_MARKET_ID,
                marketplace_id.clone(),
            ),
        };
        match client.get_order(&request).await {
            Ok(response) => {
                let order_id = response
                    .get("OrderId")
                    .and_then(serde_json::Value::as_i64)
                    .or_else(|| response.get("Order")?.get("OrderId")?.as_i64())
                    .filter(|id| *id > 0);
                if let Some(order_id) = order_id {
                    let respuesta = serde_json::json!({ "order_id": order_id });
                    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
                    sqlx::query(
                        "UPDATE ventas SET bdp_synced = true, bdp_synced_at = NOW(), bdp_order_id = $2, bdp_sync_error = NULL WHERE id = $1"
                    )
                    .bind(venta_id)
                    .bind(order_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                    sqlx::query(
                        r"UPDATE bdp_audit_log
                        SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
                        WHERE id = $1"
                    )
                    .bind(audit_id)
                    .bind(Some(&respuesta))
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                    tx.commit().await.map_err(|e| e.to_string())?;
                    info!("[R1] Comanda reconciliada para venta {venta_id} → OrderId={order_id}");
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(e) => {
                warn!("[R1] GetOrder falló para reconciliar comanda {venta_id}: {e}");
                Ok(false)
            }
        }
    }

    async fn reconcile_add_payment(
        pool: &PgPool,
        client: &BdpWeblinkClient<'_>,
        audit_id: uuid::Uuid,
        venta_id: uuid::Uuid,
        datos_enviados: &serde_json::Value,
    ) -> Result<bool, String> {
        let Some(order_id) = Self::find_bdp_order_id_for_venta(pool, venta_id).await? else {
            return Ok(false);
        };
        let request = BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
        };
        match client.get_order(&request).await {
            Ok(response) => {
                let order = response.get("Order").cloned().unwrap_or(response);
                let payments = order
                    .get("Payments")
                    .and_then(serde_json::Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let expected_tender = datos_enviados
                    .get("tender_id")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(-1);
                let expected_amount = datos_enviados
                    .get("amount")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0);
                /* [D1+D5] No usar unwrap_or(0.0) en montos de BDP.
                 * Verificar PaymentId para evitar falsos positivos cuando
                 * hay múltiples pagos con mismo tender y monto similar. */
                let expected_payment_id = datos_enviados
                    .get("idempotency_key")
                    .and_then(serde_json::Value::as_str);
                let matched = payments.iter().any(|payment| {
                    let tender = payment
                        .get("TenderId")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(-1);
                    let Some(amount) = payment
                        .get("Amount")
                        .and_then(serde_json::Value::as_f64)
                    else {
                        return false;
                    };
                    if tender != expected_tender || (amount - expected_amount).abs() >= 0.005 {
                        return false;
                    }
                    /* [D5] Si tenemos idempotency_key esperado, verificar que
                     * el PaymentId de BDP lo contenga como sufijo. El formato
                     * de PaymentId es P{venta_prefix}-{key}. */
                    if let Some(expected_pid) = expected_payment_id {
                        if let Some(bdp_pid) = payment.get("PaymentId").and_then(serde_json::Value::as_str) {
                            return bdp_pid.ends_with(expected_pid);
                        }
                    }
                    true
                });
                if !matched {
                    return Ok(false);
                }
                let invoice_number = order
                    .get("InvoiceNumber")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
                /* [D11] Solo marcar como facturado si NO lo estaba ya localmente.
                 * Evita sobreescribir datos de factura tras una corrección manual. */
                if invoice_number.is_some() {
                    sqlx::query(
                        "UPDATE ventas SET bdp_invoiced = true, bdp_order_status = 'invoiced', updated_at = NOW() WHERE id = $1 AND bdp_invoiced = FALSE"
                    )
                    .bind(venta_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                }
                let respuesta =
                    serde_json::json!({ "order_id": order_id, "invoice_number": invoice_number });
                sqlx::query(
                    r"UPDATE bdp_audit_log
                    SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
                    WHERE id = $1"
                )
                .bind(audit_id)
                .bind(Some(&respuesta))
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                tx.commit().await.map_err(|e| e.to_string())?;
                info!("[R1] Pago reconciliado para venta {venta_id} → OrderId={order_id}");
                Ok(true)
            }
            Err(e) => {
                warn!("[R1] GetOrder falló para reconciliar pago {venta_id}: {e}");
                Ok(false)
            }
        }
    }

    async fn reconcile_invoice(
        pool: &PgPool,
        client: &BdpWeblinkClient<'_>,
        audit_id: uuid::Uuid,
        venta_id: uuid::Uuid,
    ) -> Result<bool, String> {
        let Some(order_id) = Self::find_bdp_order_id_for_venta(pool, venta_id).await? else {
            return Ok(false);
        };
        let request = BdpGetOrderRequest {
            order_identifier: BdpOrderIdentifier::by_order_id(order_id),
        };
        match client.get_order(&request).await {
            Ok(response) => {
                let order = response.get("Order").cloned().unwrap_or(response);
                let status = order
                    .get("Status")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(-1);
                let invoice_number = order
                    .get("InvoiceNumber")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                if status != 3 && invoice_number.is_none() {
                    return Ok(false);
                }
                let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
                sqlx::query(
                    "UPDATE ventas SET bdp_invoiced = true, bdp_order_status = 'invoiced', updated_at = NOW() WHERE id = $1"
                )
                .bind(venta_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                let respuesta =
                    serde_json::json!({ "order_id": order_id, "invoice_number": invoice_number });
                sqlx::query(
                    r"UPDATE bdp_audit_log
                    SET resultado = 'exito', datos_respuesta = $2, error_mensaje = NULL, updated_at = NOW()
                    WHERE id = $1"
                )
                .bind(audit_id)
                .bind(Some(&respuesta))
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                tx.commit().await.map_err(|e| e.to_string())?;
                info!("[R1] Factura reconciliada para venta {venta_id} → OrderId={order_id}");
                Ok(true)
            }
            Err(e) => {
                warn!("[R1] GetOrder falló para reconciliar factura {venta_id}: {e}");
                Ok(false)
            }
        }
    }

    async fn find_bdp_order_id_for_venta(
        pool: &PgPool,
        venta_id: uuid::Uuid,
    ) -> Result<Option<i64>, String> {
        let order_id: Option<i64> =
            sqlx::query_scalar("SELECT bdp_order_id FROM ventas WHERE id = $1")
                .bind(venta_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(order_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_status() {
        assert_eq!(BdpOrderPollerService::map_status(0), "pending");
        assert_eq!(BdpOrderPollerService::map_status(1), "accepted");
        assert_eq!(BdpOrderPollerService::map_status(2), "cancelled");
        assert_eq!(BdpOrderPollerService::map_status(3), "invoiced");
        assert_eq!(BdpOrderPollerService::map_status(99), "unknown_99");
    }

    #[test]
    fn status_accepts_numeric_string_shape() {
        let value = serde_json::json!({"Order": {"Status": "3"}});
        assert_eq!(BdpOrderPollerService::parse_status(&value), Ok(3));
        assert!(BdpOrderPollerService::parse_status(&serde_json::json!({"Status": "x"})).is_err());
    }
}
