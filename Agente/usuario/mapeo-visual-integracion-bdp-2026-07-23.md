# Mapeo visual de la integración BDP — Estado real vs. lo comunicado al cliente

> **Fecha:** 2026-07-23 (actualizado 2026-07-26 — sesión completa de auditoría, testing y feature flags UI)
> **Motivo:** El cliente ha revisado la guía de integración y señala que no ve la mayoría de las funcionalidades descritas en la web. Además tiene dudas sobre compras, stock, importación de catálogo y el flujo de autorización temporal.
> **Objetivo:** Documentar dónde está realmente cada funcionalidad en el frontend, identificar inconsistencias entre lo planificado y el resultado final, y proponer soluciones.
> **Changelog:** 2026-07-26 — actualizado con: feature flags configurables desde UI (267A-4), pagos parciales implementados, compras fases 1-3 implementadas, 219 tests pasando, auditoría completa de documentación.

---

## 1. Resumen ejecutivo

El cliente tiene razón en su observación principal: **la guía describe capacidades que el backend sí implementa, pero muchas no tienen una interfaz visible o accesible de forma intuitiva**. El documento del cliente lista 6 puntos de información de BDP disponibles en Glory, pero la experiencia real se reduce a 2 botones prominentes ("Importar BDP" en Clientes y "Sync BDP" en Plano de Sala) y las demás funciones están ocultas en submenús o solo accesibles bajo condiciones específicas.

### Lo que el cliente ve hoy en la web

| Elemento visible                              | Ubicación                                                       | Qué hace realmente                                                     |
| --------------------------------------------- | --------------------------------------------------------------- | ---------------------------------------------------------------------- |
| **"Importar BDP"**                            | Pestaña Clientes → barra superior                               | Importa clientes desde BDP a Glory (lee BDP, modifica Glory)           |
| **"Sync BDP"**                                | Plano de Sala → barra de herramientas superior                  | Importa salones y mesas desde BDP al plano local                       |
| Columna "BDP" en tabla de ventas              | Lista de ventas (si integración activa)                         | Muestra estado de sync de cada venta (✅/❌/⏳) + filtro               |
| Botones pago/factura BDP                      | Fila de cada venta (si integración activa + venta sincronizada) | Registrar pago o facturar en BDP                                       |
| Badge por cliente "Código X" / "Sin vincular" | Columna BDP de tabla de clientes                                | Estado de vinculación BDP del cliente                                  |
| Botón "BDP" por cliente                       | Fila de cada cliente (si no sincronizado)                       | Vincular cliente individual con código BDP                             |
| Pestaña "BDP" en Configuración                | Configuración → pestaña BDP                                     | Configurar conexión, diagnóstico, catálogo, mapeos, menús, polling      |
| Panel "Seguridad, respaldos e historial BDP"  | Configuración → pestaña BDP → subsección                        | Snapshots, auditoría, modo de sync (solo lectura / escritura temporal) |
| Sección "Catálogo de artículos BDP"           | Configuración → pestaña BDP → sección visible                   | Tabla de mapeos con Precio y Stock + Sync catálogo/precios (237A-3+4) |
| Sección "Correspondencias Glory ↔ BDP"        | Configuración → pestaña BDP → sección visible                   | Mapeos tender, canales, artículo fallback, cliente default (237A-3)   |
| Sección "Actualización de estados"            | Configuración → pestaña BDP → sección visible                   | Toggle polling automático + intervalo (237A-3)                       |
| Sección "Modo de operaciones BDP"             | Configuración → pestaña BDP → sección visible                   | Info cards con modo actual + pointer a PanelBdpBackup (237A-3)       |
| **Sección "Funcionalidades BDP"**             | Configuración → pestaña BDP → sección visible                   | 6 toggles para activar/desactivar: auto-arming, pagos parciales, cancelar comandas, compras (lectura/borradores/conciliación). Con descripciones. (267A-4) |
| Card "Explorar menús, packs y fastfoods"      | Configuración → pestaña BDP → card expandible                   | Buscar y ver estructura de menús/packs/fastfoods por ID (237A-3)     |
| Badge "BDP: off/lectura/escritura"            | Barra superior (navbar)                                         | Indicador compacto del modo BDP actual (237A-3)                      |
| Botón 🔍 "Consultar estado BDP" por venta     | Fila de cada venta sincronizada                                 | Consulta estado individual de comanda en BDP (237A-3)                |

### Lo que el cliente NO ve (pero la guía describe)

| Funcionalidad comunicada                                         | Estado real en frontend                                                                                                                    | Gravedad               |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------- |
| Catálogo de artículos, precios, IVA, familias, códigos de barras | **Implementado** — sección "Catálogo de artículos BDP" visible directamente en pestaña BDP con tabla de mapeos + columnas Precio y Stock + botones Sync | ✅ Implementado (237A-3 + 237A-4) |
| Relación artículos Glory ↔ BDP                                   | **Implementado** — sección "Correspondencias Glory ↔ BDP" en pestaña BDP (fuera del colapsable)                                              | ✅ Implementado (237A-3) |
| Consulta de estado de comandas                                   | **Implementado** — botón 🔍 "Consultar estado BDP" por venta en `venta-row-actions.tsx`                                                      | ✅ Implementado (237A-3) |
| Menús, packs y modalidades de venta                              | **Implementado** — componente `bdp-menu-explorer.tsx` integrado en ConfigBdp como card expandible                                             | ✅ Implementado (237A-3) |
| Información de stock                                             | **Implementado** — columna Stock en tabla de mapeos artículos (viene de sync-catalog via `ExportArticles.CurrentStock`)                     | ✅ Implementado (237A-4) |
| **Pagos parciales**                                              | **Implementado** — bajo feature flag `ff_bdp_partial_payments`. Ledger local `bdp_pagos` con idempotency_key. UI en venta-row-actions.tsx.   | ✅ Implementado (267A-4) |
| **Compras (albaranes de proveedores)**                           | **Implementado** — Fases 1-3: lectura, borradores y conciliación. Bajo feature flags `ff_bdp_purchase_notes_*`.                             | ✅ Implementado (267A-4) |
| **Feature flags configurables desde UI**                         | **Implementado** — 6 toggles en Configuración → BDP → "Funcionalidades BDP" con descripciones. Sin redeploy.                               | ✅ Implementado (267A-4) |

---

## 2. Mapa detallado: dónde está cada funcionalidad BDP en el frontend

### 2.1 Configuración y conexión BDP

**Ubicación:** `Configuración` → pestaña `BDP`

**Componente:** `frontend/src/componentes/ConfigBdp.tsx`

| Sección                              | Qué muestra                                                                                                                                        | Visible para el cliente                                                                                |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Conexión BDP                         | URL, login, password, código integrador, terminal POS, empleado, perfil artículos                                                                  | ✅ Sí — campos editables                                                                               |
| Integración BDP activa               | Switch on/off con texto explicativo                                                                                                                | ✅ Sí — interruptor claro                                                                              |
| Dirección de sync (3 cajas)          | "BDP → Glory", "Glory → BDP", "Dos vías automáticas" con descripciones                                                                             | ✅ Sí — informativo                                                                                    |
| Configuración técnica (solo soporte) | **Colapsable** — oculto por defecto. Contiene: formas de pago, canales, artículo fallback, cliente por defecto, polling, exigir cliente confirmado | ⚠️ Oculto — requiere clic en "Configuración técnica (solo soporte)"                                    |
| Diagnóstico BDP                      | Botón "Probar conexión" + resultado Health/Login/Version                                                                                           | ✅ Sí                                                                                                  |
| Validar con simulador local          | Botón solo habilitado si URL es localhost                                                                                                          | ⚠️ Bloqueado en producción                                                                             |
| Tabla de mapeo de artículos          | CRUD de artículos Glory→BDP + columnas Precio y Stock + botón "Sync catálogo" + botón "Sync precios"                                               | ✅ Visible (237A-3 + 237A-4) — sección "Catálogo de artículos BDP" en pestaña BDP, fuera del colapsable |

**✅ CORREGIDO (237A-3):** La tabla de mapeo de artículos ahora es una sección visible directamente en la pestaña BDP, sin necesidad de expandir el colapsable "Configuración técnica". Ya no está a 3 niveles de profundidad.

---

### 2.2 Clientes

**Ubicación:** Pestaña `Clientes`

**Componente:** `frontend/src/componentes/ListaClientes.tsx`

| Elemento                    | Ubicación exacta                          | Comportamiento                                                             |
| --------------------------- | ----------------------------------------- | -------------------------------------------------------------------------- |
| Botón **"Importar BDP"**    | Barra superior, junto a "+ Nuevo Cliente" | Abre diálogo con preview + confirmación textual "IMPORTAR CLIENTES BDP"    |
| Columna **"BDP"** en tabla  | Última columna antes de acciones          | Badge: "Código X" (vinculado), "Error" (fallo), "Sin vincular" (pendiente) |
| Botón **"BDP"** por cliente | En cada fila, si no sincronizado          | Abre diálogo para vincular con código BDP explícito                        |

**Lo que funciona:**

- ✅ Importar clientes desde BDP (lee BDP, crea/vincula en Glory)
- ✅ Vincular cliente individual con código BDP explícito
- ✅ Preview antes de aplicar importación
- ✅ Confirmación textual obligatoria

**Lo que falta según la guía:**

- La guía dice "clientes, con una revisión previa antes de copiarlos o vincularlos" → ✅ Esto SÍ existe y funciona
- No hay indicación de cuántos clientes hay en BDP vs Glory
- No hay forma de buscar un cliente específico en BDP antes de importar

---

### 2.3 Plano de Sala (mesas)

**Ubicación:** Pestaña `Plano de Sala`

**Componente:** `frontend/src/componentes/PlanoSala.tsx`

| Elemento             | Ubicación exacta                                          | Comportamiento                                               |
| -------------------- | --------------------------------------------------------- | ------------------------------------------------------------ |
| Botón **"Sync BDP"** | Barra de herramientas superior, junto a Exportar/Importar | Abre diálogo con preview + confirmación "IMPORTAR MESAS BDP" |

**Lo que funciona:**

- ✅ Importar salones y mesas desde BDP
- ✅ Preview: muestra cuántas zonas y mesas se crearían
- ✅ Solo crea lo que falta, no modifica ni elimina existente
- ✅ Confirmación textual obligatoria

**Lo que falta:**

- No hay forma de ver qué mesas tienen en BDP antes de importar
- No hay sincronización bidireccional (mesas creadas en Glory no van a BDP)
- La importación es solo aditiva — si se renombra una mesa en BDP, Glory no se entera

---

### 2.4 Ventas y comandas

**Ubicación:** Pestaña `Ventas`

**Componentes:** `ListaVentas.tsx`, `venta-table-body.tsx`, `venta-row-actions.tsx`, `bdp-sync-badge.tsx`

| Elemento                                                     | Ubicación exacta     | Condición de visibilidad                                                                 |
| ------------------------------------------------------------ | -------------------- | ---------------------------------------------------------------------------------------- |
| Columna **"BDP"** con badge                                  | Tabla de ventas      | Solo si `bdp_sync_enabled = true` en configuración                                       |
| **Badge estados:** ✅ Sincronizada / ⏳ Pendiente / ❌ Error | Cada fila            | Siempre visible cuando integración activa                                                |
| **Filtro BDP** (synced/error/pending)                        | Cabeceras de columna | Siempre visible cuando integración activa                                                |
| Botón **"Reintentar sync"**                                  | Acciones por fila    | Solo si `bdp_synced = false` Y `bdp_sync_error` tiene valor                              |
| Botón **"Pago BDP"** (💰)                                    | Acciones por fila    | Solo si: sync activo + venta sincronizada + tiene order_id + no facturada + no cancelada |
| Botón **"Factura BDP"** (📄)                                 | Acciones por fila    | Misma condición que pago                                                                 |
| Diálogo pago BDP                                             | Al pulsar 💰         | Pide: tender ID + importe + confirmación textual "PAGAR {id} {importe}"                  |
| Diálogo factura BDP                                          | Al pulsar 📄         | Pide: confirmación textual "FACTURAR {id}"                                               |

**Lo que funciona:**

- ✅ Crear comanda en BDP (automático al crear venta si sync activo)
- ✅ Ver estado de sync por venta
- ✅ Reintentar sync fallido
- ✅ Registrar pago completo en BDP
- ✅ Facturar comanda pagada en BDP
- ✅ Protección contra duplicados y ediciones

**Lo que falta según la guía:**

- ~~"estado de comandas ya enviadas, con consultas manuales o automáticas opcionales"~~ → ✅ **CORREGIDO (237A-3):** Botón 🔍 "Consultar estado BDP" añadido en `venta-row-actions.tsx` para cada venta sincronizada. El polling automático existe y se activa desde la sección "Actualización de estados" en pestaña BDP.
- No hay forma de ver el detalle de una comanda BDP (artículos, importes, etc.) — **pendiente de decisión**
- Los botones de pago y factura están condicionados de forma estricta — si la venta no se sincronizó correctamente, no aparecen

---

### 2.5 Mapeo de artículos (catálogo)

**Ubicación (después de 237A-3):** `Configuración` → pestaña `BDP` → sección "Catálogo de artículos BDP" (visible directamente, sin colapsable)

**Componentes:** `config-bdp-mapeos.tsx` → `bdp-article-map-table.tsx`

| Elemento             | Qué hace                                                                          |
| -------------------- | --------------------------------------------------------------------------------- |
| Tabla de mapeos      | Lista artículos Glory con su código BDP, nombre, **precio** y **stock**           |
| Botón crear mapeo    | Vincula manualmente un artículo Glory con un código BDP                           |
| Botón eliminar mapeo | Elimina la vinculación                                                            |
| **"Sync catálogo"**  | Importa/actualiza artículos desde BDP. Crea mapeos automáticos. Actualiza stock.  |
| **"Sync precios"**   | Actualiza precios de artículos mapeados desde BDP (stock solo se actualiza con Sync catálogo) |

**✅ CORREGIDO (237A-3 + 237A-4):** La tabla de mapeos ya NO está oculta. Es una sección visible directamente en la pestaña BDP. Incluye columnas de Precio y Stock (implementado en 237A-4).

---

### 2.6 Respaldos, auditoría y modo de sync

**Ubicación:** `Configuración` → pestaña `BDP` → sección "Seguridad, respaldos e historial BDP"

**Componente:** `frontend/src/componentes/PanelBdpBackup.tsx`

| Función                | Qué hace                                                                   |
| ---------------------- | -------------------------------------------------------------------------- |
| Selector de modo sync  | Cambia entre "Solo lectura" y "Escritura temporal"                         |
| Crear snapshot         | Guarda estado actual de Glory (clientes, mapeos, ventas)                   |
| Restaurar snapshot     | Restaura datos locales de Glory                                            |
| Historial de auditoría | Lista de operaciones de escritura realizadas (fecha, operación, resultado) |
| Lista de snapshots     | Snapshots existentes con opción de eliminar                                |


---

### 2.7 Menús, packs y modalidades de venta

**Estado:** ✅ **Implementado (237A-3)**

Los endpoints backend existen:
- `GET /api/bdp/menus/:id`
- `GET /api/bdp/fastfoods/:id`
- `GET /api/bdp/packs/:id`

**✅ CORREGIDO (237A-3):** Componente `bdp-menu-explorer.tsx` creado e integrado en ConfigBdp como card expandible. Permite buscar menús, packs y fastfoods por ID y muestra su estructura (grupos + items) en formato de solo lectura.

### 2.8 Consulta de estado individual de comanda

**Estado:** ✅ **Implementado (237A-3)**

- Endpoint backend: `GET /api/ventas/:id/bdp-status` (implementado en `handlers/ventas.rs`)
- **✅ CORREGIDO (237A-3):** Botón 🔍 "Consultar estado BDP" añadido en `venta-row-actions.tsx` para cada venta sincronizada. Usa `fetchBdpStatus` directamente y muestra el resultado en un toast.
- El polling automático (`BdpOrderPollerService`) actualiza estados y se activa desde la sección "Actualización de estados" en pestaña BDP.

---

## 3. Compras: estado actual

### Estado: ✅ Implementado (Fases 1-3)

Las compras (albaranes de proveedores BDP) están implementadas en 3 fases, todas protegidas por feature flags:

| Fase | Qué hace | Feature flag | Endpoint |
|------|----------|-------------|----------|
| **Fase 1 — Lectura** | Sincroniza albaranes de compra desde BDP (`ExportPurchaseNotes`) | `ff_bdp_purchase_notes_read` | `GET /api/bdp/purchase-notes`, `POST /api/bdp/purchase-notes/sync` |
| **Fase 2 — Borradores** | Marca albaranes como borradores locales (sin escribir en BDP) | `ff_bdp_purchase_notes_draft` | `POST /api/bdp/purchase-notes/:id/draft` |
| **Fase 3 — Conciliación** | Vincula albaranes con gastos existentes o crea gastos nuevos | `ff_bdp_purchase_notes_receive` | `POST /api/bdp/purchase-notes/:id/reconcile` |

**Todos están desactivados por defecto.** Se activan desde Configuración → BDP → "Funcionalidades BDP".

### ¿Se puede activar?

Sí, desde la propia web. Los 3 feature flags de compras están disponibles como toggles en la sección "Funcionalidades BDP" de Configuración. No requiere cambios de código ni redeploy.

---

## 4. Stock: ¿se puede consultar desde Glory?

### Estado actual

**✅ Implementado (237A-4).** La columna Stock se muestra en la tabla de mapeos de artículos tras ejecutar "Sync catálogo". El stock viene del campo `CurrentStock` que BDP devuelve en la respuesta de `ExportArticles`.

### ¿Es técnicamente posible?

**Sí, los endpoints existen y están documentados.** BDP WebLink REST API documenta los siguientes endpoints de stock en `# WEBLINK RESTAPI.md` (líneas 9830-9991, "Categoría Stock"):

| Endpoint BDP                 | Capacidad                                                    | Documentado en WebLink | Implementado en Glory |
| ---------------------------- | ------------------------------------------------------------ | ---------------------- | --------------------- |
| `ExportArticles`             | Ya lo usamos para catálogo. Incluye campo `CurrentStock`     | ✅ Sí (línea 3685)     | ✅ Sí (sync-catalog)  |
| `/API/Warehouse/GetStock`    | Stock de un artículo en un almacén específico                | ✅ Sí (línea 9830)     | ❌ No                 |
| `/API/Warehouse/GetListStock`| Stock de múltiples artículos de una vez                      | ✅ Sí (línea 9871)     | ❌ No                 |
| `GetItemCostPrices`          | Precios de coste (UPC/PMC) de un artículo                    | ✅ Sí (línea 9553)     | ❌ No                 |

**⚠️ Importante:** Al igual que `CancelOrder`, estos endpoints podrían devolver "Subscripción no activada" si el módulo de almacén no está contratado en la instalación del restaurante. Esto debe verificarse antes de prometer la funcionalidad al cliente.

**Nota adicional:** El endpoint `ExportArticles` que ya usamos para sync de catálogo devuelve `CurrentStock` (decimal) como parte de `PricesTableDataType`. Esto significa que **podríamos obtener stock sin implementar endpoints nuevos** — solo habría que mapear el campo `CurrentStock` del response existente.

### Propuesta para consultar stock

Si el cliente quiere **consultar** stock (no administrarlo):

1. **Opción A — Columna en tabla de mapeos:** Añadir una columna "Stock" en `bdp-article-map-table.tsx` que muestre el stock actual de cada artículo mapeado, consultado al hacer "Sync catálogo".

2. **Opción B — Pantalla dedicada de stock:** Crear una nueva sección/pestaña que liste artículos con su stock actual, filtros por familia/departamento, y alertas de stock bajo.

3. **Opción C — Badge en formulario de venta:** Al seleccionar un artículo en `LineasVentaEditor`, mostrar el stock disponible junto al nombre.

**Esfuerzo estimado (si los endpoints existen):**
- Opción A: ~4h (backend: 2h + frontend: 2h)
- Opción B: ~12h (backend: 4h + frontend: 8h)
- Opción C: ~6h (backend: 2h + frontend: 4h)

**Limitación:** Solo lectura. Para modificar stock, el cliente seguiría necesitando BDP.

**Pre-requisito:** Verificar que el módulo de stock de WebLink está activo en la instalación del restaurante.

---

## 5. Importación de catálogo: ¿qué es exactamente?

### Lo que el cliente pregunta

> "Mencionas que se puede hacer importaciones de catálogo desde BDP a nuestra web. No sé si con eso te refieres al stock o a otra cosa."

### Respuesta

**La importación de catálogo NO es stock.** Son cosas diferentes:

| Concepto                         | Qué incluye                                                                                            | Endpoint BDP                           |
| -------------------------------- | ------------------------------------------------------------------------------------------------------ | -------------------------------------- |
| **Catálogo (lo que sí hacemos)** | Nombres de artículos, precios, impuestos (IVA), familias, subfamilias, códigos de barras, departamento | `ExportArticles` + `GetPricesArticles` |
| **Stock (implementado 237A-4)**   | Cantidad disponible de cada artículo en almacén                                                        | ✅ `ExportArticles` devuelve `CurrentStock` → mapeado a `stock_actual` en `bdp_article_map` → columna en tabla de mapeos. Si módulo almacén no activo, muestra "—". Endpoints dedicados `GetStock`/`GetListStock` siguen disponibles para pantalla completa futura. |

La importación de catálogo permite:

1. **Sincronizar artículos desde BDP a Glory** — crea automáticamente los mapeos Glory↔BDP
2. **Actualizar precios** — sincroniza precios de artículos ya mapeados
3. **Relacionar artículos** — cada artículo Glory queda vinculado a su código BDP

### Dónde está hoy (después de 237A-3)

**✅ CORREGIDO** — El catálogo de artículos ahora es una sección visible directamente en la pestaña BDP, sin necesidad de expandir el colapsable "Configuración técnica". La sección "Catálogo de artículos BDP" muestra:
- Tabla de mapeos Glory ↔ BDP con columnas: Código Glory, Código BDP, Nombre, **Precio**, **Stock**
- Botones "Sync catálogo" y "Sync precios" directamente accesibles
- Stock viene de `ExportArticles.CurrentStock` (mapeado en 237A-4)

La sección de mapeos técnicos (tender, canales, etc.) también se subió a nivel principal como "Correspondencias Glory ↔ BDP".

---

## 6. Autorización temporal: ¿cómo funciona y cómo mejorarla?

### Lo que el cliente pregunta

> "Mencionas que cada escritura requiere una autorización temporal porque la web está en modo lectura. Esto significa que cada vez que la persona que consulte la página y tenga que hacer una operación tendrá que cambiar de modo lectura a escritura de forma manual?"

### Cómo funciona hoy

El flujo actual es:

```
1. Estado normal: Solo lectura (BDP → Glory)
   └─ El cliente puede hacer todo lo de lectura sin autorización

2. Para escribir (crear cliente, enviar comanda, pagar, facturar):
   a. Ir a Configuración → BDP → "Seguridad, respaldos e historial BDP"
   b. Cambiar modo a "Escritura temporal"
   c. Esto crea un "arming" (permiso temporal) para UNA operación
   d. Ejecutar la operación (ej: crear comanda)
   e. El sistema vuelve AUTOMÁTICAMENTE a "Solo lectura"
```

### ¿Es manual?

**Parcialmente.** Lo que el cliente describe es correcto:

- **Sí**, alguien tiene que cambiar el modo manualmente la primera vez
- **No**, no hay que hacerlo "cada vez" — una vez activado, el arming permite **una operación** y luego se auto-desactiva
- **Pero**, para la siguiente operación hay que volver a activar manualmente

### El problema real

Para operaciones normales del restaurante (crear comandas), este flujo es **inviable en producción**. Un camarero no puede ir a Configuración cada vez que quiere enviar una comanda a BDP.

### Activación automática por operación

**✅ Implementado (247A-1) + Configurable desde UI (267A-4)**

```
Flujo anterior (problemático):
  Crear venta → "Error: BDP en modo lectura" → Ir a Config → Activar escritura → Volver → Reintentar

Flujo actual (transparente):
  Crear venta / Pagar / Facturar → [El sistema auto-arma para esta operación] → Comanda/pago/factura creada → [Auto-vuelve a lectura]
```

**Estado actual:**

1. **Auto-arming** activable desde Configuración → BDP → "Funcionalidades BDP" → toggle "Auto-arming" (`ff_bdp_auto_arm`). Sin redeploy.
2. **Pagos parciales** activable desde el mismo panel → toggle "Pagos parciales" (`ff_bdp_partial_payments`).
3. **Cancelar comandas** — toggle existe pero BDP bloquea el endpoint ("Subscripción no activada").
4. **Compras** — 3 toggles: lectura, borradores, conciliación.

**Seguridad mantenida:** El modelo sigue siendo fail-closed:
- `BdpWriteGuard::try_auto_arm()` valida server-side el destino BDP y crea un arming efímero dentro de una transacción protegida por advisory lock.
- `BdpWriteGuard::authorize()` mantiene advisory lock, fingerprint y `ensure_no_unresolved()`.
- La auditoría registra el intento con `idempotency_key`, operación y resultado.
- `BdpThrottleManager` limita la concurrencia de llamadas a BDP por destino.
- Los feature flags permiten activar/desactivar por restaurante sin redeploy.
- **219 tests** verifican el comportamiento: 73 unitarios Rust + 24 simulador + 19 artículo map + 11 servicios negocio + 92 Python.

### Alternativa: modo "operaciones activas" / toggle rápido en navbar

**✅ Implementado (247A-1)** — `BdpStatusIndicator` en `site-header.tsx` muestra "BDP: off/lectura/escritura" y permite cambiar el modo de sync mediante un dropdown que navega a Configuración → BDP.

---

## 7. Resumen de gaps: comunicado vs. realidad

### Lo que la guía dice correctamente y SÍ es visible

| Afirmación de la guía                                      | ¿Visible?                 | Dónde                                                       |
| ---------------------------------------------------------- | ------------------------- | ----------------------------------------------------------- |
| "Clientes, con revisión previa antes de copiar/vincular"   | ✅ Sí                     | Pestaña Clientes → "Importar BDP" + botón "BDP" por cliente |
| "Salones y mesas, con vista previa antes de agregar"       | ✅ Sí                     | Plano de Sala → "Sync BDP"                                  |
| "Comandas con varios artículos, cantidades, descuentos..." | ✅ Sí                     | Al crear venta con líneas                                   |
| "El estado normal es Solo lectura"                         | ✅ Sí                     | Configuración → BDP → interruptor                           |
| "Cada escritura requiere autorización temporal"            | ✅ Sí (pero problemático) | Configuración → BDP → Seguridad                             |
| "Interruptor general"                                      | ✅ Sí                     | Configuración → BDP → "Integración BDP activa"              |

### Lo que la guía dice pero NO es fácilmente visible

| Afirmación de la guía                                                     | ¿Visible?       | Dónde está realmente                                                      |
| ------------------------------------------------------------------------- | --------------- | ------------------------------------------------------------------------- |
| "Catálogo de artículos, precios, impuestos, familias y códigos de barras" | ✅ Visible      | Configuración → BDP → sección "Catálogo de artículos BDP" (fuera del colapsable, 237A-3) |
| "Relación entre artículos Glory y artículos BDP"                          | ✅ Visible      | Configuración → BDP → sección "Correspondencias Glory ↔ BDP" (fuera del colapsable, 237A-3) |
| "Estado de comandas, con consultas manuales o automáticas"                | ✅ Implementado | Botón 🔍 por venta (237A-3) + badge automático + polling configurable |
| "Formas de pago: indica qué código BDP corresponde..."                    | ✅ Visible      | Configuración → BDP → sección "Correspondencias Glory ↔ BDP" (237A-3) |
| "Canales: relaciona comedor, barra..."                                    | ✅ Visible      | Mismo lugar que formas de pago (237A-3) |
| "Artículo sin equivalencia"                                               | ✅ Visible      | Mismo lugar (237A-3) |
| "Cliente por defecto"                                                     | ✅ Visible      | Mismo lugar (237A-3) |
| "Actualización de estados"                                                | ✅ Visible      | Configuración → BDP → sección "Actualización de estados" (237A-3) |
| "Exigir cliente confirmado"                                               | ✅ Visible      | Configuración → BDP → sección "Correspondencias Glory ↔ BDP" (237A-3) |
| "Información consultiva de menús, packs y modalidades"                    | ✅ Implementado | `bdp-menu-explorer.tsx` en ConfigBdp (237A-3) |

### Lo que la guía excluye correctamente

| Exclusión                               | ¿Correcto? | Nota                                  |
| --------------------------------------- | ---------- | ------------------------------------- |
| ~~Stock~~                               | ~~✅~~     | ✅ **IMPLEMENTADO (237A-4)** — columna en tabla de mapeos |
| ~~Compras~~                             | ~~✅~~     | ✅ **IMPLEMENTADO (267A-4)** — Fases 1-3 bajo feature flags |
| ~~Pagos parciales~~                     | ~~✅~~     | ✅ **IMPLEMENTADO (267A-4)** — bajo feature flag `ff_bdp_partial_payments` |
| Transferencias                          | ✅         | No implementado                       |
| Tallas, colores                         | ✅         | No implementado                       |
| Fidelización                            | ✅         | No implementado                       |
| Sincronización bidireccional automática | ✅         | Bloqueado explícitamente              |

---

## 8. Plan de acción recomendado

### Prioridad A — Visibilidad (para que el cliente vea lo que ya existe)

1. ~~**Subir mapeos de artículos/catálogo** fuera del colapsable~~ ✅ **COMPLETADO** (237A-3) — sección "Catálogo de artículos BDP" visible directamente en pestaña BDP
2. ~~**Añadir botón "Consultar estado"** por comanda~~ ✅ **COMPLETADO** (237A-3) — botón 🔍 en `venta-row-actions.tsx`
3. ~~**Mostrar mapeos (tender, canales, etc.)** como sección visible~~ ✅ **COMPLETADO** (237A-3) — sección "Correspondencias Glory ↔ BDP" fuera del colapsable

### Prioridad B — Flujo de autorización (para que sea usable en producción)

4. ~~**Implementar auto-arming transparente**~~ ✅ **COMPLETADO** (247A-1) — `BdpWriteGuard::try_auto_arm()` + handlers con `auto_arm`/`confirmation_destino`/`idempotency_key`
5. ~~**Añadir indicador rápido de estado BDP** en la barra de navegación~~ ✅ **COMPLETADO** (237A-3 / 247A-1) — `BdpStatusIndicator` en `site-header.tsx` muestra "BDP: off/lectura/escritura" y permite cambiar modo

### Prioridad C — Funcionalidad nueva (si el cliente lo solicita)

6. ~~**Consultar stock** desde BDP~~ ✅ **COMPLETADO** (237A-4) — columna Stock en tabla de mapeos, viene de `ExportArticles.CurrentStock`
7. ~~**Pantalla de menús/packs**~~ ✅ **COMPLETADO** (237A-3) — `bdp-menu-explorer.tsx` integrado en ConfigBdp
8. **Importación de compras** si se aprueba como fase adicional

---

## 9. Apéndice: borrador de respuesta al cliente

> **Nota:** Este apéndice es un borrador interno para preparar la comunicación con el cliente.

### Sobre "no veo ninguno de esos puntos"

> Tiene razón. La guía describe capacidades del backend que no todas tienen una interfaz accesible. Las funcionalidades de catálogo, mapeos y configuración técnica están disponibles pero requieren navegar a Configuración → BDP → Configuración técnica. Vamos a reorganizar la interfaz para que todo sea más accesible.

### Sobre "importar BDP en Clientes me aparece [captura]"

> Si aparece vacío o con error, probablemente es porque:
> 1. La integración BDP no está activada (switch "Integración BDP activa" en Configuración)
> 2. La conexión BDP no está configurada correctamente (URL, login, password)
> 3. El PC del restaurante con BDP no está accesible desde el servidor
>
> La importación de clientes requiere: (a) integración activa, (b) conexión válida al BDP, (c) el PC del restaurante encendido y accesible.

### Sobre compras

> Las compras están implementadas en 3 fases: lectura de albaranes desde BDP, creación de borradores locales, y conciliación con gastos. Todo está desactivado por defecto y se activa desde Configuración → BDP → "Funcionalidades BDP". Los 3 feature flags (`ff_bdp_purchase_notes_read`, `ff_bdp_purchase_notes_draft`, `ff_bdp_purchase_notes_receive`) se pueden activar y desactivar sin necesidad de redeploy.

### Sobre "¿se puede ver stock?"

> Sí, ya está disponible. Tras ejecutar "Sync catálogo" en la sección "Catálogo de artículos BDP" de Configuración, cada artículo muestra su stock actual en una columna dedicada. El stock viene directamente de BDP. Si el módulo de almacén de BDP no está activo en la instalación del restaurante, la columna mostrará "—". Para modificar stock, hay que usar BDP directamente — en Glory es solo consulta.

### Sobre "importaciones de catálogo"

> La importación de catálogo NO es stock. Es la sincronización de nombres de artículos, precios, impuestos, familias y códigos de barras desde BDP a Glory. Permite que cada artículo Glory quede vinculado a su equivalente en BDP para que las comandas se envíen correctamente. Hoy esta función existe pero está oculta en Configuración → BDP → Configuración técnica. Vamos a hacerla más accesible.

### Sobre autorización temporal

> Ya no es necesario ir a Configuración para activar escritura temporal. El sistema tiene un modo "auto-arming" que activa automáticamente el permiso puntual cuando el usuario confirma la acción ("PAGAR {id} {importe}", "FACTURAR {id}"). Esto se activa desde Configuración → BDP → "Funcionalidades BDP" → toggle "Auto-arming". Con esto activado, el flujo es: crear venta → el sistema auto-arma → ejecuta la operación → vuelve automáticamente a solo lectura. La seguridad se mantiene: cada operación requiere confirmación textual, se registra en auditoría, y el sistema vuelve a solo lectura tras cada escritura.

---

## 10. Preguntas que el cliente podría hacer (anticipación)

> **Nota:** Esta sección anticipa preguntas probables del cliente más allá de las ya planteadas, con respuestas preparadas.

### P1: "¿Puedo cancelar una comanda desde Glory?"

> No, técnicamente no es posible. El endpoint `CancelOrder` de BDP devuelve "Subscripción no activada" — el módulo de cancelación no está contratado en la instalación del restaurante. Para cancelar comandas hay que hacerlo directamente en el TPV de BDP. Si el restaurante contrata el módulo de cancelación, podríamos integrarlo.

### P2: "¿Puedo hacer pagos parciales?"

> Sí, está implementado. Permite pagar una comanda BDP en varios pagos parciales en vez de un único pago completo. Cada pago se registra en un ledger local con clave de idempotencia para evitar duplicados. Está desactivado por defecto y se activa desde Configuración → BDP → "Funcionalidades BDP" → toggle "Pagos parciales".

### P3: "¿Qué pasa si BDP se cae o el PC del restaurante se apaga?"

> Glory sigue funcionando con normalidad — las ventas se registran localmente. Si la integración está activa, Glory intentará sincronizar cada venta con BDP automáticamente. Si BDP no está disponible, la venta queda marcada como "pendiente" con el error correspondiente. Cuando BDP vuelva, se puede reintentar la sincronización manualmente con el botón de retry. No se pierden datos.

### P4: "¿Los cambios que haga en BDP se ven reflejados en Glory automáticamente?"

> Depende del tipo de cambio:
> - **Artículos nuevos en BDP:** Se ven en Glory tras hacer "Sync catálogo" (manual)
> - **Precios cambiados en BDP:** Se ven en Glory tras hacer "Sync precios" (manual)
> - **Stock cambiado en BDP:** Se ve en Glory tras hacer "Sync catálogo" (manual)
> - **Clientes nuevos en BDP:** Se ven en Glory tras hacer "Importar BDP" (manual)
> - **Mesas nuevas en BDP:** Se ven en Glory tras hacer "Sync BDP" en Plano de Sala (manual)
>
> Las sincronizaciones de lectura (BDP → Glory) son manuales. Las de escritura (Glory → BDP) son automáticas al crear ventas, pagos y facturas.

### P5: "¿Puedo tener dos terminales BDP conectados a Glory?"

> No actualmente. La integración está diseñada para un único terminal POS por restaurante. El campo "Terminal POS" en Configuración acepta un solo ID. Si el restaurante tiene varios terminales, habría que extender la integración — es técnicamente posible pero requiere desarrollo adicional.

### P6: "¿Puedo ver el historial de comandas de BDP en Glory?"

> Parcialmente. Glory muestra el estado de cada venta sincronizada (éxito/error/pendiente) y el historial de auditoría de todas las operaciones de escritura. Sin embargo, no se puede ver el detalle completo de comandas históricas de BDP (artículos, cantidades, etc.) — eso requiere consultar BDP directamente. El polling automático actualiza los estados de comandas ya sincronizadas.

### P7: "¿Los clientes que creo en Glory se sincronizan con BDP?"

> Sí, pero de forma limitada. Si un cliente ya tiene un código BDP asignado (por importación previa), Glory lo envía a BDP al crear la comanda. Sin embargo, Glory NO crea clientes nuevos en BDP automáticamente — esto se deshabilitó por seguridad (los códigos automáticos pueden colisionar con otros del TPV). Para vincular un cliente, hay que usar "Importar BDP" o el botón "BDP" por cliente.

### P8: "¿Puedo administrar el stock desde Glory?"

> No. La integración es de solo lectura para stock. Se puede consultar el stock disponible de cada artículo tras hacer "Sync catálogo", pero para modificar cantidades (recepciones, ajustes, regularizaciones) hay que usar BDP directamente. Esto es intencional — la administración de stock requiere los módulos completos de almacén de BDP.

### P9: "¿Cuánto tiempo tarda la sincronización?"

> - **Crear comanda en BDP:** ~2-5 segundos (depende de la conexión al PC del restaurante)
> - **Sync catálogo:** ~10-30 segundos (depende del número de artículos en BDP)
> - **Sync precios:** ~5-15 segundos (artículos ya mapeados)
> - **Importar clientes:** ~5-10 segundos
> - **Sync mesas:** ~3-5 segundos
>
> Todas las operaciones muestran indicadores de carga y feedback de resultado.

### P10: "¿Qué pasa si hago una venta en Glory y otra en el TPV al mismo tiempo?"

> No hay conflicto. Cada venta en Glory genera una comanda independiente en BDP con un identificador único (`MarketplaceOrderId`). BDP trata cada comanda como una orden separada. El advisory lock de PostgreSQL previene que la misma venta de Glory se envíe dos veces a BDP simultáneamente.
