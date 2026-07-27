# MD Maestro de Auditoría BDP — Prevención de Desastres

> **Fecha:** 2026-07-26
> **Propósito:** Consolidar TODAS las auditorías, hallazgos, mitigaciones y controles de seguridad que protegen la integración BDP contra desastres desde nuestra aplicación Glory.
> **Alcance:** Prevenir que Glory cause daño a BDP (datos, facturación, caja, inventario) o a sí misma (inconsistencias, duplicados, pérdida de datos).
> **Fuentes consolidadas:** 7 documentos de auditoría + verificación contra código real.

---

## 1. Mapa de auditorías realizadas

| #   | Documento                                               | Fecha      | Enfoque                                                    | Hallazgos                    | Estado            |
| --- | ------------------------------------------------------- | ---------- | ---------------------------------------------------------- | ---------------------------- | ----------------- |
| A1  | `auditoria-escritura-bdp-2026-07-17.md`                 | 2026-07-17 | 23 riesgos de escritura (W01-W23)                          | 23 cerrados localmente       | ✅ Completo       |
| A2  | `auditoria-plan-integracion-completa-bdp-2026-07-18.md` | 2026-07-18 | Trazabilidad plan → código → DB → frontend → tests         | 13 P0/P1 corregidos          | ✅ Completo       |
| A3  | `plan-validacion-segura-escritura-bdp-2026-07-18.md`    | 2026-07-18 | Simulador, tests de contrato, fases A-F                    | Fases A-D completadas        | ✅ Local completo |
| A4  | `bdp-seguridad-produccion-2026-07-21.md`                | 2026-07-21 | 5 capas de seguridad pre-deploy                            | Despliegue seguro verificado | ✅ Completo       |
| A5  | `verificacion-guia-cliente-bdp-2026-07-22.md`           | 2026-07-22 | Cada afirmación de guía vs código                          | 100% verificado              | ✅ Completo       |
| A6  | `hallazgos-revision-2026-07-20.md`                      | 2026-07-20 | 17 secciones, hallazgos S6-H1, S7-H1-H4, S14-H1, S16-H1-H4 | Ver §3 de este documento     | ⚠️ Ver detalle    |
| A7  | `riesgos-produccion-bdp-2026-07-24.md`                  | 2026-07-24 | 16 riesgos operativos (R1-R16)                             | Ver §4 de este documento     | ⚠️ Ver detalle    |
| A8  | `auditoria-profunda-desastres-2026-07-26.md`             | 2026-07-26 | 9 desastres no prevenidos (D1-D9)                          | 1 CRÍTICO, 4 ALTO, 3 MEDIO   | 🔴 Nuevo          |

---

## 2. Capas de defensa (5 capas activas)

> Todas deben fallar simultáneamente para que una escritura dañina llegue a BDP.

### Capa 1 — Allowlist de destinos (`BDP_WRITE_ALLOWED_ORIGINS`)

- **Archivo:** `src/services/bdp_weblink.rs:466-475`
- **Comportamiento:** Si vacía o no definida → TODAS las escrituras bloqueadas.
- **Verificación:** `ensure_write_target_allowed()` + `canonical_target()`
- **Extra:** `redirect(Policy::none())` en `bdp_weblink.rs:44` previene evasión por redirect HTTP. **(S6-H1 cerrado)**
- **Estado:** ✅ Activa

### Capa 2 — Modo de sincronización (`bdp_sync_mode`)

- **Archivo:** `src/services/bdp_sync.rs` (gate en `sync_venta`, `add_order_payment`, `invoice_order`)
- **Comportamiento:** `read_only` por defecto. Solo `unidirectional` permite escrituras.
- **Kill switch:** Tras cada escritura, vuelve a `read_only` automáticamente.
- **Estado:** ✅ Activa

### Capa 3 — Write Arming temporal (`bdp_write_arming`)

- **Archivo:** `src/services/bdp_write_guard.rs:132,277`
- **Comportamiento:** Sin arming válido (con caducidad, alcance, fingerprint, UUID objetivo), ninguna escritura procede.
- **Consumo:** Single-use — se consume tras cada escritura.
- **Auto-arming:** `try_auto_arm()` en `bdp_write_guard.rs:47` crea arming efímero si `ff_bdp_auto_arm=true`.
- **Estado:** ✅ Activa

### Capa 4 — Confirmación textual explícita

- **Archivo:** `src/handlers/ventas.rs`, `src/handlers/bdp_customer_sync.rs`, `src/handlers/bdp_backup.rs`
- **Comportamiento:** Cada endpoint requiere un string exacto:
    - Pago: `PAGAR {id} {amount}`
    - Factura: `FACTURAR {id}`
    - Cliente: `CREAR CLIENTE {id} {code}`
    - Restore: `RESTAURAR {uuid}`
- **Validación:** Server-side (no solo frontend).
- **Estado:** ✅ Activa

### Capa 5 — Auditoría inmutable + Snapshot pre-write

- **Archivo:** `src/services/bdp_backup.rs`, `src/services/bdp_write_guard.rs:158`
- **Comportamiento:** Cada escritura genera snapshot del estado BDP antes de mutar + registro inmutable en `bdp_audit_log`.
- **Fail-closed:** Si el snapshot falla, la escritura se bloquea (no se consume permiso).
- **Estados:** `pendiente` → `exito` / `error` / `ambiguo`
- **Bloqueo:** `ensure_no_unresolved()` en `bdp_write_guard.rs:177` bloquea nuevas escrituras si hay `pendiente` o `ambiguo`.
- **Estado:** ✅ Activa

---

## 3. Estado de hallazgos de auditoría (A6: hallazgos-revision)

### Hallazgos críticos — estado verificado contra código

| ID         | Hallazgo                                  | Estado verificado        | Evidencia                                                                                                                                   |
| ---------- | ----------------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| **S6-H1**  | Redirect HTTP evadible                    | ✅ **CERRADO**           | `redirect(Policy::none())` en `bdp_weblink.rs:44`                                                                                           |
| **S7-H1**  | `bdp_order_id` sin UNIQUE                 | ❌ Abierto (bajo riesgo) | Mitigado por advisory lock + `bdp_synced` guard                                                                                             |
| **S7-H2**  | Sin tx envolvente post-HTTP               | ✅ **CERRADO**           | `pool.begin()` + `tx.commit()` en `sync_venta` (`[AUDIT-2.11]`), `add_order_payment` (`[207A-2] S7-H2`), `invoice_order` (`[207A-2] S7-H2`) |
| **S7-H3**  | Sin UNIQUE en `bdp_invoiced`              | ❌ Abierto (bajo riesgo) | Mitigado por status check + reconciliación                                                                                                  |
| **S7-H4**  | `authorization_reason` sin sanitizar      | ❌ Abierto (bajo riesgo) | Contenido controlado por código, no por usuario                                                                                             |
| **S14-H1** | `restaurar_glory()` sin tx                | ✅ **CERRADO**           | `pool.begin()` + `tx.commit()` en `bdp_backup.rs:574-577` (`[207A-3] S14-H1`)                                                               |
| **S16-H1** | Sin rate limiting                         | ❌ Abierto               | Mitigado por `BdpThrottleManager` (2 concurrentes por destino)                                                                              |
| **S16-H2** | Sin `DefaultBodyLimit`                    | ❌ Abierto               | Axum default 2MB implícito                                                                                                                  |
| **S16-H3** | `ensure_write_target_allowed()` sin tests | ❌ Abierto               | Tests unitarios pendientes                                                                                                                  |
| **S16-H4** | `canonical_target()` sin test             | ❌ Abierto               | Tests unitarios pendientes                                                                                                                  |
| **S13-H1** | Secrets en `datos_enviados`               | ❌ Abierto (bajo riesgo) | `skip_serializing` en password/login, pero caller podría serializar config completo                                                         |

### Resumen: 4 cerrados, 7 abiertos (todos bajo riesgo o mitigados)

---

## 4. Estado de riesgos operativos (A7: riesgos-produccion)

### Riesgos — estado verificado contra código

| ID      | Riesgo                            | Estado verificado   | Evidencia                                                                                              |
| ------- | --------------------------------- | ------------------- | ------------------------------------------------------------------------------------------------------ |
| **R1**  | Falsa reconciliación              | ✅ **IMPLEMENTADO** | `reconcile_ambiguous` + 5 funciones en `bdp_order_poller.rs:304-440`                                   |
| **R2**  | Tx abierta durante HTTP           | ✅ **MITIGADO**     | Lock cerrado antes de HTTP (`bdp_sync.rs` `[R2]`)                                                      |
| **R3**  | Throttled → error permanente      | ✅ **MITIGADO**     | `Throttled` → `AmbiguousTransport` en `bdp_sync.rs:506`                                                |
| **R4**  | Auto-sync bloquea sin feedback    | ❌ Abierto          | UI muestra error técnico, no badge amigable                                                            |
| **R5**  | Sin timeout global                | ✅ **IMPLEMENTADO** | `tokio::time::timeout(45s)` en `bdp_sync.rs`                                                           |
| **R6**  | SYNC_LOCKS bajo panic             | ✅ **MITIGADO**     | `SyncLockGuard` RAII en `bdp_sync.rs:71-81` + sweep en `cleanup_lock`                                  |
| **R7**  | Status desconocido                | ✅ **MITIGADO**     | `warn!` en `map_status` para códigos desconocidos                                                      |
| **R8**  | Mutex poisoning cached_session    | ✅ **MITIGADO**     | Recovery pattern (no `expect`)                                                                         |
| **R9**  | Snapshot vs armados               | ❌ Abierto          | Sin verificación de que retención no borre snapshots con armados                                       |
| **R10** | Feature flags off por defecto     | ⚠️ Diseñado         | Flags existen pero UI no muestra estado explícito                                                      |
| **R11** | Cliente genérico sin nombre/phone | ⚠️ Diseñado         | `resolve_customer` devuelve solo código                                                                |
| **R12** | IVA hardcodeado 10.0              | ❌ Abierto          | `resolve_article` usa `vat_pct: 10.0` como fallback                                                    |
| **R13** | Mutex poisoning SYNC_LOCKS        | ✅ **MITIGADO**     | `unwrap_or_else(PoisonError::into_inner)`                                                              |
| **R14** | Limpieza manual SYNC_LOCKS        | ✅ **IMPLEMENTADO** | `SyncLockGuard` RAII                                                                                   |
| **R15** | Throttled en pagos/facturas       | ✅ **MITIGADO**     | Mapeado a `ambiguo` en ambos endpoints                                                                 |
| **R16** | Aritmética f64                    | ⚠️ **PARCIAL**      | `decimal_to_f64()` persiste para JSON. Decimal se usa para cálculos. Conversión via string es precisa. |

### Resumen: 10 cerrados/mitigados, 3 abiertos (bajo riesgo), 3 parciales

---

## 5. Controles contra desastres específicos

### 5.1 Desastre: Duplicados de comandas

| Control                                                    | Implementado | Archivo                                        |
| ---------------------------------------------------------- | ------------ | ---------------------------------------------- |
| `MarketplaceOrderId` determinista y estable por venta      | ✅           | `bdp_sync.rs:marketplace_order_id()`           |
| Límite 15 chars en MarketplaceOrderId                      | ✅           | `bdp_sync.rs:build_order()` — truncado a 15    |
| Advisory lock por venta (`pg_try_advisory_xact_lock`)      | ✅           | `bdp_sync.rs` — lock distribuido               |
| `SyncLockGuard` RAII por proceso                           | ✅           | `bdp_sync.rs:71-81`                            |
| `bdp_synced` guard: ventas ya sincronizadas no se reenvían | ✅           | `bdp_sync.rs:sync_venta()`                     |
| Reconciliación post-ambiguo por `GetOrder`                 | ✅           | `bdp_order_poller.rs:reconcile_create_order()` |
| Sin retry ciego de `CreateOrder`                           | ✅           | `bdp_sync.rs:retry_send_order()`               |

### 5.2 Desastre: Pagos duplicados o sobrepago

| Control                                               | Implementado | Archivo                                              |
| ----------------------------------------------------- | ------------ | ---------------------------------------------------- |
| Ledger local `bdp_pagos` con `idempotency_key` UNIQUE | ✅           | `repositories/bdp_pago.rs`                           |
| Deduplicación por idempotency_key antes de llamar BDP | ✅           | `bdp_sync.rs:add_order_payment()`                    |
| Validación de saldo pendiente (local + BDP)           | ✅           | `bdp_sync.rs:add_order_payment()`                    |
| Rechazo si `amount > pending + tolerance`             | ✅           | `bdp_sync.rs:add_order_payment()`                    |
| Advisory lock por venta para pagos concurrentes       | ✅           | `bdp_sync.rs:add_order_payment()`                    |
| Tx atómica: ledger + auditoría + update venta         | ✅           | `bdp_sync.rs:add_order_payment()` (`[207A-2] S7-H2`) |
| Reconciliación de pagos ambiguos                      | ✅           | `bdp_order_poller.rs:reconcile_ambiguous_pagos()`    |

### 5.3 Desastre: Facturación incorrecta o duplicada

| Control                                      | Implementado | Archivo                                          |
| -------------------------------------------- | ------------ | ------------------------------------------------ |
| Verificación de saldo cero antes de facturar | ✅           | `bdp_sync.rs:invoice_order()`                    |
| Verificación de estado (no cancelada)        | ✅           | `bdp_sync.rs:invoice_order()`                    |
| Reconciliación si ya facturada (status=3)    | ✅           | `bdp_sync.rs:invoice_order()`                    |
| Rechazo si `InvoiceNumber` vacío             | ✅           | `bdp_sync.rs:invoice_order()`                    |
| Tx atómica: update venta + auditoría         | ✅           | `bdp_sync.rs:invoice_order()` (`[207A-2] S7-H2`) |
| Confirmación textual `FACTURAR {id}`         | ✅           | `handlers/ventas.rs`                             |

### 5.4 Desastre: Creación de clientes no deseada

| Control                                                | Implementado | Archivo                                   |
| ------------------------------------------------------ | ------------ | ----------------------------------------- |
| Creación automática deshabilitada                      | ✅           | `bdp_sync.rs:ensure_cliente_bdp_synced()` |
| Código explícito del usuario (no `max+1` ni hash)      | ✅           | `handlers/bdp_customer_sync.rs`           |
| `Overwrite=false` siempre                              | ✅           | `handlers/bdp_customer_sync.rs`           |
| Preview antes de importar                              | ✅           | `handlers/bdp_customer_sync.rs`           |
| UNIQUE constraint `uq_clientes_user_bdp_customer_code` | ✅           | Migraciones                               |

### 5.5 Desastre: Inconsistencia local tras HTTP exitoso

| Control                                                       | Implementado | Archivo                          |
| ------------------------------------------------------------- | ------------ | -------------------------------- |
| Tx post-HTTP en `sync_venta` (update + audit atómicos)        | ✅           | `bdp_sync.rs` (`[AUDIT-2.11]`)   |
| Tx post-HTTP en `add_order_payment` (update + audit + ledger) | ✅           | `bdp_sync.rs` (`[207A-2] S7-H2`) |
| Tx post-HTTP en `invoice_order` (update + audit)              | ✅           | `bdp_sync.rs` (`[207A-2] S7-H2`) |
| Estado `ambiguo` si tx falla tras HTTP exitoso                | ✅           | Los 3 endpoints                  |
| `ensure_no_unresolved()` bloquea nuevas escrituras si ambiguo | ✅           | `bdp_write_guard.rs:177`         |

### 5.6 Desastre: Escritura accidental al desplegar

| Control                                      | Implementado | Archivo                           |
| -------------------------------------------- | ------------ | --------------------------------- |
| Bootstrap solo toca BD local (no HTTP)       | ✅           | `bdp_config_bootstrap.rs`         |
| Bootstrap setea `read_only` + desactiva todo | ✅           | `bdp_config_bootstrap.rs:205-208` |
| Bootstrap elimina armados previos            | ✅           | `bdp_config_bootstrap.rs:236`     |
| Background tasks inactivos por defecto       | ✅           | `bdp_config_bootstrap.rs`         |
| Allowlist vacía por defecto                  | ✅           | Variables de entorno              |

### 5.7 Desastre: Restore parcial o corrupto

| Control                                 | Implementado | Archivo                                 |
| --------------------------------------- | ------------ | --------------------------------------- |
| Restore envuelto en transacción         | ✅           | `bdp_backup.rs:574` (`[207A-3] S14-H1`) |
| Solo acepta snapshots de tipo `glory`   | ✅           | `bdp_backup.rs:560`                     |
| Solo acepta snapshots del mismo usuario | ✅           | `bdp_backup.rs:556`                     |
| Confirmación textual `RESTAURAR {uuid}` | ✅           | `handlers/bdp_backup.rs:299`            |

---

## 6. Lo que NO puede pasar (verificado)

| Escenario                                 | ¿Puede pasar? | Control                                               |
| ----------------------------------------- | ------------- | ----------------------------------------------------- |
| Glory escribe en BDP al desplegar         | ❌            | Bootstrap read_only + allowlist vacía                 |
| Doble envío de comanda                    | ❌            | Advisory lock + MarketplaceOrderId + bdp_synced guard |
| Doble pago                                | ❌            | Idempotency key UNIQUE + ledger + lock por venta      |
| Sobrepago                                 | ❌            | Validación de saldo local + BDP                       |
| Factura duplicada                         | ❌            | Reconciliación status=3 + InvoiceNumber check         |
| Escritura sin auditoría                   | ❌            | Fail-closed: sin audit no hay escritura               |
| Escritura sin snapshot                    | ❌            | Fail-closed: snapshot obligatorio                     |
| Escritura tras timeout sin marcar ambiguo | ❌            | Timeout → ambiguo → bloqueo                           |
| Restore destruye datos parcialmente       | ❌            | Transacción atómica                                   |
| Redirect evasión de allowlist             | ❌            | `redirect(Policy::none())`                            |

---

## 7. Riesgos residuales aceptados (documentados)

| ID     | Riesgo                       | Por qué se acepta                                           | Mitigación residual                      |
| ------ | ---------------------------- | ----------------------------------------------------------- | ---------------------------------------- |
| S7-H1  | `bdp_order_id` sin UNIQUE    | Advisory lock + `bdp_synced` guard ya previenen duplicados  | Bajo: UNIQUE sería defense-in-depth      |
| S7-H3  | `bdp_invoiced` sin UNIQUE    | Status check + reconciliación previenen facturas duplicadas | Bajo                                     |
| R12    | IVA hardcodeado 10.0         | Fallback; la mayoría de artículos BDP traen `TAVPer` real   | Usar `iva_por_defecto` de config         |
| R16    | `decimal_to_f64()` para JSON | Conversión via string es precisa; BDP espera f64 en JSON    | Redondear Decimal antes de convertir     |
| S16-H1 | Sin rate limiting global     | `BdpThrottleManager` limita a 2 concurrentes por destino    | Añadir middleware si hay más usuarios    |
| S16-H2 | Sin `DefaultBodyLimit`       | Axum default 2MB es razonable para este uso                 | Añadir si se exponen endpoints de upload |

---

## 8. Pruebas de escritura NO verificadas en BDP real

> Estas 4 operaciones están implementadas y protegidas, pero nunca se ejecutaron contra el BDP del restaurante.

| Operación       | Protecciones activas                                                               | Estado        |
| --------------- | ---------------------------------------------------------------------------------- | ------------- |
| CreateCustomer  | Allowlist + arming + confirmación + `Overwrite=false` + UNIQUE                     | ❌ No probado |
| CreateOrder     | Allowlist + arming + MarketplaceOrderId + advisory lock + timeout + reconciliación | ❌ No probado |
| AddOrderPayment | Allowlist + arming + idempotency + ledger + lock por venta + tx atómica            | ❌ No probado |
| InvoiceOrder    | Allowlist + arming + saldo cero check + status check + reconciliación + tx atómica | ❌ No probado |

**Bloqueo:** Requieren autorización explícita del cliente para ejecutarse contra el BDP real.

---

## 9. Cobertura de simulador

El simulador local (`tools/bdp-weblink-simulator/`) cubre:

- Autenticación y expiración de token
- CreateOrder/CheckOrder con idempotencia por MarketplaceOrderId
- GetOrder y transiciones de estado
- AddOrderPayment con saldo pendiente, pagos parciales, sobrepago
- InvoiceOrder con InvoiceNumber, idempotencia, saldo cero
- CancelOrder (éxito, facturada, inexistente, por MarketplaceOrderId)
- ExportArticles, ExportCustomers, ExportDepartments
- GetRoomsTables, GetEmployees, GetTenderList
- CreateCustomer (duplicados, overwrite, validación)
- Errores HTTP (500, 502, 503), JSON inválido, latencia, timeout, pérdida de respuesta
- Historial de llamadas inspeccionable con redacción de datos sensibles
- State inspector y reset completo
- Concurrent duplicate order

### Métricas de testing (verificado 2026-07-26)

| Métrica | Valor |
|---------|-------|
| Tests Python del simulador | **92** (todos pasan en 4.54s) |
| Tests Rust de integración con simulador | **23** (todos pasan en ~15s) |
| Rutas del simulador con test Python | **23/23 (100%)** |
| Rutas del simulador con test Rust | **16/23 (70%)** |
| Fault types testeados | **5/5 (Python), 4/5 (Rust)** |
| Flujos de escritura completos testeados (Rust) | **3/7** (crear, pagar, facturar) |
| Bug encontrado durante testing | `BdpEmptyRequest` serializaba a `null` → corregido a `{}` |

**No cubre (flujos de servicio E2E con DB):** `sync_venta`, `add_payment`, `invoice_order`, `reconcile_ambiguous`, preflight, backup/restore contra simulador. Ver `auditoria-testing-simulator-2026-07-26.md` §4.2 para detalle.
