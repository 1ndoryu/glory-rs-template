# Plan maestro ejecutable: OS persistente de wandori.us

> **Epic:** 297A-4  
> **Fecha:** 2026-07-29  
> **Prioridad:** máxima  
> **Estado:** pendiente; identidad visual aprobada  
> **Siguiente bloque:** 297A-6 — quality gate Sentinel/VarSense (pendiente de autorización)

## 1. Autoridad y alcance

Este es el único plan que define el orden de implementación del producto. No repite especificaciones:

- Arquitectura: `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`
- Identidad visual: `Agente/documentacion/design-system/manual-identidad-visual-os-2026-07-29.md`
- Quality gate: `Agente/planes/plan-escalabilidad-sentinel-wandorius-2026-07-29.md`
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
| 1 | 297A-6 | Sentinel/VarSense + script quality gate | revisión |
| 2 | 297A-7 | ADRs + seguridad inmediata | bloqueado |
| 3 | 297A-8 | sesiones + Cuenta base | bloqueado |
| 4 | 297A-9 | runtime desktop/tablet | bloqueado |
| 5 | 297A-10 | recursos + migraciones | bloqueado |
| 6 | 297A-11 | workspace + overlay invitado | bloqueado |
| 7 | 297A-12 | launcher móvil | bloqueado |
| 8 | 297A-13 | cuenta + overlay remoto | bloqueado |
| 9 | 297A-14 | programas editoriales | bloqueado |
| 10 | 297A-15 | comercio seguro | bloqueado |
| 11 | 297A-16 | estadísticas + retiro legado | bloqueado |
| 12 | 297A-17 | hardening + SEO | bloqueado |

## 4. 297A-6 — Quality gate Sentinel/VarSense

**Estado:** planificado; no implementar hasta autorización explícita del usuario.  
**Bloquea:** todas las tareas posteriores.

### 4.1 Contrato y preflight

- [ ] Revisar/aprobar `plan-escalabilidad-sentinel-wandorius-2026-07-29.md`.
- [ ] Confirmar binarios/CLI/config reales de Sentinel y VarSense.
- [ ] Definir severidades, excludes y política de baseline.
- [ ] Definir alcance automático: incremental local y full en CI/config/migraciones.
- [ ] Definir límites de tiempo y códigos de salida.

### 4.2 Script unificado

- [ ] Crear orquestador Node multiplataforma, sin reglas duplicadas.
- [ ] Exponer un único comando público `npm run task:check -- {ID}`.
- [ ] Preflight no instala ni muta; falla con diagnóstico accionable.
- [ ] Ejecutar Sentinel y VarSense con sus configs canónicas.
- [ ] Ejecutar validaciones del stack afectado.
- [ ] Generar reporte Markdown + JSON con task ID, commit/config y tiempos.
- [ ] Redactar secretos y limitar output/logs.
- [ ] Mostrar máximo tres hallazgos y cinco recordatorios contextuales.
- [ ] Imprimir siempre la acción y el comando siguiente exactos.
- [ ] Exit no cero ante error de herramienta, regla bloqueante o test fallido.

### 4.3 Rollout

- [ ] Configurar seguridad y fallos silenciosos primero.
- [ ] Corregir baseline sin suppressions amplias.
- [ ] Añadir lifecycle/desktop.
- [ ] Añadir API/backend/analytics.
- [ ] Añadir identidad/VarSense.
- [ ] Integrar script en self-check y CI.
- [ ] Probar una fixture regresiva por categoría.

### 4.4 Salida

- [ ] CLI/LSP/editor coinciden en fixtures.
- [ ] Baseline tiene cero errores.
- [ ] Reporte identifica claramente tarea y causa de fallo.
- [ ] Una regresión de seguridad, arquitectura o CSS bloquea CI.

**Criterio de salida:** cada bloque futuro tiene supervisión automática reproducible y 297A-7 queda habilitada.

## 5. 297A-7 — ADRs y gate de seguridad inmediata

**Dependencias:** documentación canónica.  
**No habilitar:** registro, Compra o publicación de workspace.

### 5.1 Decisiones bloqueantes

- [ ] ADR-001: elegir estrategia de HTML indexable/SEO y mejora progresiva.
- [ ] ADR-002: elegir storage privado, derivados, backups y serving autorizado.
- [ ] ADR-003: elegir Payment Element y fallback de redirección/retorno.
- [ ] ADR-004: definir retención/purga de recursos, órdenes y assets comprados.
- [ ] Registrar consecuencias, rollback y tareas afectadas en cada ADR.

### 5.2 Autorización

- [ ] Migración `users.role/status` con defaults y checks.
- [ ] Definir capacidades centralizadas server-side.
- [ ] Registro request no acepta rol.
- [ ] Bootstrap/promoción admin solo server-side y auditado.
- [ ] Crear extractores `AuthenticatedUser` y `Admin/Capability`.
- [ ] Aplicar capacidad a toda mutación y lectura administrativa.
- [ ] Separar namespaces `/public`, `/me`, `/admin`, `/webhooks` o adaptador equivalente documentado.

### 5.3 Exposición pública

- [ ] Artículos públicos exigen predicado canónico de exposición.
- [ ] Slug/ID públicos no devuelven borrador/privado/papelera.
- [ ] Media pública solo incluye previews/assets autorizados.
- [ ] Retirar serving estático de entregables.
- [ ] DTO público no serializa rutas, storage keys o IDs internos.
- [ ] CORS usa allowlist de orígenes y métodos.

### 5.4 Contención inmediata

- [ ] Eliminar credenciales y auto-login/auto-registro del frontend.
- [ ] Mantener registro público apagado por feature flag server-side.
- [ ] Eliminar/deshabilitar entrega demo sin proveedor.
- [ ] Checkout legacy no acepta producto inactivo/no público.
- [ ] Toda falla crítica deja logging y respuesta no exitosa.

### 5.5 Pruebas y salida

- [ ] Usuario normal recibe 403 en cada endpoint admin.
- [ ] Invitado no obtiene recursos privados ni metadata sensible.
- [ ] Conocer URL/UUID no permite descargar.
- [ ] Registro no puede promover rol.
- [ ] Test demuestra que comercio no entrega sin webhook válido.

**Criterio de salida:** cero escalada autenticado→admin, cero borrador/asset privado público y cuatro ADRs cerrados.

## 6. 297A-8 — Sesiones seguras y Cuenta base

**Dependencia:** 297A-7.

### 6.1 Persistencia de identidad

- [ ] Crear `auth_sessions` con token hasheado, expiración, revocación y rotación.
- [ ] Cookie `HttpOnly`, `Secure`, `SameSite=Lax`.
- [ ] Eliminar JWT bearer/localStorage del contrato objetivo.
- [ ] Login con respuesta no enumerable y rate limit.
- [ ] CSRF/origin para mutaciones autenticadas.
- [ ] Logout actual y revocación de otras sesiones.

### 6.2 Cuenta como programa

- [ ] Registrar Cuenta en AppRegistry provisional/final según fase runtime.
- [ ] Estados invitado, autenticado, verificación pendiente y admin MFA pendiente.
- [ ] Login/logout/me con feedback y abort.
- [ ] Lista/revocación de sesiones activas.
- [ ] Deep links `/login` y `/register` abren Cuenta.

### 6.3 Preparación de registro

- [ ] Verificación de email.
- [ ] Recovery con token corto de un solo uso.
- [ ] Rate limits y auditoría de intentos sensibles.
- [ ] Pruebas de fijación, expiración y revocación.
- [ ] Registro permanece apagado hasta completar todo el checklist.

**Criterio de salida:** admin opera Cuenta sin token en Web Storage; sesiones pueden revocarse y errores no se silencian.

## 7. 297A-9 — Foundation del runtime desktop/tablet

**Dependencias:** quality gate 297A-6 y contratos de 297A-7; coordinar auth con 297A-8.

### 7.1 API y errores

- [ ] Orval Fetch + `tags-split`.
- [ ] OpenAPI cubre endpoints consumidos.
- [ ] Retirar tipos/cliente manual duplicado cuando exista paridad.
- [ ] Contrato `Result` y toast/feedback visible.
- [ ] Sanitizador central para contenido editorial.

### 7.2 Lifecycle y navegación

- [ ] Implementar `MountedView`/`RenderContext`.
- [ ] Router aborta/destruye vista anterior.
- [ ] Apps y loaders usan signal.
- [ ] Implementar `RouteAppAdapter` con deep links.
- [ ] Pruebas de listener duplicado y respuesta stale.

### 7.3 Runtime de escritorio

- [ ] AppRegistry con `requiredCapabilities`.
- [ ] WindowManager/reducer con IDs y una ventana activa.
- [ ] DesktopWindow envuelve contenido; app no crea chrome.
- [ ] Taskbar deriva del estado.
- [ ] Abrir, foco, minimizar, restaurar y cerrar.
- [ ] Drag y resize por bordes con bounds/clamp.
- [ ] CommandRegistry para barra y clic derecho.
- [ ] Geometría/estado versionados.

### 7.4 Analytics base

- [ ] Catálogo de eventos y dispatcher tipados.
- [ ] No emitir por cada pointermove.
- [ ] Cola limitada, keepalive y error observable.
- [ ] Eventos críticos reservados al backend.

### 7.5 Supervisión del bloque

- [ ] Ejecutar plan Sentinel/VarSense hasta el gate de esta fase.
- [ ] Corregir límites de Admin/Settings al extraer responsabilidades.
- [ ] Eliminar z-index por app y listas estáticas.
- [ ] Pruebas reducer, geometry, registry, lifecycle y DOM integrada.
- [ ] Verificar 1440×900, 1024×768, 390×844 y 320 px.

**Criterio de salida:** Perfil, Finder y Reader operan con runtime compartido, cleanup correcto y taskbar real.

## 8. 297A-10 — Catálogo de recursos y migraciones

**Dependencias:** seguridad 297A-7 y contratos 297A-9.

### 8.1 Modelo expandido

- [ ] Crear `resources` como sobre común, sin cuerpos/precios/binarios.
- [ ] Estados independientes con constraints/defaults.
- [ ] About como artículo con alias estable.
- [ ] Producto independiente de artículo.
- [ ] Assets con estado processing/clean/rejected y visibilidad.
- [ ] Product versions inmutables para entregables.

### 8.2 Migración

- [ ] Inventariar `status`, `is_visible`, `is_active`, `download_path` y paths actuales.
- [ ] Expandir esquema sin romper lecturas legacy.
- [ ] Backfill determinista y reporte de filas ambiguas.
- [ ] Cambiar services/repositorios a modelo nuevo.
- [ ] Validar conteos, constraints y rollback.
- [ ] Contraer columnas legacy solo después de paridad.

### 8.3 API y pruebas

- [ ] DTO público/admin separado.
- [ ] Resolver público usa nodo + recurso + capacidad.
- [ ] Mover referencia no altera recurso.
- [ ] Referencias múltiples no duplican contenido.
- [ ] Tests default privado y transiciones inválidas.

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

## 11. 297A-13 — Registro y overlay remoto

**Dependencias:** sesiones 297A-8, workspace 297A-11 e integración móvil 297A-12.

- [ ] Habilitar registro solo al pasar gate completo.
- [ ] Crear `user_workspace_overlays` y `user_preferences`.
- [ ] Sync con expected revision.
- [ ] Importar local, usar remoto o reset mediante decisión explícita.
- [ ] Merge por ID/campo; permisos/recursos retirados ganan.
- [ ] Conflicto mismo campo devuelve 409 y UI resoluble.
- [ ] Prueba dos pestañas, dos dispositivos y release nuevo.
- [ ] Prueba cuenta no restaura recurso retirado.

**Criterio de salida:** configuración privada continúa entre dispositivos sin overwrite silencioso.

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
- [ ] Eliminar JWT y clientes/tipos legacy.
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
