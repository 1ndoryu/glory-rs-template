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
- Cómo agregar una app (receta): `Agente/documentacion/arquitectura/guia-agregar-app-2026-07-31.md`
- Plan de programas editoriales: `Agente/planes/plan-programas-editoriales-2026-07-31.md`

## Estado y reglas

- Concepto desktop aprobado; Finder es file browser real (lee workspaceStore); Reader sigue siendo preview.
- Workspace overlay implementado: release + overlay + merge + clipboard + papelera + crear carpetas.
- Split de archivos grandes completado: command-registration (725→6), workspace-store (430→4), desktop-shell (419→3), mobile-shell (antes >300; launcher extraído a `mobile-launcher.ts`) y app-registration (276 + catálogo admin separado en `app-registration-admin.ts`).
- Sesiones opacas en cookie operativas; JWT Bearer, `localStorage` y el secreto JWT fueron retirados del backend/frontend. La ruta legacy `/admin` fue retirada en 018A-26, el serving estático de uploads en 018A-28 y los DTOs de media se separaron en 018A-29; CSS/contratos legacy restantes siguen como deuda controlada.
- **Quality tool sprint:** 13 reglas custom (P0/P1/P2) + 7 Sentinel CLI + 4 VarSense = 24 reglas activas; la cobertura operativa estimada del quality tool es ~65% del inventario de patrones automatizables definido en el plan. VarSense reconoce contratos vanilla de clases con patch reproducible (`a93b8bf0…`, 43 tests del tool). El mínimo desbloqueante está cerrado: `quality:test` 31/31, `task:check` y `self-check` reproducibles, alcance incremental, caché separada por modo, suite/build/budgets completos en CI, salida acotada y recordatorio condicional de commit. El backlog de reglas/benchmarks/paridad avanzada queda diferido en `roadmap-sentinel.md`; no bloquea el siguiente bloque técnico. El contrato OpenAPI ya no ofrece Bearer/JWT: documenta la cookie `session_id` HttpOnly; la cobertura de endpoints y el retiro del cliente manual quedan diferidos.
- Ejecutar una tarea por vez y en este orden; no saltar dependencias.
- El plan maestro contiene checklists/gates. El roadmap conserva solo pendientes.
- El quality gate está operativo; toda tarea futura debe cerrarse con `npm run task:check -- {ID}`.

## Siguiente bloque habilitado

**Siguiente bloque habilitado: 297A-17 — hardening, identidad, accesibilidad y SEO.** La paridad de contratos automatizable quedó cerrada en 018A-48/49; la limpieza residual de CSS/clases y fachadas manuales queda diferida porque requiere revisión visual o migración amplia. Las validaciones visuales/E2E de móvil, deep links, notificaciones y editores permanecen diferidas para una pasada dedicada de navegador.

> **297A-22 — Reordenamiento por arrastre con grid (implementación técnica completada).** Se adoptó `mobilePosition {col,row}` con grid compacto de 2/3 columnas; `mobileOrder` queda como fallback legacy. El drag requiere long press, Finder no hereda el orden móvil y los comandos move prev/next son alternativa accesible solo en presentación móvil. Typecheck, suite y gate pasan; queda validación visual/E2E antes del cierre documental.

**297A-12 — Runtime móvil parcial implementado.** Shell/stack, `MobileLauncher`, transición dinámica, long press, menú contextual compartido, reorder accesible, frontera de capacidades y snapshot transitorio opt-in están validados por type-check, **278 tests en 34 suites**, quality gate y self-check. La inspección de navegador confirmó tablet `768×1024`; quedan E2E táctil móvil estable, viewports 320/360/390, safe areas, teclado virtual, foco y apps críticas.

**297A-10 — Recursos y migraciones (completado).** Resource envelope, product versions, asset states, services con transacción, DTO público/admin y About seeder.

**Plan transversal 297A-4 — parcialmente cerrado.** CommandRegistry enriquecido (§2), selección+foco (§3), context menu (§2.3), keyboard move/resize (§4.1), resource-type-registry (§7), analytics envelope (§9.1/9.2), persistencia local/remota del workspace (§6, 297A-13) y tests unitarios iniciales (§11). Pendiente: clipboard/undo (§5, 297A-11), E2E multi-dispositivo, mobile formal (§2.3/3, 297A-12) y cobertura ampliada.

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
- [x] **018A-17 —** Orval Fetch `tags-split`: `npm run codegen:local` exporta OpenAPI sin BD/servidor y regenera clientes por etiqueta; el esquema de seguridad usa `session_cookie`; cobertura completa de endpoints y retiro del cliente manual siguen pendientes.
- [x] **018A-19 —** Swagger/utoipa documenta la sesión opaca `session_id` como cookie y elimina referencias de Bearer/JWT del contrato; cobertura completa y retiro del cliente manual siguen diferidos.
- [x] **018A-20 —** Paridad OpenAPI del dominio de artículos: rutas admin corregidas (`/api/admin/articles`) y clientes `articles/` regenerados; los dominios restantes siguen pendientes.
- [x] **018A-21 —** Paridad OpenAPI del dominio de proyectos: catálogo público y CRUD admin anotados; cliente `projects-handler/` regenerado. Productos, media y demás dominios siguen pendientes.
- [x] **018A-22 —** Paridad OpenAPI del dominio de productos: catálogo, CRUD admin y checkout tipado; cliente `products-handler/` regenerado. Media y demás dominios siguen pendientes.
- [x] **018A-23 —** Paridad OpenAPI de notificaciones, analytics y settings; clientes `notifications/` y `settings-handler/` regenerados. Media, workspace y auth restante siguen pendientes.
- [x] **018A-24 —** Paridad OpenAPI de sesión/cuenta y workspace: `me/logout/sessions`, release activo e historial/publicación admin con rutas `/api` correctas; clientes regenerados. Media/multipart y auth avanzada siguen pendientes.
- [x] **018A-25 —** Paridad OpenAPI de media: galería pública, biblioteca admin, papelera, restore y respuesta de upload; cliente `media-handler/` regenerado. El request multipart sigue manual por seguridad.
- [x] **018A-27 —** Cobertura OpenAPI de descargas privadas y webhook Stripe: grant opaco, firma en header, JSON crudo y errores documentados sin exponer storage ni secretos.
- [x] **018A-28 —** Retirado el serving estático público de `/uploads`; las respuestas de media exponen solo previews autorizados y existen rutas públicas/admin con validación de envelope, estado y confinamiento de path.
- [x] **018A-29 —** Separados los DTOs públicos, administrativos y de subida de media: las respuestas exponen `url`/`admin_url` y `file_name`, nunca `file_path` ni storage keys; servicios frontend y OpenAPI consumen el contrato explícito.
- [x] **018A-30 —** Alineadas las referencias activas del roadmap y planes: el shape de media usa DTOs con URLs explícitas y las validaciones visuales/E2E quedan diferidas sin reabrir bloques técnicos cerrados.
- [x] **018A-31 —** Retirados estilos CSS huérfanos confirmados (`arrow-select` y `.font-panel`); se conservan clases dinámicas usadas por el runtime y sus avisos quedan documentados como falsos positivos de análisis estático.
- [x] **018A-32 —** Orval usa un mutator compartido con cookie/CSRF/base URL y `MediaService` migra al cliente generado; CI regenera el contrato antes del gate. Los demás servicios manuales quedan para migraciones por dominio.
- [x] **018A-33 —** `ArticleService`, `ProjectService` y `ProductService` migran CRUD, listados y checkout al cliente Orval; se conserva la adaptación explícita de `url` y el envelope compartido. Auth, workspace, settings y analytics quedan para bloques posteriores.
- [x] **018A-34 —** Auth, preferencias, notificaciones, settings y analytics usan funciones Orval con estados explícitos; se conserva la sincronización de stores y el header de consentimiento. Workspace queda aislado para su propia adaptación de overlay/release.
- [x] **018A-35 —** `WorkspaceService` migra releases, publicación y overlay al cliente generado; las conversiones de `WorkspaceTree`/`WorkspaceOverlay` quedan confinadas al boundary y el cliente manual deja de tener consumidores frontend.
- [x] **018A-36 —** Se retira el objeto `api` manual del frontend; `client.ts` conserva solo `ApiError`, el mutator Orval y `unwrapGeneratedResponse`. La matriz de transporte queda cerrada; lo pendiente es cobertura visual/E2E y dominios externos.
- [x] **018A-61 —** El toggle de navegación, maximizar/restaurar y reencuadrar ventanas usan comandos del `CommandRegistry`; taskbar, móvil y titlebar ya no mutan esos estados directamente. El reencuadre batch conserva límites/maximización y emite medición semántica; validación visual/E2E permanece en 297A-17.

**Salida:** runtime compartido funciona sin chrome/listas/listeners duplicados. Orval se regenera localmente sin backend vivo; quedan cobertura total del contrato y retiro del cliente manual.

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
- [x] Split de archivos grandes: command-registration (725→6), workspace-store (430→4), desktop-shell (419→3), mobile-shell con `mobile-launcher.ts`, y modelos Rust de workspace/overlay por dominio (`workspace/`, `workspace_overlay/`).
- [x] Migración workspace_releases con seed data. *(20260731000000)*
- [x] Draft/release/preview/publicar/rollback. *(diff.ts + publish con confirmación + rollbackWorkspace)*
- [x] Organizador público separado del workspace admin personal. *(previewPublicStore + workspace:preview-public)*

**Salida:** admin publica el preview exacto; visitante reorganiza sin escribir global.

### 297A-12 — Experiencia móvil tipo launcher

**Depende de:** 297A-9/11. Tablet conserva desktop.

- [x] Prototipo visual móvil aprobado por el usuario (2026-07-30).
- [x] Launcher + MobileAppStack con las mismas apps.
- [x] Shell móvil full-screen, sin ventanas/barra superior/taskbar; validación visual por viewport pendiente.
- [x] Back/Home y carpetas consumen workspace/registry; Back/Home sincronizan URL; long press y reorder accesible consumen CommandRegistry y `mobilePosition` (`mobileOrder` solo fallback legacy).
- [x] Transición dinámica móvil↔tablet sin recarga mediante reinstanciación segura.
- [x] Cambio móvil↔tablet conserva app/recurso por URL/params; el sincronizador pausa/reanuda durante la reinstanciación y evita entradas duplicadas. El snapshot transitorio opt-in conserva formularios/scroll seguros durante la reinstanciación; la validación E2E visual sigue pendiente.
- [x] Refresh móvil reconstruye el stack seguro antes del router y conserva la app superior sin duplicarla; verificado en 390×844 con Galería.
- [ ] Pruebas visuales/E2E 320/360/390 y tablet 768; tablet `768×1024` ya fue inspeccionada sin overflow; quedan drag táctil estable, orientación, safe areas, teclado virtual, foco, scroll/formularios y apps críticas.

**Salida:** teléfono funciona como launcher sin duplicar lógica; tablet sigue como escritorio.

### 297A-18 — Tema claro/oscuro del sistema (implementado y aprobado visualmente; solo sync remota pendiente)

**Depende de:** 297A-9/12; la persistencia remota se completa con 297A-13.

- [x] Definir tokens semánticos para fondo, texto, bordes, estados, foco, selección, menús, ventanas y taskbar; ningún componente podrá fijar colores directamente. *(variables.css + migración de 8 CSS del OS)*
- [x] Añadir un único botón global `Claro/Oscuro` en el chrome del OS, con icono Lucide de 1px, etiqueta accesible y estado visible; Configuración solo reutiliza ese comando. *(comando `theme:toggle` + botón compartido en barra superior y launcher móvil, junto a la hora)*
- [x] Usar `data-theme`/atributo equivalente en el shell para que desktop, tablet y launcher móvil compartan la misma implementación; multimedia puede conservar color, el chrome sigue monocromo. *(data-tema en documentElement + override scoped para superficies del OS)*
- [x] Resolver preferencia inicial por sistema operativo y permitir override explícito. *(matchMedia + localStorage `wandorius:tema`)*
- [x] Evitar flash de tema en la primera pintura y emitir un evento `theme_changed` medible con modo. *(script inline en index.html + ThemeEvent en dispatcher)*
- [x] Guardar anónimo en overlay local y completar la resolución de la preferencia de cuenta sin sobrescribir decisiones locales silenciosamente; UI de conflicto y logout/login. *(transporte remoto, fallback offline y UI de conflicto implementados en 297A-13)*
- [x] Validar contraste AA, foco/teclado, reduced motion, zoom 200% y viewports (1440×900, 1024×768, 390×844, 320px); capturas aprobadas por el usuario. *(aprobación visual 2026-07-31; E2E formal y medición de rendimiento quedan con 297A-17)*

**Salida:** el usuario cambia claro/oscuro desde un control único y la preferencia local sobrevive; tema aprobado visualmente. El transporte remoto, fallback offline y resolución explícita local/remota están implementados; quedan E2E multi-dispositivo y Cuenta.

### 297A-19 — URLs canónicas, deep links y ventana enfocada

**Depende de:** 297A-9/11/12; integra capacidades de 297A-13. Cada app y recurso tendrá una URL compartible; la URL representa solo la ventana enfocada.

- [x] Definir contrato allowlisted para rutas públicas y parámetros; excluir IDs internos, tokens, posiciones, tamaños, z-index, clipboard y overlays privados. *(AppDeepLink + createPathDeepLink)*
- [x] Migrar parser/serializer y fallback seguro de Reader, Finder/Galería, About y Projects; apps legacy sin contrato no aceptan parámetros dinámicos.
- [x] Conectar `replacePath` a todos los cambios de foco y reservar `pushPath` para aperturas explícitas; `window-url-sync` deriva de los stores sin router paralelo.
- [ ] Mantener Back/Forward, refresh y transición desktop/tablet/móvil para el foco completo; la reconciliación de rutas documentales, parámetros inseguros, capacidades y semántica `push/replace` ya está implementada y probada, pero falta E2E real de `goBack()`/`popstate`/interacción móvil.
- [x] Añadir `Copiar URL` como comando global del toolbar, con feedback, fallback de Clipboard API/execCommand y URL canónica allowlisted. *(navigation-commands.ts + desktop-window.ts)*
- [x] Emitir `deep_link_opened` y `window_focus_changed` con propiedades allowlisted; el foco se centraliza en `window-url-sync` para no duplicar eventos entre clicks, taskbar, teclado, URL y móvil. `share_url_copied` conserva `routeName`, `appId`, `presentationMode` y `success`.
- [ ] Probar la instrumentación en sesión limpia, varias ventanas, permisos, rutas inválidas y viewports; el boundary y las rutas inválidas ya tienen cobertura unitaria.

**Salida:** copiar una URL desde cualquier app abre o enfoca esa app/recurso en otra sesión, con historial, seguridad, analítica y presentación móvil coherentes.

### 297A-20 — Iconos de escritorio con posición libre (snap-grid) (completado)

**Depende de:** 297A-11 (overlay). El usuario coloca iconos en cualquier celda del escritorio; la disposición del admin se publica y cada visitante personaliza sin afectar a otros. Plan: `plan-iconos-libres-desktop-2026-07-31.md`.

- [x] Renderizar iconos por `position {col,row}` (snap-grid 88px) con fallback al orden actual cuando no hay posición. *(grid geométrico RTL: `getGridMetrics`/`getCellAt`; `grid-auto-rows` fijo a `--sistema-icono-fila` 64px para que geometría y CSS coincidan)*
- [x] Conectar el drag existente para soltar en celda libre llamando a `moveNodePosition()`; el click sigue abriendo la app y el drag a carpeta/papelera se conserva. *(drop geométrico por celda, sin depender de `elementFromPoint`; funciona con ventanas abiertas encima)*
- [x] Resolver colisiones (desplazar ocupado a celda libre) y reencuadre al cambiar resolución/breakpoint; móvil conserva su geometría `mobilePosition` sin contaminar `position`. *(param `avoid` en `findFreeCell`/`planPlacement`; `reflowPositions` con clamping; móvil ignora posiciones desktop)*
- [x] Persistir posición en el overlay personal (`fieldOverrides` + localStorage) y permitir que el admin la publique al release.
- [x] Tests (merge/colisión/snap) y validación visual en navegador (desktop y tablet). *(37/37 tests; verificado por el usuario: «funciona bien, iconos no se juntan»)*

**Salida:** el escritorio se puede ordenar libremente; la vista pública carga la disposición del admin y cada visitante tiene su propio estado personalizado.

**Pendiente controlado:** modo depuración temporal (Ctrl+Shift+G, cuadrícula roja) que el usuario pidió mantener — eliminarlo cuando lo indique.

### 297A-22 — Reordenamiento por arrastre con grid (móvil + escritorio) [IMPLEMENTACIÓN TÉCNICA COMPLETADA]

**Depende de:** 297A-20 (snap-grid desktop) y 297A-12 (launcher móvil). Plan y decisiones aplicadas en `Agente/planes/plan-reordenamiento-arrastre-grid-2026-07-31.md`. El único gate restante es validación visual/E2E real.

- [x] Decidir y aplicar `mobilePosition`, grid compacto 2/3 columnas, long press + drag y `mobileOrder` fallback.
- [x] Modelo/merge/overlay/default release y validación Rust aceptan `mobilePosition`.
- [x] Launcher usa geometría móvil explícita; navegación queda fuera del grid editable.
- [x] Drag táctil persiste placement compacto en un batch de overlay.
- [x] Desktop conserva `position`; Finder no hereda la política de orden móvil.
- [x] Move prev/next queda como alternativa accesible móvil y escribe `mobilePosition`.
- [x] Compatibilidad legacy, tests de geometría/merge/gesto y cleanup implementados.
- [ ] Validación visual/E2E 320/360/390/768+, foco, teclado, reload/sync y móvil↔tablet.

**Salida:** reordenar iconos es por arrastre sobre celdas en móvil y escritorio; "Mover arriba/abajo" deja de ser el mecanismo; organización móvil persiste en overlay sin contaminar el desktop.

### 297A-21 — Notificaciones de novedades (campana + gestión admin)

**Depende de:** 297A-20, 297A-13 (entrega remota) y menú Admin de 297A-14. Idea nueva: campana junto al tema que avisa de contenido nuevo incluso a usuarios con estado personalizado (el overlay por diff ya les muestra lo nuevo; la campana solo añade el aviso). Plan propio al arrancar.

Plan: `Agente/planes/plan-notificaciones-2026-08-01.md`.

- [x] Definir qué genera una notificación (release público nuevo), cuándo se marca leída y política anti-spam local sin envío inmediato. *(release versionado como ID estable; estado leído acotado a 100 IDs)*
- [x] Campana en la barra superior y launcher móvil junto al tema, con contador, icono Lucide 1px, accesibilidad y estado local.
- [x] Entrega server-side por release y lectura por cuenta: `notifications` + `notification_reads`, dedupe por `release_version`, endpoints y panel admin dentro de Novedades para crear/publicar/archivar.
- [ ] Pruebas E2E: overlay personalizado, sin spam, leídas, logout/login y dos dispositivos.

**Salida:** los usuarios saben que hay novedades aunque su escritorio esté personalizado; el admin gestiona desde un panel, sin notificaciones inmediatas.

### 297A-13 — Registro y overlay remoto *(parcial: Cuenta y backend verificado implementados; correo/MFA/E2E pendientes)*

**Depende de:** 297A-8/11; integra móvil 297A-12.

- [x] Implementar registro verificado detrás de `registration_enabled=false`: cuenta pendiente, token opaco de 24 h, consumo único y login bloqueado hasta verificar. *(UI/correo real quedan diferidos)*
- [x] Transporte de preferencias de cuenta: `user_preferences`, revisión optimista, endpoint protegido, CSRF/CORS, fallback local y guardas contra respuestas obsoletas. *(migraciones 297A-13 + `preferences-sync.ts`; type-check, 209 tests, Rust y gate PASS)*
- [x] UI de resolución `remote/local` para conflictos 409; adaptador separado, modal único/idempotente, cierre al resolver/logout y etiquetado ARIA. *(preferences-conflict-ui.ts + 4 regresiones UI)*
- [x] Pruebas HTTP/integración de 401 sin sesión, 403 sin CSRF, preflight CORS con credenciales y carrera 409 con dos actualizaciones de la misma revisión; verifican router de producción, cuerpos JSON y revisión final. *(4 tests en `preferences_handler.rs`; `cargo test` PASS)*
- [x] Overlay remoto del workspace: `user_workspace_overlays`, contrato JSON validado, revisión optimista, importación local/remota, reset, merge por ID/campo, tombstones, rebase ante release nuevo y conflicto visible. *(migration `20260731120000_297a13_workspace_overlays`; `overlay-sync.ts` + `overlay-conflict-ui.ts`; type-check, 221 tests, Rust y gate PASS)*
- [ ] Prueba E2E real de dos pestañas/dispositivos y política de merge semántico entre cambios concurrentes.
- [x] **Cuenta como app del escritorio:** registrar en AppRegistry como singleton público con estados invitado/autenticado/admin; la UI de verificación y MFA quedan como estados futuros del backend. *(account-view.ts + AppRegistry)*
- [x] **Estado de sesión visible:** control en barra superior y launcher móvil junto al tema; abre Cuenta y refleja Entrar/Cuenta/Cuenta · admin con etiqueta accesible. *(desktop-menu-bar.ts + mobile-shell.ts)*
- [x] **Login dentro de la app:** deslogueado, Cuenta muestra login dentro de su ventana; `/login` es deep link canónico y el wrapper legacy reutiliza la misma vista. Registro y `/register` permanecen cerrados hasta completar backend verificado.
- [x] Backend de recuperación: solicitud no enumerable, token hashado de 1 h, cambio de contraseña y revocación de sesiones. *(UI, MFA/passkey y E2E quedan diferidos)*
- [x] Rate limit de login, auditoría hash de intentos y logout limpia clipboard/undo. *(migration `20260801030000_297a13_auth_audit` + handler)*
- [x] **018A-57 —** Cuenta invitado ofrece formularios internos de registro y recuperación; el backend mantiene `registration_enabled=false` y no se exponen tokens ni se activa correo real.
- [x] **018A-59 —** Registro, solicitud de recuperación y confirmación de recuperación usan un bucket de 3 intentos por IP cada 5 minutos, separado del bucket de login de 5/min; rate limit distribuido queda diferido.
- [x] **018A-60 —** Registro, verificación y recuperación dejan auditoría server-side con tipo, resultado, usuario cuando es seguro e IP hasheada; no se guardan email, contraseña ni token.
- [x] **018A-58 —** Sincronizar la salida documental de 297A-13: formularios internos hechos; UI de tokens, correo real, MFA y E2E siguen pendientes.

**Salida:** preferencias, overlay remoto y Cuenta base tienen transporte seguro, control de revisión, validación server-side, resolución visible `remote/local`, login/logout, formularios frontend y auditoría sensible; 297A-13 permanece abierto por UI de tokens/correo real, E2E multi-dispositivo/móvil, MFA y rate limit distribuido.

### 297A-14 — Programas editoriales *(parcial)*

**Depende de:** 297A-9/10/11. Plan: `Agente/planes/plan-programas-editoriales-2026-07-31.md`.

- [x] Vertical de artículos/About: `article-editor` lazy, admin-only, lifecycle abortable, create→update, multimedia asociada al ID actual, evento tipado y listado Admin sin carreras.
- [x] Editor de proyectos: app lazy admin-only, listado separado, lifecycle/eventos, GET por ID, create→update, URL tri-state y sincronización transaccional de título/visibilidad/lifecycle del resource envelope.
- [x] Editor de productos: app lazy admin-only, CRUD admin completo en `/api/admin/products`, nace inactivo/private, validación backend de precio/moneda, sincronización transaccional del envelope y filtro público `active + public`.
- [x] Contrato de rutas admin alineado: servicios frontend migrados a `/api/admin/...` (backend anida todo bajo `/api`); `GET /api/admin/workspace/releases/{version}` para rollback; `MediaService`/galería usan `MediaPublic`/`MediaAdmin` con URLs explícitas y cliente generado.
- [x] Biblioteca de media. *(app lazy admin-only, papelera soft delete + restore, object URLs revocadas, utils test 6/6 — F4)*
- [x] Menú Admin por capacidades y paridad sin ampliar `admin.ts`. *(matriz de paridad congelada en `matriz-paridad-admin-2026-07-31.md`; comandos `resource:edit/publish/unpublish` materializan acciones declaradas; menú por capacidades del Admin queda con el resto de 297A-14)*
- [x] Paridad F5 técnica: autosave compartido por tipo de recurso — `utils/autosave.ts` (`createDebouncedSaver` genérico) aplicado a artículos, proyectos y productos (create→update idempotente, sin tocar editorial, evento de dominio solo en created) — y comandos de recurso fail-closed con tests (15/15).
- [ ] E2E visual desktop/tablet/móvil del vertical editorial.

**Salida parcial:** los editores de artículos, proyectos y productos viven como programas reutilizables; el epic editorial permanece abierto hasta completar media, paridad y E2E.

### 297A-15 — Comercio seguro

**Depende de:** 297A-7/10/14.

- [x] Tienda como programa/carpeta visual, checkout dentro del OS y programas Pedidos/Descargas con estados vacíos seguros; catálogo público SQL filtra privados. *(worker, historial server-side y proveedor real quedan diferidos)*
- [x] Product versions inmutables y endpoint de descarga privado con path traversal fail-closed.
- [x] Orden idempotente por cliente + clave y webhook firmado con registro de eventos repetibles.
- [x] Entitlements, grants opacos temporales y outbox deduplicado; worker acotado con backoff y rotación de grant implementado (018A-42).
- [x] Compra invitada soportada por checkout público; quedan reembolso/chargeback, scheduler/proveedor real y pruebas E2E con Stripe/Resend.

**Salida:** cliente no concede acceso; comprador recibe la versión adquirida.

### 297A-16 — Analytics, Estadísticas y retiro legado

**Depende de:** 297A-9/11–15.

- [x] Consentimiento/retención y eventos esenciales/opcionales: banner `unknown/granted/denied`, tracker bloqueado por defecto, header server-side, hashes SHA-256 y purga admin 30–730 días. *(plan `Agente/planes/plan-analytics-privacidad-2026-08-01.md`; revisión legal/E2E pendientes)*
- [x] Batch acotado, inserción multi-fila y deduplicación por `event_id`; eventos críticos de pago permanecen server-side.
- [x] Agregados y Estadísticas separados del dispatcher; app admin con paneles Overview/Content/OS/Commerce/Reliability y exportación JSON.
- [x] **018A-26 —** Retirada la ruta frontend legacy `/admin`; Admin permanece como app interna registrada y “Nuevo proyecto” abre `project-editor` mediante `openAppWindow` con guardia admin.
- [x] **018A-39 —** Estadísticas queda reflejada como app `analytics` y la acción declarada `properties` se ejecuta mediante `resource:properties`, que abre una ventana reutilizable de metadatos locales sin exponer IDs internos.
- [x] **018A-50 —** Cerrar la paridad automatizable de contratos públicos/admin (media, artículos, productos, proyectos, notificaciones y settings); JWT Bearer, uploads estáticos y DTOs internos ya no forman parte de los boundaries públicos.
- [ ] Revisar y retirar CSS/clases legacy restantes con VarSense y validación visual; queda diferido y no bloquea hardening.

**Salida:** una sola administración y métricas privadas/tipadas.

### 297A-17 — Hardening, identidad, accesibilidad y SEO

**Depende de:** 297A-6–16.

- [ ] MFA/passkey y recuperación avanzada.
- [x] Estrategia SEO base: sitemap/robots dinámicos, metadata OG/Twitter, canonical y JSON-LD sin incluir drafts.
- [ ] Manual visual en desktop/tablet/móvil.
- [ ] Teclado, foco, zoom, reduced motion y multimedia accesible.
- [x] Quality gate/CI automatizados: type-check local; suite frontend completa, build y budgets gzip en `task:check --ci`.
- [ ] E2E críticos, threat review, observabilidad y runbook; se mantienen diferidos por requerir navegador, revisión humana o entorno de producción.

**Salida:** preparado para revisión de producción; deploy continúa fuera de alcance.

### 297A-23 — Deuda SOLID del runtime de apps *(F3–F5 técnicas/documentales completadas; validación visual separada)*

**Depende de:** 297A-19 (deep links). Plan y evidencia: `Agente/planes/plan-deuda-solid-runtime-2026-07-31.md`.

- [x] Centralizar la jerarquía de capacidades en `runtime/capability.ts`, consumida por registry, adapter, comandos, menú, recursos, workspace y sesión (OCP/DRY).
- [x] Resolver la rama `authenticated`: se conserva para Cuenta y futuras superficies autenticadas; no se retira.
- [x] Añadir política fail-closed y regresión para capacidades corruptas.
- [x] Completar la división de `route-app-adapter.ts`: acceso/dedup en `app-instances.ts`, frontera móvil/cleanup en `runtime-presentation.ts`; coordinador medido con coordinación efectiva <120 líneas (SRP).
- [x] Test anti-drift workspace → AppRegistry: detecta `unregistered-app` y `missing-refId`, excluye folders/shortcuts y permite apps internas sin icono.

**Evidencia del tramo:** TypeScript PASS, Vitest 267/267 en la validación final, `task:check` PASS, `self-check` PASS, Sentinel/VarSense sin errores bloqueantes. F3–F5 técnicas/documentales están completadas; el contrato `publicLocator` separa `refId` interno de referencias públicas allowlisted. `npm run check:back` y `npm test` usan la BD derivada por rama y confirmaron compilación, clippy y 17/17 tests Rust PASS. La validación visual/E2E del runtime permanece controlada en 297A-24.

### 297A-24 — Investigar y resolver: cierre automático de ventanas al abrir otra

**Depende de:** 297A-19 (deep links). **Bug reproducido** — investigar y resolver, no parchear. Plan: `Agente/planes/plan-cierre-automatico-ventanas-2026-07-31.md`.

- [x] Corregir la causa raíz con S1: `window-url-sync` no proyecta `/` cuando existe cualquier app runtime abierta; Perfil/shell queda fuera del catálogo y no dispara reconciliación destructiva.
- [x] Fijar con tests: app no canónica y Perfil no cierran otras apps; cerrar la última app permite `/`; la guardia respeta desktop/tablet frente a móvil.
- [x] Resolver el mismatch `resourceId` vs `slug` de Reader: `resourceId` interno ya no se interpreta como slug público; apps internas siguen sin URL canónica por diseño; recursos sin `publicLocator` muestran feedback seguro. La resolución futura requiere envelope público autorizado.
- [ ] Prueba visual manual desktop/móvil (apertura canónica/no canónica, Perfil, refresh, Back/Home); `task:check` y `self-check` ya pasan.

**Salida:** abrir una ventana nunca cierra las demás; el cierre masivo solo ocurre cuando el usuario navega realmente fuera de las apps; lección registrada.

### 297A-25 — Política de carga de apps pesadas (decisión aceptada)

**Depende de:** 297A-9/11/12. ADR: `Agente/documentacion/arquitectura/adr-carga-apps-pesadas-2026-07-31.md`. Plan: `Agente/planes/plan-carga-apps-pesadas-2026-07-31.md`.

- [x] Confirmar que el runtime no instancia apps al arrancar: `windowStore` no monta apps del catálogo; el shell solo dibuja iconos.
- [x] Confirmar `registerLazy` y medir build: bundle principal ~159.66 KB minificado/~46.05 KB gzip; Tiptap ~294.64 KB/~87.48 KB gzip en chunk separado.
- [x] Establecer convención: apps grandes, WASM, WebGL, media avanzada o dependencias pesadas usan `registerLazy`; apps pequeñas no se migran por uniformidad.
- [x] Definir lifecycle: `MountedView.destroy()` + `AbortSignal` liberan listeners, workers, timers, object URLs, audio y GPU.
- [x] Decidir que `preload` y `heavy` no se agregan todavía; solo se activarán con una app real, ADR y medición.
- [ ] Validar teardown GPU, concurrencia y Network cuando exista la primera app WebGL real.

**Salida:** una app pesada futura puede agregarse siguiendo la guía y `registerLazy` sin tocar el arranque ni el shell; las decisiones GPU/precarga quedan condicionadas a evidencia real.

### 297A-26 — Preferencias de tema embebidas en la ventana Cuenta (completado)

**Depende de:** 297A-13 (sync de preferencias) y 297A-18 (tema claro/oscuro). Petición del usuario: el modal global de conflicto de preferencias debe vivir dentro de la ventana Cuenta, no como modal del sistema.

- [x] Panel de preferencias siempre visible dentro de la ventana Cuenta: selector de tema (sistema/claro/oscuro) con `aria-pressed` y etiqueta accesible. *(preferences-panel.ts + components.css)*
- [x] El bloque de resolución de conflicto aparece embebido bajo el selector solo cuando el sync está en conflicto (dispositivo ≠ cuenta); reutiliza las clases `.preferences-conflict*` existentes. *(buildConflictContent)*
- [x] Cambiar tema desde el panel sincroniza con la cuenta (`themeStore.set` fuente `user` → queueLocalUpdate); cambios de tema externos (barra superior, atajo, móvil) se reflejan vía `subscribeSimple`. *(render reactivo a preferencesSyncStore + themeStore)*
- [x] Eliminar el modal global: se borran `preferences-conflict-ui.ts` (+test) y `preferences-conflict-panel.ts` (+test); `main.ts` deja de inicializarlo. *(cleanup íntegro, sin referencias residuales)*
- [x] Tests del panel (8 casos: selector, aria-pressed, cambio interno, cambio externo, conflicto, conservar dispositivo, usar cuenta, limpiar cuenta) y validación en navegador: panel visible en Cuenta, conflicto aparece al iniciar sesión con tema distinto y se resuelve sin modal. *(Vitest 286/286 PASS, type-check limpio)*

**Salida:** las preferencias y la resolución de conflicto viven en la ventana Cuenta; el sistema ya no muestra modales globales de preferencias al iniciar sesión.

## Detalle operativo de las tareas pendientes

Este bloque amplía el alcance verificable sin duplicar los manuales canónicos. Cada tarea debe cerrarse con su plan enlazado, prueba positiva y casos negativos; ninguna casilla se marca por intención.

### 297A-9 — Foundation del runtime

- [x] Matriz de `AppRegistry`, `CommandRegistry` y `RouteAppAdapter` con capacidades, rutas, eventos y teardown.
- [x] Exportar OpenAPI sin levantar backend y generar Orval `tags-split` con configuración portable (`npm run codegen:local`); quedan cobertura total de endpoints y retiro del cliente manual.
- [x] Tests de IDs, capacidades, disponibilidad por presentación e idempotencia; eventos críticos quedan server-side.
- [x] **018A-61 —** Navegación externa, maximizar/restaurar y reencuadre batch están registrados como comandos únicos; las superficies delegan en ellos y los eventos `external_nav_toggled`, `window_maximized` y `windows_reframed` quedan tipados.
- [ ] Prueba visual completa del shell en todos los viewports y zoom 200% (pendiente de servidor/navegador estable).

### 297A-12 — Experiencia móvil tipo launcher

- [x] Long press, menú contextual, reorder accesible y `mobilePosition` implementados con `CommandRegistry` compartido.
- [x] Back/Home, carpetas, deep links, refresco y transición móvil/tablet conservan el estado en código y tests.
- [ ] Validación visual/E2E final en 320/360/390/768px, safe areas, teclado virtual, foco, scroll y apps críticas.
- [x] Móvil reutiliza comandos, permisos, recursos y analítica del escritorio; solo cambia `presentationMode`.

### 297A-19 — URLs canónicas, deep links y ventana enfocada

- [x] Gramática versionada y mapping allowlisted `app/resource/instance`; parámetros son input no confiable.
- [x] `RouteAppAdapter` valida capacidades y devuelve 403/404 seguro sin enumerar privados ni serializar tokens.
- [x] Foco/z-order se sincroniza con History API y Back/Home sin listeners stale.
- [ ] E2E en sesión limpia, refresh, varias ventanas, permisos, breakpoint, scroll/formulario y deduplicación.
- [x] Un único evento de navegación/deep link; SEO queda limitado a recursos públicos activos.

### 297A-13 — Registro y overlay remoto

- [x] Implementar registro verificado y recuperación detrás de feature flag; rate limit/login/logout, auditoría hash de acciones sensibles, limpieza de clipboard, formularios internos de Cuenta y rate limit específico por IP están operativos. *(correo real, UI de tokens, MFA, rate limit distribuido y E2E quedan diferidos)*
- [x] Sincronizar preferencias y overlay con revisión esperada, actualización optimista, validación server-side, fallback offline y conflicto 409 visible. *(preferencias + `user_workspace_overlays`; gate y self-check PASS)*
- [x] Definir importación local, uso remoto, reset explícito, merge por ID/campo, tombstones y rebase ante release nuevo; probar autorización, payload inválido, corrupción persistida y revisión inicial sin fila fantasma.
- [ ] Probar E2E dos pestañas/dispositivos y decidir merge semántico para cambios concurrentes no resolubles por reemplazo local/remoto.

### 297A-14 — Programas editoriales

Plan canónico: `Agente/planes/plan-programas-editoriales-2026-07-31.md`.

- [x] Vertical de artículos/About, proyectos, productos versionados y media con draft/private/public, papelera, rollback y autosave.
- [x] Matriz de paridad del Admin y acciones por capacidades server-side; Finder/clipboard preservan referencias y permisos.
- [ ] E2E visual desktop/tablet/móvil del vertical editorial.

### 297A-15 — Comercio seguro

- [x] Modelar Tienda, Checkout, Pedidos y Descargas como programas del OS; checkout usa ProductService/idempotencia y el release incluye sus nodos. *(historial/grants consultables y UI operativa avanzada quedan diferidos)*
- [x] `idempotency_key`, `UNIQUE provider_event_id`, webhook firmado y outbox deduplicado implementados.
- [x] Entitlement/grant corto y compra invitada implementados; quedan reembolso, chargeback, worker de outbox y E2E con proveedor.

### 297A-16 — Analytics, Estadísticas y retiro legado

- [x] Definir consentimiento, retención, anonimización y derechos operativos: opt-in local, purga admin, hashes SHA-256 y ausencia de `user_id` en analytics; revisión legal/E2E queda pendiente.
- [x] Ingesta batch acotada, multi-fila e idempotente por `event_id`; agregados separados del audit.
- [x] Completar paneles Overview/Content/OS/Commerce/Reliability y exportación; `analytics` es un programa admin independiente y conserva el panel legacy como compatibilidad.
- [x] **018A-28 —** Retirar serving estático de `/uploads`: previews públicos solo para `active + public + clean`, previews admin para recursos activos, y paths confinados al storage configurado.
- [x] **018A-29 —** Separar DTO público/admin/upload de media del modelo interno de storage; `url` y `admin_url` quedan como contratos explícitos para API, OpenAPI y frontend.
- [x] **018A-31 —** Limpiar CSS huérfano confirmado con VarSense; las clases construidas dinámicamente no se eliminan por un falso positivo.
- [x] **018A-32 —** Añadir mutator único de Orval y migrar MediaService al contrato generado.
- [x] **018A-33 —** Migrar ArticleService, ProjectService y ProductService al cliente generado; la matriz restante queda acotada a auth, workspace, settings, analytics, notifications y preferences.
- [x] **018A-34 —** Migrar auth, preferences, notifications, settings y analytics al cliente generado; workspace queda como último dominio frontend con adaptación propia.
- [x] **018A-35 —** Migrar `WorkspaceService` y retirar el último consumidor frontend del cliente manual; mantener el modelo rico del runtime separado de los DTOs OpenAPI.
- [x] **018A-36 —** Retirar el cliente manual sin borrar la política común de sesión/CSRF/errores; verificar que no quedan consumidores de `api.get/post/...`.
- [x] **018A-37 —** `test:changed` selecciona tests por grafo de imports en vez de ejecutar toda la suite ante cualquier cambio fuente; `writeAtomic` reintenta carreras Windows. `test:full` sigue siendo el modo explícito total.
- [x] **018A-38 —** Dividir el contrato frontend `Article` en identidad, contenido y publicación para cumplir ISP sin cambiar el tipo público; el warning `large-interface-isp` queda resuelto.
- [x] **018A-39 —** Completar la acción `properties` de la matriz de recursos y registrar la app `properties`; el menú contextual ya no declara una acción sin ejecutor.
- [x] **018A-40 —** Retirar tres utilidades CSS huérfanas (`gap-sm`, `mt-md`, `mb-lg`) tras búsqueda estática en fuentes y estilos; se conservan clases dinámicas con consumidores comprobados.
- [x] **018A-50 —** Completar la matriz de paridad de contratos automatizables y documentar el backlog residual de CSS/fachadas manuales como deuda no bloqueante.
- [x] **018A-51 —** El gate CI ejecuta la suite frontend completa; el trabajo local conserva alcance incremental y no inicia procesos innecesarios.
- [x] **018A-52 —** La caché del quality gate separa huellas local/CI para impedir que un PASS de `test:full` se reutilice como cobertura local sin ejecutar la suite.
- [x] **018A-53 —** CI construye el frontend y valida budgets gzip del entry JS, entry CSS y chunk JS mayor; local no ejecuta build para conservar el ciclo rápido.
- [ ] Retirar CSS/clases huérfanas restantes con VarSense tras revisión visual; diferido.

### 297A-17 — Hardening, identidad, accesibilidad y SEO

- [ ] Completar MFA/passkey, recuperación y threat review con casos negativos de sesión, CSRF, capacidades, pagos, grants y webhooks.
- [x] Validar SEO base: HTML público, sitemap, robots, metadata y Open Graph sin exponer drafts ni rutas privadas; queda auditoría final.
- [ ] Verificar manual visual, teclado, foco, live regions, zoom 200%, reduced motion, alto contraste y multimedia accesible.
- [x] **018A-51 —** CI ejecuta `type-check` y `test:full` frontend solo con `task:check --ci`; local mantiene type-check y selección incremental para no degradar el equipo.
- [x] **018A-53 —** CI ejecuta el build y aplica budgets gzip configurables de 65 KB para entry JS, 16 KB para entry CSS y 120 KB para el chunk JS mayor.
- [x] **018A-54 —** Documentar preflight, deploy, health, backup/restore y rollback mediante Coolify Manager; no se ejecuta producción ni se habilita SSH.
- [x] **018A-55 —** Sincronizar el resumen del quality gate con `quality:test` 31/31 y la separación de evidencia local/CI.
- [ ] Ejecutar E2E críticos y observabilidad real; deploy continúa fuera de alcance. El split estructural de modelos ya está cerrado: `workspace/` y `workspace_overlay/` agrupan DTOs, validación, locators y tests sin suppressions. Para backend se debe usar `npm test`/`npm run check:back`, que derivan la BD por rama y aplican el contexto correcto.
- [x] **018A-37 —** El selector frontend incremental y la escritura atómica Windows quedan cubiertos por fixtures del orquestador; la suite completa sigue reservada para `test:full`/CI.
- [x] **018A-38 —** Separar el contrato Article en subinterfaces composables; type-check y Sentinel confirman que las vistas conservan el mismo boundary.
- [x] **018A-41 —** Separar el registro de apps públicas y administrativas; `app-registration.ts` queda bajo 300 líneas y los registros/capacidades permanecen sin cambios.
- [x] **018A-42 —** Desacoplar la entrega de commerce del webhook: `--process-commerce-outbox` reclama lotes con `SKIP LOCKED`, reintenta con backoff y rota el hash del grant sin persistir tokens en claro.
- [x] **018A-43 —** Cerrar el alcance mínimo de Sentinel/VarSense; el gate reproducible desbloquea el roadmap principal y el backlog avanzado queda diferido explícitamente.
- [x] **018A-44 —** Retirar el alias `font-panel.ts` ya sin lógica de fuentes; la app Configuración conserva su registro y delega desde `settings-panel.ts`, sin cambiar su comportamiento.
- [x] **018A-45 —** Retirar siete tokens CSS sin consumidores confirmados por VarSense (`--radio`, aliases de sidebar, z-index legacy y transición); se conserva `--contenido-max` y el chrome activo.
- [x] **018A-46 —** Separar los contratos públicos/admin de productos del modelo interno; catálogo y artículos ya no serializan `download_path` ni IDs de Stripe, mientras checkout/webhook conservan el modelo privado.
- [x] **018A-47 —** Restringir `GET /api/settings` a una allowlist de presentación pública y renombrar el cliente frontend a `getPublic`; claves de auth/admin dejan de exponerse por defecto.
- [x] **018A-48 —** Separar `ProjectAdminResponse` y `ProjectPublicResponse`; el catálogo público deja de exponer `sort_order`/`is_visible`, mientras el editor conserva esos metadatos solo detrás de capacidad admin.
- [x] **018A-49 —** Separar contratos de notificaciones públicas, de cuenta y admin; visitantes ya no reciben `created_by`/`status`, y el panel administrativo conserva los campos completos detrás de `AdminUser`.

## Revisión SOLID y escalabilidad por fase

Cada fase termina con esta revisión antes de marcar su salida. La revisión debe quedar evidenciada en el plan de la fase, en el quality gate y en el commit; no se acepta “lo refactorizamos después”.

### Checklist común de cierre

- [x] **SRP/ISP:** cada módulo tiene una responsabilidad clara, las interfaces exponen solo capacidades necesarias y los componentes no mezclan chrome, contenido, persistencia y analítica. *(checkpoints S1/S3 y refactor DomAttrs/runtime)*
- [x] **OCP/DIP:** nuevas apps, recursos, comandos y temas se agregan mediante registros/adaptadores; no se crean `if/else` globales ni copias por plataforma. *(registries + adapters + guía agregar-app)*
- [x] **Límites:** componentes/CSS ≤300 líneas, lifecycle/store/hook ≤120 y utils ≤150; excepciones heredadas quedan reportadas por Sentinel. *(gate sin errores bloqueantes)*
- [x] **Contratos:** tipos, DTOs, errores, permisos y eventos son explícitos; no hay estado duplicado, listeners sin teardown, I/O silencioso ni roundtrips N+1 en los bloques cerrados.
- [x] **Escalabilidad:** se prueba un segundo caso real (otra app, recurso, usuario, tema o viewport), se revisan índices/paginación/cache y se documenta el impacto de migración y rollback en los bloques cerrados.
- [x] **Calidad:** Sentinel/VarSense, type-check, tests y gates de los bloques cerrados pasan; las pruebas visuales/E2E restantes están separadas como pendientes explícitas.

### Foco obligatorio por fase

- [x] **297A-9 Runtime:** registry/adapters y WindowManager no conocen apps concretas; agregar una app no modifica el shell ni duplica listeners.
- [x] **297A-10 Recursos:** services/repositories/DTOs permanecen separados; un nuevo `resourceKind` no altera recursos existentes ni filtra metadata privada.
- [x] **297A-11 Workspace:** referencias, overlay, clipboard y papelera son composables; mover/copiar un tipo nuevo conserva atomicidad, permisos y undo.
- [x] **297A-12 Móvil:** launcher y desktop consumen las mismas apps/comandos; un nuevo breakpoint no crea una app paralela ni pierde estado de ruta. *(E2E visual sigue pendiente)*
- [x] **297A-18 Tema:** el botón solo despacha un comando y los tokens viven en el sistema visual; agregar un tercer tema de prueba no requiere reescribir componentes.
- [x] **297A-19 Deep links:** parser/serializer y foco viven en adaptadores; añadir una app o recurso no modifica el router global ni serializa estado privado. *(E2E History API sigue pendiente)*
- [x] **297A-13 Cuentas:** merge, sesión y preferencias se resuelven por servicios/adaptadores; otro proveedor de identidad no duplica el flujo ni restaura tombstones. *(registro/verificación y E2E siguen pendientes)*
- [x] **297A-14 Editorial:** editores comparten primitives y capacidades; añadir un tipo de documento no amplía el monolito Admin ni copia ventanas. *(E2E visual sigue pendiente)*
- [x] **297A-15 Comercio:** pago, webhook, entitlement y grants son servicios independientes; otro proveedor o versión no cambia la autoridad server-side. *(UI/worker/proveedor real siguen pendientes)*
- [x] **297A-16 Analytics:** catálogo, dispatcher y agregados son extensibles; añadir un evento no expone datos ni obliga a reescribir paneles existentes. *(consentimiento/paneles siguen pendientes)*
- [x] **018A-54 —** El runbook de Coolify cubre rollback sin SSH y deja explícitos los límites de evidencia; queda pendiente la prueba operativa en un entorno autorizado.
- [x] **018A-55 —** El resumen del roadmap coincide con los 31 tests del orquestador y el alcance pesado exclusivo de CI.
- [ ] **297A-29 Configuración legacy + Perfil admin:** el toolbar expone acciones por capacidad sin `if/else` en el shell; la app Configuración se conserva y queda pendiente de escalar a otra cosa (p. ej. panel de ajustes del sistema); añadir una acción admin futura es un comando más, no un cambio de shell.

### 297A-29 — App Configuración conservada + Perfil configurable por admin (fuentes/tamaños estáticos)

**Depende de:** 297A-27 (overlay), 297A-28 (guardado settings), 297A-13 (capacidades) y 297A-19 (toolbar). Plan: `Agente/planes/plan-configuracion-legacy-2026-07-31.md`. Petición del usuario: borrar las configuraciones de fuentes y tamaños (todo estático con JetBrains Mono y valores fijos), **NO eliminar la app Configuración** (queda pendiente de escalar a otra cosa en el futuro) y dejar la configuración de Perfil dentro de la ventana Perfil con un botón en el toolbar visible solo para admins.

- [x] Fase 1 — Fuentes/tamaños estáticos: neutralizar `fontStore` y `loadSavedFonts()`, fijar tokens en `variables.css` (JetBrains Mono en todo, `--nav-width` fijo ≥360px), migrar consumidores legacy, eliminar `font-constants.ts`/`font-helpers.ts`/tab Fuentes/Tamaños. *(commit 297A-29 F1)*
- [x] Fase 2 — Toolbar reactivo a capacidad: `createAppToolbar` se suscribe a `authStore` (login/logout en vivo) y se crea el comando genérico admin-only con `isAvailable` (sin `if/else` en el shell). *(commit 297A-29 F2)*
- [x] Fase 3 — Perfil configurable: extraer controles de perfil a `profile-settings.ts`, toolbar en `shell-profile` con botón admin-only, fix del borde (`.desktop-profile-window .profile-foto` respeta el token) y persistencia vía `POST /api/admin/settings`. *(commit 297A-29 F3)*
- [ ] Fase 4 — (pendiente, NO eliminar) Escalar la app Configuración: se conserva tal cual (registro `settings`, nodo admin, botón de menú y tab `'fuentes'` de Admin). Futuro: decidir si se convierte en panel de ajustes del sistema o se integra en otra app. No hay eliminación ni reescritura ahora.
- [ ] Fase 5 — (futuro, no implementar) Escalar la app Configuración a un panel de ajustes del sistema y re-introducir el selector de fuente de usuario con buena arquitectura cuando exista el panel de control.

**Salida:** app Configuración conservada (pendiente de escalar, sin eliminación); Perfil se configura desde su ventana con botón admin-only; fuentes/tamaños 100% estáticos; bug del borde y límite de nav resueltos; mecanismo genérico de toolbar por capacidad.

### 317A-1 — Barra de pestañas universal (createTabs) para Admin y futuras apps

**Petición del usuario:** las tabs del HTML de Admin deben ser un componente universal y autocontenido — sin depender del padding/margin del contenedor — porque otras apps tendrán tabs también y no queremos complicar la vida. Motivo: evita duplicar recetas de navegación por pestaña en cada app.

- [x] Crear `frontend/src/components/ui/tabs.ts` (`createTabs`): role=tablist/tab + aria-selected, clase activa, `select()` programático, `onSwitch` al cambiar e inicial. *(317A-1)*
- [x] CSS en `components.css` (`@layer components`): `.barra-tabs` autocontenida (gap/margin/border propios), `.barra-tabs__tab` y `.barra-tabs__tab--activa`.
- [x] Migrar `admin.ts`: eliminar el hack inline de `style.fontWeight` y las utilidades externas (`flex-fila gap-lg mb-lg border-bottom`); usar `createTabs` con las 6 tabs.
- [x] Eliminar CSS muerto en `pages.css`: `.config-tabs-nav` y `.config-tabs-content` (prototipo local; la receta ahora vive en el componente).
- [x] Test `tabs.test.ts`: render, estado activo por defecto, `initial`, onSwitch por clic, `select()` programático, onSwitch inicial. 6/6 PASS.
- [x] Validación: `tsc --noEmit` OK, suite vitest 322/322 PASS, gate `task:check -- 317A-1`.

**Salida:** cualquier app puede usar `createTabs({tabs, initial, onSwitch})` y obtener una barra de pestañas accesible y autocontenida; Admin migrado sin inline styles; CSS muerto retirado.

### 317A-2 — Estado vacío universal (createVacio) centrado y capitalizado

**Petición del usuario:** los estados vacíos deben estar centralizados en un componente universal (todos), con texto centrado que ocupe el 100% del ancho y la altura, y la primera letra en mayúscula.

- [x] Crear `frontend/src/components/ui/empty-state.ts` (`createVacio`): wrapper `div.vacio` con `role="status"` + `<p>` interno con la primera letra capitalizada. *(317A-2)*
- [x] CSS `.vacio` en `components.css`: flex centrado, `width/height: 100%`, `flex: 1`, `min-height` de respaldo y `grid-column: 1/-1` para grids; estilo de texto movido a `.vacio p`.
- [x] Fill-height en ventanas: `.app-contenedor` (admin + projects) sin tocar `.desktop-window__body`; cadena `admin-pagina → admin-contenido → admin-lista` con flex; `.pagina-contenido` para galería/proyectos; `media-library__grid` con `flex:1`.
- [x] Migrar los 16 sitios de `.vacio` a `createVacio`: admin-articles/products/projects (6), admin.ts stats, gallery, projects, router not-found, article/product/project-editor (3), media-library (2), mobile-stack.
- [x] Test `empty-state.test.ts`: render, role=status, capitalización, texto ya capitalizado, string vacío. 4/4 PASS.
- [x] Validación: `tsc --noEmit` OK, suite vitest 326/326 PASS.
- [x] Gate `task:check -- 317A-2` PASS + verificación en navegador (Admin "No hay articulos" centrado a toda altura).

**Salida:** cualquier estado vacío/error usa `createVacio(texto)` y queda centrado, llenando el contenedor, con mayúscula inicial y rol accesible; sin recetas `.vacio` duplicadas en cada archivo.

### 317A-3 — Toolbar del article-editor con iconos Lucide (receta `.boton-icono`)

**Petición del usuario:** al abrir el editor de artículos, la toolbar muestra texto ("negrita", "italica", "codigo", "h2", "h3", "lista"…) en lugar de iconos — "esto se ve muy mal, supongo eran iconos".

- [x] Tokens en `variables.css`: `--sistema-boton-icono-tamano: 20px` y `--sistema-boton-icono-svg-tamano: 14px`.
- [x] Receta `.boton-icono` en `components.css` (regla 9.1: receta base primero): grid centrado, tamaño desde token, SVG hereda `--sistema-icono-trazo`, focus-visible 1px, nombre accesible vía `aria-label` del consumidor.
- [x] Migrar `createToolbar()` en `article-editor.ts`: botones `{ label, icon, action }` con iconos Lucide (Bold, Italic, Code, Heading2, Heading3, List, ListOrdered, Quote, SeparatorHorizontal, Image, AudioLines, Video), `aria-label` + `title`, child `createElement(icon)`.
- [x] Fix del falso positivo Sentinel: ampliar `CLASES_BOTON_SISTEMA` en el core (static + react) con `boton-icono` y variantes kebab reales del proyecto (`boton-pequeno`, `boton-mediano`, `boton-grande`); test de regresión añadido. *(regla 8: implementar prevención)*
- [x] Validación: `tsc --noEmit` OK, suite vitest 341/341 PASS, gate `task:check -- 317A-3` PASS (sentinel 0e/0w, varsense 0e/14w, frontend PASS).
- [x] Verificación en navegador: toolbar del article-editor muestra iconos Lucide 1px horizontales; el usuario confirmó el orden y el aspecto.

**Salida:** la toolbar del editor usa iconos Lucide de 1px con nombre accesible; la receta `.boton-icono` queda disponible para cualquier toolbar futura; Sentinel no la marca como botón ad-hoc.

### 317A-5 — Persistencia de sesión de ventanas (reload conserva el escritorio)

**Petición del usuario:** «al recargar todo se reinicia desde cero; al recargar todo debería aparecer como antes». El workspace (iconos), el tema y el sidebar ya persisten; las ventanas abiertas no. Plan: `Agente/planes/plan-persistencia-sesion-ventanas-2026-08-01.md`.

- [x] Módulo `window-session.ts` (captura versionada en `wandorius:window-session` v1) + `window-session-restore.ts` (restauración fail-closed por catálogo/capacidad).
- [x] `openRestoredWindow` en window-manager (bounds/state/zIndex/focused/preMaximizeBounds explícitos) + `ensureNextZIndexAbove` en window-store.
- [x] Persistencia reactiva con debounce 200ms, flush en `pagehide`, `pause()`/`resume()` durante transiciones de presentación (evita persistir un escritorio vacío con `closeAllWindows`).
- [x] `main.ts`: `await restoreWindowSession()` antes de `initRouter()` (la URL enfoca la app ya restaurada sin duplicar); cleanup libera suscripciones.
- [x] Móvil: stack restaurado en orden (top = foco); secciones desktop/mobile independientes (una no pisa a la otra).
- [x] Tests 22 (captura, versionado, sanitize con regresión maximizada+foco, fail-closed admin/retirada, dispatch desktop/móvil, debounce/pause/flush).
- [x] E2E visual: recargar desktop/tablet conserva ventanas/geometría/estado/foco; móvil conserva el stack. Verificado en navegador: 1024×768 (Perfil + Galería restauradas) y 390×844 (Galería restaurada en launcher móvil).

**Salida:** recargar reconstruye la sesión de presentación como estaba; la restauración nunca abre apps fuera de catálogo ni sin capacidad.

### 317A-4 — Identidad visual de formularios y botones OS en el article-editor

**Petición del usuario (3 puntos):**
1. Los labels de formulario ("extracto", "imagen de portada") deben tener la primera letra en mayúscula.
2. Los botones dentro de una ventana del OS deben llevar borde 1px sin redondear (aspecto OS); fuera de ventanas (páginas públicas) siguen como texto subrayado.
3. Los botones de solo icono de la toolbar del editor no llevan borde (están bien), solo necesitan más separación.
4. *(Refinamiento posterior)* Los tabs de `.barra-tabs` NO deben llevar borde aunque usen `.boton` — son navegación, no botones de acción.
5. *(Experimenta aprobada)* Barra de pestañas vertical alineada a la izquierda con layout de 2 columnas (tabs | contenido). El estado activo tiene opacidad plena y peso medio; los inactivos bajan a opacidad 0.45. Nueva regla visual registrada en el manual §13.

- [x] Header de `components.css` actualizado: regla de botones según superficie (borde 1px dentro de `.desktop-window`, `.movilApp`, `.modal-contenido`, `.confirm-contenido`; texto subrayado fuera).
- [x] Regla contextual de botones OS: `.desktop-window .boton, .movilApp .boton, .modal-contenido .boton, .confirm-contenido .boton { border: var(--borde); padding: var(--espacio-xs) var(--espacio-sm); }` + exclusión `.barra-tabs .boton` (tabs sin borde).
- [x] Labels de formulario capitalizados: `.campo-etiqueta` y `.preferences-panel__etiqueta` con `::first-letter { text-transform: uppercase }` (NO `capitalize`, que subiría preposiciones).
- [x] Toolbar del article-editor con más separación: `gap-sm` → `gap-md` en `article-editor-ui.ts`.
- [x] Manual de identidad visual actualizado: regla de mayúscula inicial en labels (§6), regla de botones según superficie (§13) y nueva regla de barra de pestañas vertical 2 columnas (§13).
- [x] Validación: `tsc --noEmit` OK, gate `task:check -- 317A-4` PASS (sentinel 0e/0w, varsense 0e/14w, frontend PASS).
- [x] Verificación en navegador: labels con mayúscula inicial, botones OS con borde 1px dentro de la ventana (`.admin-contenido .boton` = 1px sólido, padding 4px/8px), tabs sin borde con opacidad activo 1 / inactivo 0.45, toolbar más separada, y layout 2 columnas (tabs izquierda x=87, contenido derecha x=197, misma fila, gap 24px) funcionando incluso en ventana estrecha 322px sin overflow.

**Salida:** los formularios del OS capitalizan la primera letra de sus labels, los botones de acción dentro de superficies OS ganan borde 1px (los tabs y botones de icono no), la toolbar del editor respira mejor, y la barra de pestañas queda como navegación vertical izquierda en 2 columnas con estados de opacidad; la identidad visual queda documentada en el manual (§13).

### 018A — Barra de acciones inferior de ventana (regla aprobada)

**Regla:** las acciones primarias de una ventana viven en `.desktop-window__actions`, franja inferior del chrome (debajo del body padded, fuera de su padding y scroll), con los botones al final (derecha). Aprobada visualmente el 2026-08-01. Plan: `Agente/planes/plan-barra-acciones-ventanas-2026-08-01.md`.

- [x] Slot `actions` en runtime chain + Admin (tabs con alta rellenan; sin alta ocultan). *(018A-1)*
- [x] Manual identidad §9 y guía agregar-app actualizados. *(018A-2)*
- [x] Fase 1: alcance móvil del slot — el stack móvil monta la misma franja (`MountedView.actions`) debajo del contenido; `.movilApp` gana tercera fila. *(018A-4)*
- [x] Fase 2: migrar editores (article/project/product) a la franja (fijar + crear/guardar, compactos). *(018A-5)*
- [x] Fase 3: inventario de ventanas restantes — Biblioteca y tab sitio de Admin pasan a la franja; Configuración/Cuenta/Finder/Papelera/Proyectos documentados sin franja (justificado). *(018A-6)*
- [x] Fase 4: prevención — test de regresión del slot en `desktop-window.test.ts`; regla Sentinel/VarSense evaluada como no viable (semántica, alto ruido; cubierta por reglas existentes). *(018A-7)*
