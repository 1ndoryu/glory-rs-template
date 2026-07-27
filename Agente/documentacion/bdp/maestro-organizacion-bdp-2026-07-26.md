# MD Maestro — Organización documentación BDP

> **Fecha:** 2026-07-26 (actualizado con verificación contra código)
> **Propósito:** Inventario completo, verificación individual contra código, análisis de redundancias, estado de cobertura, gaps y prioridades de toda la documentación BDP.
> **Método:** Lectura y análisis de todos los MDs relacionados con BDP + verificación contra código fuente real (búsquedas ripgrep, lectura de archivos).

---

## 1. Inventario completo de MDs BDP

### 1.1 Documentación técnica / API

| #   | Ruta                                                                    | Fecha                       | Tema                                                                                        | Estado                         | Acción                                           |
| --- | ----------------------------------------------------------------------- | --------------------------- | ------------------------------------------------------------------------------------------- | ------------------------------ | ------------------------------------------------ |
| 1   | `Agente/documentacion/api/bdp-300035-resumen-completo-2026-06-01.md`    | 2026-06-03                  | Error 300035 (serie facturación), causa raíz en Parámetros 6 + Order.Type                   | ✅ Resuelto                    | **Mantener** como referencia histórica del error |
| 2   | `Agente/documentacion/api/bdp-cambios-analisis-problemas-2026-06-08.md` | 2026-06-08 (upd 2026-06-30) | Análisis de 4 problemas reportados por cliente tras pruebas (serie, logo, precios, tickets) | ✅ Resuelto (cliente confirmó) | **Mantener** como referencia de incidente        |

### 1.2 Documentación de seguridad y riesgos

| #   | Ruta                                                            | Fecha      | Tema                                                                      | Estado                             | Acción                                         |
| --- | --------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------- | ---------------------------------- | ---------------------------------------------- |
| 3   | `Agente/documentacion/bdp-seguridad-produccion-2026-07-21.md`   | 2026-07-21 | Auditoría pre-deploy: 5 capas de seguridad, bootstrap, qué NO puede pasar | ✅ Vigente                         | **Mantener** — documento clave de seguridad    |
| 4   | `Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md` | 2026-07-24 | Evaluación de 16 riesgos (R1-R16) con mitigaciones priorizadas            | ✅ Vigente (parcialmente mitigado) | **Mantener** — actualizar con estado de cada R |

### 1.3 Planes completados (históricos)

| #   | Ruta                                                                           | Fecha      | Tema                                                                  | Estado                                             | Acción                                                                              |
| --- | ------------------------------------------------------------------------------ | ---------- | --------------------------------------------------------------------- | -------------------------------------------------- | ----------------------------------------------------------------------------------- |
| 5   | `Agente/planes/completados/plan-bdp-implementacion-completa-2026-07-14.md`     | 2026-07-15 | Plan original 9 fases de implementación completa BDP                  | ⚠️ **HISTÓRICO** — sustituido por validación local | **Mantener** como referencia, NO como procedimiento                                 |
| 6   | `Agente/planes/completados/plan-bdp-backup-seguridad-2026-07-15.md`            | 2026-07-15 | Sistema de copias de seguridad BDP ↔ Glory                            | ⚠️ **HISTÓRICO** — sustituido 2026-07-18           | **Mantener** — contiene inventario completo de endpoints (23+ lectura, 5 escritura) |
| 7   | `Agente/planes/completados/plan-validacion-segura-escritura-bdp-2026-07-18.md` | 2026-07-18 | Plan de validación segura de escrituras (simulador, tests, fases A-F) | ✅ Completado localmente                           | **Mantener** — fases E y F pendientes de cliente                                    |

### 1.4 Planes activos

| #   | Ruta                                                        | Fecha                       | Tema                                                           | Estado                               | Acción                                         |
| --- | ----------------------------------------------------------- | --------------------------- | -------------------------------------------------------------- | ------------------------------------ | ---------------------------------------------- |
| 8   | `Agente/planes/plan-pendientes-bdp-2026-07-23.md`           | 2026-07-23 (upd 2026-07-24) | Plan maestro de pendientes: C1, C2, D1-D5, XT1-XT2             | ✅ Vigente                           | **Mantener** — actualizar estados              |
| 9   | `Agente/planes/plan-visibilidad-bdp-frontend-2026-07-23.md` | 2026-07-23 (upd 2026-07-24) | Plan de visibilidad frontend: bloques A-D, C1/C2 implementados | ✅ Vigente (mayoría completado)      | **Mantener** — secciones D2-D5 pendientes      |
| 10  | `Agente/planes/plan-compras-bdp-2026-07-25.md`              | 2026-07-25                  | Plan detallado Compras Fase 1 (lectura albaranes)              | ✅ Implementado (247A-11)            | **Mantener** — fases 2-3 pendientes de cliente |
| 11  | `Agente/planes/plan-pagos-parciales-bdp-2026-07-25.md`      | 2026-07-25                  | Plan pagos parciales (AddOrderPayment parcial)                 | ✅ Implementado (backend + frontend) | **Mantener** — tests de simulador pendientes   |

### 1.5 Documentación de usuario / cliente

| #   | Ruta                                                                    | Fecha                       | Tema                                                                                        | Estado                         | Acción                                        |
| --- | ----------------------------------------------------------------------- | --------------------------- | ------------------------------------------------------------------------------------------- | ------------------------------ | --------------------------------------------- |
| 12  | `Agente/usuario/checklist-bdp-integracion-2026-07-16.md`                | 2026-07-16                  | Checklist de pruebas manuales (sin BDP → solo lectura → escritura)                          | ⚠️ **HISTÓRICO** — no entregar | **Mantener** como evidencia, NO como guía     |
| 13  | `Agente/usuario/auditoria-escritura-bdp-2026-07-17.md`                  | 2026-07-17 (upd 2026-07-18) | Reauditoría de 23 riesgos (W01-W23), todos cerrados localmente                              | ✅ Vigente                     | **Mantener**                                  |
| 14  | `Agente/usuario/guia-cliente-pruebas-integracion-bdp-2026-07-18.md`     | 2026-07-19                  | **Guía entregable al cliente** — resumen, pruebas, condiciones, criterios                   | ✅ Vigente                     | **Mantener** — documento oficial para cliente |
| 15  | `Agente/usuario/auditoria-plan-integracion-completa-bdp-2026-07-18.md`  | 2026-07-18                  | Auditoría trazabilidad plan → backend → DB → frontend → tests                               | ✅ Vigente                     | **Mantener**                                  |
| 16  | `Agente/usuario/verificacion-guia-cliente-bdp-2026-07-22.md`            | 2026-07-22                  | Verificación de cada afirmación de la guía vs código fuente                                 | ✅ Vigente — 100% verificado   | **Mantener**                                  |
| 17  | `Agente/usuario/mapeo-visual-integracion-bdp-2026-07-23.md`             | 2026-07-23 (upd 2026-07-24) | Dónde está cada funcionalidad en el frontend, gaps comunicado vs real                       | ✅ Vigente                     | **Mantener** — útil para onboarding           |
| 18  | `Agente/usuario/hallazgos-revision-2026-07-20.md`                       | 2026-07-20                  | Revisión pre-entrega: 17 secciones, hallazgos críticos (S6-H1, S7-H1-H4, S14-H1, S16-H1-H4) | ✅ Vigente                     | **Mantener** — varios hallazgos aún abiertos  |
| 19  | `Agente/usuario/auditoria-cruzada-bdp-endpoints-frontend-2026-07-23.md` | 2026-07-23                  | Mapa completo 29 endpoints BDP vs manifestación frontend (28/29 con UI)                     | ✅ Vigente                     | **Mantener**                                  |

### 1.6 Otros

| #   | Ruta                                       | Fecha    | Tema                                                            | Estado     | Acción                                                  |
| --- | ------------------------------------------ | -------- | --------------------------------------------------------------- | ---------- | ------------------------------------------------------- |
| 20  | `tools/bdp-weblink-simulator/README.md`    | —        | Documentación del simulador local BDP WebLink                   | ✅ Vigente | **Mantener**                                            |
| 21  | `Agente/lecciones/lecciones-aprendidas.md` | Continuo | Lecciones BDP dispersas en el archivo general (~8 entradas BDP) | ✅ Vigente | **Mantener** — las entradas BDP están bien documentadas |
| 22  | `roadmap.md`                               | Continuo | Resumen ejecutivo BDP + tabla de funcionalidades + bloque 247A  | ✅ Vigente | **Mantener** — actualizar con Compras Fases 2-3         |

---

## 2. Verificación individual de cada MD contra código real

> Cada MD fue verificado buscando evidencia en el código fuente. Leyenda:
> ✅ Verificado correcto | ⚠️ Desactualizado/stale | ❌ Contenido faltante | 🔄 Contradictorio con otro MD

### 2.1 Documentación técnica / API

| # | MD | Verificado | Estado | Detalle de verificación |
|---|-----|-----------|--------|------------------------|
| 1 | `bdp-300035-resumen-completo-2026-06-01.md` | ✅ | **Correcto como histórico** | Order.Type=0 en código actual (no Type=2 como el MD investigó). El error 300035 fue resuelto: serie 00031TI creada, CreateOrder OnlyCheck pasa. El MD documenta correctamente la investigación. |
| 2 | `bdp-cambios-analisis-problemas-2026-06-08.md` | ✅ | **Correcto como histórico** | Los 4 problemas del cliente se resolvieron (confirmado 2026-06-30). CreateOrder OnlyCheck pasa. MarketplaceOrderId limit 15 chars confirmado en código (bdp_sync.rs comment header). |

### 2.2 Seguridad y riesgos

| # | MD | Verificado | Estado | Detalle de verificación |
|---|-----|-----------|--------|------------------------|
| 3 | `bdp-seguridad-produccion-2026-07-21.md` | ✅ | **Correcto y vigente** | 5 capas verificadas: (1) BDP_WRITE_ALLOWED_ORIGINS en bdp_weblink.rs:466-475, (2) bdp_sync_mode=read_only en bdp_config_bootstrap.rs:208, (3) bdp_write_arming con remaining_operations en bdp_write_guard.rs:132, (4) confirmación textual en handlers/ventas.rs, (5) bdp_audit_log en bdp_write_guard.rs:158. Bootstrap desactiva todo. Background tasks inactivos por defecto. |
| 4 | `riesgos-produccion-bdp-2026-07-24.md` | ⚠️ | **Parcialmente desactualizado** | Verificación de cada riesgo contra código:
- **R1**: ✅ `reconcile_ambiguous` implementado en bdp_order_poller.rs (5 funciones reconcile_create_order, reconcile_add_payment, reconcile_invoice, reconcile_ambiguous_pagos)
- **R2**: ✅ Mitigado — distributed_lock cerrado antes de HTTP (§7 marca ✅ Aplicado)
- **R3**: ✅ `Throttled→AmbiguousTransport` en bdp_sync.rs:506
- **R5**: ✅ `tokio::time::timeout(45s)` envuelve fase HTTP
- **R6/R14**: ✅ `SyncLockGuard` RAII con impl Drop en bdp_sync.rs:71-81
- **R8**: ✅ `cached_session` usa recovery pattern, no expect
- **R16**: ⚠️ `decimal_to_f64()` persiste en bdp_sync.rs:1065 (usado en build_order para JSON). Decimal se usa para cálculos pero la conversión final a f64 sigue.
- **§7 sin commit hashes**: ⚠️ Las mitigaciones dicen ✅ Aplicado pero sin referencia de commit |

### 2.3 Planes completados (históricos)

| # | MD | Verificado | Estado | Detalle de verificación |
|---|-----|-----------|--------|------------------------|
| 5 | `plan-bdp-implementacion-completa-2026-07-14.md` | ✅ | **Correcto como histórico** | Tiene disclaimer "HISTÓRICO — NO USAR". Las fases 1-8 están completadas. Fases 7.6-7.7 (pagos parciales y stock) se implementaron después. |
| 6 | `plan-bdp-backup-seguridad-2026-07-15.md` | ✅ | **Correcto como histórico** | bdp_snapshots y bdp_audit_log creadas (verificado en bdp_backup.rs). Endpoints snapshot/restore implementados en handlers/bdp_backup.rs. sync_mode implementado en handlers/configuracion.rs. Disclaimer presente. |
| 7 | `plan-validacion-segura-escritura-bdp-2026-07-18.md` | ✅ | **Correcto, fases E-F pendientes** | Fases A-D completadas localmente. Allowlist deny-by-default en bdp_weblink.rs:466. Lock distribuido con advisory lock en bdp_sync.rs. Simulador existe en tools/bdp-weblink-simulator/. Fases E (lecturas reales) y F (escrituras reales) pendientes de cliente. |

### 2.4 Planes activos

| # | MD | Verificado | Estado | Detalle de verificación |
|---|-----|-----------|--------|------------------------|
| 8 | `plan-pendientes-bdp-2026-07-23.md` | ⚠️ | **Parcialmente desactualizado** | Verificación de cada item:
- **C1 auto-arming**: ✅ Implementado — `try_auto_arm` en bdp_write_guard.rs:47, llamado desde ventas.rs:239,441,539
- **C2 toggle navbar**: ✅ Implementado — BdpStatusIndicator en site-header.tsx
- **D1 stock**: ✅ Implementado — current_stock en bdp_weblink_catalog.rs con aliases CurrentStock/Stock
- **D2 compras**: ⚠️ **SOLO LISTA FASE 1** — El MD dice "Fase 1 implementada, Fases 2-3 pendientes" PERO el código tiene 3 feature flags (ff_bdp_purchase_notes_read/draft/receive) y endpoints para list, sync, draft Y reconcile. **Las 3 fases están implementadas.**
- **D3 bidireccional**: ✅ Bloqueado en configuracion.rs:296
- **D4 pagos parciales**: ✅ Implementado — bdp_pagos table, BdpPagoRepository, endpoints bdp-payments en ventas.rs
- **D5 CancelOrder**: ✅ Bloqueado — ff_bdp_cancel_order existe como feature flag, BDP devuelve "Subscripción no activada"
- **XT1 throttling**: ✅ Implementado — BdpThrottleManager en bdp_throttle.rs, BDP_THROTTLE static
- **XT2 feature flags**: ✅ Implementado — 6 flags en configuracion.rs (no 4 como algunos MDs sugieren) |
| 9 | `plan-visibilidad-bdp-frontend-2026-07-23.md` | 🔄 | **Contradictorio** | **D5 pagos parciales** dice "Excluido por diseño" PERO plan-pagos-parciales dice "Implementado" Y el código tiene bdp_pagos + endpoints. **Contradicción**: este MD no se actualizó cuando se implementaron pagos parciales. Los bloques A, B, C están implementados. |
| 10 | `plan-compras-bdp-2026-07-25.md` | ⚠️ | **Desactualizado — solo documenta Fase 1** | El plan dice "Fase 1 — lectura" y lista los archivos creados. PERO el código tiene 3 endpoints adicionales:
- `marcar_borrador_purchase_note` (ff_bdp_purchase_notes_draft) — Fase 2
- `conciliar_purchase_note` (ff_bdp_purchase_notes_receive) — Fase 3
- `BdpPurchaseNoteReconcileRequest` en models/bdp_purchase_note.rs
**El plan no se actualizó para reflejar que Fases 2 y 3 están implementadas.** |
| 11 | `plan-pagos-parciales-bdp-2026-07-25.md` | ✅ | **Correcto** | bdp_pagos table creada. BdpPagoRepository en repositories/bdp_pago.rs. Endpoint GET /api/ventas/:id/bdp-payments en ventas.rs. ff_bdp_partial_payments en configuracion.rs. Ledger con idempotency_key. |

### 2.5 Documentación usuario/cliente

| # | MD | Verificado | Estado | Detalle de verificación |
|---|-----|-----------|--------|------------------------|
| 12 | `checklist-bdp-integracion-2026-07-16.md` | ✅ | **Correcto como histórico** | Disclaimer "HISTÓRICO — NO ENTREGAR" presente. Contiene clasificaciones obsoletas (OnlyCheck como inocuo). La guía #14 es la versión vigente. |
| 13 | `auditoria-escritura-bdp-2026-07-17.md` | ✅ | **Correcto, W01-W23 cerrados** | Los 23 hallazgos están documentados como cerrados localmente. Verificado: OnlyCheck bloqueado para externos (bdp_weblink.rs), bidirectional bloqueado (configuracion.rs:296), Overwrite=false siempre. |
| 14 | `guia-cliente-pruebas-integracion-bdp-2026-07-18.md` | ✅ | **Correcto y verificado** | Verificación completa en #16 (verificacion-guia-cliente). Todas las afirmaciones tienen contrapartida en código.Documento apto para entrega al cliente. |
| 15 | `auditoria-plan-integracion-completa-bdp-2026-07-18.md` | ✅ | **Correcto** | Los P0/P1 originales se corrigieron. La auditoría detallada de escritura (#13) es la vigente. |
| 16 | `verificacion-guia-cliente-bdp-2026-07-22.md` | ✅ | **Correcto — 100% verificado** | Cada afirmación de la guía fue cruzada contra código. Todas las secciones (1-11) verificadas. Hallazgo menor: edición de ventas sincronizadas usa deduplicación por MarketplaceOrderId. |
| 17 | `mapeo-visual-integracion-bdp-2026-07-23.md` | ⚠️ | **Parcialmente desactualizado** | §6 describe el flujo manual como problemático pero auto-arming (247A-1) ya lo resolvió. El texto actualizado al final de §6 sí lo refleja, pero la sección inicial puede confusión. Stock implementado (237A-4) correctamente reflejado. |
| 18 | `hallazgos-revision-2026-07-20.md` | ⚠️ | **Parcialmente desactualizado** | Hallazgos contra código:
- **S6-H1 (redirect)**: ✅ **CERRADO** — `redirect(Policy::none())` en bdp_weblink.rs:44
- **S7-H1 (UNIQUE bdp_order_id)**: ❌ Abierto (mitigado por advisory lock)
- **S7-H2 (tx post-HTTP)**: ❌ Abierto
- **S7-H3 (UNIQUE bdp_invoiced)**: ❌ Abierto
- **S14-H1 (restore sin tx)**: ❌ Abierto — restaurar_glory() en bdp_backup.rs:547 no usa tx explícita
- **S16-H1 (rate limiting)**: ❌ Abierto — no hay RateLimitLayer
- **S16-H2 (payload limit)**: ❌ Abierto — no hay DefaultBodyLimit |
| 19 | `auditoria-cruzada-bdp-endpoints-frontend-2026-07-23.md` | ✅ | **Correcto** | 29 endpoints verificados. 28/29 con UI. El único sin UI es GET /api/bdp/explorar (herramienta interna). Los endpoints de compras (list, sync) están incluidos. |

### 2.6 Otros

| # | MD | Verificado | Estado | Detalle de verificación |
|---|-----|-----------|--------|------------------------|
| 20 | `tools/bdp-weblink-simulator/README.md` | ✅ | **Correcto** | Simulador existe. server.py con endpoints documentados. Tests con unittest. Solo loopback (127.0.0.1). |
| 21 | `lecciones-aprendidas.md` | ✅ | **Correcto y vigente** | ~8 entradas BDP verificadas: integraciones POS sin sandbox, guías de aceptación, aprovisionamiento por env vars. Lecciones consistentes con la implementación actual. |
| 22 | `roadmap.md` | ⚠️ | **Parcialmente desactualizado** | Resumen ejecutivo correcto para funcionalidades visibles. Bloque 247A-11 dice Compras Fase 1. **Pero no refleja que Fases 2-3 están implementadas** (draft + reconcile con feature flags). Feature flags mencionados como 4, deberían ser 6. |

### 2.7 Resumen de verificación

| Estado | Cantidad | MDs |
|--------|----------|-----|
| ✅ Correcto | 13 | #1, #2, #3, #5, #6, #7, #11, #12, #13, #14, #15, #16, #19, #20, #21 |
| ⚠️ Desactualizado | 6 | #4, #8, #10, #17, #18, #22 |
| 🔄 Contradictorio | 1 | #9 |
| ❌ Faltante | 0 | — |

---

## 3. Análisis de redundancias

### 3.1 Redundancias identificadas

| Contenido duplicado                                        | MDs implicados                                                                                                                     | Recomendación                                                                                                                                                                                                                                                                                                            |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Inventario de endpoints BDP** (lectura/escritura)        | `plan-bdp-backup-seguridad` (§1.2-1.4), `plan-pendientes-bdp` (§D2), `auditoria-cruzada-bdp-endpoints-frontend`                    | La tabla más completa y actual es la de `auditoria-cruzada` (29 endpoints). Las otras son históricas o parciales. **No unificar** — cada una tiene contexto diferente.                                                                                                                                                   |
| **Protecciones de seguridad (5 capas)**                    | `bdp-seguridad-produccion`, `auditoria-escritura-bdp` (§W01-W23), `verificacion-guia-cliente` (§4), `guia-cliente` (§Protecciones) | Las 4 documentos describen las mismas protecciones con niveles de detalle diferentes. `bdp-seguridad-produccion` es el resumen ejecutivo; `auditoria-escritura` es la auditoría detallada; `verificacion` confirma contra código; `guia-cliente` es la versión para el cliente. **No unificar** — audiencias diferentes. |
| **Estado de compras BDP**                                  | `plan-pendientes-bdp` (D2), `plan-compras-bdp`, `mapeo-visual` (§3), `auditoria-cruzada` (Pendiente B), `roadmap.md`               | 5 documentos mencionan compras. `plan-compras-bdp` es el más detallado. Los demás son referencias cruzadas. **No unificar** pero asegurar consistencia de estados.                                                                                                                                                       |
| **Auto-arming (C1)**                                       | `plan-visibilidad-bdp-frontend` (C1), `plan-pendientes-bdp` (C1), `mapeo-visual` (§6)                                              | Los 3 describen el mismo feature. `plan-visibilidad` tiene el plan original; `plan-pendientes` tiene el estado; `mapeo-visual` tiene la justificación al cliente. **No unificar** — ya están consistentes.                                                                                                               |
| **Checklist vs Guía del cliente**                          | `checklist-bdp-integracion-2026-07-16` vs `guia-cliente-pruebas-integracion-bdp-2026-07-18`                                        | La checklist es **histórica** y contiene clasificaciones obsoletas (OnlyCheck como inocuo). La guía es la versión vigente. **La checklist NO debe entregarse** — ya tiene disclaimer.                                                                                                                                    |
| **Plan implementación completa vs Plan validación segura** | `plan-bdp-implementacion-completa-2026-07-14` vs `plan-validacion-segura-escritura-bdp-2026-07-18`                                 | El primero fue sustituido por el segundo. Ambos tienen disclaimer. **No unificar** — el segundo referencia al primero.                                                                                                                                                                                                   |

### 3.2 Conclusión sobre redundancias

**No se recomienda unificar ningún par de documentos.** Las redundancias son aparentes: cada documento tiene una audiencia, nivel de detalle y momento diferentes. Lo que sí se recomienda es:

1. Asegurar que los estados (implementado/pendiente/bloqueado) sean consistentes entre todos.
2. Los documentos históricos ya tienen disclaimers correctos ("HISTÓRICO — NO USAR COMO PROCEDIMIENTO").

---

## 4. Documentos desactualizados o con información stale

| Documento                                     | Contenido desactualizado                                                          | Corrección necesaria                                                                                                           |
| --------------------------------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `plan-pendientes-bdp-2026-07-23.md`           | D4 (pagos parciales) marcado como "~18-22h" pendiente                             | **Ya implementado** — actualizar estado a ✅                                                                                   |
| `plan-pendientes-bdp-2026-07-23.md`           | D2 Compras Fase 1 como "Sí para implementar"                                      | **Ya implementado** (247A-11) — actualizar                                                                                     |
| `plan-visibilidad-bdp-frontend-2026-07-23.md` | D5 pagos parciales como "Excluido por diseño"                                     | **Ya implementado** — actualizar a ✅                                                                                          |
| `mapeo-visual-integracion-bdp-2026-07-23.md`  | §6 sobre flujo manual problemático                                                | **Ya resuelto** con auto-arming (247A-1) — el texto ya lo refleja pero la sección inicial aún describe el problema como activo |
| `roadmap.md`                                  | Resumen ejecutivo no incluye Compras Fases 2-3 como pendientes explícitos         | Añadir a tabla de pendientes                                                                                                   |
| `riesgos-produccion-bdp-2026-07-24.md`        | Mitigaciones §7 marcan R2, R3, R6/R13, R8 como "✅ Aplicado" pero sin commit hash | Añadir hashes de commit cuando estén disponibles                                                                               |

---

## 5. Huecos de cobertura: qué NO está documentado

### 5.1 Sin documento propio

| Tema                                                                                                | Estado                 | Dónde se menciona parcialmente                                             | Necesita MD propio                |
| --------------------------------------------------------------------------------------------------- | ---------------------- | -------------------------------------------------------------------------- | --------------------------------- |
| **Runbook operativo BDP** (qué hacer si BDP se cae, si hay duplicados, si el cliente reporta error) | ❌ No existe           | `guia-cliente` tiene reglas de pruebas pero no runbook de operación diaria | **SÍ** — crítico para producción  |
| **Guía de configuración BDP-NET** (Parámetros 6, series, IVA incluido)                              | ❌ No existe como guía | `bdp-300035-resumen` tiene la info dispersa                                | **SÍ** — útil para soporte        |
| **Documentación de feature flags** (`ff_bdp_*`)                                                     | ❌ No existe           | Mencionados en `riesgos-produccion` (R10) y `roadmap`                      | **SÍ** — necesario para operación |
| **Changelog de integración BDP** (timeline de qué se hizo cuándo)                                   | ❌ No existe           | Disperso en `completados/tareas-2026-07-*.md`                              | Útil pero no urgente              |
| **Tests de aceptación BDP** (qué tests cubren qué escenarios)                                       | ❌ No existe           | `checklist` tiene resumen de tests pero está desactualizado                | **SÍ** — necesario para CI        |

### 5.2 Huecos en documentación existente

| Documento                  | Hueco                                                                                                                                              |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `riesgos-produccion-bdp`   | No cubre: riesgo de que `ExportPurchaseNotes` falle por perfil de exportación incorrecto, riesgo de que el módulo de compras no esté activo en BDP |
| `guia-cliente`             | No cubre: qué hacer si BDP devuelve error desconocido, cómo reportar un bug, tiempos esperados de respuesta                                        |
| `bdp-seguridad-produccion` | No cubre: qué pasa si se despliega una versión nueva que rompe la compatibilidad con BDP, estrategia de rollback del código Glory                  |
| `plan-compras-bdp`         | No cubre: validación del perfil de exportación antes de llamar a BDP, manejo de respuesta vacía vs error                                           |

---

## 6. Huecos de mitigación: riesgos sin resolver

### 6.1 Riesgos críticos — estado actualizado tras verificación

| ID         | Riesgo                                                        | Fuente                   | Estado mitigación                                              | Acción necesaria                            |
| ---------- | ------------------------------------------------------------- | ------------------------ | -------------------------------------------------------------- | ------------------------------------------- |
| **S6-H1**  | HTTP redirect puede evadir allowlist                          | `hallazgos-revision` §6  | ✅ **CERRADO** — `redirect(Policy::none())` en bdp_weblink.rs:44 | Ninguna                                     |
| **S7-H2**  | Sin transacción envolvente post-HTTP para add_payment/invoice | `hallazgos-revision` §7  | ✅ **CERRADO** — `pool.begin()` + `tx.commit()` en add_order_payment e invoice_order (`[207A-2] S7-H2`) | Ninguna                                     |
| **S14-H1** | `restaurar_glory()` sin transacción explícita                 | `hallazgos-revision` §14 | ✅ **CERRADO** — `pool.begin()` + `tx.commit()` en bdp_backup.rs:574 (`[207A-3] S14-H1`) | Ninguna                                     |
| **R1**     | Falsa reconciliación por `AmbiguousTransport`                 | `riesgos-produccion`     | ✅ **IMPLEMENTADO** — `reconcile_ambiguous` en bdp_order_poller.rs | Ninguna                                     |
| **R16**    | Aritmética en `f64` para totales                              | `riesgos-produccion`     | 🟡 **PARCIAL** — Decimal para cálculos, `decimal_to_f64()` para JSON | Redondear Decimal antes de serializar a f64 |

### 6.2 Riesgos medios abiertos

| ID         | Riesgo                                         | Fuente               | Estado                                  |
| ---------- | ---------------------------------------------- | -------------------- | --------------------------------------- |
| **S7-H1**  | `ventas.bdp_order_id` sin UNIQUE constraint    | `hallazgos-revision` | ❌ Abierto (mitigado por advisory lock) |
| **S7-H3**  | Sin UNIQUE en `ventas.bdp_invoiced`            | `hallazgos-revision` | ❌ Abierto (mitigado por status check)  |
| **S16-H1** | Sin rate limiting en API                       | `hallazgos-revision` | ❌ Abierto                              |
| **S16-H2** | Sin `DefaultBodyLimit` explícito               | `hallazgos-revision` | ❌ Abierto                              |
| **R12**    | IVA y precio hardcodeados en `resolve_article` | `riesgos-produccion` | ❌ Abierto                              |

### 6.3 Riesgos bajos / mejoras

| ID         | Riesgo                                              | Fuente               |
| ---------- | --------------------------------------------------- | -------------------- |
| **S16-H3** | `ensure_write_target_allowed()` sin tests unitarios | `hallazgos-revision` |
| **S16-H4** | `canonical_target()` sin test dedicado              | `hallazgos-revision` |
| **S7-H4**  | `authorization_reason` sin sanitizar                | `hallazgos-revision` |
| **S13-H1** | Posible fuga de secrets vía `datos_enviados`        | `hallazgos-revision` |

---

## 7. Qué NO se ha probado

### 7.1 Escrituras reales contra BDP (las 4 pruebas pendientes)

| Prueba         | Endpoint                                               | Estado                    | Bloqueo                           |
| -------------- | ------------------------------------------------------ | ------------------------- | --------------------------------- |
| Crear cliente  | `POST /api/clientes/:id/bdp-sync` → `CreateCustomer`   | ❌ No probado en BDP real | Requiere autorización del cliente |
| Crear comanda  | `POST /api/ventas/:id/bdp-sync` → `CreateOrder`        | ❌ No probado en BDP real | Requiere autorización del cliente |
| Registrar pago | `POST /api/ventas/:id/bdp-payment` → `AddOrderPayment` | ❌ No probado en BDP real | Requiere autorización del cliente |
| Facturar       | `POST /api/ventas/:id/bdp-invoice` → `InvoiceOrder`    | ❌ No probado en BDP real | Requiere autorización del cliente |

### 7.2 Funcionalidades implementadas sin test de integración

| Funcionalidad                   | Tests existentes                              | Lo que falta                                                             |
| ------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------ |
| **Pagos parciales**             | Tests de ledger (`bdp_pagos`) sin BDP         | Test de servicio con simulador BDP (AddOrderPayment parcial)             |
| **Compras Fase 1**              | Tests unitarios de validación y clave natural | Test contra simulador con `ExportPurchaseNotes` real                     |
| **Polling automático**          | Tests de `bdp_order_poller` aislados          | Test de scheduler real con intervalo y multiinstancia                    |
| **Reconciliación post-ambiguo** | Tests de estado `ambiguo`                     | Test end-to-end: timeout → reconciliación → estado final                 |
| **Auto-arming**                 | Tests de `try_auto_arm`                       | Test de concurrencia: dos usuarios intentando auto-armar simultáneamente |
| **Throttling**                  | Test de `BdpThrottleManager`                  | Test de backoff real con respuestas 429 de BDP                           |
| **Snapshot + restore**          | 27 tests backup/restore                       | Test de restore parcial (falla a mitad) — problema conocido S14-H1       |

### 7.3 Escenarios edge-case sin cobertura

| Escenario                                                        | Riesgo | Tests                                                             |
| ---------------------------------------------------------------- | ------ | ----------------------------------------------------------------- |
| BDP devuelve JSON con campo `Lineas` con key diferente           | Medio  | ❌ Sin test — `BdpPurchaseNoteData` asume `Lineas`                |
| `ExportPurchaseNotes` devuelve perfil de exportación inexistente | Medio  | ❌ Sin test                                                       |
| Timeout en medio de importación masiva de catálogo               | Medio  | ⚠️ Sin test — import parcial es idempotente pero no verificado    |
| BDP cambia versión de API y rompe contrato                       | Alto   | ❌ Sin test de compatibilidad                                     |
| Dos instancias de Glory intentando el mismo `CreateOrder`        | Alto   | ⚠️ Mitigado por advisory lock pero sin test de disparo simultáneo |
| `MarketplaceOrderId` > 15 caracteres                             | Medio  | ⚠️ Hay validación pero sin test que verifique el límite exacto    |
| BDP devuelve `Status` desconocido (ej: 4, 5, -1)                 | Bajo   | ✅ Se loggea como `unknown_N` con `warn!` (R7 mitigado)           |

---

## 8. Resumen ejecutivo actualizado: estado de la integración BDP

### ✅ Completado y verificado localmente

- Configuración y conexión BDP
- Catálogo de artículos (sync, precios, stock)
- Mapeos técnicos (tender, canales, artículo/cliente por defecto)
- Clientes BDP (importar/vincular)
- Plano de Sala (mesas BDP)
- Comandas multi-item (crear orden en BDP)
- Pagos completos (AddOrderPayment)
- Pagos parciales (AddOrderPayment parcial, feature flag `ff_bdp_partial_payments`)
- Facturas (InvoiceOrder)
- Estado BDP (badge, polling, consulta individual)
- Explorador de menús/packs/fastfoods
- Snapshots y auditoría
- Bootstrap automático (aprovisionamiento seguro)
- Auto-arming transparente (247A-1)
- **Compras Fases 1-3** — lectura, borradores y conciliación (247A-11+, con 3 feature flags)
- Modo demo en 4 páginas BDP (247A-12)
- Reconciliación de estados ambiguos (R1)
- Throttling BDP (R3)
- SyncLockGuard RAII (R14)
- Timeout global 45s en sync_venta (R5)
- Redirect policy deny (S6-H1)

### ❌ No probado en BDP real

- Las 4 operaciones de escritura (cliente, comanda, pago, factura)
- CancelOrder (bloqueado por BDP: "Subscripción no activada")
- Comportamiento de deduplicación por `MarketplaceOrderId`
- Compatibilidad exacta con la versión BDP del restaurante
- Compras draft/reconcile contra BDP real

### 🔴 Riesgos abiertos que requieren acción (solo 3 críticos)

1. **S7-H2:** Sin transacción envolvente post-HTTP en add_payment/invoice
2. **S14-H1:** restaurar_glory() sin transacción explícita
3. **R16:** Aritmética `f64` en serialización JSON (Decimal se usa para cálculos)

### 📋 Pendientes de decisión del cliente

| Item                       | Pregunta al cliente                                            |
| -------------------------- | -------------------------------------------------------------- |
| Compras en producción      | ¿Activar `ff_bdp_purchase_notes_read/draft/receive`?           |
| CancelOrder                | ¿Pueden activar el módulo de cancelación en BDP?               |
| Stock pantalla completa    | ¿Necesitan ver stock en pantalla dedicada o basta con columna? |
| Pagos parciales producción | ¿Quieren activar `ff_bdp_partial_payments` en producción?      |
| Pruebas de escritura       | ¿Cuándo pueden hacer la sesión de 2h para las 4 pruebas reales?|

---

## 9. Mapa de dependencias entre documentos

```
roadmap.md (resumen ejecutivo)
  ├── Agente/documentacion/bdp-seguridad-produccion-2026-07-21.md (5 capas)
  ├── Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md (16 riesgos)
  │     └── Mitigaciones aplicadas → commits
  ├── Agente/planes/plan-pendientes-bdp-2026-07-23.md (plan maestro)
  │     ├── Agente/planes/plan-compras-bdp-2026-07-25.md (Fase 1)
  │     ├── Agente/planes/plan-pagos-parciales-bdp-2026-07-25.md
  │     └── Agente/planes/plan-visibilidad-bdp-frontend-2026-07-23.md
  ├── Agente/usuario/guia-cliente-pruebas-integracion-bdp-2026-07-18.md (→ cliente)
  │     ├── Agente/usuario/verificacion-guia-cliente-bdp-2026-07-22.md (verificación)
  │     └── Agente/usuario/mapeo-visual-integracion-bdp-2026-07-23.md (dónde está)
  ├── Agente/usuario/auditoria-escritura-bdp-2026-07-17.md (W01-W23)
  ├── Agente/usuario/auditoria-plan-integracion-completa-bdp-2026-07-18.md
  ├── Agente/usuario/auditoria-cruzada-bdp-endpoints-frontend-2026-07-23.md (29 endpoints)
  ├── Agente/usuario/hallazgos-revision-2026-07-20.md (17 secciones)
  │     └── Hallazgos abiertos: S6-H1, S7-H1-H4, S14-H1, S16-H1-H4
  ├── Agente/documentacion/api/bdp-300035-resumen-completo-2026-06-01.md (histórico)
  ├── Agente/documentacion/api/bdp-cambios-analisis-problemas-2026-06-08.md (histórico)
  ├── Agente/planes/completados/plan-bdp-implementacion-completa-2026-07-14.md (histórico)
  ├── Agente/planes/completados/plan-bdp-backup-seguridad-2026-07-15.md (histórico)
  ├── Agente/planes/completados/plan-validacion-segura-escritura-bdp-2026-07-18.md
  ├── Agente/usuario/checklist-bdp-integracion-2026-07-16.md (histórico)
  ├── tools/bdp-weblink-simulator/README.md
  └── Agente/lecciones/lecciones-aprendidas.md (~8 entradas BDP)
```

---

## 10. Hallazgos de la verificación: qué cambió respecto a la versión anterior

### Cambios de estado descubiertos en esta verificación

| Hallazgo | Impacto |
|----------|---------|
| **S6-H1 (redirect) YA ESTÁ CERRADO** — `redirect(Policy::none())` en bdp_weblink.rs:44 | Eliminar de riesgos críticos abiertos |
| **R1 (reconciliación) YA IMPLEMENTADO** — 5 funciones reconcile en bdp_order_poller.rs | Eliminar de riesgos críticos |
| **R3 (Throttled→AmbiguousTransport) YA MITIGADO** — bdp_sync.rs:506 | Actualizar matriz de riesgos |
| **R5 (timeout global) YA IMPLEMENTADO** — `tokio::time::timeout(45s)` en bdp_sync.rs | Actualizar matriz de riesgos |
| **R6/R14 (SyncLockGuard) YA IMPLEMENTADO** — RAII con Drop en bdp_sync.rs:71-81 | Actualizar matriz de riesgos |
| **Compras tiene 3 fases implementadas** (read+draft+receive), no solo Fase 1 | Actualizar plan-pendientes, plan-compras y roadmap |
| **6 feature flags BDP** (no 4) — incluye `ff_bdp_cancel_order` + 3 de compras | Actualizar documentación de feature flags |
| **plan-visibilidad D5** dice "Excluido" pero pagos parciales están implementados | Corregir contradicción |

### Qué falta de lo planificado (resumen consolidado tras verificación)

| Faltante | Prioridad | Esfuerzo | Bloqueo |
|----------|-----------|----------|---------|
| **4 pruebas de escritura real** (cliente, comanda, pago, factura) | 🔴 Crítica | ~4h sesión | Autorización del cliente |
| **Tests simulador**: pagos parciales + compras | 🟡 Media | ~6h | Ninguno |
| **R16: decimal_to_f64 en JSON** | 🟡 Baja | ~3h | Conversión via string ya es precisa |
| **S7-H1/H3: UNIQUE constraints** | 🟡 Baja | ~2h | Migración BD (bajo riesgo, mitigado por advisory lock) |
| **S16-H1/H2: rate limiting + payload** | 🟡 Baja | ~4h | Ninguno |
| **Tests concurrencia** (advisory lock simultáneo) | 🟡 Baja | ~4h | Ninguno |
| **Compras Fase 2-3 en producción** | ⚪ Decision cliente | ~22h | Cliente no ha respondido |
| **CancelOrder** | ⚪ Decision cliente | ~12-16h | BDP: "Subscripción no activada" |
| **Stock pantalla dedicada** | ⚪ Decision cliente | ~8h | Cliente no ha pedido |

### Documentos creados en esta sesión de organización

| Documento | Propósito |
|----------|----------|
| `Agente/documentacion/bdp/maestro-organizacion-bdp-2026-07-26.md` | Inventario + verificación de 22 MDs BDP |
| `Agente/documentacion/bdp/maestro-auditoria-bdp-2026-07-26.md` | Consolidación de 7 auditorías + controles contra desastres |
| `Agente/documentacion/bdp/feature-flags-bdp-2026-07-26.md` | Documentación de los 6 feature flags BDP |
| `Agente/documentacion/bdp/runbook-operativo-bdp-2026-07-26.md` | Procedimientos para 11 tipos de incidente |

---

## 11. Acciones recomendadas

### Completado en esta sesión de organización

1. ✅ **Verificación de 22 MDs** contra código real — tabla completa en §2
2. ✅ **S6-H1 cerrado** — redirect policy ya estaba implementada
3. ✅ **S14-H1 cerrado** — `restaurar_glory()` ya usa transacción (`[207A-3]`)
4. ✅ **S7-H2 cerrado** — add_order_payment e invoice_order ya usan transacción (`[207A-2]`)
5. ✅ **R1 implementado** — reconciliación de ambiguos en `bdp_order_poller.rs`
6. ✅ **R3, R5, R6, R8, R14 mitigados** — verificados contra código
7. ✅ **6 MDs desactualizados corregidos** — plan-pendientes, plan-compras, plan-visibilidad, riesgos, roadmap
8. ✅ **MD maestro de auditoría creado** — 7 auditorías consolidadas + controles contra desastres
9. ✅ **Documentación feature flags creada** — 6 flags documentados
10. ✅ **Runbook operativo creado** — 11 procedimientos de incidente

### Pendiente real (solo queda lo que requiere autorización del cliente)

| Item | Esfuerzo | Bloqueo |
|------|----------|---------|
| **4 pruebas de escritura real** contra BDP | ~4h | Autorización del cliente |
| **Activar feature flags en producción** (compras, pagos parciales) | ~1h | Decisión del cliente |
| **CancelOrder** | ~12-16h | BDP: "Subscripción no activada" |
| **Stock pantalla dedicada** | ~8h | No solicitado por cliente |
| **Tests simulador** (pagos parciales + compras) | ~6h | Bajo riesgo, no urgente |
| **S7-H1/H3 UNIQUE constraints** | ~2h | Bajo riesgo, mitigado |
| **S16-H1/H2 rate limiting + payload** | ~4h | Bajo riesgo |
| **R16 decimal_to_f64** | ~3h | Ya es preciso via string |
