# Plan detallado: Pendientes integración BDP

> **Fecha:** 2026-07-23 (actualizado 2026-07-24)
> **Alcance:** Solo planificación. Sin código, sin comandos al BDP real.
> **Contexto:** Tras completar las mejoras de visibilidad (237A-3), implementación de stock (237A-4), mejoras de UX/auto-arming (247A-1) y fix del índice parcial (247A-3), quedan pendientes decisiones y funcionalidades que requieren análisis profundo antes de implementar. Ver `Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md` para los riesgos de producción identificados.


---

## Resumen ejecutivo (revisado)

| # | Item | Tipo | Estado actual | ¿Listo para implementar? | Esfuerzo estimado |
|---|------|------|---------------|--------------------------|-------------------|
| C1 | Auto-arming (escritura automática con confirmación dinámica + idempotency) | Mejora de flujo | ✅ Implementado (247A-1) | **Sí** — backend `BdpWriteGuard::try_auto_arm` + handlers | ~12-14h |
| C2 | Toggle rápido en navbar | Mejora UX (admin) | ✅ Implementado (247A-1) | **Sí** — badge interactivo en `site-header.tsx` | ~3h |
| D1 | Verificación stock + parser defensivo | Verificación | Implementado básico (237A-4) | **Sí** — parser defensivo proactivo ya | ~2h parser + opcional 8h pantalla dedicada |
| D2 | **Compras** | Funcionalidad nueva | **Fases 1-3 implementadas** (247A-11+): lectura, borradores (`ff_bdp_purchase_notes_draft`) y conciliación (`ff_bdp_purchase_notes_receive`) | **Sí** — las 3 fases están en código. Pendiente activar flags en producción | ~30h total implementadas (8h+10h+12h). Pendiente: activación producción + pruebas BDP real |
| D3 | Sincronización bidireccional automática | Funcionalidad nueva | Bloqueado explícitamente en código | **Rechazado firme** — riesgo crítico sin mitigación viable | N/A — no implementar |
| D4 | Pagos parciales | Funcionalidad nueva | ✅ Implementado (backend + frontend + reconciliación de ambiguos). Feature flag `ff_bdp_partial_payments`. | **Sí** — tests de simulador pendientes | ~18-22h implementados. Pendiente: test simulador + activación producción |
| D5 | CancelOrder | Funcionalidad nueva | Bloqueado por BDP ("Subscripción no activada") | **Pendiente activación BDP**, estimación realista | ~12-16h (si BDP activa módulo) |
| **XT1** | Throttling/semáforo BDP | Cross-cutting | ✅ Implementado (247A-1) | **Sí** — `BdpThrottleManager` en `src/services/bdp_throttle.rs` | ~3-4h |
| **XT2** | Feature flags por restaurante | Cross-cutting | ✅ Implementado (247A-1) | **Sí** — columnas en `configuracion_restaurante` | ~4-5h |

---

## C1 — Auto-arming (escritura automática al operar)

### Problema actual

Hoy, para enviar una venta a BDP, registrar un pago o facturar, el usuario debe:

1. Ir a Configuración → BDP → "Seguridad, respaldos e historial BDP"
2. Pulsar "Activar escritura temporal"
3. Confirmar texto de seguridad
4. Volver a Ventas
5. Realizar la operación (enviar a BDP / pagar / facturar)
6. El sistema desarma automáticamente después de la operación

**Esto es 5 pasos para una operación que debería ser 1.** En un restaurante con ritmo, esto es inaceptable.

### Diseño propuesto (revisado)

**Hallazgos críticos del diseño original:**

- `CONFIRMAR` literal es vulnerable a **memoria muscular**: un usuario con prisa lo escribe sin leer y dispara operaciones en BDP por error.
- Si el usuario abre 2 pestañas/modales en paralelo y hace doble-clic, ambos requests pasan el TTL de 60s y BDP recibe la operación dos veces.
- Si el usuario ya activó el modo escritura global manual, el prompt de auto-arming es ruido y produce fricción innecesaria.

**Flujo nuevo (1 paso):**

1. El usuario pulsa "Enviar a BDP" / "Pagar en BDP" / "Facturar en BDP" directamente desde la fila de la venta
2. Se abre un modal de confirmación con **dos defensas combinadas**:
   - **Checkbox explícito** "Entiendo que esta operación modifica datos en BDP" — debe estar marcado para habilitar el botón.
   - **Confirmación dinámica** según el tipo de operación:
     - Pago: el usuario escribe el **monto exacto** con 2 decimales (ej: `45.00`).
     - Factura: el usuario escribe el **número de venta** (ej: `V-2026-00123`).
     - Comanda/Sync: el usuario escribe el **total de líneas** (ej: `5 lineas`).
   - Botón "Cancelar" / "Confirmar y ejecutar"
3. El frontend genera un **idempotency_key** (UUID v4) por intento de operación y lo envía al backend.
4. Al confirmar:
   - El backend crea un arming efímero (`bdp_write_arming` con TTL de 60 segundos) solo si no existe ya un arming manual activo.
   - **Si existe arming manual global activo:** auto-arming lo respeta y omite el prompt (ya está autorizado).
   - **Si no:** crea arming efímero, ejecuta la operación, desarma automáticamente.
   - El backend deduplica por `idempotency_key`: si llega el mismo UUID dos veces, la segunda llamada devuelve el resultado cacheado sin re-disparar BDP.
   - Registra en auditoría como `AUTO_ARMING` (distinguir del arming manual).

### Arquitectura backend (revisada)

```
Frontend
  → Genera idempotency_key = UUID v4()
  → POST /api/ventas/:id/bdp-sync
    Body: { auto_arm: true, confirmation_text: "45.00", idempotency_key: "uuid" }

BdpSyncService::sync_venta()
  → 1. Validar idempotency_key (si existe en cache, devolver resultado cacheado)
  → 2. Validar confirmation_text según tipo de operación
     (monto exacto para pago, número venta para factura, líneas para comanda)
  → 3. Si existe BdpWriteGuard::is_armed(user_id) → usar arming manual existente
     Si no → BdpWriteGuard::authorize_inline(user_id, "auto_arm", ttl=60s)
  → 4. Ejecutar operación con timeout y retry controlado
  → 5. Si éxito → BdpWriteGuard::disarm(user_id) + cachear resultado por idempotency_key
  → 6. Si falla → NO desarmar (dejar que el siguiente intento del mismo UUID falle rápido)
  → 7. Audit log: tipo="auto_arm", operacion="create_order", idempotency_key, IP
```

**Errores dedicados (fail-closed):**

| HTTP Code | Cuándo | Mensaje |
|-----------|--------|---------|
| 409 Conflict | confirmation_text no coincide con monto/número esperado | `confirmation_mismatch` |
| 428 Precondition Required | bdp_auto_arm desactivado en config | `auto_arm_disabled` |
| 429 Too Many Requests | rate limit excedido (>10 auto-armings/minuto) | `auto_arm_rate_limit` |
| 409 Conflict | idempotency_key repetido con resultado diferente | `idempotency_replay_mismatch` |

**Cambios necesarios:**

| Capa | Archivo | Cambio |
|------|---------|--------|
| Backend | `src/services/bdp_write_guard.rs` | Nuevo método `authorize_inline()` que crea arming efímero sin pasar por el endpoint manual |
| Backend | `src/handlers/ventas.rs` | Aceptar campo `auto_arm: bool` + `confirmation_text: String` en requests de sync/pago/factura |
| Backend | `src/handlers/bdp_customer_sync.rs` | Mismo patrón para CreateCustomer |
| Backend | `src/handlers/configuracion.rs` | Mantener endpoint manual actual como fallback |
| Frontend | `venta-row-actions.tsx` | Modal de confirmación inline en lugar de navegar a Configuración |
| Frontend | `bdp-sync-badge.tsx` | Indicador de que auto-arming está disponible |

### Seguridad

| Riesgo | Mitigación |
|--------|------------|
| Alguien pulsa "Enviar" sin intención | Confirmación textual obligatoria ("CONFIRMAR") |
| Reutilización de arming | TTL de 60 segundos, un solo uso |
| Bypass del flujo manual | Mantener endpoint manual como alternativa; auto-arming es opt-in en configuración |
| Auditoría insuficiente | Log diferenciado: `AUTO_ARMING` vs `MANUAL_ARMING` con IP, timestamp, operación |
| Rate limiting | Máximo 10 auto-armings por minuto por usuario |

### Configuración necesaria

Nuevo campo en `configuracion_restaurante`:
```sql
bdp_auto_arm BOOLEAN NOT NULL DEFAULT FALSE
```

- `FALSE` (default): comportamiento actual (arming manual obligatorio)
- `TRUE`: permite auto-arming con confirmación textual

**El usuario decide si activar esto.** Si no lo activa, el flujo actual sigue funcionando.

### Esfuerzo: ~10-12h

| Tarea | Tiempo |
|-------|--------|
| Backend: `authorize_inline()` + validación | 3h |
| Backend: modificar handlers sync/pago/factura | 2h |
| Frontend: modal de confirmación | 3h |
| Frontend: integración con venta-row-actions | 2h |
| Tests + auditoría | 2h |

### Recomendación: **IMPLEMENTAR**

Es la mejora de usabilidad más impactante. Sin esto, la integración de escritura es usable pero incómoda.

---

## C2 — Toggle rápido en navbar

### Problema

El indicador BDP en la navbar (`BdpStatusIndicator`) muestra el estado actual pero no permite cambiarlo. Para cambiar de "Solo lectura" a "Escritura temporal" hay que ir a Configuración → BDP → Panel de seguridad.

### Diseño propuesto

Convertir el badge `BdpStatusIndicator` en un componente interactivo:

```
[🟢 BDP Conectado] → click → Dropdown:
  ┌─────────────────────────────────┐
  │ Estado: Solo lectura             │
  │ ─────────────────────────────── │
  │ 🔒 Activar escritura temporal   │
  │ 📊 Ver historial BDP            │
  │ ⚙️ Configuración BDP            │
  └─────────────────────────────────┘
```

### Cambios necesarios

| Capa | Archivo | Cambio |
|------|---------|--------|
| Frontend | `site-header.tsx` | Convertir `BdpStatusIndicator` de badge estático a dropdown interactivo |
| Frontend | Nuevo hook | `useBdpSyncMode.ts` — mutation para cambiar modo |
| Backend | Ninguno | Reutilizar `PUT /api/configuracion/bdp/sync-mode` existente |

### Seguridad

- Solo visible para usuarios con rol admin/manager
- Cambiar a escritura requiere el flujo de arming existente (o auto-arming si C1 está activo)

### Esfuerzo: ~3h

### Recomendación: **IMPLEMENTAR (ortogonal a C1)**

C2 **no se vuelve obsoleto con C1**. Cubre un caso distinto y legítimo para administradores:

- **Inspección sin acción.** El admin necesita ver rápidamente el modo actual de BDP (solo lectura / escritura temporal) sin tener que abrir Configuración.
- **Armado explícito para reconciliación masiva.** Un admin que necesita sincronizar 20 ventas pendientes antiguas puede querer activar el modo escritura manual global, recorrer la lista y procesarlas todas sin escribir "el monto exacto" 20 veces (cada modal de C1 lo pediría). Con C2 + arming manual existente, el admin hace UNA activación y procesa N operaciones.
- **Bloqueo de emergencia.** Si BDP está reportando fallos intermitentes, el admin quiere poder poner Glory en solo lectura en un click desde la navbar sin abrir Configuración.
- **Visibilidad del estado.** El dropdown expone también "Ver historial BDP" y "Configuración BDP" — enlaces rápidos para diagnóstico.

C2 no compite con C1: C2 es para **administradores** (uso infrecuente, consciente, en bloque), C1 es para **operadores** (cada venta, flujo rápido).

---

## D1 — Verificación de stock real

### Estado actual

Implementado en 237A-4:
- `ExportArticles` → `current_stock: Option<Decimal>` con aliases `CurrentStock` + `Stock`
- Mapeo a `stock_actual` en `bdp_article_map`
- Columna Stock en tabla de mapeos
- Info log si ningún artículo trae stock

### Lo que falta verificar

1. **¿`ExportArticles` devuelve `CurrentStock` en la respuesta real?**
   - El campo está documentado en `PricesTableDataType` (línea 3685 del manual WebLink)
   - Podría estar anidado dentro de un sub-array `Prices` en vez de ser top-level
   - Si está anidado, el parser actual no lo captura → columna mostrará "—" para todos

2. **¿El módulo de almacén está activo en la instalación del restaurante?**
   - Si no está activo, `CurrentStock` vendrá como `None` siempre
   - El info log diagnosticará esto automáticamente

### Plan de verificación (revisado: defensivo proactivo)

**No esperar al BDP real para hacer el parser resiliente.** Implementar **Opción A desde ya** para que el código sea robusto a ambas estructuras del JSON.

**Paso 1 — Parser defensivo (ahora, ~2h):**

En `bdp_weblink_catalog.rs` ajustar el parsing de `BdpExportArticleItem`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct BdpArticlePriceEntry {
    #[serde(default, alias = "CurrentStock", alias = "Stock")]
    pub current_stock: Option<Decimal>,
    #[serde(flatten)]
    pub other: serde_json::Value, // campos restantes del manual
}

#[derive(Debug, Clone, Deserialize)]
pub struct BdpExportArticleItem {
    /* ... campos existentes ... */
    #[serde(default, alias = "CurrentStock", alias = "Stock")]
    pub current_stock: Option<Decimal>,

    /// Variantes donde BDP anida stock dentro de Prices[]
    /// Algunos campos de Prices[] ya están extraídos al root por flatten;
    /// este campo se mantiene para futura deduplicación.
    #[serde(default)]
    pub prices: Vec<BdpArticlePriceEntry>,
}
```

Lógica de selección del stock final:
```rust
let stock_final = current_stock.or_else(|| {
    prices.iter()
        .find_map(|p| p.current_stock)
});
```

Resultado: si BDP devuelve stock en root → lo usa. Si lo anida en `Prices[]` → lo busca ahí. Si ninguno → None → "—" en UI.

**Paso 2 — Verificar con BDP real (cuando el cliente haga pruebas):**
- Ejecutar "Sync catálogo" desde la app
- Revisar logs: el info log indica cuántos artículos trajeron stock y cuántos no
- Si todos traen None → el módulo de almacén no está activo (esperado, no es bug)

**Paso 3 — Si el cliente necesita pantalla dedicada (consultar antes):**
- **Opción B**: Implementar `GET /api/bdp/stock` que agrega datos de mapeo + `CurrentStock` en una vista unificada

### Endpoints BDP disponibles para stock

| Endpoint | Método | Descripción | Estado en Glory |
|----------|--------|-------------|-----------------|
| `ExportArticles` (campo `CurrentStock`) | Ya implementado | Stock viene como campo del artículo | ✅ Implementado, pendiente verificación real |
| `/API/Warehouse/GetStock` | POST | Stock de un artículo en un almacén específico | ❌ No implementado |
| `/API/Warehouse/GetListStock` | POST | Stock de múltiples artículos de una vez | ❌ No implementado |
| `/API/Warehouse/UpdateStock` | POST | Actualizar stock (escritura) | ❌ No implementado, fuera de alcance |
| `/API/Warehouse/Regularizations` | POST | Regularizaciones de inventario | ❌ No implementado, fuera de alcance |
| `/API/Warehouse/Transfers` | POST | Transferencias entre almacenes | ❌ No implementado, fuera de alcance |

### Si se implementa pantalla dedicada de stock (Opción B)

**Nuevos componentes:**

| Capa | Archivo nuevo | Descripción |
|------|---------------|-------------|
| Backend | `src/handlers/bdp_stock.rs` | Handler para `GET /api/bdp/stock` — consulta stock por artículo o todos |
| Backend | `src/services/bdp_weblink.rs` | Método `get_stock()` y `get_list_stock()` |
| Backend | `src/services/bdp_weblink_catalog.rs` | Structs `BdpGetStockRequest`, `BdpGetStockResponse` |
| Frontend | `bdp-stock-panel.tsx` | Tabla de stock con filtros por artículo/familia/almacén |
| Frontend | Hook | `useBdpStock.ts` — query para obtener stock |

**Flujo:**
1. Usuario hace click en "Ver stock" desde tabla de mapeos o sección dedicada
2. Frontend llama `GET /api/bdp/stock?almacen=1`
3. Backend llama a `GetListStock` en BDP
4. Muestra tabla con: Artículo | Código BDP | Stock actual | Almacén | Última actualización

### Esfuerzo

| Tarea | Tiempo |
|-------|--------|
| Verificar estructura respuesta ExportArticles | 2h |
| Ajustar parser si CurrentStock está anidado | 2h |
| Implementar endpoints GetStock/GetListStock | 3h |
| Pantalla dedicada de stock | 5h |
| **Total Opción B** | **~12h** |

### Recomendación: **VERIFICAR PRIMERO, luego decidir**

1. Verificar si `ExportArticles` ya trae stock (paso 1-2)
2. Si sí → no hacer nada más, la columna ya funciona
3. Si no → consultar al cliente si necesita pantalla dedicada de stock

---

## D2 — Compras (integración de proveedores)

### ¿Qué son las "compras" en BDP?

BDP tiene un módulo completo de gestión de compras que incluye:

| Concepto | Descripción |
|----------|-------------|
| **Proveedores** | Catálogo de proveedores con datos fiscales |
| **Albaranes de compra** | Documentos que registran la recepción de mercancía |
| **Facturas de compra** | Facturas emitidas por proveedores |
| **Recepciones** | Confirmación física de mercancía recibida |
| **Órdenes de compra** | Pedidos a proveedores |

### Endpoints BDP disponibles

| Endpoint | Método | Descripción |
|----------|--------|-------------|
| `ExportPurchaseNotes` | POST | Exportar albaranes de compra por perfil y rango de fechas/proveedores |
| `ExportManagmentDocumentsByExportProfile` | POST | Exportar albaranes y facturas de gestión (cabeceras + líneas + vencimientos) |

**Documentación encontrada en `# WEBLINK RESTAPI.md`:**

- **Línea 9614:** `ExportPurchaseNotes` — exporta albaranes de compra con estructura definida por perfil de exportación
- **Parámetros:** `InitialSupplier`, `FinalSupplier` (rango de proveedores), perfil de exportación
- **Respuesta incluye:** `Serie_Albaran`, `Num_Albaran`, `Fecha_Albaran`, `Cod_Proveedor`, `Nom_Proveedor`, líneas con artículos, cantidades, precios

### ¿Por qué no se incluyó originalmente?

1. **Complejidad:** Es un dominio completo propio (proveedores + albaranes + recepciones + conciliaciones con inventario). No es un endpoint simple como "crear comanda".
2. **Tiempo:** La integración original priorizó el flujo core del restaurante (crear comandas, pagar, facturar). Compras es un flujo secundario.
3. **Riesgo:** Integrar compras mal podría crear descorrelaciones contables entre Glory y BDP.

### Diseño propuesto (revisado: 3 fases, no 2)

**Fase 1 — Solo lectura (~8h):**

| Capa | Archivo nuevo | Descripción |
|------|---------------|-------------|
| Backend | `src/services/bdp_weblink.rs` | Método `export_purchase_notes()` |
| Backend | `src/services/bdp_weblink_catalog.rs` | Structs `BdpPurchaseNoteRequest/Response` |
| Backend | `src/handlers/bdp_purchases.rs` | `GET /api/bdp/purchases` — listar albaranes de compra |
| Frontend | `bdp-purchases-panel.tsx` | Tabla de albaranes con filtros por fecha/proveedor |
| Migration | `bdp_purchase_notes` | Tabla local para cachear albaranes importados |

**Fase 2 — Crear borradores en BDP, sin afectar inventario (~10h):**

Justificación: las compras reales rara vez coinciden 1-a-1 con el albarán (mercancía dañada, entregas parciales). Escribir directamente al inventario de BDP sin paso de reconciliación es arriesgado.

| Capa | Archivo nuevo | Descripción |
|------|---------------|-------------|
| Backend | `src/services/bdp_sync.rs` | `create_purchase_note_draft()` — crea albarán en estado "pendiente" en BDP |
| Frontend | `FormularioCompraBdp.tsx` | Formulario para crear borrador con proveedor, artículos, cantidades |
| Migration | `bdp_suppliers` | Tabla local de proveedores importados de BDP |
| Audit | `bdp_audit` | Log obligatorio de cada borrador creado |

**Fase 3 — Recepción y reconciliación (~12h):**

| Capa | Archivo nuevo | Descripción |
|------|---------------|-------------|
| Backend | `src/services/bdp_sync.rs` | `receive_purchase_note()` — confirma recepción física y actualiza inventario en BDP |
| Backend | `src/services/compra_reconciliacion.rs` | Servicio que compara borrador vs lo recibido, marca diferencias |
| Frontend | `FormularioRecepcionCompra.tsx` | Formulario para confirmar/ajustar cantidades recibidas por línea |

### Flujo de datos (Fase 1 — lectura)

```
BDP WebLink API
  → ExportPurchaseNotes (POST, con rango fechas/proveedores)
    → Response: lista de albaranes con cabeceras + líneas
      → Backend: parse, upsert en bdp_purchase_notes
        → Frontend: tabla filtrable
```

### Riesgos

| Riesgo | Nivel | Mitigación |
|--------|-------|------------|
| Módulo no activo en BDP | Alto | Verificar con "Subscripción no activada" antes de prometer |
| Complejidad del dominio | Alto | Empezar solo lectura, validar con cliente antes de escritura |
| Descuadrages contables | Medio | Solo lectura elimina este riesgo; escritura requiere conciliación |
| Endpoints no documentados completamente | Medio | El manual tiene la estructura de respuesta pero no todos los campos están documentados |

### Esfuerzo total (revisado): ~34h

| Fase | Tiempo |
|------|--------|
| Fase 1 (solo lectura) | ~8h |
| Fase 2 (borradores sin inventario) | ~10h |
| Fase 3 (recepción + reconciliación) | ~12h |
| Tests + integración | ~4h |

### Estado actual

- **Fase 1 — Lectura de albaranes**: ✅ Implementada en `247A-11`.
  - Backend: `export_purchase_notes` + structs + repositorio + handler.
  - Frontend: página `/bdp/compras` con tabla, filtros y sync.
  - Migración: `bdp_purchase_notes` con cache local.
  - Tests: parsing de fechas, serialización de request/response, unit tests.
- **Fase 2 — Crear borradores**: ⏳ Pendiente de consulta cliente.
- **Fase 3 — Recepción y reconciliación**: ⏳ Pendiente de consulta cliente.

### Recomendación: **PENDIENTE DE CONSULTA AL CLIENTE**

Preguntar:
1. ¿El módulo de compras está activo en su BDP?
2. ¿Necesitan ver albaranes de compra desde Glory?
3. ¿Necesitan crear albaranes desde Glory o solo consultar?

Si solo necesitan consultar → **Fase 1 ya está lista**.
Si necesitan crear → **Fases 2 (10h)** sin tocar inventario.
Si necesitan ciclo completo → **Fases 2 + 3 (22h)**.

---

## D3 — Sincronización bidireccional automática

### Estado actual

En `src/handlers/configuracion.rs:296`:
```rust
"Modo BDP inválido; use read_only o unidirectional. 
bidirectional está bloqueado hasta que exista un contrato implementado y auditado."
```

**El código rechaza explícitamente el modo `bidirectional`.**

### ¿Qué implica la bidireccionalidad?

| Dirección | Actualmente | Con bidireccionalidad |
|-----------|-------------|----------------------|
| BDP → Glory | Manual (botones "Sync") | Automático (polling/webhooks) |
| Glory → BDP | Manual (auto-arming propuesto en C1) | Automático (detección de cambios) |

### ¿Por qué está bloqueado?

1. **Bucles de sincronización:** Si Glory cambia un artículo → BDP recibe el cambio → BDP notifica a Glory → Glory procesa → BDP recibe... bucle infinito.
2. **Resolución de conflictos:** ¿Qué pasa si el mismo artículo se modifica en Glory y BDP simultáneamente? ¿Quién gana?
3. **Pérdida de datos:** Un cambio automático mal procesado podría sobreescribir datos maestros de BDP.
4. **Event sourcing:** Necesitaríamos un sistema completo de eventos con timestamps, deduplicación y rollback.

### ¿Podría implementarse técnicamente?

**Sí, pero con restricciones severas:**

| Componente | Necesario | Complejidad |
|------------|-----------|-------------|
| Webhooks BDP → Glory | BDP no tiene webhooks nativos | Polling frecuente (cada 30s) |
| Detección de cambios Glory → BDP | Triggers en BD o event log | Media |
| Resolución de conflictos | Timestamps + regla "último gana" o "BDP gana siempre" | Alta |
| Deduplicación | IDempotency keys por operación | Media |
| Rollback automático | Snapshot antes de cada sync + restore si falla | Alta |
| Monitoreo | Dashboard de sync status, errores, conflictos | Media |

### Esfuerzo: ~40-60h incluso para versiones limitadas

| Componente | Tiempo |
|------------|--------|
| Diseño del contrato de sincronización | 8h |
| Implementación polling bidireccional | 12h |
| Resolución de conflictos | 10h |
| Tests de estrés (bucles, colisiones) | 6h |
| Monitoreo y alertas | 4h |
| Auditoría cruzada y rollback | 6h |

### Riesgos que persisten incluso con versiones limitadas

1. **BDP no soporta webhooks** -> requiere polling agresivo para cualquier implementación.
2. **Polling cada 5 min + resolución "BDP gana":** el retraso de 5 min destruye la propuesta de valor de la escritura local.
3. **Colisiones de inventario:** si Glory y BDP modifican el mismo artículo antes del siguiente poll, el cliente ve cambios "fantasma" sin origen.
4. **Bucles endémicos:** toda solución "limitada" acaba pidiendo más alcance al acercarse a un caso útil — la versión limitada se vuelve completa o se abandona.

### Recomendación: **RECHAZAR FIRME**

No ofrecer NINGUNA versión limitada, ni siquiera reducida a solo catálogo. La sincronización manual con auto-arming (C1) cubre el caso real (operador escribe una venta, no 100). Si el cliente pide sync automática, explicar que BDP no está diseñado para ello y que construir un event-sourcing distribuido encima de un API sin push sería esfuerzo enorme, valor cuestionable y riesgo de pérdida de datos cierto.

**Alternativa recomendada (si el cliente insiste):** Implementar `read-only diff` — un cronjob que muestra "estos 3 artículos cambiaron en BDP desde tu última sync" como aviso, sin aplicar cambios automáticamente. Visibilidad sin riesgo de sobreescritura.

---

## D4 — Pagos parciales

### Riesgo crítico identificado: race condition entre dispositivos

**Escenario peligroso:** Dos camareros en dos tablets distintas abren la misma mesa simultáneamente. Ambos ven saldo pendiente = €100. Ambos ejecutan pago parcial de €30 concurrentemente.

```
Hilo A: GetOrder → pending=€100 → AddOrderPayment(30) ✅ BDP procesa
Hilo B: GetOrder → pending=€100 (lee antes de que A termine) → AddOrderPayment(30) ✅ BDP procesa
Resultado: el cliente paga €60 de deuda €100, pero BDP queda con dos pagos sum = €60 sin ledger que los reconcilie.
```

**Mitigación obligatoria (no negociable):** implementar lock distribuido por `venta_id` antes de cualquier lectura + escritura a BDP.

### Diseño propuesto (revisado)

```rust
// Pseudocódigo del handler
let lock_key = format!("bdp_payment_lock:{}", venta_id);
let _guard = redis_lock.acquire(lock_key, ttl=10s).await?;

let current_bdp_state = client.get_order(bdp_order_id).await?;
let pending_amount = current_bdp_state.pending_amount;

if requested > pending_amount + 0.005 {
    return Err(BdpPaymentError::ExceedsPending {
        requested, pending_amount
    });
}

let partial_payment = bdp_partial_payments::create(...).await?;
client.add_order_payment(bdp_order_id, requested).await?;
partial_payment.mark_completed().await?;
```

Tabla nueva `bdp_payment_locks` (o Redis) con TTL corto (10s) dedicado a evitar duplicación concurrente.

### ¿Por qué están bloqueados?

1. **BDP no soporta pagos parciales nativos:** La API `AddOrderPayment` acepta un monto, pero no hay mecanismo para "fraccionar" una comanda en múltiples pagos.
2. **Riesgo de descuadres:** Si Glory registra un pago parcial pero BDP no lo entiende así, la caja cuadrará en Glory pero no en BDP.
3. **Idempotencia:** Sin un ledger independiente, no sabríamos cuánto se ha pagado realmente vs. cuánto Glory cree que se ha pagado.

### ¿Podría implementarse técnicamente?

**Sí, con un ledger independiente:**

| Componente | Descripción |
|------------|-------------|
| Tabla `bdp_partial_payments` | Registra intenciones de pago parcial: venta_id, monto, estado, timestamp |
| Servicio `PartialPaymentService` | Gestiona el ciclo de vida de pagos parciales |
| Validación contra BDP | Antes de cada pago parcial, consulta `GetOrder` para obtener saldo real |
| Reconciliación | Si BDP reporta un monto diferente al esperado, alertar y bloquear |

### Flujo propuesto

```
Usuario quiere pagar €30 de una comanda de €100
  → Frontend: muestra "Saldo pendiente: €100. ¿Cuánto pagar?"
  → Usuario ingresa €30
  → Backend:
    1. Consulta GetOrder → total=€100, paid=€0, pending=€100
    2. Valida: €30 ≤ €100 (OK)
    3. Registra en bdp_partial_payments: {venta_id, monto=30, estado="pendiente"}
    4. Llama AddOrderPayment(amount=30)
    5. Si éxito → actualiza estado a "completado"
    6. Si error → marca estado a "error", no reintenta
  → Frontend: muestra "Pago parcial de €30 registrado. Saldo pendiente: €70"
```

### Riesgos

| Riesgo | Nivel | Mitigación |
|--------|-------|------------|
| Descuadre Glory vs BDP | Alto | Reconciliación automática tras cada pago |
| BDP rechaza pago parcial | Medio | Validar contra GetOrder antes de enviar |
| Reintento automático duplica pago | Alto | Idempotency key por pago, no reintento automático |
| Usuario confundido por estado parcial | Medio | UI clara: "Pagado: €30 / Pendiente: €70" |

### Esfuerzo (revisado): ~18-22h

| Tarea | Tiempo |
|-------|--------|
| Migration + modelo `bdp_partial_payments` | 2h |
| **Lock distribuido por venta_id (Redis o tabla)** | **3h** |
| Servicio de pagos parciales con lock | 5h |
| Validación contra GetOrder | 2h |
| Frontend: selector de monto + indicador visual de saldo | 4h |
| Reconciliación automática + casos de drift | 4h |
| Tests + race conditions (tests concurrentes) | 2h |

### Recomendación: **PENDIENTE DE CONSULTA AL CLIENTE**

Preguntar:
1. ¿Necesitan pagar comandas en partes?
2. ¿Con qué frecuencia?
3. ¿Tienen varios dispositivos accediendo a la misma mesa simultáneamente?
4. ¿Están dispuestos a asumir el riesgo de descuadres mientras se implementa el ledger?

Si la respuesta es sí → implementar **con lock distribuido obligatorio**.
Si es no → mantener bloqueo actual.

---

## D5 — CancelOrder

### Estado actual

En `src/services/bdp_weblink.rs:253-258`:
```rust
pub async fn cancel_order(&self, request: &BdpCancelOrderRequest) -> Result<BdpResponse, BdpWeblinkError> {
    self.post_authenticated_json(BDP_PATH_CANCEL_ORDER, request).await
}
```

**El método existe pero no está expuesto vía REST.** Al probarlo, BDP devuelve: `"Subscripción no activada"`.

### Documentación BDP

En `# WEBLINK RESTAPI.md:7456`:
```
### CancelOrder
Elimina una comanda en la base de datos.
Parámetros: OrderId, MarketplaceOrderId, MarketId, RoomNumber, TableNumber
```

**El endpoint existe en BDP pero requiere activación de módulo.**

### Diseño propuesto (si BDP activa el módulo)

| Capa | Archivo | Cambio |
|------|---------|--------|
| Backend | `src/handlers/ventas.rs` | Nuevo endpoint `POST /api/ventas/:id/bdp-cancel` |
| Backend | `src/services/bdp_sync.rs` | Método `cancel_order()` con validaciones |
| Frontend | `venta-row-actions.tsx` | Botón 🗑️ "Cancelar en BDP" |

**Validaciones antes de cancelar:**

1. Consultar `GetOrder` para verificar que la comanda existe en BDP
2. Verificar que el estado no sea `cancelled` (status=2) ni `invoiced` (status=3)
3. Verificar que la venta local esté sincronizada (`bdp_synced = true`)
4. Registrar en auditoría: motivo de cancelación (opcional), usuario, timestamp

**Flujo:**

```
Usuario pulsa "Cancelar en BDP" en venta-row-actions
  → Modal: "¿Cancelar comanda #X en BDP? Esta acción no se puede deshacer."
  → [Campo opcional: motivo]
  → [Confirmar] / [Cancelar]
  → Backend:
    1. GetOrder → verificar estado
    2. CancelOrder(order_id)
    3. Actualizar bdp_order_status = "cancelled" en Glory
    4. Audit log
```

### Seguridad

| Riesgo | Mitigación |
|--------|------------|
| Cancelar comanda en preparación | Confirmación explícita + motivo obligatorio |
| Cancelar comanda ya cancelada | Validar estado antes de enviar |
| Cancelar comanda facturada | Bloquear si status = invoiced |
| Pérdida de materia prima | Documentar que cancelar no revierte preparación |

### Complejidad real mucho mayor que el flujo feliz

**Factores no contemplados en la estimación inicial de 4h:**

1. **Reversión de impuestos en Glory.** La venta original ya tiene IVA aplicado. Una cancelación debe:
   - Emitir nota de crédito o marcar la factura original como anulada.
   - Revertir la partida de IVA en el resumen diario del restaurante.
   - Decidir si Glory permite cancelar ventas facturadas o solo no facturadas (afecta a qué subset aplica).
2. **Ticket de cocina.** BDP puede haber impreso el ticket en cocina antes de la cancelación. Cancelar en BDP no avisa a la cocina. Hay que decidir si mostrar mensaje al usuario "Cancelar no retira el ticket físico — avisar manualmente".
3. **Interacción con pagos parciales (D4).** Si la venta tiene pagos parciales, CancelOrder debe decidir:
   - ¿Devolver los pagos parciales?
   - ¿Marcar como cancelada pero los pagos quedan como "anticipo"?
   - ¿Bloquear cancelación si hay pagos parciales no reintegrados?
4. **Liberación de mesa.** La mesa puede estar ocupada en el plano de sala. Cancelación debe liberar la mesa al estado original.
5. **Notificación de inventario.** Si la cancelación se hace después de preparada la comanda, no revierte consumo de materia prima. Documentar claramente.
6. **Disponibilidad del método.** Hoy el método `cancel_order()` existe en `bdp_weblink.rs:253` pero BDP devuelve "Subscripción no activada". Toda la implementación queda bloqueada hasta que BDP active el módulo.

### Esfuerzo (revisado): ~12-16h

| Tarea | Tiempo |
|-------|--------|
| Backend: handler + servicio + validaciones | 3h |
| Backend: lógica de reversión de impuestos + nota de crédito | 3h |
| Backend: integración con pagos parciales (D4) si D4 implementado | 2h |
| Backend: liberación de mesa + notificación | 1h |
| Frontend: modal + botón + mensajes contextuales | 2h |
| Documentación de "no revierte cocina" en UI | 1h |
| Tests + casos borde (facturada, con pagos, sin sync) | 2-4h |

### Recomendación: **PENDIENTE DE BDP** + revisión interna

1. **Preguntar al cliente:** ¿Pueden activar el módulo de cancelación en BDP?
2. **Si sí →** Implementar (12-16h, riesgo medio por impuestos/D4).
3. **Si no →** Mantener como limitación documentada. No implementar sin el módulo activo porque no se puede probar.

---

## Actualización 247A-7 — Mitigaciones críticas implementadas (R1, R5, R14)

> **Fecha:** 2026-07-25  
> **Objetivo:** Cerrar las mitigaciones técnicas críticas restantes antes de producción. Compras/pagos parciales siguen pendientes de decisión del cliente.

### Mitigaciones técnicas críticas — Estado

| ID | Riesgo | Solución técnica | Estado | Riesgo residual |
|---|---|---|---|---|
| **R1** | Comandas/pagos/facturas marcados `ambiguo` sin reconciliar | Worker `reconcile_ambiguous_orders` en `bdp_order_poller`; consulta `GetOrder` y cierra auditorías `ambiguo`. | ✅ Implementado | Bajo si GetOrder funciona; si BDP no tiene registro, queda en ambiguo para revisión manual. |
| **R5** | `sync_venta` puede acumular llamadas BDP sin límite global | Fase HTTP envuelta en `tokio::time::timeout(Duration::from_secs(45))`. | ✅ Implementado | Bajo; si se alcanza el timeout se marca ambiguo. |
| **R14** | Limpieza manual de `SYNC_LOCKS` | Guard RAII `SyncLockGuard` con `impl Drop { cleanup_lock }`. | ✅ Implementado | Bajo; elimina olvidos de cleanup. |
| **R2-nota** | Lock cross-instance perdido tras early commit | Documentado. Si se despliega en multi-instance, evaluar `pg_advisory_lock` de sesión o columna `bdp_sync_status`. |  Documentado | Medio en multi-instance; bajo en single-instance. |

### Compras (D2) — Planificación refinada

**Alcance recomendado:** Empezar con **lectura de albaranes** (Fase 1) y detenerse hasta que el cliente valide utilidad y estado del módulo en BDP.

**Fases:**
1. **Fase 1 — Lectura de albaranes (8h):**
   - Backend: `export_purchase_notes` en `bdp_weblink.rs` + structs `BdpPurchaseNoteRequest/Response`.
   - Handler: `GET /api/bdp/purchase-notes`.
   - Frontend: `bdp-purchase-notes-panel.tsx` con filtros fecha/proveedor.
   - Migration: `bdp_purchase_notes` cache local.
2. **Fase 2 — Crear borradores (10h):** (solo si cliente lo pide)
   - Formulario de compra con proveedor, artículos, cantidades.
   - Endpoint `POST /api/bdp/purchase-notes` que crea borrador en BDP sin tocar inventario.
3. **Fase 3 — Recepción y reconciliación (12h):** (solo si cliente lo pide)
   - Confirmar cantidades recibidas y actualizar BDP.

**Preguntas clave al cliente:**
1. ¿El módulo de compras/albaranes está activo en su BDP?
2. ¿Necesitan ver albaranes desde Glory o solo desde BDP?
3. ¿Necesitan crear/recepcionar compras desde Glory?

### Pagos parciales (D4) — Planificación refinada

**Alcance recomendado:** Solo si el cliente confirma que lo necesita y acepta riesgo residual de conciliación.

**Componentes obligatorios:**
1. **Ledger local:** tabla `bdp_partial_payments(venta_id, monto, estado, idempotency_key, created_at)`.
2. **Lock distribuido por venta_id:** evitar pagos concurrentes sobre la misma orden (Redis o tabla Postgres con TTL).
3. **Idempotencia:** cada intento genera UUID; si se repite, devuelve resultado cacheado.
4. **UI de saldo:** mostrar "Pagado: X / Pendiente: Y" en tiempo real tras cada pago.
5. **Reconciliación periódica:** polling consulta `GetOrder` y corrige estado si hay drift.

**Preguntas clave al cliente:**
1. ¿Necesitan pagar comandas en varias partes?
2. ¿Varios dispositivos/tabletas acceden a la misma mesa a la vez?
3. ¿Aceptan que, mientras se implementa, pueda haber pequeños descuadres a conciliar?

### Riesgos transversales a vigilar

- **Semaforo/throttling (XT1):** Actualmente rechaza con `Throttled` y se mapea a ambiguo. Revisar que el límite de 2 concurrentes no sea excesivamente restrictivo bajo carga real.
- **Feature flags (XT2):** `ff_bdp_auto_arm`, `ff_bdp_partial_payments`, `ff_bdp_purchase_notes_*` están desactivados por defecto. UI debe mostrar explícitamente cuando una función está bloqueada por flag.
- **Auto-arming (C1):** Ya implementado. Asegurar que la confirmación dinámica no sea trivial (evitar texto fijo "CONFIRMAR"). Mejorar a pregunta contextual: monto exacto, número de venta, etc.
- **Toggle navbar (C2):** Ya implementado. Falta restringir a admins (pendiente de store de auth).
- **Stock (D1):** Implementado como columna en tabla de mapeos. Si el cliente necesita pantalla dedicada, requiere endpoints `GetStock`/`GetListStock` y ~12h adicionales.

---

## Concerns transversales (aplican a varios items)

### XT1 — Throttling y rate limiting contra BDP

**Problema:** los TPV de BDP corren en hardware local del restaurante, frecuentemente modestos. Si `C1 (Auto-arming)` o `D4 (Pagos parciales)` permiten ráfagas de operaciones, podemos saturar el servidor BDP y provocar timeouts en cascada.

**Mitigación obligatoria (cross-cutting):** implementar semáforo global de concurrencia para todas las operaciones de **escritura y lectura** a BDP. El throttling no debe limitarse a escrituras; `GetOrder`, `ExportArticles` y otras lecturas también saturan el TPV.

```rust
// src/services/bdp_throttle.rs (nuevo)
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::{Semaphore, SemaphorePermit};

pub struct BdpThrottle {
    /// Semáforo global por base_url de BDP (máx 2 concurrentes por defecto)
    global: Semaphore,
    /// Cola lógica: contador de requests esperando
    waiting: AtomicU64,
    /// Base URL para diagnóstico
    base_url: String,
}

pub struct BdpThrottleManager {
    per_target: DashMap<String, Arc<BdpThrottle>>,
}

impl BdpThrottleManager {
    pub fn get_or_create(&self, base_url: &str) -> Arc<BdpThrottle> { ... }
    pub async fn acquire(&self, base_url: &str) -> Result<BdpThrottleGuard, ThrottleError> { ... }
}
```

**Configuración en `configuracion_restaurante`:**
- `bdp_max_concurrent_requests` (default 2, mínimo 1)
- `bdp_request_queue_limit` (default 50; pasado esto devolver 503)
- `bdp_request_timeout_secs` (default 30s)

**Ámbito del semáforo:**
- Todas las llamadas a `BdpWeblinkClient` (reads + writes)
- Key: `base_url` del restaurante (un restaurante no bloquea a otro)
- El semáforo vive en `AppState` como `BdpThrottleManager`

**Visibilidad:** endpoint `GET /api/bdp/diagnostics` expone `queue_depth`, `active_requests`, `max_concurrent`.

### XT2 — Feature flags / despliegue progresivo

**Problema:** `C1`, `D4`, `D5` son cambios que afectan a operaciones críticas (pagos, facturación, cancelación). Lanzarlos globalmente sin posibilidad de apagar por cliente es un riesgo operacional.

**Mitigación:** feature flags por restaurante integrados en `configuracion_restaurante`.

**Decisión arquitectónica:** Usar columnas booleanas en `configuracion_restaurante` en vez de tabla separada. Razones:
- Los flags configuran comportamiento del **módulo BDP interno** de un único restaurante.
- La configuración ya se carga en caché/memoizada en cada request; un JOIN a tabla separada añadiría latencia.
- El modelo Rust `ConfiguracionRestaurante` ya existe y se usa en todos los handlers.

```sql
ALTER TABLE configuracion_restaurante
  ADD COLUMN ff_bdp_auto_arm BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN ff_bdp_partial_payments BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN ff_bdp_cancel_order BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN ff_bdp_purchase_notes_read BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN ff_bdp_purchase_notes_draft BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN ff_bdp_purchase_notes_receive BOOLEAN NOT NULL DEFAULT FALSE;
```

**Features a flaggear:**
- `ff_bdp_auto_arm` (C1)
- `ff_bdp_partial_payments` (D4)
- `ff_bdp_cancel_order` (D5)
- `ff_bdp_purchase_notes_read` (D2 Fase 1)
- `ff_bdp_purchase_notes_draft` (D2 Fase 2)
- `ff_bdp_purchase_notes_receive` (D2 Fase 3)

**Flujo de rollout recomendado:**
1. Implementar feature en código, marcado como `enabled = false` por defecto.
2. Activar manualmente en el restaurante piloto desde Configuración.
3. Validar 1-2 semanas de uso real.
4. Si funciona, dejar como opt-in para otros restaurantes. No activar globalmente.

### Concerns transversales — Documentación cliente actualizada

Cuando se implementen `C1`, `D4`, `D5` o cualquier feature D2, actualizar también:

- `Agente/usuario/guia-cliente-pruebas-integracion-bdp-2026-07-18.md` — añadir sección sobre la nueva feature con captura o mockup.
- `Agente/usuario/mapeo-visual-integracion-bdp-2026-07-23.md` — registrar nueva entrada visual donde corresponda.
- Si la feature añade toggle en UI → añadir a la sección "Lo que el cliente ve hoy".

---

## Priorización recomendada

### Si el cliente quiere la mejor experiencia posible (revisado):

**Pre-requisitos cross-cutting primero:**

| Orden | Item | Por qué |
|-------|------|---------|
| 0a | **XT2: Feature flags** | Base para activar C1, D4, D5 por restaurante sin redeploy |
| 0b | **XT1: Throttling BDP** | Base para evitar saturar el TPV local con C1/D4 |
| 1 | **C1: Auto-arming** | Impacto máximo en usabilidad diaria |
| 2 | **C2: Toggle navbar** | Ortogonal a C1 — admin bulk reconciliations |
| 3 | **D1: Validar parser stock** | 2h, ya implementado, sólo falta parser defensivo |
| 4 | **D2: Compras (Fase 1)** | Solo lectura, 8h, gather data sin tocar inventario |
| 5 | **D5: CancelOrder** | 12-16h si BDP activa el módulo |
| 6 | **D4: Pagos parciales** | 18-22h con lock distribuido |
| 7 | **D2: Compras (Fase 2)** | 10h borradores sin inventario |
| 8 | **D2: Compras (Fase 3)** | 12h recepción y reconciliación |
| **NUNCA** | **D3: Bidireccional** | Rechazado firme. No backlog. |

### Si el cliente quiere lo mínimo viable:

| Orden | Item | Por qué |
|-------|------|---------|
| 0 | **XT2: Feature flags** | Base minima para op-in por restaurante |
| 1 | **D1: Parser defensivo stock** | 2h implementación proactiva |
| 2 | **C1: Auto-arming** | Mejora crítica de UX |
| 3 | **C2: Toggle navbar** | Complemento administrativo, 3h |

---

## Preguntas para el cliente

1. **Auto-arming (C1):** ¿Quieren activar escritura automática con confirmación dinámica (monto/ID/lineas) + idempotency, o mantener el flujo manual actual? ¿Están cómodos con que el operador diario pueda ejecutar operaciones BDP con un solo modal?
2. **Toggle navbar (C2):** ¿El restaurante tiene un管理员 (admin/manager) que necesite inspección rápida + arming bulk? Esto justifica C2 incluso con C1 activo.
3. **Stock (D1):** ¿La columna actual en la tabla de mapeos basta o necesitan pantalla dedicada con stock por almacén/familia?
4. **Compras (D2):** ¿El módulo de compras está activo en su BDP? ¿Necesitan solo consultar o también crear albaranes? ¿Las compras reales requieren conciliación (mercancía dañada/entregas parciales)?
5. **CancelOrder (D5):** ¿Pueden activar el módulo de cancelación en su BDP? ¿Tienen ventas facturadas que necesiten cancelarse por error (cliente cambia de opinión, plato equivocado)?
6. **Pagos parciales (D4):** ¿Necesitan pagar comandas en partes (ej: parte en efectivo, parte con tarjeta)? ¿Con qué frecuencia? ¿Cuántos dispositivos distintos acceden a la misma mesa simultáneamente?
7. **Bidireccional (D3):** ¿Necesitan sincronización automática o con los botones manuales actuales + auto-arming basta? (Recomendación firme: rechazar.)
8. **Feature flags:** ¿Están cómodos con que las nuevas features se activen opcionalmente por restaurante primero, en lugar de venir activadas por defecto?

---

## Documentación de referencia

| Documento | Contenido |
|-----------|-----------|
| `# WEBLINK RESTAPI.md` | Manual completo de la API WebLink (11000+ líneas) |
| `Agente/usuario/mapeo-visual-integracion-bdp-2026-07-23.md` | Mapeo visual de dónde está cada funcionalidad |
| `Agente/usuario/auditoria-cruzada-bdp-endpoints-frontend-2026-07-23.md` | Tabla cruzada endpoints ↔ frontend |
| `Agente/planes/plan-visibilidad-bdp-frontend-2026-07-23.md` | Plan original de visibilidad |
| `Agente/documentacion/api/bdp-integration-status-2026-06-07.md` | Estado de integración completo |
| `Agente/usuario/auditoria-adversarial-bdp-2026-07-22.md` | Auditoría de seguridad |
| `Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md` | Evaluación de riesgos en producción tras fix ON CONFLICT |
