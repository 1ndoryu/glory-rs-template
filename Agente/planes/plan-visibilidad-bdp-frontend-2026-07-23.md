# Plan de implementación — Completar visibilidad BDP en frontend

> **Fecha:** 2026-07-23 (actualizado 2026-07-24 tras 247A-1)
> **Origen:** `mapeo-visual-integracion-bdp-2026-07-23.md` — análisis de gaps entre backend implementado y UI visible
> **Objetivo:** Que cada funcionalidad BDP implementada en el backend tenga su interfaz correspondiente en el frontend, accesible de forma intuitiva
> **Changelog:** 2026-07-24 — marcado C1, C2, XT1 y XT2 como implementados en 247A-1. 2026-07-24 (247A-4) — añadida referencia a evaluación de riesgos BDP en producción (`Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md`).

---

## Bloque A — Visibilidad inmediata (funcionalidad ya existe, solo falta exponerla)

### A1: Exponer catálogo BDP como sección de primer nivel

**Problema:** La tabla de mapeos de artículos, los botones "Sync catálogo" y "Sync precios" están enterrados en Configuración → BDP → "Configuración técnica (solo soporte)" → `config-bdp-mapeos.tsx` → `bdp-article-map-table.tsx`. Tres niveles de profundidad.

**Solución:** Extraer `BdpArticleMapTable` del colapsable y mostrarlo como sección visible en la pestaña BDP de Configuración, con su propio header "Catálogo de artículos BDP".

**Archivos a modificar:**
- `frontend/src/componentes/ConfigBdp.tsx` — reorganizar layout, sacar tabla del colapsable
- `frontend/src/components/config-bdp-mapeos.tsx` — posible refactor para separar catálogo de mapeos técnicos

**Esfuerzo:** ~2h
**Riesgo:** BAJO — solo reorganización de UI, sin cambios de lógica

---

### A2: Exponer mapeos técnicos (tender, canales, cliente por defecto) como sección visible

**Problema:** Los campos de configuración técnica (formas de pago, canales, artículo fallback, cliente por defecto, polling) están dentro del mismo colapsable que el catálogo.

**Solución:** Mantener como sección separada pero visible (no colapsable por defecto), con un header claro como "Correspondencias Glory ↔ BDP" y una nota informativa de que dependen de la instalación del restaurante.

**Archivos a modificar:**
- `frontend/src/componentes/ConfigBdp.tsx` — quitar el colapsable `mostrarMapeos` o hacer que los mapeos técnicos estén visibles por defecto

**Esfuerzo:** ~1h
**Riesgo:** BAJO

---

### A3: Añadir botón "Consultar estado BDP" por venta

**Problema:** El endpoint `GET /api/ventas/:id/bdp-status` y el hook `useBdpStatus` existen pero NO hay botón en la UI que los use. El usuario solo ve el estado a través del badge automático.

**Solución:** Añadir un `TooltipButton` con ícono de lupa/reloj en `VentaRowActions` que consulta el estado individual de una comanda BDP y muestra el resultado en un toast o popover.

**Archivos a modificar:**
- `frontend/src/components/venta-row-actions.tsx` — añadir botón + lógica de consulta
- `frontend/src/api/bdp.ts` — el hook `useBdpStatus` ya existe, solo hay que usarlo

**Esfuerzo:** ~2h
**Riesgo:** BAJO — hook ya implementado, solo falta el botón

---

### A4: Exponer toggle de polling automático en vista principal

**Problema:** El switch "Actualizar estados automáticamente" (`bdp_poll_enabled`) está enterrado en `config-bdp-mapeos.tsx` dentro del colapsable de Configuración técnica.

**Solución:** Mover el toggle a la vista principal de `ConfigBdp.tsx`, cerca del switch "Integración BDP activa", con texto explicativo.

**Archivos a modificar:**
- `frontend/src/componentes/ConfigBdp.tsx` — añadir toggle de polling
- `frontend/src/components/config-bdp-mapeos.tsx` — quitar el toggle de aquí (o mantenerlo duplicado)

**Esfuerzo:** ~1h
**Riesgo:** BAJO

---

## Bloque B — UI nueva para funcionalidad backend existente

### B1: Pantalla de menús, packs y fastfoods (solo lectura)

**Problema:** Los endpoints `GET /api/bdp/menus/:id`, `GET /api/bdp/fastfoods/:id`, `GET /api/bdp/packs/:id` existen en el backend. Los hooks generados (`useGetFastfoodDefinition`, etc.) existen en `frontend/src/api/generated/bdp-mapeos/bdp-mapeos.ts`. Pero NO hay ningún componente de UI que los consuma.

**Solución:** Crear un componente `BdpMenuExplorer.tsx` que permita buscar/consultar menús, packs y fastfoods por ID, mostrando su estructura en formato de solo lectura. Ubicarlo como subsección dentro del área de Catálogo BDP (Bloque A1).

**Archivos a crear:**
- `frontend/src/components/bdp-menu-explorer.tsx` — componente de consulta

**Archivos a modificar:**
- `frontend/src/componentes/ConfigBdp.tsx` — integrar el componente

**Esfuerzo:** ~4h
**Riesgo:** BAJO — solo lectura, hooks ya generados

---

### B2: Indicador rápido de estado BDP en barra de navegación

**Problema:** No hay forma de saber el estado de la integración BDP sin ir a Configuración.

**Solución:** Añadir un indicador compacto en la barra lateral o header que muestre:
- "BDP: ✅ Activo" (sync enabled + conexión OK)
- "BDP: ⚠️ Solo lectura" (sync enabled pero sin permisos de escritura)
- "BDP: ❌ Desactivado" (sync disabled)

**Archivos a modificar:**
- `frontend/src/components/layout.tsx` o componente de navegación equivalente

**Esfuerzo:** ~2h
**Riesgo:** BAJO

---

## Bloque C — Mejoras de flujo (requieren decisión de producto)

### C1: Auto-arming para operaciones de escritura

**Problema:** Hoy, para pagar/facturar una venta en BDP, el usuario debe ir a Configuración → BDP → Seguridad → cambiar modo a "Escritura temporal". Esto es inviable en operación diaria.

**Solución propuesta:** Que los botones de pago/factura (que ya tienen confirmación textual) activen automáticamente el arming, ejecuten la operación, y vuelvan a read_only. La confirmación textual del frontend sería la autorización.

**⚠️ Advertencia:** Esto modifica el modelo de seguridad fail-closed diseñado intencionalmente. Requiere:
- Revisar `BdpWriteGuard::authorize()` para aceptar arming inline
- Validar confirmación textual server-side (no solo UI)
- Mantener advisory lock, fingerprint y `ensure_no_unresolved()`
- Registrar en auditoría que el arming fue automático

**Archivos a modificar (backend):**
- `src/services/bdp_write_guard.rs` — nuevo método `authorize_inline()` o similar
- `src/handlers/ventas.rs` — modificar endpoints de pago/factura para auto-armar

**Archivos a modificar (frontend):**
- `frontend/src/components/venta-row-actions.tsx` — los botones ya existen, solo cambiarían el flujo backend

**Esfuerzo:** ~10-12h (backend: 6-8h, frontend: 4h)
**Riesgo:** ALTO — modifica modelo de seguridad

**Estado:** ✅ **Implementado (247A-1)** — `BdpWriteGuard::try_auto_arm()` crea arming efímero bajo advisory lock, con idempotencia por `idempotency_key` y devolución del resultado cacheado en reintentos. Los handlers de sync/pago/factura aceptan `auto_arm` + `confirmation_destino`.

---

### C2: Toggle rápido de modo escritura en barra de navegación

**Alternativa más simple a C1.** Si no se implementa auto-arming, al menos mover el selector de modo de sync (solo lectura / escritura temporal) a un lugar accesible, no enterrado en Configuración.

**Archivos a modificar:**
- `frontend/src/components/layout.tsx` — toggle rápido en navbar
- `frontend/src/componentes/PanelBdpBackup.tsx` — el selector ya existe aquí

**Esfuerzo:** ~3h
**Riesgo:** BAJO

**Estado:** ✅ **Implementado (247A-1)** — `BdpStatusIndicator` en `site-header.tsx` muestra el modo actual y permite cambiar a "Solo lectura" / "Escritura temporal" mediante `setSyncMode`, navegando a Configuración → BDP.

---

## Bloque D — Funcionalidad nueva (pendiente de decisión del usuario)

### D1: Consulta de stock desde BDP (solo lectura)

**Estado:** ✅ **IMPLEMENTADO (237A-4)** — Opción rápida aplicada.

**Qué se hizo:**
- Campo `current_stock: Option<Decimal>` añadido a `BdpExportArticleItem` con aliases `CurrentStock`/`Stock`
- `sync_catalog()` mapea `current_stock` → `stock_actual` en la tabla `bdp_article_map`
- Migration `20260723000000_bdp_article_map_stock` añade columna `stock_actual NUMERIC(14,4)`
- Columna Stock + Precio visible en `bdp-article-map-table.tsx`
- Warn log en sync_catalog si ningún artículo trae stock (módulo almacén inactivo)
- Si `CurrentStock` viene `None`, la columna muestra "—"

**Queda como mejora futura:**
2. **Opción completa (~8h):** Implementar `GetStock`/`GetListStock` como endpoints nuevos + pantalla dedicada de stock.

---

### D2: Módulo de compras

**Estado:** ❌ **EXCLUIDO POR DISEÑO** — decisión de alcance del producto
**Esfuerzo:** ~20-30h adicionales
**Nota:** Requiere proveedores, albaranes, recepciones. El endpoint de compras de BDP tiene estructura diferente a comandas.

---

### D3: Modificación de stock

**Estado:** ❌ **EXCLUIDO POR DISEÑO** — solo lectura en la integración actual
**Nota:** Para modificar stock, el cliente debe usar BDP directamente.

---

### D4: Sincronización bidireccional automática

**Estado:** ❌ **EXCLUIDO POR DISEÑO** — bloqueado explícitamente en `configuracion.rs:296`
**Nota:** Solo existe `read_only` y `unidirectional`. `bidirectional` está rechazado.

---

### D5: Pagos parciales

**Estado:** ✅ **IMPLEMENTADO** — backend + frontend + reconciliación de ambiguos.
**Nota:** Implementado bajo feature flag `ff_bdp_partial_payments`. Ledger local en tabla `bdp_pagos` con idempotency_key. Endpoint `GET /api/ventas/:id/bdp-payments`. UI en `venta-row-actions.tsx`. Ver `Agente/planes/plan-pagos-parciales-bdp-2026-07-25.md` para detalles.
**Pendiente:** tests de servicio con simulador BDP, activación en producción.

---

## Resumen de esfuerzo

| Bloque | Tareas | Esfuerzo | Riesgo | Estado |
|--------|--------|----------|--------|--------|
| **A** — Visibilidad inmediata | A1+A2+A3+A4 | ~6h | BAJO | ✅ Implementado 23 julio 2026 |
| **B** — UI nueva | B1+B2 | ~6h | BAJO | ✅ Implementado 23 julio 2026 |
| **C** — Mejoras de flujo | C1 auto-arming + C2 toggle navbar + XT1 throttling + XT2 feature flags | ~16-20h | MEDIO | ✅ Implementado 247A-1 |
| **D** — Funcionalidad nueva | D1 stock implementado | ~2h | BAJO | ✅ Implementado 23 julio 2026 (237A-4) |
| **D** — Excluidos | D2+D3+D4+D5 | N/A | N/A | ❌ Pendiente revisión usuario |

**Total si se implementa A+B+C2+D1(opción rápida):** ~16h
**Total si se implementa A+B+C1+D1(opción completa):** ~30h

---

## Orden de ejecución recomendado

```
Fase 1 (Bloque A): A1+A2+A3+A4 — Desenterrar lo que ya existe (~6h)
    ↓ Validar: typecheck + verificar que la UI se ve correctamente
Fase 2 (Bloque B): B1+B2 — Crear UI para funcionalidad sin interfaz (~6h)
    ↓ Validar: typecheck + verificar navegación
Fase 3 (Bloque C): C1 o C2 según decisión del usuario (~3-12h)
    ↓ Validar: tests de escritura contra simulador
Fase 4 (Bloque D): D1 si se aprueba (~2-8h según opción)
```

✅ Fases 1, 2, 3 (C1 + C2 + XT1 + XT2) y 4 (stock) implementadas en 237A-3, 237A-4 y 247A-1.
C1 (auto-arming), C2 (toggle navbar), XT1 (throttling) y XT2 (feature flags) ya están en el código.
Pendientes de decisión del usuario: compras (D2), bidireccional (D4), pagos parciales (D5), CancelOrder (pendiente activación módulo BDP).
