# wandori.us — Roadmap

> **Producto:** blog, portfolio y tienda digital dentro de un OS retro minimalista
> **Stack:** Rust/Axum + PostgreSQL + Vanilla TypeScript/Vite
> **Deploy:** no planificado
> **Epic:** 297A-4 — OS persistente, cuentas, programas y comercio
> **Visual:** identidad desktop y prototipo móvil aprobados; shell móvil real y transición dinámica por URL/params implementados, validación visual/E2E pendiente

## Fuentes canónicas

- Índice: `Agente/documentacion/indice-documentacion-2026-07-29.md`
- Arquitectura: `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`
- Identidad: `Agente/documentacion/design-system/manual-identidad-visual-os-2026-07-29.md`
- Plan maestro: `Agente/planes/plan-escritorio-persistente-cuentas-admin-apps-2026-07-29.md`
- Plan móvil: `Agente/planes/plan-experiencia-movil-launcher-2026-07-29.md`
- Quality gate: `Agente/planes/completados/plan-escalabilidad-sentinel-wandorius-2026-07-29.md`
- Prevención: `Agente/prevencion/prevencion-wandorius-sentinel-varsense-2026-07-29.md`

## Estado y reglas

- Concepto desktop aprobado; Finder es file browser real (lee workspaceStore); Reader sigue siendo preview.
- Workspace overlay implementado: release + overlay + merge + clipboard + papelera + crear carpetas.
- Split de archivos grandes completado: command-registration (725→6), workspace-store (430→4), desktop-shell (419→3).
- Sesiones opacas en cookie operativas; JWT localStorage eliminado del frontend. `/admin` legacy y uploads públicos siguen como deuda controlada.
- **Quality tool sprint completo:** 13 reglas custom (P0/P1/P2) + 7 Sentinel CLI + 4 VarSense = 24 reglas activas, ~65% cobertura de hallazgos. VarSense reconoce clases dinámicas de fábricas DOM vanilla. Auditoría v4 al 83% (19/23). ISP refactor DomAttrs (33→6 sub-interfaces). 156 tests en 11 suites.
- Ejecutar una tarea por vez y en este orden; no saltar dependencias.
- El plan maestro contiene checklists/gates. El roadmap conserva solo pendientes.
- El quality gate está operativo; toda tarea futura debe cerrarse con `npm run task:check -- {ID}`.

## Siguiente bloque habilitado

**297A-12 — Runtime móvil parcial implementado.** El shell/stack real está validado por type-check, tests y quality gate. Quedan transición dinámica móvil↔tablet, pruebas visuales en navegador, reorder/long press y cierre de apps críticas.

**297A-10 — Recursos y migraciones (completado).** Resource envelope, product versions, asset states, services con transacción, DTO público/admin y About seeder.

**Plan transversal 297A-4 — parcialmente cerrado.** CommandRegistry enriquecido (§2), selección+foco (§3), context menu (§2.3), keyboard move/resize (§4.1), resource-type-registry (§7), analytics envelope (§9.1/9.2). Pendiente: clipboard/undo (§5, 297A-11), persistencia (§6, 297A-13), mobile (§2.3/3, 297A-12), tests unitarios (§11, requiere vitest).

## Pendientes ordenados

### 297A-9 — Foundation del runtime

**Depende de:** 297A-6/7; coordina identidad 297A-8.

- [x] MountedView/AbortSignal y RouteAppAdapter activado con interceptor.
- [x] AppRegistry, WindowManager y CommandRegistry.
- [x] Taskbar reactivo derivado de windowStore con iconos Lucide.
- [x] Dispatcher analítico tipado.
- [x] Drag y resize de ventanas por bordes con boundary clamping.
- [x] Atajos de teclado (Escape, Meta+m, Ctrl+Shift+ArrowRight).
- [x] contentWindow oculto en rutas manejadas por apps.
- [ ] Orval Fetch tags-split (requiere backend corriendo con OpenAPI).

**Salida:** runtime compartido funciona sin chrome/listas/listeners duplicados. Orval pendiente hasta tener backend operativo.

### 297A-10 — Recursos y migraciones (en ejecución)

**Depende de:** 297A-7/9.

- [x] Catálogo `resources`, estados ortogonales y defaults privados. *(migration + model + repo)*
- [x] About como artículo (system_alias); producto independiente (article_id opcional).
- [x] Asset states (processing/clean/rejected) en media.
- [x] Product versions inmutables para entregables.
- [x] Backfill resources desde datos existentes.
- [x] Services actualizados: crear via transacción envelope + registro con ID compartido.
- [x] DTO público/admin separado. *(ArticlePublic sin system_alias/status/is_pinned)*
- [x] About seeder/alias para página about. *(GET /api/articles/alias/{alias}, PUT /admin/articles/{id}/alias)*

**Salida:** organizar referencias no altera publicación y ninguna metadata privada se filtra.

### 297A-11 — Workspace público y overlay invitado

**Depende de:** 297A-9/10.

- [x] Árbol, referencias, initial windows y IDs estables. *(default-release.ts + workspace types)*
- [x] Overlay local: additions/overrides/tombstones. *(workspace-store.ts + overlay-mutations.ts)*
- [x] Clipboard, ciclos, papelera por capa y reset. *(clipboard.ts + trash app)*
- [x] Finder como file browser real (lee workspaceStore, breadcrumb, drag/drop, context menu). *(finder-preview.ts rewrite)*
- [x] WorkspaceNodeType incluye 'resource' + resourceKind. *(types.ts alineado con manual §6.2)*
- [x] RenderContext con params para parámetros de instancia. *(lifecycle.ts + openAppWindow + openWindow)*
- [x] Sistema de menús unificado (CommandRegistry como fuente única). *(toolbar refs + createAppToolbar)*
- [x] App toolbar automático en todas las ventanas. *(createDesktopWindow siempre renderiza toolbar)*
- [x] Split de archivos grandes: command-registration (725→6), workspace-store (430→4), desktop-shell (419→3).
- [x] Migración workspace_releases con seed data. *(20260731000000)*
- [x] Draft/release/preview/publicar/rollback. *(diff.ts + publish con confirmación + rollbackWorkspace)*
- [x] Organizador público separado del workspace admin personal. *(previewPublicStore + workspace:preview-public)*

**Salida:** admin publica el preview exacto; visitante reorganiza sin escribir global.

### 297A-12 — Experiencia móvil tipo launcher

**Depende de:** 297A-9/11. Tablet conserva desktop.

- [x] Prototipo visual móvil aprobado por el usuario (2026-07-30).
- [x] Launcher + MobileAppStack con las mismas apps.
- [x] Shell móvil full-screen, sin ventanas/barra superior/taskbar; validación visual por viewport pendiente.
- [x] Back/Home y carpetas consumen workspace/registry; Back/Home sincronizan URL; long press y reorder siguen pendientes.
- [x] Transición dinámica móvil↔tablet sin recarga mediante reinstanciación segura.
- [x] Cambio móvil↔tablet conserva app/recurso por URL/params; estados transitorios siguen pendientes.
- [ ] Pruebas visuales/E2E 320/360/390 y tablet 768.

**Salida:** teléfono funciona como launcher sin duplicar lógica; tablet sigue como escritorio.

### 297A-13 — Registro y overlay remoto

**Depende de:** 297A-8/11; integra móvil 297A-12.

- [ ] Habilitar registro verificado.
- [ ] Overlay remoto y preferencias.
- [ ] Importar local/usar remoto/reset explícito.
- [ ] Merge por ID/campo y 409 visible.
- [ ] Pruebas dos dispositivos/release nuevo.

**Salida:** cuenta sincroniza sin sobrescribir ni restaurar recursos retirados.

### 297A-14 — Programas editoriales

**Depende de:** 297A-9/10/11.

- [ ] Editor de artículos/About.
- [ ] Editor de proyectos.
- [ ] Editor de productos privado/inactivo por defecto.
- [ ] Biblioteca de media.
- [ ] Menú Admin por capacidades y paridad sin ampliar `admin.ts`.

**Salida:** administración editorial vive en programas reutilizables del OS.

### 297A-15 — Comercio seguro

**Depende de:** 297A-7/10/14.

- [ ] Tienda como carpeta, Compra y Pedidos.
- [ ] Product versions y storage privado.
- [ ] Orden idempotente y webhook validado/transaccional.
- [ ] Entitlements, grants y outbox.
- [ ] Compra invitada, reembolso/revocación y pruebas negativas.

**Salida:** cliente no concede acceso; comprador recibe la versión adquirida.

### 297A-16 — Analytics, Estadísticas y retiro legado

**Depende de:** 297A-9/11–15.

- [ ] Consentimiento/retención y eventos esenciales/opcionales.
- [ ] Batch idempotente y eventos críticos server-side.
- [ ] Agregados, Estadísticas y audit separado.
- [ ] Paridad y eliminación de `/admin`, JWT, uploads y contratos/CSS legacy.

**Salida:** una sola administración y métricas privadas/tipadas.

### 297A-17 — Hardening, identidad, accesibilidad y SEO

**Depende de:** 297A-6–16.

- [ ] MFA/passkey, estrategia SEO, sitemap y metadata.
- [ ] Manual visual en desktop/tablet/móvil.
- [ ] Teclado, foco, zoom, reduced motion y multimedia accesible.
- [ ] Quality gate/CI completos y E2E críticos.
- [ ] Threat review, performance, observabilidad y runbook.

**Salida:** preparado para revisión de producción; deploy continúa fuera de alcance.

## Detalle operativo de las tareas pendientes

Este bloque amplía el alcance verificable sin duplicar los manuales canónicos. Cada tarea debe cerrarse con su plan enlazado, prueba positiva y casos negativos; ninguna casilla se marca por intención.

### 297A-9 — Foundation del runtime

- [ ] Definir la matriz de `AppRegistry`, `CommandRegistry` y `RouteAppAdapter`: capacidades, rutas profundas, evento emitido y teardown con `AbortSignal`.
- [ ] Conectar OpenAPI y Orval en modo `tags-split`, sin editar `generated.ts`; mantener la misma envoltura de éxito/error entre cliente y servidor.
- [ ] Cubrir IDs únicos, capacidades, disponibilidad por presentación e idempotencia con Vitest; eventos críticos deben quedar server-side.
- [ ] Ejecutar prueba visual del shell en 1440x900, 1024x768, 390x844 y 320px, incluyendo foco, teclado y zoom 200%.

### 297A-12 — Experiencia móvil tipo launcher

- [ ] Completar long press, menú contextual y reorder accesible; el orden personal se guarda como `mobileOrder` y no altera el release público.
- [ ] Probar Back/Home, carpetas, deep links, refresco, orientación, safe areas, teclado virtual, overflow y cambio móvil/tablet sin perder URL, foco, scroll o formularios.
- [ ] Verificar 320/360/390px y tablet 768px, rendimiento y apps críticas: Cuenta, Finder, Reader, Editor, Store, Checkout, Descargas, Configuración y Estadísticas.
- [ ] Confirmar que móvil reutiliza comandos, permisos, recursos y analítica del escritorio; solo cambia `presentationMode`.

### 297A-13 — Registro y overlay remoto

- [ ] Implementar registro verificado, login/logout, recuperación, rate limit y auditoría detrás de feature flag; logout limpia clipboard/undo.
- [ ] Sincronizar overlay y preferencias con revisión esperada, actualización optimista con rollback y conflicto 409 visible.
- [ ] Definir importación local, uso remoto, reset explícito y merge por ID/campo; probar dos pestañas/dispositivos, pérdida de red y release nuevo.

### 297A-14 — Programas editoriales

- [ ] Congelar la matriz de paridad del Admin legado y migrar acciones a programas con capacidades server-side y audit trail.
- [ ] Cubrir artículos/About, proyectos, productos versionados y media con draft/private/public, preview, publicación inmutable, rollback, papelera y autosave.
- [ ] Validar que mover referencias no muta recursos, que MIME lo decide el backend y que copiar/cortar/pegar respeta colisiones, historial y permisos.

### 297A-15 — Comercio seguro

- [ ] Modelar Tienda como carpeta y Product/Checkout/Orders/Descargas como programas; validar precio, versión, disponibilidad y storage exclusivamente en backend.
- [ ] Usar idempotency key, `UNIQUE provider_event_id`, webhook verificado y outbox transaccional; nunca conceder acceso desde el navegador.
- [ ] Emitir entitlement/grant corto y probar invitado, login posterior, reembolso, chargeback, revocación, reintentos y fallos del proveedor.

### 297A-16 — Analytics, Estadísticas y retiro legado

- [ ] Definir consentimiento, retención, anonimización, derechos y allowlist de propiedades; no enviar contenido ni IDs sensibles.
- [ ] Implementar ingesta batch idempotente con rate limit y transacción; separar eventos agregados de audit inmutable.
- [ ] Entregar paneles Overview, Content, OS, Commerce y Reliability con fórmulas, roles, zona horaria, estados vacío/carga/error y exportación.
- [ ] Retirar `/admin`, JWT, uploads y contratos/CSS legacy mediante una matriz de paridad y rollback documentado.

### 297A-17 — Hardening, identidad, accesibilidad y SEO

- [ ] Completar MFA/passkey, recuperación y threat review con casos negativos de sesión, CSRF, capacidades, pagos, grants y webhooks.
- [ ] Validar HTML público, sitemap, metadata y Open Graph sin exponer drafts ni rutas privadas.
- [ ] Verificar manual visual, teclado, foco, live regions, zoom 200%, reduced motion, alto contraste y multimedia accesible.
- [ ] Ejecutar Sentinel, VarSense, type-check, tests, E2E, presupuestos de rendimiento, observabilidad y runbook Coolify; deploy continúa fuera de alcance.
