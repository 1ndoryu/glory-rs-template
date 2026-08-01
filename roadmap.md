# wandori.us — Roadmap

> **Producto:** blog, portfolio y tienda digital dentro de un OS retro minimalista.
> **Stack:** Rust/Axum + PostgreSQL + Vanilla TypeScript/Vite.
> **Deploy:** fuera de alcance hasta instrucción explícita; producción solo con Coolify Manager.
> **Epic:** 297A-4 — OS persistente, cuentas, programas y comercio.
> **Visual:** desktop/tablet y prototipo móvil aprobados; el chrome sigue monocromo, minimalista y con JetBrains Mono.

## Fuentes canónicas

- Índice: `Agente/documentacion/indice-documentacion-2026-07-29.md`
- Arquitectura: `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`
- Identidad: `Agente/documentacion/design-system/manual-identidad-visual-os-2026-07-29.md`
- Plan maestro: `Agente/planes/plan-escritorio-persistente-cuentas-admin-apps-2026-07-29.md`
- Plan móvil: `Agente/planes/plan-experiencia-movil-launcher-2026-07-29.md`
- Quality gate: `Agente/planes/completados/plan-escalabilidad-sentinel-wandorius-2026-07-29.md`
- Prevención: `Agente/prevencion/prevencion-wandorius-sentinel-varsense-2026-07-29.md`
- Tema claro/oscuro: `Agente/planes/plan-modo-oscuro-os-2026-07-31.md`
- Juego bosque multijugador 3D: `Agente/planes/plan-juego-bosque-multijugador-2026-08-01.md`
- Assets y terreno del bosque 3D: `Agente/planes/plan-assets-terreno-bosque-3d-2026-08-01.md`
- ADR de renderer, assets y terreno: `Agente/documentacion/arquitectura/adr-bosque-3d-assets-terreno-2d-2026-08-01.md`
- Deep links: `Agente/planes/plan-deep-links-ventanas-2026-07-31.md`
- Apps editoriales: `Agente/planes/plan-programas-editoriales-2026-07-31.md`
- Interacción y medición: `Agente/planes/plan-contratos-interaccion-comandos-medicion-2026-07-29.md`

## Cómo leer este archivo

- Esta lista contiene únicamente trabajo pendiente. Las casillas no se duplican en otro bloque.
- El estado base implementado se resume abajo para conservar contexto sin convertir el roadmap en un historial.
- El detalle de cada entrega terminada vive en `Agente/completados/` y en los planes archivados.
- Una tarea solo se marca cuando tiene evidencia de código, pruebas, quality gate y, si es UI, navegador.
- El cierre normal usa `npm run task:check -- {ID}` y `npm run self-check -- -TareaId {ID}`.

## Estado base implementado (resumen operativo)

- **Runtime y shell:** `MountedView` con `AbortSignal`/teardown, `AppRegistry`, `WindowManager`, `CommandRegistry`, `RouteAppAdapter`, taskbar reactivo, foco/z-index, drag y resize por bordes, atajos, analytics tipado y toolbar común. Las apps devuelven contenido; el shell crea el chrome.
- **Workspace:** release público inmutable + overlay personal, merge por ID/campo, tombstones, clipboard, undo, papelera, carpetas, Finder real, drag/drop, menús unificados, referencias por `resourceKind`, publicar/preview/rollback y organizador público aislado del workspace admin.
- **Recursos y programas:** envelope de recursos con estados editorial/visibilidad/lifecycle/comercio independientes, DTOs público/admin/upload, About como recurso, versiones de producto, biblioteca de media con soft delete/restore y editores lazy de artículos, proyectos y productos con autosave compartido.
- **Comercio:** Tienda, Checkout, Pedidos y Descargas son programas del OS; idempotencia de órdenes, webhook firmado, outbox deduplicado, entitlements y grants temporales. Precio, pago, entrega y autorización permanecen server-side.
- **Cuenta y preferencias:** sesión opaca en cookie HttpOnly, login dentro de Cuenta, recuperación hashada, rate limits, auditoría sin secretos, Cuenta como app singleton, preferencias/overlay remotos con revisión optimista y conflicto `local/remote` embebido en Cuenta. Admin no usa overlay personal.
- **Presentaciones:** desktop/tablet usa ventanas; móvil usa launcher y apps a pantalla completa con las mismas apps, comandos, permisos, rutas y analítica. La transición móvil↔tablet reinstancia de forma segura y conserva el recurso enfocado.
- **Tema e identidad:** modo claro/oscuro con tokens semánticos, botón único, persistencia local/remota, prevención de flash y evento `theme_changed`; Lucide oficial a 1px; botones, tabs, estados vacíos, labels, toolbar TipTap y franja inferior de acciones siguen recetas compartidas.
- **Persistencia y URLs:** sesión versionada de ventanas con restauración fail-closed, debounce/flush y stack móvil; deep links allowlisted, foco representado por la URL, `Copiar URL`, History API y eventos de navegación/foco. La restauración reconstruye acciones desde `MountedView`, nunca persiste DOM.
- **Layout de iconos:** escritorio usa `position` snap-grid; móvil usa `mobilePosition` con fallback `mobileOrder`; colisiones, reflow y Finder están separados por presentación.
- **Contratos y seguridad:** OpenAPI `tags-split`, mutator compartido, clientes Orval por dominio y retiro del cliente manual; sin Bearer/JWT público, sin serving estático de uploads, sin DTOs internos ni storage keys en respuestas públicas.
- **Calidad y arquitectura:** quality gate incremental local/full CI con Sentinel + VarSense, cachés separadas, `test:changed`, suite frontend completa en CI, builds/budgets gzip, runbook Coolify y checkpoints SOLID/OCP/DIP/SRP documentados. El mínimo desbloqueante está cerrado: `quality:test` 31/31 y 24 reglas activas.
- **Correcciones recientes relevantes:** se resolvieron la ruta legacy `/admin`, visibilidad editorial de proyectos (018A-83), contratos de URL/autosave, select nativo, `createEl` para `textarea`, Reader TipTap, sincronización del Finder, iconos por registro único, rejilla compacta y bordes/flechas del tema oscuro. La carpeta vacía "Galería" se sustituyó por "Documentos" con subcarpetas por tipo y sync de media al workspace (018A-87): los archivos subidos aparecen en el Finder, se abren con visor, se retiran al moverlos a la papelera y se restauran. El menú contextual ahora funciona dentro de las carpetas con acciones de creación (nuevo artículo/proyecto/producto, subir archivo, nueva carpeta, pegar) y el clic en ítems del Finder y del escritorio muestra selección visual con los tokens del OS (018A-88). El clic derecho dentro de las carpetas responde en todo el alto del panel del Finder, no solo sobre los ítems (018A-89). El menú sobre una carpeta dentro del Finder ofrece gestión completa — Abrir, Renombrar, Cortar, Copiar, Pegar en y Eliminar con borrado seguro (confirmación + subárbol restaurable) y `Ctrl+V` con destino (018A-90); el crear permanece en el fondo. La restauración de sesión conserva el chrome inferior de las apps (`MountedView.actions`) validado visualmente en desktop y tablet sin duplicar ventanas ni alterar geometría/taskbar/URL (018A-69). El fallback local del prototipo Bosque/Bosque 3D evita que un release local anterior al registro oculte los accesos durante desarrollo, sin sobrescribir la organización del release ni activar apps en producción (018A-92). Una carpeta vacía del Finder ya no muestra texto (el grid queda en blanco y el clic derecho sigue abriendo el menú) y la barra de ruta tiene botón "volver a la carpeta anterior" con historial de navegación, deshabilitado en la raíz (018A-91). Los detalles y gotchas permanecen archivados.

## Siguiente bloque habilitado

**018A-66 — Separar overlay personal de la sesión admin.** Validar en navegador login, logout y recarga con usuario admin: no debe aparecer el modal de conflicto ni el aviso `workspace actualizado`; con cuenta no-admin el conflicto solo aparece ante revisiones local/remota incompatibles. Después se continúa con hardening/E2E.

## Pendientes ordenados

### GAME-01 — Bosque multijugador 3D dentro del OS (planificado, bloqueado)

**Depende de:** cerrar el bloque habilitado actual y sus gates de runtime, sesiones/capacidades, workspace, carga lazy y validación visual. Plan canónico: `Agente/planes/plan-juego-bosque-multijugador-2026-08-01.md`.

- [ ] Aprobar ADRs de identidad temporal, salas, contrato realtime, presupuesto y mapa versionado; el renderer Three.js, los assets GLB externos y el terreno lógico 2D ya tienen ADR aprobado.
- [ ] Implementar por fases: app lazy/Three.js, mapa finito por chunks, sala server-authoritative, presencia, personaje, `Assets 3D`, editor admin 2D y publicación.
- [ ] Mantener el objetivo inicial en salas de 8 jugadores, snapshots a baja frecuencia, interés por proximidad y salas bajo demanda.
- [ ] Validar teardown al cerrar, límites de mensajes/mapa/assets, permisos server-side, reconexión y rollback de versiones.

**Gate/salida:** el plan GAME-01 queda aprobado y cada fase tiene su propio ID, gate `task:check`, pruebas de navegador y evidencia de carga antes de iniciar la siguiente.

### 018A-66 — Separar overlay personal de la sesión admin

**Depende de:** 297A-13 y capacidades server-side. El contrato automatizado ya confirma que admin no solicita overlay remoto ni abre `workspace actualizado`.

- [ ] Validar en navegador login, logout y recarga con usuario admin; no debe aparecer el modal de conflicto ni el aviso `workspace actualizado`.
- [ ] Validar con una cuenta no-admin que el conflicto siga apareciendo únicamente cuando existan revisiones local/remota incompatibles.

**Gate/salida:** admin publica el release global; solo cuentas personales resuelven overlay remoto.

### 018A-73 — Refactor de deuda CSS en `components.css`

**Depende de:** revisión visual de páginas públicas y del sistema de recetas.

- [ ] Dividir `components.css` (supera 600 líneas) por dominio/receta sin cambiar contratos visuales.
- [ ] Mover el bloque de botones a `Button.css` y migrar consumidores a la receta compartida.
- [ ] Quitar border/padding locales de `.notificaciones__item` y `.notificacionesAdmin__item`; consumir recetas base.
- [ ] Sustituir `.comercio__producto h3` por `modalTitulo` o el token equivalente.
- [ ] Ejecutar VarSense/Sentinel y verificar que no se introduzcan clases huérfanas ni especificaciones duplicadas.

**Gate/salida:** `components.css` queda bajo el límite acordado o dividido por responsabilidad, sin lints bloqueantes.

### 297A-17 — Hardening, identidad, accesibilidad y SEO

**Depende de:** 297A-6–16 y de la revisión CSS anterior.

- [ ] Completar MFA/passkey, recuperación avanzada y threat review con casos negativos de sesión, CSRF, capacidades, pagos, grants y webhooks.
- [ ] Auditar SEO final: HTML público, sitemap, robots, canonical, metadata, Open Graph/Twitter y JSON-LD sin drafts ni rutas privadas.
- [ ] Verificar el manual visual en desktop, tablet y móvil, incluyendo claro/oscuro y los tamaños aprobados.
- [ ] Verificar teclado, foco, live regions, zoom 200%, reduced motion, alto contraste y multimedia accesible.
- [ ] Ejecutar E2E críticos, observabilidad real y el runbook de operación; deploy sigue fuera de alcance y no se usa SSH.

**Gate/salida:** checklist de hardening y accesibilidad evidenciado en navegador, tests y quality gate.

### 297A-9 — Validación visual completa del shell

**Depende de:** runtime y recetas visuales implementadas.

- [ ] Revisar shell, ventanas, taskbar, menú contextual, foco y estados en 1440×900, 1024×768, 390×844 y 320px.
- [ ] Repetir con zoom 200%, teclado y claro/oscuro; registrar cualquier overflow, foco perdido o contraste incorrecto.

**Gate/salida:** capturas y observaciones documentadas; no quedan regresiones visuales del chrome base.

### 297A-12 — Experiencia móvil tipo launcher

**Depende de:** 297A-9/11. Tablet conserva el escritorio.

- [ ] Ejecutar E2E visual/táctil en 320/360/390px y tablet 768px.
- [ ] Verificar long press/drag estable, orientación, safe areas, teclado virtual, foco, scroll y apps críticas.
- [ ] Confirmar refresh, transición móvil↔tablet y sincronización de URL sin duplicar el stack.

**Gate/salida:** launcher móvil funciona a los viewports definidos y conserva estado sin crear lógica paralela.

### 297A-19 — URLs canónicas, deep links y ventana enfocada

**Depende de:** 297A-9/11/12/13.

- [ ] Probar Back/Forward, `popstate`, refresh y transición desktop/tablet/móvil; la URL debe enfocar solo la ventana activa.
- [ ] Probar sesión limpia, varias ventanas, permisos, rutas inválidas, parámetros inseguros, scroll/formulario y deduplicación.
- [ ] Verificar `pushPath` solo en aperturas explícitas, `replacePath` en foco y eventos allowlisted sin datos privados.

**Gate/salida:** E2E de History API y deep links compartibles sin serializar IDs internos, tokens, posiciones ni overlays.

### 297A-22 — Reordenamiento por arrastre con grid

**Depende de:** 297A-12 y overlay.

- [ ] Validar visual/E2E 320/360/390/768+, foco y teclado.
- [ ] Confirmar long press, colisiones, reload/sync, persistencia de `mobilePosition` y transición móvil↔tablet.
- [ ] Verificar que Finder no herede el orden móvil y que move prev/next siga siendo alternativa accesible.

**Gate/salida:** orden móvil compacto y persistente, sin contaminar `position` desktop.

### 297A-21 — Notificaciones de novedades

**Depende de:** 297A-13 y releases versionados.

- [ ] E2E con overlay personalizado, deduplicación anti-spam, marcar leída, logout/login y dos dispositivos.
- [ ] Confirmar que campana, contador y panel admin respeten capacidades y no filtren eventos privados.

**Gate/salida:** una release pública produce una notificación idempotente y la lectura queda aislada por cuenta.

### 297A-13 — Registro, Cuenta y overlay remoto

**Depende de:** 297A-9/11/18.

- [ ] Ejecutar E2E real en dos pestañas/dispositivos y decidir/documentar merge semántico para cambios concurrentes no resolubles por local/remoto.
- [ ] Completar correo real, UI de verificación/token y habilitación controlada de registro; mantener tokens opacos, expirables y de un solo uso.
- [ ] Definir MFA/passkey de Cuenta y rate limit distribuido cuando exista infraestructura autorizada.
- [ ] Verificar logout/login, resolución de conflictos, reset, tombstones y corrupción persistida en todos los modos de presentación.

**Gate/salida:** Cuenta, preferencias y overlay sobreviven concurrencia y recargas sin filtrar secretos ni pisar cambios silenciosamente.

### 297A-14 — Programas editoriales

**Depende de:** 297A-9/10/11. Plan: `Agente/planes/plan-programas-editoriales-2026-07-31.md`.

- [ ] Ejecutar E2E visual desktop/tablet/móvil del vertical editorial completo: artículos/About, proyectos, productos, media, papelera, publicación, rollback y autosave.
- [ ] Cubrir permisos admin, estados draft/private/public, errores de red, teardown de apps lazy y acciones de toolbar.

**Gate/salida:** editores reutilizables y programas de contenido funcionan en las tres presentaciones sin carreras ni referencias rotas.

### 297A-15 — Comercio seguro

**Depende de:** 297A-7/10/14.

- [ ] Implementar y probar reembolsos y chargeback con autoridad server-side e idempotencia.
- [ ] Integrar scheduler/proveedor real y confirmar el worker de outbox con backoff, observabilidad y recuperación.
- [ ] Ejecutar E2E con Stripe/Resend o proveedores autorizados: checkout invitado, pago, webhook, entitlement, grant y descarga privada.

**Gate/salida:** el comprador recibe solo la versión adquirida; fallos de pago/webhook no conceden acceso ni duplican órdenes.

### 297A-16 — Analytics, estadísticas y retiro legado

**Depende de:** 297A-9/11–15.

- [ ] Completar revisión legal y E2E de consentimiento, retención, anonimización, purga y derechos operativos.
- [ ] Retirar CSS/clases legacy restantes con VarSense después de revisión visual; no eliminar clases dinámicas válidas.
- [ ] Verificar métricas de acciones, apps, ventanas, artículos, imágenes, compras, errores y releases sin `user_id` ni datos privados.

**Gate/salida:** eventos críticos son medibles, deduplicados y auditables; el panel no expone datos fuera de capacidad.

### 297A-24 — Cierre automático de ventanas

**Depende de:** 297A-19. La causa raíz y las regresiones automatizadas ya están corregidas.

- [ ] Prueba visual desktop/móvil para apertura canónica/no canónica, Perfil, refresh, Back/Home y varias ventanas.
- [ ] Confirmar que abrir una app nunca cierre otras y que solo una navegación real fuera del runtime permita cerrar el conjunto.

**Gate/salida:** no hay cierre destructivo por reconciliación de URL; taskbar, foco y ventanas permanecen coherentes.

### 297A-25 — Política de carga de apps pesadas

**Depende de:** 297A-9/11/12. ADR: `Agente/documentacion/arquitectura/adr-carga-apps-pesadas-2026-07-31.md`.

- [ ] Cuando exista la primera app WebGL/WASM/media avanzada, validar teardown GPU, concurrencia, Network, workers, timers, object URLs y memoria.
- [ ] Medir si hace falta `preload`/`heavy`; no activar flags sin una app real, métrica y ADR.

**Gate/salida:** la app pesada se carga lazy, libera recursos al cerrar y no degrada el arranque ni el resto del OS.

### 297A-29 — Escalar la app Configuración sin eliminarla

**Depende de:** 297A-27, 297A-28, 297A-13 y 297A-19. Las fases de fuentes estáticas, toolbar por capacidad y Perfil admin ya están implementadas.

- [ ] Fase 4: decidir si Configuración se convierte en panel de ajustes del sistema o se integra en otra app; conservar registro `settings`, nodo admin, menú y compatibilidad actual mientras se decide.
- [ ] Fase 5: diseñar, aprobar y solo entonces implementar un panel escalable; cualquier selector de fuente futuro debe usar tokens, persistencia y una receta compartida, sin reintroducir estado global duplicado.

**Gate/salida:** Configuración sigue funcionando durante la migración y una nueva acción admin se agrega por comando/capacidad, no mediante `if/else` en el shell.

## Revisión SOLID y escalabilidad obligatoria

Cada bloque pendiente debe evidenciar antes de cerrarse:

- **SRP/ISP:** chrome, contenido, persistencia, permisos y analítica permanecen separados; interfaces exponen solo lo necesario.
- **OCP/DIP:** nuevas apps, recursos, comandos, temas y breakpoints se agregan por registros/adaptadores, sin duplicar shell ni listeners.
- **Límites:** componentes/CSS ≤300 líneas, lifecycle/store/hook ≤120 y utils ≤150; una excepción requiere justificación en Sentinel.
- **Contratos y seguridad:** tipos, DTOs, errores, capacidades, eventos y resultados son explícitos; sin HTML inseguro, I/O silencioso, N+1, secretos o estado privado en URL.
- **Escalabilidad:** probar un segundo caso real, revisar índices/paginación/cache, teardown, migración y rollback.
- **Calidad:** Sentinel/VarSense, type-check, tests, navegador y `task:check` pasan; los fallos preexistentes se documentan y no se silencian.

## Reglas operativas no negociables

- Un bloque entregable usa commit explícito con ID; un diagnóstico/prototipo compartido no se fuerza a commit, pero el reporte debe recordar revisar `git status`.
- Todo deploy, restart, logs, backup, restore, health o exec de producción pasa por `coolify-manager-rs`; nunca SSH/docker/scp/curl directo.
- Comandos largos tienen timeout y señal de readiness; no se espera indefinidamente ni se reintenta a ciegas.
- Tras cada commit y antes de cerrar una sesión se relee este roadmap completo.
