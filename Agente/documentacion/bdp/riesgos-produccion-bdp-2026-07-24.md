# Evaluación de riesgos BDP en producción

> **Fecha:** 2026-07-24  
> **Contexto:** Cierre del sprint BDP (247A-1 / 247A-2 / 247A-3). Se corrigió el índice parcial `bdp_audit_log(user_id, idempotency_key) WHERE idempotency_key IS NOT NULL` y su `ON CONFLICT` asociado.  
> **Objetivo:** Anticipar fallos reales en producción, priorizarlos y proponer mitigaciones concretas antes de que el cliente use la integración a diario.

**Nota de contexto:** 247A-3 corrigió el único `ON CONFLICT` contra un índice parcial encontrado en el codebase (`bdp_audit_log`). Los tests de backend y frontend pasan, pero la integración real con BDP introduce riesgos operativos que este documento detalla.

---

## 1. Estado de validación tras 247A-3

| Validación | Resultado |
| --- | --- |
| `cargo test --lib --bins --test bdp_* --test haddock_db` | ✅ 66 passed, 0 failed |
| `cd frontend && npx tsc --noEmit` | ✅ Sin errores |
| Revisión de índices parciales + `ON CONFLICT` | ✅ Solo afecta a `bdp_audit_log`; corregido en 247A-3 |

Los tests `bdp_readonly.rs` siguen en `#[ignore]` porque requieren una instancia real BDP; no se ejecutan en CI local.

---

## 2. Matriz de riesgos resumida

| ID | Riesgo | Severidad | Probabilidad | Mitigación inmediata |
| --- | --- | --- | --- | --- |
| R1 | Falsa reconciliación por `AmbiguousTransport` sin reintento | Crítica | Media | Monitorear auditoría `ambiguo`; añadir reconciliación retrasada |
| R2 | Transacción abierta durante llamadas HTTP a BDP | Alta | Media | Cerrar tx antes del HTTP; reabrir solo para UPDATE final |
| R3 | `BdpWeblinkError::Throttled` se trata como error permanente (`Rejected`) | Alta | Media | Mapear a `AmbiguousTransport` o reintentar con backoff en escrituras |
| R4 | Auto-sync de cliente bloquea la comanda sin feedback al usuario | Alta | Alta | Mostrar mensaje explícito en UI y/o permitir cliente genérico |
| R5 | `sync_venta` hace login + GetArticle + CreateOrder sin timeout por operación | Media | Alta | Añadir `tokio::time::timeout` envolvente en cada llamada BDP |
| R6 | Crecimiento ocasional de `SYNC_LOCKS` bajo panic | Media | Baja | Confirmar cleanup en todos los caminos; evaluar `DashMap` |
| R7 | Respuestas BDP con `Status` desconocido se almacenan como `unknown_N` | Media | Baja | Definir estados esperados; alertar en `unknown_*` |
| R8 | `cached_session` mutex poisoning detendría login futuros | Media | Muy baja | Reemplazar `Mutex` por `RwLock` sin panics o reiniciar proceso |
| R9 | Política de retención de snapshots vs. armados vigentes | Media | Baja | Confirmar que no se borren snapshots con armados activos |
| R10 | Feature flags `ff_bdp_*` desactivados por defecto (UX) | Baja | Alta | Mostrar estado en UI y tooltips |

---

## 3. Riesgos detallados

### R1 — Falsa reconciliación por `AmbiguousTransport`

**Ubicación:** `src/services/bdp_sync.rs:389-435`

```rust
Err(BdpSyncError::AmbiguousTransport(msg)) => {
    let marketplace_id = Self::marketplace_order_id(venta.id);
    let request = BdpGetOrderRequest { ... };
    match client.get_order(&request).await { ... }
}
```

**Escenario de fallo:**
1. Glory envía `CreateOrder` a BDP.
2. BDP aplica la orden pero la respuesta HTTP se pierde (timeout de 20s en `bdp_weblink.rs:39`).
3. `retry_send_order` intenta reconciliar llamando a `GetOrder` por `MarketplaceOrderId`.
4. Si BDP no indexa `MarketplaceOrderId` inmediatamente, o si la suscripción no permite `GetOrder`, la reconciliación devuelve `Ambiguous`.
5. El registro de auditoría queda en `ambiguo` y la venta en `bdp_synced=false`.
6. **Riesgo:** el camarero puede volver a intentar enviar la venta, generando un duplicado si BDP no deduplica por `MarketplaceOrderId`.

**Mitigación recomendada:**
- Añadir un job de reconciliación retrasada (polling cada X minutos) para filas `bdp_audit_log` en `ambiguo`.
- Antes de permitir re-envío manual, comprobar `GetOrder` por `MarketplaceOrderId` y marcar como exitoso si existe.
- Exponer en UI claramente el estado `ambiguo` con acción de reconciliar.

---

### R2 — Transacción abierta durante llamadas HTTP a BDP

**Ubicación:** `src/services/bdp_sync.rs:137-181` y `:304-344`

**Escenario de fallo:**
- Se adquiere `pg_try_advisory_xact_lock` en la transacción `distributed_lock`.
- Se hacen llamadas HTTP a BDP *dentro* de la vida de esa transacción (login, GetArticle, CreateOrder).
- Finalmente se hace `distributed_lock.commit().await` después de todo el HTTP.
- **Problema:** el lock evita que otra instancia procese la misma venta, pero mantiene una conexión de Postgres abierta y ocupada durante todo el ciclo HTTP (potencialmente decenas de segundos). Bajo carga concurrente, esto puede agotar el pool de conexiones de la aplicación y ralentizar otras operaciones.
- Si el proceso muere entre el HTTP y el commit, el lock se libera y otra instancia podría reenviar; la deduplicación por `MarketplaceOrderId` mitiga esto, pero no lo elimina.

**Mitigación recomendada:**
- Cerrar la transacción `distributed_lock` inmediatamente después de adquirir/advertir el lock lógico, y reabrir una nueva transacción solo para el UPDATE final.
- No mantener conexiones de base de datos abiertas durante I/O externo.
- Documentar claramente que el lock actual evita concurrencia pero no garantiza exactly-once si hay crash post-HTTP.

---

### R3 — Throttling tratado como error permanente

**Ubicación:** `src/services/bdp_sync.rs:472-473`

```rust
crate::services::bdp_weblink::BdpWeblinkError::Throttled(message) => {
    BdpSyncError::Rejected(format!("BDP throttled: {message}"))
}
```

**Escenario de fallo:**
- `BDP_THROTTLE` limita a 2 peticiones concurrentes por destino.
- Si se supera, `BdpWeblinkError::Throttled` se lanza.
- En `send_order` se mapea a `Rejected`, es decir, error permanente.
- La venta se marca como error BDP y no se reintenta.
- **Riesgo:** bajo tráfico concurrente se pueden perder comandas por un rechazo temporal que no debería ser permanente.

**Mitigación recomendada:**
- Mapear `Throttled` a `AmbiguousTransport` en escrituras, no a `Rejected`.
- O añadir reintentos con backoff en el propio `BdpThrottleManager` (cola de espera con timeout).

---

### R4 — Auto-sync de cliente bloquea comanda sin feedback claro

**Ubicación:** `src/services/bdp_sync.rs:180-214`

```rust
if config.bdp_auto_sync_customers {
    if let Some(cliente_id) = venta.cliente_id {
        if let Some(bdp_code) = Self::ensure_cliente_bdp_synced(...).await {
            // ok
        } else {
            let msg = "Creación automática BDP deshabilitada: asigne y verifique un código explícito...";
            // venta se marca con error
        }
    }
}
```

**Escenario de fallo:**
- Se activa `bdp_auto_sync_customers`.
- Un cliente no tiene `bdp_customer_code`.
- La venta se bloquea y se guarda `bdp_sync_error`.
- El usuario en el TPV/Glory no ve por qué no se envió la comanda, o ve un mensaje técnico.

**Mitigación recomendada:**
- En UI de ventas, mostrar badge "Cliente sin mapeo BDP" antes de cobrar.
- Permitir fallback a `bdp_default_customer_code` si está configurado, explícitamente.
- Añadir notificación/toast con el mensaje de `bdp_sync_error`.

---

### R5 — Falta de timeout por operación en `sync_venta`

**Ubicación:** `src/services/bdp_sync.rs:69-344`

**Escenario de fallo:**
- `BdpWeblinkClient` tiene timeout de 20s por petición HTTP.
- Pero `sync_venta` puede hacer varias llamadas: `login`, `get_article`, `get_pos_articles`, `create_order`.
- Si cada una tarda 19s, el total supera 60s fácilmente.
- En un entorno HTTP/1.1 sin keep-alive, esto puede acumular conexiones y agotar el pool de Postgres si se mantiene tx abierta.

**Mitigación recomendada:**
- Envolver el bloque HTTP de `sync_venta` en `tokio::time::timeout(Duration::from_secs(30), ...)`, no solo cada petición.
- Cerrar la transacción `distributed_lock` antes de hacer HTTP; reabrir solo para el UPDATE final.

---

### R6 — Crecimiento ocasional de `SYNC_LOCKS`

**Ubicación:** `src/services/bdp_sync.rs:60-61, 991-1002`

**Escenario de fallo:**
- `SYNC_LOCKS` es un `HashMap` global con `Arc<TokioMutex<()>>`.
- `cleanup_lock` elimina la entrada si `Arc::strong_count(entry) <= 2` y hace sweep si `map.len() > 100`.
- Si hay un panic antes de `cleanup_lock`, la entrada persiste hasta el siguiente sweep.
- El riesgo no es de crecimiento ilimitado (el sweep lo contiene), pero puede causar picos transitorios de memoria bajo fallos masivos.

**Mitigación recomendada:**
- Confirmar que todos los caminos de salida de `sync_venta` llaman a `cleanup_lock` (actualmente sí, salvo panic).
- Considerar `DashMap` para mejorar concurrencia y reducir contención.

---

### R7 — Estados BDP desconocidos

**Ubicación:** `src/services/bdp_order_poller.rs:184-198`

```rust
fn map_status(code: i64) -> String {
    match code {
        0 => "pending".to_string(),
        1 => "accepted".to_string(),
        2 => "cancelled".to_string(),
        3 => "invoiced".to_string(),
        other => format!("unknown_{other}"),
    }
}
```

**Escenario de fallo:**
- BDP devuelve un status no documentado (ej. 4, 5, -1).
- Se almacena `unknown_4` y no se alerta.
- El usuario no sabe qué significa.

**Mitigación recomendada:**
- Añadir log `warn` cuando `map_status` reciba un código desconocido.
- Considerar enviar alerta a administrador o incluir en dashboard de monitorización.

---

### R8 — Mutex poisoning en caché de sesión

**Ubicación:** `src/services/bdp_weblink.rs:140, 168`

```rust
let cache = self.cached_session.lock().expect("session cache poisoned");
```

**Escenario de fallo:**
- Si un hilo hace panic mientras sostiene el `Mutex`, el mutex queda "poisoned".
- Cualquier llamada futura a `login()` hará `expect` y panicará el proceso.
- Aunque es poco probable, un solo panic deja caído el servicio.

**Mitigación recomendada:**
- Usar `std::sync::RwLock` o `parking_lot::Mutex`, o manejar el poisoning con `lock().map_err(...)`.
- En Rust moderno, considerar `tokio::sync::Mutex` si el acceso es async.

---

### R9 — Integridad del snapshot en `bdp_write_arming`

**Ubicación:** `src/services/bdp_write_guard.rs:147-218`, `migrations/20260718300000_bdp_write_safety_v2.up.sql`

**Escenario de fallo:**
- `try_auto_arm` inserta en `bdp_write_arming` con `snapshot_id` obtenido de `bdp_snapshots`.
- `BdpWriteGuard::authorize` verifica que `snapshot_id IS NOT NULL` en el UPDATE.
- La migración ya define `snapshot_id UUID REFERENCES bdp_snaphots(id)` (comportamiento `NO ACTION` por defecto). Por tanto, borrar un snapshot referenciado fallará a menos que se use `ON DELETE CASCADE` o se fuerce.
- **Riesgo residual:** una política de retención/configuración que fuerce el borrado de snapshots sin invalidar armados asociados rompería el flujo de auto-arming.

**Mitigación recomendada:**
- Confirmar que la política de retención no borra snapshots con armados vigentes.
- Considerar `ON DELETE CASCADE` si se quiere limpieza automática, o `ON DELETE RESTRICT` para protegerse.

---

### R10 — Feature flags desactivados por defecto (UX)

**Ubicación:** `src/services/bdp_weblink.rs:config()` y `src/services/haddock.rs`

**Escenario de fallo:**
- Los flags `ff_bdp_auto_arm`, `ff_bdp_partial_payments`, etc. son `false` por defecto.
- Un usuario que espera "auto-arming" no sabrá por qué no funciona.
- La UI muestra el toggle pero no explica que el flag también debe estar activo.

**Mitigación recomendada:**
- En la pantalla de configuración BDP, mostrar el estado de cada feature flag.
- Si una acción requiere un flag desactivado, deshabilitar el botón con tooltip explicativo.

---

### R11 — Fallback de cliente genérico no incluye nombre/teléfono

**Ubicación:** `src/services/bdp_sync.rs:847-899`

**Escenario de fallo:**
- Cuando la venta no tiene `cliente_id`, `resolve_customer` devuelve solo el código `bdp_default_customer_code`; `name` y `phone` son `None`.
- Si BDP requiere un `Customer.Name` para ciertos tipos de pedido, el request podría ser rechazado.
- Si el restaurante usa `bdp_default_customer_code` para agrupar ventas de mostrador, pierde trazabilidad.

**Mitigación recomendada:**
- Añadir en configuración BDP campos opcionales "nombre cliente por defecto" y "teléfono cliente por defecto".
- O documentar explícitamente que el fallback solo funciona si BDP acepta `Customer` vacío.

---

### R12 — IVA y precio por defecto hardcodeados en `resolve_article`

**Ubicación:** `src/services/bdp_sync.rs:690-750`

**Escenario de fallo:**
- Si `GetArticle` o `get_pos_articles` no devuelven `Price1` o `TAVPer`, el código usa `price: 0.0` y `vat_pct: 10.0` por defecto.
- El total de la línea en `build_order` se calcula con datos locales de Glory (`venta.importe_base`, `venta.importe_iva`), no con los de BDP.
- Si BDP valida que los totales coinciden con el artículo, el request puede ser rechazado.
- Si se acepta, podría generar discrepancias contables entre Glory y BDP.

**Mitigación recomendada:**
- Mapear el IVA configurado en `configuracion_restaurante.iva_por_defecto` en lugar de 10.0 fijo.
- Rechazar explícitamente el envío si no se puede resolver un precio/IVA válido, en lugar de enviar 0.
- Añadir validación de totales previa al envío.

---

## 4. Recomendaciones priorizadas

### Inmediatas (antes de poner en producción)

1. **R3 (Throttling → Rejected):** cambiar el mapeo de `Throttled` a `AmbiguousTransport` para escrituras, o implementar reintentos con backoff en `BdpThrottleManager`.
2. **R2 (Lock transaccional durante HTTP):** refactorizar `sync_venta` para no mantener la transacción abierta durante llamadas HTTP; usar lock solo para operaciones locales.
3. **R4 (Cliente sin mapeo):** mejorar mensajes de error en UI de ventas.

### Corto plazo (primeras 2 semanas en producción)

4. **R1 (Reconciliación ambigua):** implementar job de reconciliación periódica y dashboard de auditoría `ambiguo`.
5. **R9 (Integridad snapshot):** revisar constraints de FK y política de retención de snapshots.

### Medio plazo

6. **R8 (Mutex poisoning):** reemplazar mutexes con primitives sin panic.
7. **R6 (SYNC_LOCKS):** evaluar `DashMap` o TTL para evitar crecimiento.
8. **R5 (Timeout global):** añadir timeout envolvente en `sync_venta`.

---

## 5. Conclusión

La integración BDP está **funcionalmente completa y los tests pasan**, pero tiene varios puntos de fricción en producción. Los más importantes a vigilar son:

- **Duplicados por fallos ambiguos de red** (R1, R2, R3).
- **Bloqueo silencioso de ventas por falta de mapeo de cliente** (R4).
- **Timeouts acumulados sin control global** (R5).

Con las mitigaciones propuestas, el riesgo de fallos graves en producción se reduce a **bajo/medio**.

---

## 6. Referencias

- `src/services/bdp_write_guard.rs` — fix 247A-3 del `ON CONFLICT` parcial.
- `src/services/bdp_sync.rs` — lógica de envío de comandas y reconciliación.
- `src/services/bdp_weblink.rs` — cliente HTTP y caché de sesión.
- `src/services/bdp_order_poller.rs` — polling de estados y reconciliación.
- `src/services/bdp_throttle.rs` — semáforo de concurrencia.

## 7. Mitigaciones aplicadas

| ID | Mitigación | Estado | Verificación código (2026-07-26) |
| --- | --- | --- | --- |
| R1 | Worker `reconcile_ambiguous` en `bdp_order_poller.rs` con 5 funciones (create_order, add_payment, invoice, ambiguous_pagos) | ✅ Aplicado | `bdp_order_poller.rs:304-440` — reconcile_ambiguous_pagos + reconcile_create_order + reconcile_add_payment + reconcile_invoice |
| R2 | Cerrar `distributed_lock` inmediatamente tras adquirir `pg_try_advisory_xact_lock`, liberando la conexión de Postgres antes de las llamadas HTTP. | ✅ Aplicado | `bdp_sync.rs` — lock cerrado antes de fase HTTP |
| R3 | `BdpWeblinkError::Throttled` se mapea a `AmbiguousTransport` en `send_order`, evitando rechazo permanente por throttling. | ✅ Aplicado | `bdp_sync.rs:506` — `Throttled(message)` → `AmbiguousTransport(format!("BDP throttled: {message}"))` |
| R5 | Fase HTTP de `sync_venta` envuelta en `tokio::time::timeout(Duration::from_secs(45))`. | ✅ Aplicado | `bdp_sync.rs` — timeout global de 45s |
| R6/R13 | `SyncLockGuard` RAII con `impl Drop` que llama `cleanup_lock`. Eliminación de llamadas manuales. | ✅ Aplicado | `bdp_sync.rs:71-81` — struct SyncLockGuard + impl Drop + impl SyncLockGuard |
| R7 | `bdp_order_poller` loguea `warn` cuando recibe un status desconocido de BDP. | ✅ Aplicado | `bdp_order_poller.rs:184-198` — map_status con warn para unknown | 
| R8 | `cached_session` ya no hace `expect` al tomar el lock; se recupera del poisoning. | ✅ Aplicado | `bdp_weblink.rs` — recovery pattern, no expect |
| R14 | Limpieza manual de `SYNC_LOCKS` sustituida por RAII guard. | ✅ Aplicado | Ver R6/R13 — SyncLockGuard en bdp_sync.rs:71 |

## 8. Nuevos riesgos identificados en esta sesión

### R13 — Mutex poisoning en `SYNC_LOCKS`

**Ubicación:** `src/services/bdp_sync.rs:95` y `:1012`

```rust
let mut map = SYNC_LOCKS.lock().expect("SYNC_LOCKS poisoned");
```

**Escenario:**
- Un hilo paniquea mientras sostiene el `Mutex` global `SYNC_LOCKS`.
- `expect` tumba todo el proceso en adelante.

**Mitigación aplicada:** reemplazar por `unwrap_or_else(|e| e.into_inner())`, que recupera el lock y permite continuar. A largo plazo, migrar a `parking_lot::Mutex` o `tokio::sync::Mutex` para evitar el poisoning.

### R14 — Limpieza manual de `SYNC_LOCKS`

**Ubicación:** múltiples `Self::cleanup_lock(venta.id)` en `src/services/bdp_sync.rs`

**Escenario:**
- Si se añade un nuevo `return` en `sync_venta` y se olvida llamar a `cleanup_lock`, la entrada permanece en memoria hasta el sweep.
- Un `panic` intermedio salta todas las llamadas manuales.

**Mitigación recomendada:** crear un guard RAII (`SyncLockGuard`) que implemente `Drop` y llame a `cleanup_lock`, eliminando las llamadas manuales.

### R15 — `Throttled` no se trata como ambiguo en pagos y facturas

**Ubicación:** `src/services/bdp_sync.rs:1130-1195` (`add_order_payment`) y `:1300-1400` (`invoice_order`)

**Escenario:**
- `add_order_payment` e `invoice_order` propagan `BdpWeblinkError::Throttled` como un mensaje de error de tipo `String`.
- El caller puede interpretar el throttling como un error permanente y no reintentar.
- Si el pago o factura sí se aplicó en BDP, queda inconsistente.

**Mitigación:** 🟡 Aplicado — `Throttled` se mapea a `resultado = 'ambiguo'` en ambos endpoints, alineado con `send_order`. Aún falta un worker de reconciliación periódica para pagos/facturas ambiguas.

### R16 — Aritmética en `f64` para totales y descuentos

**Ubicación:** `src/services/bdp_sync.rs:378` y `build_order`

```rust
let linea_total = (precio * cantidad) - descuento;
```

**Escenario:**
- `f64` acumula errores de precisión (ej. `0.1 + 0.2`).
- Si BDP valida que el `Total` de la comanda coincide con la suma de líneas, un descuadre de centimos puede rechazar el `CreateOrder`.

**Mitigación recomendada:** realizar todos los cálculos con `rust_decimal::Decimal`, redondear a 2 decimales y convertir a `f64` solo al serializar el JSON.
