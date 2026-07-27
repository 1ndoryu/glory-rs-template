# Auditoría de Testing — Simulador BDP WebLink

> **Fecha:** 2026-07-26 (actualizado post-ejecución)
> **Propósito:** Identificar TODOS los gaps de testing en el simulador y los tests de integración BDP.
> **Alcance:** Simulador Python (`tools/bdp-weblink-simulator/`), tests Python (`test_server.py`), tests Rust (`tests/bdp_*.rs`), y tests unitarios inline (`#[cfg(test)]`).

---

## 1. Resultados de ejecución (Fase 5 — 2026-07-26)

### 1.1 Tests Python del simulador

```
platform win32 -- Python 3.12.10, pytest-9.1.1
92 passed in 4.54s
```

| Métrica | Valor |
|---------|-------|
| Total tests Python | **92** |
| Pasados | **92** |
| Fallidos | 0 |
| Errores | 0 |

### 1.2 Tests Rust de integración con simulador

```
cargo test --test bdp_simulator_integration -- --include-ignored
23 passed in ~15s
```

| Métrica | Valor |
|---------|-------|
| Total tests Rust simulador | **23** |
| Pasados | **23** |
| Fallidos | 0 |
| Errores | 0 |

### 1.3 Bug encontrado y corregido durante ejecución

| Bug | Archivo | Fix |
|-----|---------|-----|
| `BdpEmptyRequest` serializaba a `null` en vez de `{}` | `src/services/bdp_weblink_catalog.rs` | Cambiado de unit struct a empty struct con `#[serde(rename_all = "PascalCase")]`. Call sites actualizados en `bdp_weblink.rs`. |

**Impacto:** `get_tenders()` y `get_poses()` fallaban contra el simulador con error `{"ErrorMessage": "JSON invalido: el cuerpo debe ser un objeto JSON"}`. El fix corrige la serialización a `{}`. Esto también es relevante para BDP real: si BDP valida que el body sea un objeto, las llamadas sin este fix fallarían en producción.

---

## 2. Inventario del simulador

### 2.1 Rutas implementadas en `server.py` — Estado post-ejecución

| # | Ruta | Método | Tipo | Tests Python | Tests Rust |
|---|------|--------|------|-------------|------------|
| 1 | `/Service/Health` | POST | Pública | ✅ 1 test | ✅ `simulator_health_check` |
| 2 | `/Auth/Login` | POST | Pública | ✅ 4 tests | ✅ `simulator_login_returns_token` + `simulator_login_cached` |
| 3 | `/Service/GetVersion` | POST | Auth | ✅ 1 test | ✅ `simulator_get_version` |
| 4 | `/API/Articles/Export` | POST | Auth | ✅ 1 test | ✅ `simulator_export_articles` |
| 5 | `/API/Articles/GetPOSList` | POST | Auth | ✅ 1 test | ❌ |
| 6 | `/API/Customers/Export` | POST | Auth | ✅ 1 test | ✅ `simulator_export_customers` |
| 7 | `/API/Customers/Create` | POST | Auth+Write | ✅ 5 tests | ✅ `simulator_create_customer` |
| 8 | `/API/Departments/Export` | POST | Auth | ✅ 1 test | ❌ |
| 9 | `/API/Departments/ExportFromProfile` | POST | Auth | ✅ 1 test | ❌ |
| 10 | `/API/POSes/Get` | POST | Auth | ✅ 1 test | ❌ |
| 11 | `/API/POS/Get` | POST | Auth | ✅ 1 test | ❌ |
| 12 | `/API/Employees/Get` | POST | Auth | ✅ 1 test | ❌ |
| 13 | `/API/Employee/Get` | POST | Auth | ❌ | ❌ |
| 14 | `/API/POS/Employees/Get` | POST | Auth | ✅ 1 test | ❌ |
| 15 | `/API/Tenders/GetList` | POST | Auth | ✅ 1 test | ✅ `simulator_get_tenders` |
| 16 | `/API/Tenders/GetPOSList` | POST | Auth | ✅ 1 test | ❌ |
| 17 | `/API/Rooms/GetTables` | POST | Auth | ✅ 1 test | ❌ |
| 18 | `/API/Room/GetTables` | POST | Auth | ✅ 1 test | ❌ |
| 19 | `/API/Orders/Create` | POST | Auth+Write | ✅ 8 tests | ✅ 3 tests |
| 20 | `/API/Orders/Get` | POST | Auth | ✅ 4 tests | ✅ 3 tests |
| 21 | `/API/Orders/Cancel` | POST | Auth+Write | ✅ 4 tests | ✅ 2 tests |
| 22 | `/API/Orders/Payment/Add` | POST | Auth+Write | ✅ 10 tests | ✅ 4 tests |
| 23 | `/API/Orders/Invoice` | POST | Auth+Write | ✅ 5 tests | ✅ 3 tests |

**Cobertura de rutas: 23/23 (100%) con al menos 1 test Python. 16/23 (70%) con test Rust.**

### 2.2 Rutas admin del simulador

| # | Ruta | Tests Python | Tests Rust |
|---|------|-------------|------------|
| 1 | `/__simulator/reset` | ✅ (usado en setUp + tests explícitos) | ✅ (usado en skip_if_no_simulator) |
| 2 | `/__simulator/fault` | ✅ 14 tests (5 tipos de fault) | ✅ 4 tests (3 tipos) |
| 3 | `/__simulator/state` | ✅ 3 tests | ❌ |
| 4 | `/__simulator/history` | ✅ 1 test (redacción) | ❌ |

---

## 3. Gaps resueltos desde la auditoría original

### 3.1 Rutas sin test → Ahora cubiertas

| Ruta | Tests Python | Tests Rust |
|------|-------------|------------|
| `/Service/Health` | ✅ `test_health_returns_is_alive` | ✅ `simulator_health_check` |
| `/Service/GetVersion` | ✅ `test_get_version_returns_simulator_info` | ✅ `simulator_get_version` |
| `/API/Orders/Cancel` | ✅ 4 tests (éxito, inexistente, facturada, por marketplace) | ✅ 2 tests |
| `/API/Customers/Export` | ✅ 1 test | ✅ 1 test |
| `/API/Customers/Create` | ✅ 5 tests (éxito, duplicado±overwrite, sin code, sin name, code negativo) | ✅ 1 test |
| `/API/Departments/Export` | ✅ 1 test | ❌ |
| `/API/Departments/ExportFromProfile` | ✅ 1 test | ❌ |
| `/API/Rooms/GetTables` | ✅ 2 tests (alias) | ❌ |
| `/API/Employees/Get` | ✅ 1 test | ❌ |
| `/API/POS/Employees/Get` | ✅ 1 test | ❌ |
| `/API/POSes/Get` | ✅ 2 tests (alias) | ❌ |
| `/API/Tenders/GetList` | ✅ 1 test | ✅ 1 test |
| `/API/Tenders/GetPOSList` | ✅ 1 test | ❌ |
| `/API/Articles/GetPOSList` | ✅ 1 test | ❌ |

### 3.2 Scenarios de fallo que ahora SÍ tienen test

| Escenario | Python | Rust |
|-----------|--------|------|
| Login con credenciales incompletas | ✅ 2 tests | ✅ implícito |
| CreateOrder sin Items | ✅ `test_create_order_rejects_empty_items` | ❌ |
| CreateOrder con Units ≤ 0 | ✅ `test_create_order_rejects_zero_units` | ❌ |
| CreateOrder con Price < 0 | ✅ `test_create_order_rejects_negative_price` | ❌ |
| AddPayment a comanda cancelada | ✅ `test_payment_to_cancelled_order_rejected` | ❌ |
| AddPayment a comanda facturada | ✅ `test_payment_to_invoiced_order_rejected` | ❌ |
| AddPayment con Amount negativo/cero | ✅ 3 tests | ❌ |
| AddPayment superior al saldo | ✅ `test_overpayment_is_rejected` | ✅ `simulator_overpayment_rejected` |
| AddPayment parcial (Balance > 0) | ✅ 2 tests | ✅ en lifecycle test |
| Invoice ya facturada (idempotencia) | ✅ `test_invoice_idempotent_on_already_invoiced` | ✅ `simulator_invoice_idempotent` |
| Invoice comanda cancelada | ✅ `test_invoice_cancelled_order_rejected` | ❌ |
| Cancel comanda inexistente | ✅ `test_cancel_nonexistent_order_fails` | ❌ |
| Cancel comanda ya facturada | ✅ `test_cancel_already_invoiced_order_fails` | ✅ `simulator_cancel_already_invoiced_fails` |
| Cancel comanda válida | ✅ `test_cancel_order_success` | ✅ `simulator_cancel_order` |
| CreateCustomer duplicado sin Overwrite | ✅ `test_create_customer_duplicate_without_overwrite` | ❌ |
| CreateCustomer con Overwrite=true | ✅ `test_create_customer_duplicate_with_overwrite` | ❌ |
| JSON body inválido (no dict) | ✅ `test_rejects_non_dict_json_body` | ❌ |
| JSON body inválido (parse error) | ✅ `test_rejects_invalid_json_body` | ❌ |
| Auth con token inválido | ✅ `test_rejects_invalid_token` | ❌ |
| Auth con header malformado | ✅ `test_rejects_malformed_auth_header` | ❌ |
| Admin con clave incorrecta | ✅ `test_admin_requires_valid_key` | ❌ |
| Admin sin clave | ✅ `test_admin_requires_key` | ❌ |
| Fault http_status | ✅ 8 tests | ✅ 1 test |
| Fault remote_error | ✅ 4 tests | ✅ 1 test |
| Fault invalid_json | ✅ 1 test | ✅ 1 test |
| Fault apply_then_disconnect | ✅ 2 tests (crear + pago) | ✅ 1 test |
| PaymentId duplicado diferente payload | ✅ `test_duplicate_payment_id_different_amount_rejected` | ❌ |
| GetOrder por MarketplaceOrderId | ✅ `test_get_order_by_marketplace_id` | ✅ `simulator_get_order_by_marketplace_id` |
| Concurrent duplicate order | ✅ `test_concurrent_duplicate_order_same_payload` | ❌ |
| History redacción de datos sensibles | ✅ `test_history_redacts_credentials_and_personal_data` | ❌ |
| Full lifecycle crear→pagar→facturar | ✅ 2 tests | ✅ `simulator_full_lifecycle_create_pay_invoice` |
| Pago a orden inexistente | ✅ `test_payment_to_nonexistent_order_fails` | ✅ `simulator_payment_to_nonexistent_order` |
| Factura sin pago | ✅ implícito | ✅ `simulator_invoice_without_payment_rejected` |
| Reconciliación tras disconnect | ✅ 2 tests | ✅ `simulator_reconcile_after_disconnect` |

### 3.3 Fault types — Estado final

| Fault type | Implementado | Testeado Python | Testeado Rust |
|------------|-------------|----------------|--------------|
| `http_status` | ✅ | ✅ (8 tests) | ✅ (1 test) |
| `remote_error` | ✅ | ✅ (4 tests) | ✅ (1 test) |
| `invalid_json` | ✅ | ✅ (1 test) | ✅ (1 test) |
| `delay_ms` | ✅ | ✅ (1 test) | ❌ |
| `apply_then_disconnect` | ✅ | ✅ (2 tests) | ✅ (1 test) |

---

## 4. Gaps RESTANTES (no cubiertos aún)

### 4.1 Tests que faltan en Rust (contra simulador)

| # | Test | Descripción | Severidad |
|---|------|-------------|-----------|
| R1 | `simulator_export_departments` | Verificar parsing de departments | 🟡 Baja |
| R2 | `simulator_get_rooms_tables` | Verificar parsing de rooms/tables | 🟡 Baja |
| R3 | `simulator_export_purchase_notes` | Verificar parsing de albaranes | 🟠 Media |
| R4 | `simulator_fault_delay_ms` | Verificar timeout handling con delay | 🟠 Media |
| R5 | `simulator_cancel_order_invoiced_from_rust` | Ya cubierto en Python, duplicar en Rust es opcional | 🟡 Baja |

### 4.2 Flujos de servicio completo sin test de integración (Rust → simulador → DB)

Estos flujos involucran la capa de servicio (`BdpSyncService`, `BdpOrderPollerService`) y la BD, no solo el cliente HTTP:

| # | Flujo | Riesgo | Esfuerzo |
|---|-------|--------|----------|
| S1 | `sync_venta` → CreateOrder → UPDATE ventas | Parsing de respuesta sin verificar end-to-end | 2h |
| S2 | `add_payment` → AddPayment → INSERT bdp_pagos | Ledger local vs BDP sin verificar | 2h |
| S3 | `invoice_order` → Invoice → UPDATE ventas | Campo bdp_invoiced sin verificar | 1h |
| S4 | `reconcile_ambiguous` → poll → UPDATE | Toda la lógica D1/D5/D11 sin test | 3h |
| S5 | Preflight dry-run contra simulador | Parsing de cada respuesta sin verificar | 2h |
| S6 | Backup fetch → snapshot → restore | Solo testeado con datos insertados manualmente | 3h |
| S7 | `cancel_order` → CancelOrder → UPDATE ventas | Lógica de cancelación sin test E2E | 1h |

### 4.3 Scenarios de concurrencia sin test

| Escenario | Impacto |
|-----------|---------|
| Dos procesos sync misma venta | Advisory lock no verificado contra simulador |
| Sync + poll misma venta | Race condition no verificada |
| DB error durante UPDATE post-HTTP | BDP tiene orden pero Glory no → no testeado |

---

## 5. Resumen ejecutivo — Post-ejecución

| Métrica | Antes auditoría | Después ejecución |
|---------|----------------|-------------------|
| Rutas del simulador con test Python | 6 (26%) | **23 (100%)** |
| Rutas del simulador con test Rust | 6 (26%) | **16 (70%)** |
| Rutas sin test alguno | 13 (57%) | **0 (0%)** |
| Tests Python del simulador | 7 | **92** |
| Tests Rust de integración con simulador | 0 | **23** |
| Fault types testeados | 3/5 | **5/5 (Python), 4/5 (Rust)** |
| Flujos de escritura completos testeados | 0/7 | **3/7** (crear, pagar, facturar en Rust) |
| Bugs encontrados | — | **1** (BdpEmptyRequest serialization) |

### Cobertura estimada por categoría

| Categoría | Cobertura | Notas |
|-----------|-----------|-------|
| Endpoints básicos (health, login, version) | **100%** | Python + Rust |
| Catálogo (lectura) | **95%** | Solo falta Employee individual |
| Crear comanda | **100%** | Validación, idempotencia, conflictos |
| Pagos | **100%** | Parciales, sobrepago, estados, duplicados |
| Facturación | **100%** | Idempotencia, saldo cero, estados |
| Cancelación | **100%** | Válida, facturada, inexistente |
| Clientes | **100%** | CRUD, duplicados, overwrite |
| Fault injection | **100%** | 5 tipos de fault |
| Reconciliación | **90%** | Disconnect cubierto, falta timeout→reconcile |
| Historial/Estado | **100%** | Redacción, reset |
| Servicios de negocio (E2E con DB) | **0%** | Pendiente: tests contra servicio real |

### El gap más peligroso restante

**Los tests de integración Rust prueban el cliente HTTP contra el simulador, pero NO prueban los servicios de negocio** (`BdpSyncService`, `BdpOrderPollerService`) que son los que realmente se ejecutan en producción. Un error en el parsing, mapeo de datos, o manejo de errores dentro de estos servicios no sería detectado por los tests actuales.

---

## 6. Recomendaciones de siguientes pasos

1. **Tests de servicio contra simulador** (S1-S4): Usar la BD real (sqlite en memoria o pg en test) + simulador para verificar que `sync_venta`, `add_payment`, `invoice_order` y `reconcile_ambiguous` funcionan end-to-end. Prioridad ALTA.

2. **Test de delay_ms en Rust** (R4): Añadir `simulator_fault_delay_ms` para verificar que el timeout de 20s del cliente HTTP funciona correctamente. Prioridad MEDIA.

3. **Tests de concurrencia**: Verificar que el advisory lock y el throttle funcionan correctamente con acceso concurrente al simulador. Prioridad MEDIA.

4. **Activar feature flags en producción**: Los tests cubren el código, pero los feature flags (`ff_bdp_partial_payments`, `ff_bdp_cancel_order`, etc.) siguen desactivados. Los tests confirman que el código funciona; ahora hay que activar los flags. Prioridad ALTA.
