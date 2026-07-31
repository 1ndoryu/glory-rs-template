# wandori.us — Roadmap

> **Producto:** blog, portfolio y tienda digital dentro de un OS retro minimalista
> **Stack:** Rust/Axum + PostgreSQL + Vanilla TypeScript/Vite
> **Deploy:** no planificado
> **Epic:** 297A-4 — OS persistente, cuentas, programas y comercio
> **Visual:** identidad desktop y prototipo móvil aprobados; shell móvil, transición dinámica y primera interacción launcher implementados, validación visual/E2E pendiente

## Fuentes canónicas

- Índice: `Agente/documentacion/indice-documentacion-2026-07-29.md`
- Arquitectura: `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`
- Identidad: `Agente/documentacion/design-system/manual-identidad-visual-os-2026-07-29.md`
- Plan maestro: `Agente/planes/plan-escritorio-persistente-cuentas-admin-apps-2026-07-29.md`
- Plan móvil: `Agente/planes/plan-experiencia-movil-launcher-2026-07-29.md`
- Quality gate: `Agente/planes/completados/plan-escalabilidad-sentinel-wandorius-2026-07-29.md`
- Prevención: `Agente/prevencion/prevencion-wandorius-sentinel-varsense-2026-07-29.md`
- Tema claro/oscuro: `Agente/planes/plan-modo-oscuro-os-2026-07-31.md`
- Iconos libres del escritorio: `Agente/planes/completados/plan-iconos-libres-desktop-2026-07-31.md`
- Checkpoints SOLID/escalabilidad: `Agente/documentacion/arquitectura/checkpoints-solid-escalabilidad-2026-07-31.md`
- URLs canónicas y foco: `Agente/planes/plan-deep-links-ventanas-2026-07-31.md`

## Estado y reglas

- Concepto desktop aprobado; Finder es file browser real (lee workspaceStore); Reader sigue siendo preview.
- Workspace overlay implementado: release + overlay + merge + clipboard + papelera + crear carpetas.
- Split de archivos grandes completado: command-registration (725→6), workspace-store (430→4), desktop-shell (419→3).
- Sesiones opacas en cookie operativas; JWT localStorage eliminado del frontend. `/admin` legacy y uploads públicos siguen como deuda controlada.
- **Quality tool sprint:** 13 reglas custom (P0/P1/P2) + 7 Sentinel CLI + 4 VarSense = 24 reglas activas; la cobertura operativa estimada del quality tool es ~65% del inventario de patrones automatizables definido en el plan. VarSense reconoce contratos vanilla de clases con patch reproducible (`a93b8bf0…`, 43 tests del tool). Último gate 297A-19: PASS, VarSense 0 errores/2 avisos informativos, Sentinel 0 errores + 75 warnings heredados, custom 0 errores/3 informativos. Auditoría v4 reporta por separado 57/78 hallazgos arquitectónicos potencialmente detectables (73%) y 19/23 correcciones del checklist base (83%); no son denominadores comparables. ISP refactor DomAttrs (33→6 sub-interfaces). Frontend: 203 tests en 19 suites.
- Ejecutar una tarea por vez y en este orden; no saltar dependencias.
- El plan maestro contiene checklists/gates. El roadmap conserva solo pendientes.
- El quality gate está operativo; toda tarea futura debe cerrarse con `npm run task:check -- {ID}`.

## Siguiente bloque habilitado

**297A-20 — Iconos libres con snap-grid (completado).** Posición libre por celda con colisión resuelta, drop geométrico (ya no se pierde bajo ventanas), reflow por resolución y persistencia en overlay. Validado por el usuario en navegador; detalle en `Agente/completados/tareas-2026-07-31.md`.

> **297A-22 — Reordenamiento por arrastre con grid (PENDIENTE DE REVISIÓN).** El usuario detectó que "Mover arriba/abajo" (swap de `mobileOrder`) no debe ser el mecanismo de reorden: ni en escritorio (donde no mueve nada visible, el grid usa `position`) ni en móvil (debería ser por arrastre sobre celdas, como el escritorio). Plan de diseño listo para revisión en `Agente/planes/plan-reordenamiento-arrastre-grid-2026-07-31.md` — **no ejecutar hasta aprobación de las decisiones abiertas (sección 8 del plan).**

**297A-12 — Runtime móvil parcial implementado.** Shell/stack, transición dinámica, long press, menú contextual compartido, reorder accesible, frontera de capacidades y snapshot transitorio opt-in están validados por type-check, 203 tests en 19 suites y quality gate. Quedan pruebas visuales/E2E en navegador, safe areas, teclado virtual, foco y apps críticas.

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
- [x] Back/Home y carpetas consumen workspace/registry; Back/Home sincronizan URL; long press y reorder accesible consumen CommandRegistry y `mobileOrder`.
- [x] Transición dinámica móvil↔tablet sin recarga mediante reinstanciación segura.
- [x] Cambio móvil↔tablet conserva app/recurso por URL/params; el sincronizador pausa/reanuda durante la reinstanciación y evita entradas duplicadas. El snapshot transitorio opt-in conserva formularios/scroll seguros durante la reinstanciación; la validación E2E visual sigue pendiente.
- [ ] Pruebas visuales/E2E 320/360/390 y tablet 768; orientación, safe areas, teclado virtual, foco, scroll/formularios y apps críticas.

**Salida:** teléfono funciona como launcher sin duplicar lógica; tablet sigue como escritorio.

### 297A-18 — Tema claro/oscuro del sistema (implementado y aprobado visualmente; solo sync remota pendiente)

**Depende de:** 297A-9/12; la persistencia remota se completa con 297A-13.

- [x] Definir tokens semánticos para fondo, texto, bordes, estados, foco, selección, menús, ventanas y taskbar; ningún componente podrá fijar colores directamente. *(variables.css + migración de 8 CSS del OS)*
- [x] Añadir un único botón global `Claro/Oscuro` en el chrome del OS, con icono Lucide de 1px, etiqueta accesible y estado visible; Configuración solo reutiliza ese comando. *(comando `theme:toggle` + botón compartido en barra superior y launcher móvil, junto a la hora)*
- [x] Usar `data-theme`/atributo equivalente en el shell para que desktop, tablet y launcher móvil compartan la misma implementación; multimedia puede conservar color, el chrome sigue monocromo. *(data-tema en documentElement + override scoped para superficies del OS)*
- [x] Resolver preferencia inicial por sistema operativo y permitir override explícito. *(matchMedia + localStorage `wandorius:tema`)*
- [x] Evitar flash de tema en la primera pintura y emitir un evento `theme_changed` medible con modo. *(script inline en index.html + ThemeEvent en dispatcher)*
- [ ] Guardar anónimo en overlay local y sincronizar la preferencia de cuenta sin sobrescribir decisiones locales silenciosamente; logout/login y conflictos de preferencia. *(bloqueado por 297A-13 overlay remoto)*
- [x] Validar contraste AA, foco/teclado, reduced motion, zoom 200% y viewports (1440×900, 1024×768, 390×844, 320px); capturas aprobadas por el usuario. *(aprobación visual 2026-07-31; E2E formal y medición de rendimiento quedan con 297A-17)*

**Salida:** el usuario cambia claro/oscuro desde un control único y la preferencia local sobrevive; tema aprobado visualmente; falta solo la sync remota (297A-13).

### 297A-19 — URLs canónicas, deep links y ventana enfocada

**Depende de:** 297A-9/11/12; integra capacidades de 297A-13. Cada app y recurso tendrá una URL compartible; la URL representa solo la ventana enfocada.

- [x] Definir contrato allowlisted para rutas públicas y parámetros; excluir IDs internos, tokens, posiciones, tamaños, z-index, clipboard y overlays privados. *(AppDeepLink + createPathDeepLink)*
- [x] Migrar parser/serializer y fallback seguro de Reader, Finder/Galería, About y Projects; apps legacy sin contrato no aceptan parámetros dinámicos.
- [x] Conectar `replacePath` a todos los cambios de foco y reservar `pushPath` para aperturas explícitas; `window-url-sync` deriva de los stores sin router paralelo.
- [ ] Mantener Back/Forward, refresh y transición desktop/tablet/móvil para el foco completo; la reconciliación de rutas documentales, parámetros inseguros, capacidades y semántica `push/replace` ya está implementada y probada, pero falta E2E real de `goBack()`/`popstate`/interacción móvil.
- [ ] Añadir `Copiar URL` con feedback, protección de drafts/privados/grants y redirects canónicos; el fallback seguro y boundary allowlisted ya están implementados.
- [ ] Medir `deep_link_opened`, `window_focus_changed` y `share_url_copied`; probar sesión limpia, varias ventanas, permisos, rutas inválidas y viewports. El boundary y las rutas inválidas ya tienen cobertura unitaria.

**Salida:** copiar una URL desde cualquier app abre o enfoca esa app/recurso en otra sesión, con historial, seguridad, analítica y presentación móvil coherentes.

### 297A-20 — Iconos de escritorio con posición libre (snap-grid) (completado)

**Depende de:** 297A-11 (overlay). El usuario coloca iconos en cualquier celda del escritorio; la disposición del admin se publica y cada visitante personaliza sin afectar a otros. Plan: `plan-iconos-libres-desktop-2026-07-31.md`.

- [x] Renderizar iconos por `position {col,row}` (snap-grid 88px) con fallback al orden actual cuando no hay posición. *(grid geométrico RTL: `getGridMetrics`/`getCellAt`; `grid-auto-rows` fijo a `--sistema-icono-fila` 64px para que geometría y CSS coincidan)*
- [x] Conectar el drag existente para soltar en celda libre llamando a `moveNodePosition()`; el click sigue abriendo la app y el drag a carpeta/papelera se conserva. *(drop geométrico por celda, sin depender de `elementFromPoint`; funciona con ventanas abiertas encima)*
- [x] Resolver colisiones (desplazar ocupado a celda libre) y reencuadre al cambiar resolución/breakpoint; móvil conserva el orden del launcher (`mobileOrder` no se contamina). *(param `avoid` en `findFreeCell`/`planPlacement`; `reflowPositions` con clamping; móvil ignora posiciones)*
- [x] Persistir posición en el overlay personal (`fieldOverrides` + localStorage) y permitir que el admin la publique al release.
- [x] Tests (merge/colisión/snap) y validación visual en navegador (desktop y tablet). *(37/37 tests; verificado por el usuario: «funciona bien, iconos no se juntan»)*

**Salida:** el escritorio se puede ordenar libremente; la vista pública carga la disposición del admin y cada visitante tiene su propio estado personalizado.

**Pendiente controlado:** modo depuración temporal (Ctrl+Shift+G, cuadrícula roja) que el usuario pidió mantener — eliminarlo cuando lo indique.

### 297A-22 — Reordenamiento por arrastre con grid (móvil + escritorio) [PENDIENTE DE REVISIÓN]

**Depende de:** 297A-20 (snap-grid desktop) y 297A-12 (launcher móvil). Plan en `Agente/planes/plan-reordenamiento-arrastre-grid-2026-07-31.md`. **No ejecutar hasta revisión y aprobación de las decisiones abiertas (sección 8).**

- [ ] Revisar y aprobar: opción de modelo (recomendada: `mobilePosition` con paridad), grid móvil apretado vs con huecos, destino de `workspace:move-up/down`, gesto long press + drag, vida de `mobileOrder`. *(pendiente del usuario)*
- [ ] Modelo de datos: `mobilePosition {col,row}` en tipos/fieldOverrides; `mobileOrder` deprecado a fallback; actualizar `merge.ts`, `overlay-mutations.ts`, `getChildren` y `default-release.ts`.
- [ ] Launcher móvil como snap-grid: geometría reutilizada de `icon-grid.ts` parametrizada por columnas fijas (3/2); render con `mobilePosition` + fallback `mobileOrder`.
- [ ] Drag táctil en launcher: long press → modo edición; movimiento >umbral → drag; soltar en celda → `planPlacement` → persistir `mobilePosition`.
- [ ] Escritorio: quitar del menú contextual el swap sin efecto visible; drag desktop conserva `position` (297A-20 sin regresión).
- [ ] Alternativa accesible: reemplazar `workspace:move-up/down` por comandos sobre celdas (cumple 297A-12 §9).
- [ ] Migración de datos y limpieza del swap; tests unitarios + validación visual (320/360/390/768+, drag táctil, foco, teclado, reload/sync).

**Salida:** reordenar iconos es por arrastre sobre celdas en móvil y escritorio; "Mover arriba/abajo" deja de ser el mecanismo; organización móvil persiste en overlay sin contaminar el desktop.

### 297A-21 — Notificaciones de novedades (campana + gestión admin)

**Depende de:** 297A-20, 297A-13 (entrega remota) y menú Admin de 297A-14. Idea nueva: campana junto al tema que avisa de contenido nuevo incluso a usuarios con estado personalizado (el overlay por diff ya les muestra lo nuevo; la campana solo añade el aviso). Plan propio al arrancar.

- [ ] Definir qué genera una notificación (release nuevo, recursos agregados/actualizados), cuándo se marca leída y política anti-spam sin envío inmediato.
- [ ] Campana en la barra superior (junto al tema) con contador, icono Lucide 1px, accesibilidad y estado local/remoto.
- [ ] Entrega de novedades por overlay (297A-13) y panel Admin para publicar/descartar novedades, integrado en el menú Admin por capacidades (297A-14).
- [ ] Pruebas: usuario con overlay personalizado recibe aviso de novedades y las ve; casos negativos (sin spam, leídas, logout/login).

**Salida:** los usuarios saben que hay novedades aunque su escritorio esté personalizado; el admin gestiona desde un panel, sin notificaciones inmediatas.

### 297A-13 — Registro y overlay remoto

**Depende de:** 297A-8/11; integra móvil 297A-12. Plan maestro §6.2 y manual de identidad §10/13.

- [ ] Cuenta como app del escritorio: registrar en AppRegistry con estados invitado/autenticado/verificación pendiente/MFA; deep links `/login` y `/register` abren la app Cuenta.
- [ ] Icono de estado de sesión en la barra superior (junto al tema) que abre la app Cuenta y refleja login/logout; el login/registro se hace dentro de la misma app cuando está deslogueado (sin modal de página completa).
- [ ] Habilitar registro verificado.
- [ ] Overlay remoto y preferencias.
- [ ] Importar local/usar remoto/reset explícito.
- [ ] Merge por ID/campo y 409 visible.
- [ ] Pruebas dos dispositivos/release nuevo.

**Salida:** la cuenta es un programa del OS con estado visible desde la barra superior; el usuario se registra/inicia sesión dentro de la app Cuenta y su configuración sincroniza sin sobrescribir ni restaurar recursos retirados.

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

### 297A-19 — URLs canónicas, deep links y ventana enfocada

- [ ] Definir gramática versionada y mapping `app/resource/instance` con singleton o multiinstancia explícitos; parámetros son input no confiable.
- [ ] Resolver URL en `RouteAppAdapter` con capacidades server-side, 403/404 seguro, sin enumerar privados ni incluir tokens, grants o rutas internas.
- [ ] Sincronizar foco/z-order con History API: `replaceState` para foco, `pushState` para navegación, Back/Home según presentación y sin listeners stale.
- [ ] Probar URL desde sesión limpia, refresh, dos ventanas, recurso privado, app inexistente, cambio de breakpoint, scroll/formulario y deduplicación de instancia.
- [ ] Emitir un único evento de navegación/deep link y verificar SEO solo para recursos públicos activos.

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

## Revisión SOLID y escalabilidad por fase

Cada fase termina con esta revisión antes de marcar su salida. La revisión debe quedar evidenciada en el plan de la fase, en el quality gate y en el commit; no se acepta “lo refactorizamos después”.

### Checklist común de cierre

- [ ] **SRP/ISP:** cada módulo tiene una responsabilidad clara, las interfaces exponen solo capacidades necesarias y los componentes no mezclan chrome, contenido, persistencia y analítica.
- [ ] **OCP/DIP:** nuevas apps, recursos, comandos y temas se agregan mediante registros/adaptadores; no se crean `if/else` globales ni copias por plataforma.
- [ ] **Límites:** componentes/CSS ≤300 líneas, lifecycle/store/hook ≤120 y utils ≤150; dividir antes de superar el límite y justificar cualquier excepción.
- [ ] **Contratos:** tipos, DTOs, errores, permisos y eventos son explícitos; no hay estado duplicado, listeners sin teardown, I/O silencioso ni roundtrips N+1.
- [ ] **Escalabilidad:** se prueba un segundo caso real (otra app, recurso, usuario, tema o viewport), se revisan índices/paginación/cache y se documenta el impacto de migración y rollback.
- [ ] **Calidad:** Sentinel/VarSense, type-check, tests y prueba funcional/visual pasan; cualquier falso positivo queda documentado en `Agente/prevencion/`.

### Foco obligatorio por fase

- [ ] **297A-9 Runtime:** registry/adapters y WindowManager no conocen apps concretas; agregar una app no modifica el shell ni duplica listeners.
- [ ] **297A-10 Recursos:** services/repositories/DTOs permanecen separados; un nuevo `resourceKind` no altera recursos existentes ni filtra metadata privada.
- [ ] **297A-11 Workspace:** referencias, overlay, clipboard y papelera son composables; mover/copiar un tipo nuevo conserva atomicidad, permisos y undo.
- [ ] **297A-12 Móvil:** launcher y desktop consumen las mismas apps/comandos; un nuevo breakpoint no crea una app paralela ni pierde estado de ruta.
- [ ] **297A-18 Tema:** el botón solo despacha un comando y los tokens viven en el sistema visual; agregar un tercer tema de prueba no requiere reescribir componentes.
- [ ] **297A-19 Deep links:** parser/serializer y foco viven en adaptadores; añadir una app o recurso no modifica el router global ni serializa estado privado.
- [ ] **297A-13 Cuentas:** merge, sesión y preferencias se resuelven por servicios/adaptadores; otro proveedor de identidad no duplica el flujo ni restaura tombstones.
- [ ] **297A-14 Editorial:** editores comparten primitives y capacidades; añadir un tipo de documento no amplía el monolito Admin ni copia ventanas.
- [ ] **297A-15 Comercio:** pago, webhook, entitlement y grants son servicios independientes; otro proveedor o versión no cambia la autoridad server-side.
- [ ] **297A-16 Analytics:** catálogo, dispatcher y agregados son extensibles; añadir un evento no expone datos ni obliga a reescribir paneles existentes.
- [ ] **297A-17 Hardening:** las reglas se ejecutan igual en local/CI y el runbook cubre rollback; ninguna excepción de Sentinel/VarSense oculta deuda estructural.
