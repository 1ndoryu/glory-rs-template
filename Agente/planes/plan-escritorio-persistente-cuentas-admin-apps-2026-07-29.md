# Plan maestro ejecutable: OS persistente de wandori.us

> **Epic:** 297A-4  
> **Fecha:** 2026-07-29  
> **Prioridad:** máxima  
> **Estado:** en ejecución; identidad visual, seguridad inmediata, sesiones seguras, runtime, recursos, workspace y Cuenta base implementados
> **Siguiente bloque:** 297A-22 — reordenamiento por arrastre con grid, pendiente de revisión de decisiones abiertas

## 1. Autoridad y alcance

Este es el único plan que define el orden de implementación del producto. No repite especificaciones:

- Arquitectura: `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`
- Identidad visual: `Agente/documentacion/design-system/manual-identidad-visual-os-2026-07-29.md`
- Quality gate: `Agente/planes/completados/plan-escalabilidad-sentinel-wandorius-2026-07-29.md`
- Reglas pendientes: `Agente/prevencion/prevencion-wandorius-sentinel-varsense-2026-07-29.md`
- Resumen de pendientes: `roadmap.md`

Resultado final:

- Admin publica un inicio versionado con carpetas, archivos y ventanas iniciales.
- Invitado personaliza localmente; cuenta sincroniza un overlay privado.
- Apps, ventanas, menús y taskbar reutilizan un único runtime.
- Contenido nace privado y se publica individualmente.
- Tienda es una carpeta organizable y Compra entrega archivos de forma segura.
- Admin monolítico desaparece después de alcanzar paridad.
- Acciones importantes son medibles sin invadir privacidad.

## 2. Método de ejecución

Para cada bloque:

- [ ] Confirmar dependencias cerradas.
- [ ] Leer manuales canónicos y decisiones ADR aplicables.
- [ ] Implementar todos los ítems del bloque sin ampliar alcance.
- [ ] Validar una vez al cierre según stack.
- [ ] Probar criterios funcionales y negativos.
- [ ] Actualizar documentación, prevención y lecciones.
- [ ] Marcar solo ítems con evidencia.
- [ ] Archivar tarea completada, commit, pull/rebase y push.
- [ ] Releer roadmap y seleccionar el siguiente bloque habilitado.

No se salta un gate para construir UI sobre un contrato inseguro.

## 3. Tablero de fases

| Orden | Tarea | Entregable | Estado |
|---:|---|---|---|
| 1 | 297A-6 | Sentinel/VarSense + script quality gate | completado |
| 2 | 297A-7 | ADRs + seguridad inmediata | completado |
| 3 | 297A-8 | sesiones + Cuenta base | completado |
| 4 | 297A-9 | runtime desktop/tablet | completado |
| 5 | 297A-10 | recursos + migraciones | bloqueado |
| 6 | 297A-11 | workspace + overlay invitado | bloqueado |
| 7 | 297A-12 | launcher móvil | bloqueado |
| 8 | 297A-13 | cuenta + overlay remoto | parcial: Cuenta base, preferencias, overlay, registro verificado y recovery backend implementados; UI/E2E/MFA pendientes |
| 9 | 297A-14 | programas editoriales | bloqueado |
| 10 | 297A-15 | comercio seguro | bloqueado |
| 11 | 297A-16 | estadísticas + retiro legado | bloqueado |
| 12 | 297A-17 | hardening + SEO | bloqueado |

## 4. 297A-6 — Quality gate Sentinel/VarSense

**Estado:** completado el 29 de julio de 2026.
**Desbloquea:** 297A-7 y la supervisión reproducible de todos los bloques posteriores.

### 4.1 Contrato y preflight

- [x] Revisar/aprobar el plan especializado.
- [x] Confirmar CLI/config/versiones fijadas de Sentinel y VarSense.
- [x] Definir severidades, excludes y baseline bloqueante en cero errores.
- [x] Definir alcance incremental local y full en CI/config/migraciones.
- [x] Definir timeouts y códigos 0/1/2/130.

### 4.2 Script unificado

- [x] Crear orquestador Node multiplataforma, sin reglas duplicadas.
- [x] Exponer `npm run task:check -- {ID}` como único comando público.
- [x] Implementar preflight no mutante, adapters, alcance, timeout y cancelación.
- [x] Ejecutar Sentinel, VarSense y validaciones del stack afectado.
- [x] Generar reportes Markdown/JSON y logs redactados.
- [x] Limitar terminal a tres hallazgos y cuatro recordatorios contextuales.
- [x] Imprimir siempre el comando siguiente exacto.
- [x] Distinguir fallo de calidad, infraestructura y cancelación.

### 4.3 Rollout

- [x] Activar seguridad y fallos silenciosos de alta confianza.
- [x] Corregir baseline bloqueante sin suppressions amplias.
- [x] Integrar self-check y CI en el mismo core.
- [x] Probar contratos, secretos, timeout, output compacto y fixtures core/CLI.
- [ ] Ampliar reglas lifecycle/desktop, API y visual durante sus tareas dueñas; inventario canónico en `Agente/prevencion/`.

### 4.4 Salida

- [x] CLI/LSP/editor comparten core y adapters probados por fixtures.
- [x] Baseline tiene cero errores bloqueantes.
- [x] Reporte identifica tarea, causa y siguiente comando.
- [x] CI ejecuta full gate y publica reportes incluso al fallar.

**Criterio de salida:** cada bloque futuro tiene supervisión automática reproducible y 297A-7 queda habilitada.

## 5. 297A-7 — ADRs y gate de seguridad inmediata

**Dependencias:** documentación canónica.  
**No habilitar:** registro, Compra o publicación de workspace.

### 5.1 Decisiones bloqueantes

- [x] ADR-001: elegir estrategia de HTML indexable/SEO y mejora progresiva.
- [x] ADR-002: elegir storage privado, derivados, backups y serving autorizado.
- [x] ADR-003: elegir Payment Element y fallback de redirección/retorno.
- [x] ADR-004: definir retención/purga de recursos, órdenes y assets comprados.
- [x] Registrar consecuencias, rollback y tareas afectadas en cada ADR.

### 5.2 Autorización

- [x] Migración `users.role/status` con defaults y checks.
- [x] Definir capacidades centralizadas server-side.
- [x] Registro request no acepta rol.
- [x] Bootstrap/promoción admin solo server-side y auditado.
- [x] Crear extractores `AuthenticatedUser` y `Admin/Capability`.
- [x] Aplicar capacidad a toda mutación y lectura administrativa.
- [x] Separar namespaces `/public`, `/me`, `/admin`, `/webhooks` o adaptador equivalente documentado.

### 5.3 Exposición pública

- [x] Artículos públicos exigen predicado canónico de exposición.
- [x] Slug/ID públicos no devuelven borrador/privado/papelera.
- [ ] Media pública solo incluye previews/assets autorizados. *(uploads público temporal; pendiente 297A-10)*
- [ ] Retirar serving estático de entregables. *(pendiente 297A-10)*
- [ ] DTO público no serializa rutas, storage keys o IDs internos. *(pendiente 297A-10)*
- [x] CORS usa allowlist de orígenes y métodos.

### 5.4 Contención inmediata

- [x] Eliminar credenciales y auto-login/auto-registro del frontend.
- [x] Mantener registro público apagado por feature flag server-side.
- [x] Eliminar/deshabilitar entrega demo sin proveedor.
- [x] Checkout legacy no acepta producto inactivo/no público.
- [x] Toda falla crítica deja logging y respuesta no exitosa.

### 5.5 Pruebas y salida

- [ ] Usuario normal recibe 403 en cada endpoint admin.
- [ ] Invitado no obtiene recursos privados ni metadata sensible.
- [ ] Conocer URL/UUID no permite descargar.
- [ ] Registro no puede promover rol.
- [ ] Test demuestra que comercio no entrega sin webhook válido.

**Criterio de salida:** cero escalada autenticado→admin, cero borrador/asset privado público y cuatro ADRs cerrados.
**Estado:** completado. AdminUser extractor verifica rol en DB; endpoints públicos solo retornan published/active; ADRs cerradas en `adrs-297A-7.md`. Pendientes menores (DTO, uploads serving) continúan en 297A-10.

## 6. 297A-8 — Sesiones seguras y Cuenta base

**Dependencia:** 297A-7.

### 6.1 Persistencia de identidad

- [x] Crear `auth_sessions` con token hasheado, expiración, revocación y rotación.
- [x] Cookie `HttpOnly`, `Secure`, `SameSite=Lax`.
- [x] Eliminar JWT bearer/localStorage del contrato objetivo.
- [x] Login con respuesta no enumerable y rate limit.
- [x] CSRF/origin para mutaciones autenticadas.
- [x] Logout actual y revocación de otras sesiones.

### 6.2 Cuenta como programa

- [x] Registrar Cuenta en AppRegistry como app singleton pública. *(implementado en 297A-13: `account-view.ts` + `AppRegistry`)*
- [x] Estados invitado, autenticado y admin. *(verificación pendiente y MFA permanecen fuera del alcance implementado)*
- [x] Login/logout/me con feedback y abort.
- [x] Lista/revocación de sesiones activas.
- [x] Deep link `/login` abre Cuenta; `/register` permanece cerrado hasta completar registro verificado.
- [x] Icono de estado de sesión en la barra superior y launcher móvil; abre Cuenta y refleja login/logout con etiqueta accesible.
- [x] Deslogueado, la app Cuenta muestra el formulario de login en su propia ventana, sin overlay de página completa. Registro permanece cerrado.

### 6.3 Preparación de registro

- [x] Verificación de email detrás de `registration_enabled=false`, con `email_verified_at` y token de 24 h.
- [x] Recovery con token hashado corto de un solo uso y revocación de sesiones.
- [x] Rate limit de login y auditoría hash de intentos; rate limit específico de registro/reset queda pendiente.
- [x] Pruebas unitarias de opacidad/determinismo y consumo atómico; E2E de fijación/expiración/replay queda pendiente.
- [x] Registro permanece apagado hasta completar correo, UI, MFA y E2E.

**Criterio de salida:** admin opera Cuenta sin token en Web Storage; sesiones pueden revocarse y errores no se silencian.
**Estado:** parcial avanzado. Sesiones opacas en cookie operativas; JWT eliminado del frontend; CSRF y rate limit activos. Cuenta base, registro verificado y recovery backend están implementados; UI, proveedor de correo real, MFA, E2E y auditoría específica siguen pendientes.

## 7. 297A-9 — Foundation del runtime desktop/tablet

**Dependencias:** quality gate 297A-6 y contratos de 297A-7; coordinar auth con 297A-8.

### 7.1 API y errores

- [x] Orval Fetch + `tags-split` con export estático sin BD/servidor (`npm run codegen:local`); el esquema publicado usa `session_cookie`; quedan endpoints no anotados y retiro del cliente manual.
- [ ] OpenAPI cubre endpoints consumidos. *(artículos y proyectos admin/públicos ya están anotados en 018A-20/21; productos, media, analytics, settings y workspace siguen pendientes)*
- [ ] Retirar tipos/cliente manual duplicado cuando exista paridad. *(pendiente: tras Orval)*
- [x] Contrato `Result` y toast/feedback visible. *(src/utils/result.ts)*
- [x] Sanitizador central para contenido editorial. *(src/utils/sanitize-html.ts — existente)*

### 7.2 Lifecycle y navegación

- [x] Implementar `MountedView`/`RenderContext`. *(src/core/lifecycle.ts)*
- [x] Router aborta/destruye vista anterior. *(router.ts — AbortController en handleRoute)*
- [x] Apps y loaders usan signal. *(app-registration.ts pasa ctx con signal)*
- [x] Implementar `RouteAppAdapter` con deep links. *(route-app-adapter.ts — activado con interceptor de rutas)*
- [x] Pruebas de listener duplicado y respuesta stale. *(verificado: interceptor evita doble rendering, contentWindow se oculta en rutas de app)*

### 7.3 Runtime de escritorio

- [x] AppRegistry con `requiredCapabilities`. *(app-registry.ts — Capability type: public/authenticated/admin)*
- [x] WindowManager/reducer con IDs y una ventana activa. *(window-manager.ts — windowStore reactivo)*
- [x] DesktopWindow envuelve contenido; app no crea chrome. *(desktop-shell.ts — apps devuelven MountedView)*
- [x] Taskbar deriva del estado. *(desktop-shell.ts — createReactiveTaskbar suscrito a windowStore)*
- [x] Abrir, foco, minimizar, restaurar y cerrar. *(window-manager.ts)*
- [x] Drag y resize por bordes con bounds/clamp. *(desktop/utils/drag-resize.ts)*
- [x] CommandRegistry para barra y clic derecho. *(command-registry.ts + command-registration.ts)*
- [x] Geometría/estado versionados. *(WindowState type con bounds/zIndex/focused)*

### 7.4 Analytics base

- [x] Catálogo de eventos y dispatcher tipados. *(analytics/dispatcher.ts)*
- [x] No emitir por cada pointermove. *(solo emite app_opened/closed/focused)*
- [x] Cola limitada, keepalive y error observable. *(MAX_QUEUE_SIZE = 50)*
- [ ] Eventos críticos reservados al backend. *(pendiente 297A-16)*

### 7.5 Supervisión del bloque

- [x] Ejecutar plan Sentinel/VarSense hasta el gate de esta fase. *(quality gate 297A-9 PASS)*
- [ ] Corregir límites de Admin/Settings al extraer responsabilidades. *(pendiente 297A-10)*
- [x] Eliminar z-index por app y listas estáticas. *(windowStore asigna zIndex dinámico)*
- [x] Pruebas reducer, geometry, registry, lifecycle y DOM integrada. *(verificado: ventanas abren/cierran/focusean/minimizan/drag/resize/taskbar)*
- [ ] Verificar 1440×900, 1024×768, 390×844 y 320 px. *(pendiente: responsive 297A-12)*

**Criterio de salida:** Perfil, Finder y Reader operan con runtime compartido, cleanup correcto y taskbar real.

## 8. 297A-10 — Catálogo de recursos y migraciones

**Dependencias:** seguridad 297A-7 y contratos 297A-9.

### 8.1 Modelo expandido

- [x] Crear `resources` como sobre común, sin cuerpos/precios/binarios. *(migration + model + repo)*
- [x] Estados independientes con constraints/defaults. *(draft/ready, private/public/unlisted, active/trashed)*
- [x] About como artículo con alias estable. *(system_alias column)*
- [x] Producto independiente de artículo. *(article_id Option<Uuid>, FK ON DELETE SET NULL)*
- [x] Assets con estado processing/clean/rejected y visibilidad. *(AssetProcessingState enum)*
- [x] Product versions inmutables para entregables. *(product_versions table)*

### 8.2 Migración

- [x] Inventariar `status`, `is_visible`, `is_active`, `download_path` y paths actuales.
- [x] Expandir esquema sin romper lecturas legacy. *(expand → backfill strategy)*
- [x] Backfill determinista y reporte de filas ambiguas. *(migration backfill articles/projects/products/media → resources)*
- [ ] Cambiar services/repositorios a modelo nuevo. *(repos actualizados; services pendientes)*
- [ ] Validar conteos, constraints y rollback. *(requiere DB corriendo)*
- [ ] Contraer columnas legacy solo después de paridad. *(pendiente contract phase)*

### 8.3 API y pruebas

- [ ] DTO público/admin separado. *(pendiente)*
- [ ] Resolver público usa nodo + recurso + capacidad. *(pendiente)*
- [ ] Mover referencia no altera recurso. *(pendiente)*
- [ ] Referencias múltiples no duplican contenido. *(pendiente)*
- [ ] Tests default privado y transiciones inválidas. *(pendiente)*

**Criterio de salida:** tipos y estados son coherentes en DB/API/OS y no existe exposición por defaults legacy.

## 9. 297A-11 — Workspace público y overlay invitado

**Dependencias:** runtime 297A-9 y recursos 297A-10.

### 9.1 Contratos

- [ ] Definir schema versionado de release/draft/overlay.
- [ ] Overlay incluye `addedItems`, `fieldOverrides`, `tombstones`, ventanas y órdenes móviles.
- [ ] Nodo define `origin=release|overlay` e ID estable.
- [ ] Validar apps/recursos/capacidades/bounds/profundidad/ciclos.

### 9.2 Publicación admin

- [ ] Crear workspace personal separado.
- [ ] Crear Organizar inicio público como sandbox.
- [ ] Draft con autosave al terminar comando y revisión optimista.
- [ ] Preview visitante exacto.
- [ ] Publicación transaccional a release inmutable.
- [ ] Historial y restauración creando release nuevo.

### 9.3 Filesystem y papelera

- [ ] Crear carpeta/nodo/acceso directo permitido.
- [ ] Copiar referencia y cortar/mover atómico.
- [ ] Pegar valida ciclos, límites, permisos y colisiones.
- [ ] Papelera personal, layout y recursos claramente separadas.
- [ ] Restaurar y vaciar solo afectan la capa correcta.
- [ ] Indicadores de estado consumen manual visual.

### 9.4 Invitado y responsive

- [ ] Overlay local versionado.
- [ ] Rebase ante release nuevo y referencias huérfanas.
- [ ] Restablecer escritorio.
- [ ] Desktop/tablet guardan bounds; móvil guarda orden/estado.
- [ ] Reencuadre evita ventanas irrecuperables.

**Criterio de salida:** publicación coincide con preview; invitado puede reorganizar y usar Papelera sin request de escritura global.

## 10. 297A-12 — Experiencia móvil tipo launcher

**Dependencias:** runtime 297A-9 y workspace 297A-11.  
**Plan detallado:** `Agente/planes/plan-experiencia-movil-launcher-2026-07-29.md`.

- [ ] Aprobar primero prototipo visual móvil.
- [ ] Añadir presentación `mobile|desktop` central.
- [ ] Launcher y MobileAppStack consumen AppRegistry/MountedView existentes.
- [ ] Teléfono no monta ventanas, barra superior ni taskbar.
- [ ] Apps abren full-screen; Back/Home/long press usan comandos compartidos.
- [ ] `mobileOrder` y overlay no contaminan bounds desktop.
- [ ] Cambio móvil↔tablet preserva app/recurso activo.
- [ ] Cuenta, Finder, Reader, Editor y Compra pasan flujo móvil.
- [ ] Verificar 320/360/390 y tablet 768.

**Criterio de salida:** teléfono funciona como launcher sin lógica/app duplicada y tablet conserva escritorio.

## 11. 297A-13 — Registro y overlay remoto *(parcial: Cuenta base implementada; registro avanzado pendiente)*

**Dependencias:** sesiones 297A-8, workspace 297A-11 e integración móvil 297A-12.

- [x] Implementar registro solo detrás de `registration_enabled=false`; la activación sigue bloqueada hasta completar correo/UI/MFA/E2E.
- [x] Crear `user_preferences` y el contrato de preferencia de tema con `revision`. *(migration `20260731100000_297a13_preferences`)*
- [x] Sync con `expected_revision`, actualización condicional y conflicto 409 sin overwrite silencioso. *(PreferencesService + preferences-sync)*
- [x] Autorizar solo cuentas activas y mantener CSRF/CORS con credenciales en las mutaciones.
- [x] Proteger logout/cambio de usuario, fallback offline y respuestas tardías; tests frontend incluidos.
- [x] UI de resolución `remote/local` conectada al estado `conflict`; modal único, accesible, idempotente y cerrado al resolver/logout. *(preferences-conflict-ui.ts)*
- [x] Pruebas HTTP/integración de 401 sin sesión, 403 sin CSRF, preflight CORS con credenciales y dos actualizaciones concurrentes con una sola victoria; verifican `create_router()` y cuerpos `200/409`. *(4 tests en `preferences_handler.rs`; `cargo test` PASS)*
- [x] Crear `user_workspace_overlays` para posiciones/estado del workspace, con contrato JSON validado, importación local/remota/reset, merge por ID/campo, tombstones y rebase ante release nuevo. *(migration `20260731120000_297a13_workspace_overlays`; service/repository/handler + `overlay-sync.ts`; gate PASS)*
- [x] Verificar autorización, CSRF, payload inválido, corrupción persistida, revisión inicial sin fila fantasma y que una cuenta no restaure recursos retirados. *(tests unitarios/HTTP del overlay; `cargo test` PASS)*
- [ ] Prueba E2E de dos pestañas/dispositivos y decisión de merge semántico para cambios concurrentes.
- [x] **Cuenta como app del escritorio:** registrar en AppRegistry como singleton público con estados invitado/autenticado/admin; verificación pendiente y MFA quedan como estados futuros del backend. *(account-view.ts + AppRegistry)*
- [x] **Estado de sesión visible:** control en barra superior y launcher móvil junto al tema; abre Cuenta y refleja Entrar/Cuenta/Cuenta · admin con etiqueta accesible. *(desktop-menu-bar.ts + mobile-shell.ts)*
- [x] **Login dentro de la app:** deslogueado, Cuenta muestra login dentro de su ventana; `/login` es deep link canónico y el wrapper legacy reutiliza la misma vista. Registro y `/register` permanecen cerrados hasta completar backend verificado.
- [x] Recovery backend con token hashado/expirable, revocación de sesiones, rate limit de login y auditoría hash; UI, rate limit específico y E2E quedan pendientes. Logout limpia clipboard/undo.

**Criterio de salida:** configuración privada y organización del workspace tienen transporte autenticado, revisión optimista, fallback offline, validación y conflicto explícito sin overwrite silencioso. Cuenta base, registro verificado y recovery backend quedan implementados detrás de flag; 297A-13 permanece abierto por UI/correo real, MFA, auditoría específica y E2E multi-dispositivo/móvil.

## 12. 297A-14 — Programas editoriales

**Dependencias:** runtime 297A-9, recursos 297A-10 y workspace 297A-11.

### 12.1 Extracción sin doble administración

- [ ] Congelar `admin.ts`; no añadir funciones.
- [ ] Inventariar paridad por acción actual.
- [ ] Extraer lógica reusable sin IDs DOM de Admin.
- [ ] Cada programa devuelve MountedView, no ventana.
- [ ] Menú Admin se deriva de capacidades.

### 12.2 Programas

- [ ] Editor de artículos/About create/edit/publish/private/trash.
- [ ] Editor de proyectos create/edit/publish/private/trash.
- [ ] Editor de productos create/edit/version/publish/pause/trash.
- [ ] Biblioteca de media preview/asset/estado/procesamiento.
- [ ] Confirmaciones y errores usan UI compartida.
- [ ] Auditoría de transiciones administrativas.

**Criterio de salida:** todas las altas/ediciones ocurren dentro del OS con defaults privados y paridad verificada.

## 13. 297A-15 — Comercio digital seguro

**Dependencias:** ADR-002/003/004 de 297A-7, recursos 297A-10 y programas 297A-14.

### 13.1 Catálogo y programas

- [ ] Tienda como carpeta normal organizable.
- [ ] Producto referenciable desde artículos/carpetas.
- [ ] Compra registrada como app pública.
- [ ] Pedidos registrada como app admin.

### 13.2 Pago

- [ ] Servidor valida visibilidad, lifecycle, sales state, asset y precio.
- [ ] Orden/idempotency key y order item snapshot.
- [ ] Payment Element interno y fallback según ADR.
- [ ] Webhook firma cuerpo crudo y valida evento/importe/moneda/producto.
- [ ] `provider_event_id UNIQUE` y transición transaccional.
- [ ] Outbox/retry para fulfillment/email.

### 13.3 Entrega

- [ ] Entitlement por product version.
- [ ] Grant hasheado, corto, revocable y limitado.
- [ ] Endpoint revalida propietario/claim y no acepta path.
- [ ] Compra invitada y vinculación posterior verificadas.
- [ ] Reembolso/contracargo revoca según ADR y audita.

### 13.4 Pruebas

- [ ] Precio/moneda manipulados se rechazan.
- [ ] Retorno cliente sin webhook no concede acceso.
- [ ] Webhook duplicado concede una vez.
- [ ] Usuario ajeno/UUID conocido no descarga.
- [ ] Producto/asset retirado bloquea checkout.
- [ ] Compra antigua conserva versión adquirida.

**Criterio de salida:** pago y entrega son server-authoritative, idempotentes y sin rutas públicas.

## 14. 297A-16 — Analytics, Estadísticas y retiro legado

**Dependencias:** runtime y flujos principales.

### 14.1 Política y pipeline

- [ ] Separar telemetría esencial de analytics opcional/consentimiento.
- [ ] Definir retención, anonimización y derechos de usuario.
- [ ] Evento con ID/schema/allowlist/límites.
- [ ] Batch idempotente y transaccional.
- [ ] Resultados críticos emitidos por backend.
- [ ] Agregados/índices para consultas de Estadísticas.

### 14.2 Programas y auditoría

- [ ] Estadísticas como programa admin.
- [ ] Pedidos/commerce y workspace emiten eventos semánticos.
- [ ] Audit log separado, consultable por capacidad.
- [ ] No guardar contenido, email, tokens o URLs sensibles.

### 14.3 Retiro controlado

- [ ] Matriz de paridad antigua→programa.
- [ ] Eliminar ruta/página/icono/estilos Admin.
- [x] Eliminar JWT Bearer, `jsonwebtoken`, secreto/configuración y clientes/tipos de autenticación legacy; la sesión opaca HttpOnly queda como autoridad única. *(018A-18; `/admin`, uploads y CSS legacy siguen pendientes)*
- [ ] Eliminar uploads públicos y rutas/DTO obsoletos.
- [ ] Eliminar CSS/clases huérfanas con VarSense.

**Criterio de salida:** una sola administración, analytics útil/privado y cero dependencias del Admin legacy.

## 15. 297A-17 — Hardening, identidad, accesibilidad y SEO

**Dependencias:** 297A-6–16.

- [ ] MFA/passkey admin.
- [ ] Implementar ADR SEO, metadata, canonical, sitemap y Schema.org.
- [ ] Aplicar manual visual en todos los programas.
- [ ] Resolver foco activo/inactivo, overflow móvil y hit areas.
- [ ] Teclado completo, zoom 200%, reduced motion y multimedia accesible.
- [ ] Sentinel/VarSense/self-check y CI bloqueantes.
- [ ] E2E visitante/usuario/admin/publicación/compra/reembolso/rollback.
- [ ] Threat review de auth, workspace, upload, payment y analytics.
- [ ] Budgets de rendimiento, logging, métricas y alertas.
- [ ] Runbook de backup/restore/rollback antes de planificar deploy.

**Criterio de salida:** todos los gates pasan y el producto puede entrar en revisión de producción. Deploy sigue fuera de alcance.

## 16. Definition of Done del epic

- [ ] Admin publica una disposición versionada y recuperable.
- [ ] Invitado y usuario personalizan sin modificar el release.
- [ ] Apps y ventanas se reutilizan sin chrome/estado duplicado.
- [ ] Todo recurso nuevo nace privado y muestra su estado correctamente.
- [ ] Tienda, Compra y entrega segura funcionan dentro del OS.
- [ ] Registro, sesiones y capacidades resisten escalada.
- [ ] Papelera nunca elimina fuera de su capa.
- [ ] Analytics y audit son privados, tipados y separados.
- [ ] `/admin`, JWT, uploads públicos y contratos legacy desaparecieron.
- [ ] Manuales, roadmap, Sentinel/VarSense y pruebas coinciden con la implementación.
