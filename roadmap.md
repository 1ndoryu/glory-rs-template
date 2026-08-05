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
- Motor agnóstico para futuros juegos: `Agente/planes/plan-glory-render-motor-juegos-2026-08-01.md`
- ADR del repositorio `glory-render`: `Agente/documentacion/arquitectura/adr-glory-render-repositorio-agnostico-2026-08-01.md`
- Deep links: `Agente/planes/plan-deep-links-ventanas-2026-07-31.md`
- Apps editoriales: `Agente/planes/plan-programas-editoriales-2026-07-31.md`
- Interacción y medición: `Agente/planes/plan-contratos-interaccion-comandos-medicion-2026-07-29.md`
- Guard de ejecuciones pesadas y targets Cargo: `Agente/planes/plan-heavy-run-guard-2026-08-02.md`
- Sentinel global agnóstico por proyecto/rama (incluye guard/orquestación; VarSense como analizador): `Agente/planes/plan-global-quality-guard-agnostico-2026-08-02.md`
- Optimización Sentinel/VarSense: `Agente/planes/plan-optimizacion-sentinel-varsense-2026-08-02.md`

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
- **Correcciones recientes relevantes:** se resolvieron la ruta legacy `/admin`, visibilidad editorial de proyectos (018A-83), contratos de URL/autosave, select nativo, `createEl` para `textarea`, Reader TipTap, sincronización del Finder, iconos por registro único, rejilla compacta y bordes/flechas del tema oscuro. La carpeta vacía "Galería" se sustituyó por "Documentos" con subcarpetas por tipo y sync de media al workspace (018A-87): los archivos subidos aparecen en el Finder, se abren con visor, se retiran al moverlos a la papelera y se restauran. El menú contextual ahora funciona dentro de las carpetas con acciones de creación (nuevo artículo/proyecto/producto, subir archivo, nueva carpeta, pegar) y el clic en ítems del Finder y del escritorio muestra selección visual con los tokens del OS (018A-88). El clic derecho dentro de las carpetas responde en todo el alto del panel del Finder, no solo sobre los ítems (018A-89). El menú sobre una carpeta dentro del Finder ofrece gestión completa — Abrir, Renombrar, Cortar, Copiar, Pegar en y Eliminar con borrado seguro (confirmación + subárbol restaurable) y `Ctrl+V` con destino (018A-90); el crear permanece en el fondo. La restauración de sesión conserva el chrome inferior de las apps (`MountedView.actions`) validado visualmente en desktop y tablet sin duplicar ventanas ni alterar geometría/taskbar/URL (018A-69). El fallback local del prototipo Bosque/Bosque 3D evita que un release local anterior al registro oculte los accesos durante desarrollo, sin sobrescribir la organización del release ni activar apps en producción (018A-92). Una carpeta vacía del Finder ya no muestra texto (el grid queda en blanco y el clic derecho sigue abriendo el menú) y la barra de ruta tiene botón "volver a la carpeta anterior" con historial de navegación, deshabilitado en la raíz (018A-91). El grid del Finder alinea sus iconos al inicio, igual que el grid del escritorio, en vez de centrarlos (018A-93). La selección del Finder ya no se refleja en el escritorio (y viceversa): el `selectionStore` se escala por superficie (`desktop`/`finder`) y cada una solo refleja su propia selección, sin romper copiar/cortar (018A-95). Los planes GAME-01 y de assets/terreno ahora exigen auditoría de SOLID, rendimiento, escalabilidad, seguridad, observabilidad y accesibilidad al cierre de cada fase, con evidencia antes de avanzar (018A-94). GAME-01 añadió el fixture `game-playable` lazy/full-bleed: movimiento offline con `game-core`, cámara limitada, teclado/D-pad, pausa background y teardown WebGL; type-check, 40 tests, build, diff-check y navegador en `/forest-playable` pasan. Después se añadió el contrato puro `MapVersion` con terreno por chunks, manifiesto de assets, instancias, spawns, cuotas fail-closed, adaptación a colliders X/Z y 41 tests; build/diff-check y navegador siguen verdes. Endpoint, validación server-side, persistencia, realtime, identidad, editor y mediciones repetidas de GPU/memoria siguen pendientes. Gobernanza del escritorio (038A-2): la Papelera y los nodos de sistema (`trash`, `admin`, `settings`, `profile`, `about`) no pueden eliminarse (guard backend en `validate_release_tree` + guards frontend `tombstoneNode`/`tombstoneSubtree`/`workspace:trash`), y el contenido publicado (artículos/medios ready/public/active) SIEMPRE se materializa en la release efectiva server-side (`find_public_content` + `materialize_content_nodes`, sin mutar la release) bajo Notas/Documentos, para cualquier versión activa y cualquier cliente — solo desaparece con eliminación real en BD; `ArticleService::update` sincroniza el envelope al publicar/despublicar, y el borrado de artículos es soft delete transaccional con papelera y restore (028A-12). Validado por stack (`cargo build`/`--tests` EXIT 0, frontend sin errores TS); gate diferido por submódulo `tools/sentinel` sucio del hilo 028A-6. Los detalles y gotchas permanecen archivados.

## Siguiente bloque habilitado

**018A-66 — Separar overlay personal de la sesión admin.** El código está cerrado (`52bf6e0c`): `overlay-sync` corta la sincronización con capacidad admin (clearOverlaySync) y la UI de conflicto cierra el modal en `render` si la sesión pasa a admin (guardia anti-flash ante órdenes de notificación distintos). Se añadió cobertura de la guardia UI (`overlay-conflict-ui.test.ts`): admin + estado conflict → sin modal; cuenta normal + conflict → modal abierto y se cierra al pasar a admin vía clearOverlaySync; sin conflict → sin modal. 89/89 tests del workspace y type-check PASS. **Pendiente de validación en navegador** (requiere sesión admin real del usuario): login, logout y recarga con admin sin modal de conflicto ni aviso `workspace actualizado`; con cuenta no-admin el conflicto solo aparece ante revisiones local/remota incompatibles. Después se continúa con hardening/E2E.

### 028A-3 — Guard global de ejecuciones pesadas y limpieza de targets

**Prioridad:** P0 antes de seguir acumulando validaciones Rust. El quality gate ya tiene cooldown y el shim CMD; queda revisar/autorizar la carga del interceptor en perfiles PowerShell sin sobrescribir configuración ajena.

- [x] Limitar `--full`, `cargo test`, `cargo clippy` y `cargo bench` a una ejecución por proyecto cada 3 horas, con un único proceso pesado simultáneo.
- [x] Degradar un full bloqueado a `local-light` y dejar la razón, hora de reintento y comando de excepción en el reporte.
- [x] Interceptar `cargo` a través de `run-with-db` y del shim `cargo.cmd`; los comandos ligeros siguen pasando sin compilar tests.
- [x] Mantener `C:\tmp\glory-target` bajo cuota de 15 GB y retención de 7 días, preservando targets con marcador de proceso activo.
- [x] Validar el guard con tests unitarios, `quality:test` y limpieza en dry-run; limpiar los targets antiguos detectados (se liberaron aproximadamente 29 GB).
- [x] Revisar el perfil PowerShell 7/Windows PowerShell y ejecutar `quality:install-guard -InstallProfile` con autorización explícita; backups y rollback quedaron registrados.
- [x] Bloquear validaciones directas (`npx vitest`, `npm run test:*`, type-check/lint/build y Cargo de validación) desde PowerShell/CMD; todas recomiendan `npm run task:check -- <TareaId>`.
- [x] Cubrir Bash/Git Bash y shells no interactivos: instalar `global-quality-guard.sh` en `.bashrc`/`.bash_profile` y `BASH_ENV`, con resolución por workspace/rama; bloquear también `rustfmt` directo.

**Gate/salida:** ningún agente puede iniciar accidentalmente un full o `cargo test` durante el cooldown desde los wrappers PowerShell/CMD/Bash disponibles; el uso de `--allow-heavy` queda visible en reportes y la cuota de targets se mantiene sin borrar procesos activos. Invocaciones con ruta absoluta y shells iniciados con `--noprofile --norc` quedan fuera del alcance del interceptor y deben bloquearse en la capa de ejecución del agente, no mediante un script de proyecto.

### 028A-6 — Guard global agnóstico por proyecto y rama (migración incremental)

**Depende de:** 028A-5 y de aprobar `Agente/planes/plan-global-quality-guard-agnostico-2026-08-02.md`.

- [x] Definir y validar localmente la política v2 en `scripts/quality/policy.mjs`, sin reutilizar silenciosamente el `sentinel.config.json` v1 del analizador.
- [x] Añadir `quality:doctor -- --migrate --dry-run`; produce migración en memoria, `writes: []` y no modifica perfiles/archivos.
- [x] Hacer que el guard de transición aplique `enforce`/`observe`/`pass-through` para una política v2 válida y conserve fallback seguro para v1/legacy.
- [x] Cubrir el contrato con 60 tests de quality y fixtures de claves desconocidas, paths inseguros, modos, wildcard, migración, lockfile, checkout modificado e identidad de caché/reporte.
- [x] Añadir `policyHash`/identidad de política al reporte y fingerprint de caché; cambiar la política invalida PASS anteriores.
- [x] Añadir `sentinel.lock.json` con runtime/analyzers fijados, versión/protocolo/commit/hash, patch local declarado, validación preflight, `git archive` reproducible y rechazo de checkouts modificados; runtime global queda explícitamente `project-adapter`.
- [x] Añadir generador local del lock (`quality:lock --check|--write`, `quality:doctor --lock`) con modo solo lectura, backup `.bak`, escritura atómica y protección contra symlinks; no instala runtime ni modifica analyzers.
- [x] Alinear el inventario documental local: README raíz, `roadmap-sentinel.md`, matriz de paridad e índice canónico describen el gate, `varsense all`, lockfile, branch-key, retención y límites del runtime global.
- [x] Completar `doctor --migrate --dry-run`: mapea Sentinel v1, quality, VarSense y tools a un preview aditivo, rechaza claves desconocidas y conserva `writes: []` sin modificar contratos.
- [x] Definir precedencia local de perfiles: `--profile` CLI > `GLORY_QUALITY_PROFILE` > autodetección; ambos solo aceptan perfiles declarados y un perfil explícito filtra etapas incluso con `--full`/`--ci`.
- [x] Cerrar fuente canónica local: únicamente `sentinel.config.json` ancestro determina política/hash; `AGENTS.md`, `quality.config.json` y scripts auxiliares no se interpretan como reglas. Política inválida falla doctor/CI sin bloquear comandos desconocidos.
- [ ] Extraer el runtime y los shims a una instalación estable fuera de cualquier repositorio o rama. *(bloqueado: runtime/repos upstream no presentes en este checkout)*
- [ ] Migrar wandori.us al runtime global sin duplicar reglas ni dejar rutas hardcodeadas en perfiles. *(depende de la anterior)*
- [ ] Probar matriz multi-proyecto/multi-rama en PowerShell 5/7, CMD, CI, pipes y códigos de salida, con rollback. *(depende de runtime global)*

**Gate/salida del tramo local:** contrato v2, dry-run, guard y tests pasan; el gate global multi-proyecto/multi-shell no se declara cerrado hasta instalar el runtime fijado y ejecutar la matriz externa.

### 028A-8 — Optimización medible de Sentinel y VarSense (en curso, tramos 1–4 cerrados)

**Depende de:** 028A-5 y 028A-6. Plan canónico: `Agente/planes/plan-optimizacion-sentinel-varsense-2026-08-02.md`.

- [x] Corregir la degradación full→local-light para que el alcance efectivo no siga ejecutando análisis completo tras el cooldown: `scope.mjs` separa requested/automatic/effective/fullReason/heavyDeferred y `task-check.mjs` adquiere el lease pesado también para automaticFull; un full diferido degrada a local-light real (verificado en gate 028A-8 con `full · ejecución incremental (heavy-deferred)`).
- [ ] Compartir un manifiesto de alcance/hashes entre etapas y añadir métricas de descubrimiento, parseo, caché, archivos reutilizados, RSS y CPU. *(el `scope-manifest.json` único con hashes ya está emitido y `run-frontend-tests` lo consume; quedan las métricas por etapa)*
- [ ] Añadir modo incremental de VarSense con `--files-from`, índices persistentes de variables/clases y invalidación por dependencias. *(el adapter `--files-from` y el índice persistente están fijados: upstream `11f0932` en el `main` consumido, `capabilities.persistentIndex=true`, lock regenerado y gate real con reutilización (loaded=363, reused=364, reparsed=0); queda la selección de dependencias con el índice inverso para ampliar `--files-from` con consumidores)*
- [ ] Optimizar Sentinel con caché por archivo/índice global, sin duplicar reglas con custom ni reducir cobertura.
- [ ] Validar p50/p95, paridad CLI/LSP/VS Code/Zed, rollback y una matriz con otro proyecto/estructura.

**Gate/salida:** local-light representativo ≤12 s p95, VarSense incremental ≤3 s p95 sin cambios globales, Sentinel incremental ≤3 s p95 y full CI conserva paridad.

### 028A-5 — Novedades: popover de campana + admin "novedades" con borrado

**Depende de:** 297A-21 (notificaciones) y las recetas de popover/modal del OS.

- [x] La app `notifications` deja de ser una ventana: la campana (desktop y launcher móvil) abre un popover compacto de ~300px anclado, con lista de hasta ~40 avisos, scroll de ~260px, recargar, marcar todas y cierre por clic fuera/Escape.
- [x] El admin de novedades vive en la app Admin como tab "novedades": listado de avisos, modal "+ nuevo aviso" y cambio de estado con `createSelect`.
- [x] Borrado de avisos (incluidos los publicados) con confirmación; las lecturas se limpian en cascada (`ON DELETE CASCADE`).
- [x] El tag de estado muestra etiqueta en español (borrador/publicado/archivado) en una línea (`white-space: nowrap`).
- [x] Retiro del código legacy (`notifications-view.ts`, `notifications-admin.ts`) y de los estilos `.notificaciones`/`.notificacionesAdmin`.

**Gate/salida:** popover y tab "novedades" validados en navegador (crear, cambiar estado y borrar un aviso publicado); type-check, clippy y suite frontend verdes.

### Plan de gobernanza del workspace (028A-10..14)

**Fuente canónica:** `Agente/planes/plan-gobernanza-workspace-2026-08-02.md` (aprobado). Objetivo: dar al admin control real del escritorio desde la app Admin (release activo, validación, publicación) y garantizar coherencia para que nada borrado o en draft aparezca en la siguiente release.

- [x] **028A-10 — Release v3 con árbol canónico:** la migración `20260802010000_028a10_release_v3` publica v3 con `documentos` + 4 subcarpetas, `projects`, `profile`, `about`, `settings`, `admin`, `trash` (Papelera), `store`, `orders` y `downloads`; excluye `snake` (nodo fantasma) y `game/game3d/gamePlayable` (prototipos GAME-01 ocultados). Gate PASS: `task:check -- 028A-10` (sentinel/varsense/rust/frontend/custom), `GET /api/workspace/release` = v3 con 14 nodos, navegador desktop + launcher móvil muestran Papelera/Tienda/Pedidos/Descargas y la app Papelera renderiza vacía.
- [x] **028A-15 — Ejecutar los tests de integración del guard de publish (028A-11):** `cargo test --test workspace_publish` confirmado contra BD de rama: 4/4 PASS. Requirió corregir el fixture del test de summary (`publish_accepts_valid_tree_and_computes_summary`): [038A-2] exige el id canónico `about` del guard de sistema, pero el test pasaba `about-{uniq}` (el guard rechazaba el release con "El release debe contener el nodo de sistema 'about'"); `about` ya existe en la v1 sembrada, así que no entra en `added` y `nodeCount` es 7 (5 nodos de sistema + folder + recurso). El gate `task:check -- 028A-11` no pudo ejecutarse por el submódulo `tools/sentinel` sucio (otro hilo), mismo falso positivo conocido; la evidencia queda en los 4/4 tests HTTP contra PostgreSQL.
- [x] **028A-12 — Unificar el borrado de artículos (eliminar nodo fantasma):** soft delete transaccional (`trashed` + `deleted_at`, migración `20260805000000_028a12_article_soft_delete`), `GET /api/admin/articles/trashed` + `POST /api/admin/articles/{id}/restore`, sync del envelope `resources` (en `update_resource_metadata` con COALESCE y `soft_delete_kind_tx`/`restore_kind_tx`), evento `ArticleEditorSavedEvent.operation='deleted'` publicado al borrar desde el admin y tombstone del nodo (`removeArticleNode`) en `article-notas-sync.ts` + retiro directo del nodo al recibir el evento `deleted`. Test integral `tests/article_soft_delete.rs` (crear→borrar→papelera→envelope trashed→restore→active). Validado por stack (`cargo build`/`--tests` EXIT 0, frontend sin errores TS); gate diferido por submódulo `tools/sentinel` sucio (hilo 028A-6).
- [x] **028A-13 — Backend de gobernanza:** `GET /api/admin/workspace/control`, `POST /api/admin/workspace/releases/{version}/validate` (dry-run), `POST /api/admin/workspace/releases/{version}/activate`, DTO ligero de `list_releases`; OpenAPI/Orval + tests de permisos (AdminUser/CSRF). Migración `20260803000000_028a13_release_activation` (columna `is_active` + índice único parcial `idx_workspace_releases_active`). DTOs con `rename_all="camelCase"` (bug snake_case detectado y corregido al probar en vivo). Gate PASS `task:check -- 028A-13`. Verificado: validate/activate/control responden 200, activar v3 restaura la Papelera (14 nodos).
- [x] **028A-14 — Panel "Escritorio" en la app Admin:** estado actual (control), historial de versiones con badge activa/nodos/fecha, validar (dry-run con issues/brokenRefs en detalle), activar (con confirmación y guard de validez) y publicar (`publishWorkspace`); tab `escritorio` primero en la app Admin. Directory `frontend/src/pages/admin-workspace.ts` con patrón WeakMap + guard de generación. RESUELTO incidente observable: el panel detectó "sin versión activa" (mutación externa dejó v1..v5 inactivas), validó y activó v3 → `GET /api/workspace/release` volvió a servir v3 con 14 nodos incl. `trash`. Gate PASS: `task:check -- 028A-14` (30 archivos).

## Pendientes ordenados

### 038A-2 — Duración por etapa en el quality report

**Depende de:** nada (solicitud directa del usuario). El orquestador ya medía `durationMs` por etapa en `latest.json`; faltaba renderizarlo en el Markdown y en la salida compacta.

- [x] Renderizar `stage.durationMs` en `latest.md` con `formatDuration` (ms < 1s, s con 1 decimal) y duración total legible en `scripts/quality/reporter.mjs`.
- [x] Añadir la duración por etapa a la salida compacta de terminal (`compactLines`) sin añadir líneas (mantiene el límite validado por `reporter.test.mjs`).
- [x] `npm run quality:test` 45/45 PASS; render verificado con `task:check` (sentinel 6.7s, varsense 10.1s, rust 26.1s, frontend 6.0s, docs 10ms, custom 81ms).

**Gate/salida:** el reporte Markdown muestra la duración de cada etapa y la salida compacta también; tests del orquestador verdes. Gate completo del repo pendiente de errores TS ajenos en `about.ts`/`admin.ts` (otro frente en curso).

### 038A-4 — Inventario de documentación desactualizada + README de Sentinel alineado con v0.4.0

**Depende de:** nada (solicitud directa del usuario). Origen: el plan 028A-6 no contemplaba qué documentación corrige la migración a plano global.

- [x] Añadir al plan `028A-6` la sección "Documentación afectada e inventario de correcciones" con todos los MDs desactualizados (README/help.txt/rules.md/CHANGELOG de sentinel, README/CHANGELOG de varsense, sincronización de repos dev con commits fijados, README raíz, roadmap-sentinel, matriz-paridad, índice de documentación).
- [x] Reescribir `code-sentinel/README.md` eliminando la era IA (análisis IA, toggle IA, config `aiAnalysis.*`, alias Gemini; todo eliminado en 0.4.0) y documentando el estado real: CLI `analyze` + `--files-from`, exit codes 0/1/2, JSON `schemaVersion: '1'`, validación estricta de `sentinel.config.json` (incl. `portableBoundaries`), catálogo completo de reglas del `ruleRegistry` y rol de plano global. Commit `95ac5b0` en `1ndoryu/glory-sentinel`.
- [x] Sincronizar el README actualizado al `main` externo de `glory-sentinel` (`9f4ed4d`), que es el checkout consumido por el gate mediante `sourcePathEnv`.

**Gate/salida:** README de sentinel sin restos de IA y coherente con el código fijado; el resto del inventario queda planificado en 028A-6 para implementarse con cada fase.

### 028A-16 — Auditoría del uso de excepciones del guard (prevención cooldown)

**Depende de:** coordinar con `Agente/planes/plan-heavy-run-guard-2026-08-02.md` (el otro agente posee `scripts/quality/`). Fuente: `Agente/prevencion/prevencion-cooldown-guard-2026-08-02.md`.

- [ ] Registrar en un log auditable (`.quality-reports/heavy-overrides.log`) cada activación de la excepción (`--allow-heavy`, `GLORY_QUALITY_ALLOW_HEAVY`, `GLORY_HEAVY_RUN_TOKEN`): timestamp, source, comando completo, cwd, PID.
- [ ] Exigir motivo (`--heavy-reason "<motivo>"`) para activar la excepción y mostrarlo en el reporte del gate.
- [ ] El agente solo usa las excepciones del guard con autorización explícita del usuario en el mismo turno; nunca para "no esperar".

**Gate/salida:** cualquier uso de la excepción queda trazado y visible; el agente no intenta saltarse el cooldown sin autorización explícita.

### GAME-01 — Bosque multijugador 3D dentro del OS (planificado, bloqueado)

**Depende de:** cerrar el bloque habilitado actual y sus gates de runtime, sesiones/capacidades, workspace, carga lazy y validación visual. Plan canónico: `Agente/planes/plan-juego-bosque-multijugador-2026-08-01.md`.

- [ ] Aprobar ADRs de identidad temporal, salas, contrato realtime, presupuesto y mapa versionado; el renderer Three.js, los assets GLB externos y el terreno lógico 2D ya tienen ADR aprobado.
- [ ] Implementar por fases: app lazy/Three.js, mapa finito por chunks, sala server-authoritative, presencia, personaje, `Assets 3D`, editor admin 2D y publicación.
- [ ] Mantener el objetivo inicial en salas de 8 jugadores, snapshots a baja frecuencia, interés por proximidad y salas bajo demanda.
- [ ] Validar teardown al cerrar, límites de mensajes/mapa/assets, permisos server-side, reconexión y rollback de versiones.
- [x] **297A-26 — Contrato frontend de MapVersion y fixture offline:** `game-core/map-version.ts` valida terreno por chunks, manifiesto de assets, instancias, spawns, bounds, transforms, referencias e IDs reservados; `game-playable` consume el adaptador `MapVersion → WorldMap`. Gate PASS: Sentinel/VarSense, type-check, 41 tests, build, diff-check y navegador `/forest-playable`. Backend, persistencia, endpoint, realtime, identidad, editor y mediciones GPU/memoria siguen pendientes.
- [x] **297A-27 — Contrato MapVersion compartido frontend/backend:** Rust añade `models::game_map::MapVersion` con JSON camelCase, `deny_unknown_fields`, proxy opcional, validación fail-closed de cuotas/bounds/chunks/referencias/transform/spawns y parseo acotado por bytes; frontend rechaza campos desconocidos en los mismos niveles. Gate PASS: `cargo fmt --check`, `cargo check`, 9 tests Rust, type-check, 23 tests frontend, build y diff-check PASS. No incluye endpoint, persistencia, publicación, realtime, identidad, editor ni límite de profundidad HTTP.
- [x] **297A-28 — Lectura pública persistida de MapVersion:** migración `game_map_versions`, snapshot JSONB inmutable con límite de tamaño, hash SHA-256, índice de una versión activa, repository/service/handler `GET /api/game/maps/:map_id` y envelope público sin campos administrativos. Gate PASS: Sentinel/VarSense, `cargo check`, 11 tests `models::game_map`, type-check, 23 tests frontend, build y diff-check.
- [x] **297A-30 — Publicación admin versionada de MapVersion:** `POST /api/admin/game/maps` con `AdminUser`/CSRF, `expectedVersion`, canonicalización/hash, advisory lock, activación atómica, límite de body de 4 MiB y request OpenAPI. Implementación cerrada y validada; sin realtime, identidad invitada ni editor.
- [x] **297A-31 — Fixture e integración real de publicación MapVersion:** `tests/game_map_publish.rs` cubre 401, admin/no-admin, CSRF, `mapId` incoherente, persistencia + GET público, 413, stale revision, segunda versión, concurrencia, una sola activa y trigger UPDATE/DELETE; migración incremental fija `published_by` como autoría inmutable con `ON DELETE RESTRICT`. Gate parcial: 7/7 tests PostgreSQL reales PASS, `cargo check --tests`, formato y diff-check PASS. Los snapshots de test quedan con IDs únicos por diseño inmutable; sin chunks visibles, realtime, identidad invitada ni editor.
- [x] **297A-32 — Selección visible y medición local del fixture:** `MapChunkCache` indexa instancias por chunk, calcula la ventana con bounds relativos, limita chunks/instancias/assets y aplica eviction LRU; `FramePerformanceMonitor` reporta p50/p95/max y frames sobre presupuesto. El renderer carga/retira props del fixture y libera geometrías al retirar objetos. Gate frontend PASS: type-check, 33 tests dirigidos, build y diff-check.
- [x] **297A-33 — Terreno visible por chunks y cache visual:** `buildTerrainMeshData` genera posiciones/índices/superficies puros y `GamePlayableVisualCache` materializa `BufferGeometry` solo para chunks visibles. Props repetidos reutilizan geometría/materiales mediante prototipos y `clone(true)`; teardown idempotente libera terreno y prototipos. Gate frontend PASS: type-check, 36 tests dirigidos, build y diff-check.
- [x] **297A-34 — Batching InstancedMesh y métricas locales del renderer:** props sólidos repetidos se agrupan por tipo en `THREE.InstancedMesh` con límite de 128 instancias, los contornos conservan la gramática visual y aplican la transformación real de `AssetInstance`. El fixture expone draw calls, triángulos, geometrías, texturas y heap JS opcional mediante `data-*`; 40 tests, type-check, build y diff-check PASS.
- [x] **297A-35 — Cache visual persistente y culling por batch:** el terreno visible se mantiene en un LRU visual de hasta 12 chunks, se retira de la escena sin destruirse durante evictions temporales y solo libera geometría al superar el límite; los `InstancedMesh` activan `frustumCulled` y actualizan `boundingSphere` tras cambiar matrices. 42 tests, type-check, build y diff-check PASS.
- [x] **297A-36 — Presupuesto local medible del renderer:** `evaluateGamePerformanceBudget` evalúa p95 de frame, draw calls, triángulos, geometrías, texturas y heap JS opcional con estados `pass`/`fail`/`unknown`; exige 30 muestras para frame, no trata `renderer.info` ausente como cero y publica únicamente `data-renderer-budget-*` locales. 17 tests dirigidos, type-check, build y diff-check PASS. No representa memoria GPU física ni abre realtime.
- [x] **297A-37 — Diagnóstico WebGL y pérdida de contexto:** `detectWebGL` prueba WebGL2/WebGL, libera el contexto temporal cuando existe `WEBGL_lose_context` y el fixture muestra fallback accesible antes de montar Three.js. El controller escucha `webglcontextlost` sobre el canvas real, detiene RAF y libera listeners/input/scene al cerrar. 23 tests dirigidos, type-check, build y diff-check PASS; no sustituye una medición física de GPU.
- [x] **297A-38 — Lazy loading y lifecycle repetido del fixture:** `AppRegistry.isLazy('game-playable')` verifica que el registro no resuelve la app pesada antes de instanciarla. Las pruebas cubren 12 abortos antes del montaje y 12 ciclos reales de mount/destroy con handles independientes, sin acumular input, escena ni RAF. 36 tests dirigidos, type-check, build y diff-check PASS. La evidencia es de carga/lifecycle lógico, no de memoria GPU física.- [x] **297A-39 — Contrato realtime v1 sin transporte:** `game-realtime.ts` y `models/game_realtime.rs` alinean envelope versionado, join con ticket opaco, intents por secuencia, heartbeat/ack, snapshots filtrados, errores allowlisted, límites de bytes/frecuencia y validación fail-closed. Se cubren campos desconocidos, UTF-8 inválido, Unicode por puntos de código, controles C0/C1/DEL, secuencias replay/jump, timestamps negativos, entidades duplicadas y posiciones finitas. Frontend: type-check, 26 tests dirigidos y build PASS; Rust: fmt, check y 8 tests PASS; no incluye upgrade WebSocket, identidad invitada, salas ni autoridad de movimiento.
- [x] **297A-40 — Ticket de juego firmado sin transporte:** `services/game_ticket.rs` emite y consume tickets `g1.game` ligados a UUID server-side, con HMAC, TTL por defecto de 30 s, máximo de 60 s, límite de 512 bytes, nonce y consumo single-use acotado a 4096 entradas. Tests cubren manipulación, secreto incorrecto, propósito, UUID, expiración, reloj inválido, replay, poda y token sobredimensionado. Rust: fmt/check y 7 tests del servicio PASS; no incluye endpoint HTTP, upgrade WebSocket, hub Glory, identidad invitada ni salas.
- [x] **297A-41 — Emisión HTTP autenticada del ticket:** `POST /api/game/ticket` usa `AuthUser` y CSRF, resuelve el subject UUID server-side mediante `GameTicketStore`, responde solo `{ ticket }`, mantiene el UUID fuera del token y falla cerrado si falta `GLORY_GAME_TICKET_SECRET`. Incluye configuración, OpenAPI, router con estado compartible y 3 pruebas HTTP reales en PostgreSQL temporal migrado; fmt/check, tests unitarios, integración HTTP, export OpenAPI y diff-check PASS. No incluye upgrade WebSocket, hub Glory, identidad invitada, salas ni autoridad de movimiento.
- [x] **297A-42 — Frontera de upgrade WebSocket del juego:** `/api/game/ws` acepta el upgrade sin ticket en query/cookie, exige `join` como primer mensaje, aplica límite global de conexiones, timeout de handshake de 5 s, validación del envelope realtime, consumo single-use del ticket opaco y cierre fail-closed. Una conexión autenticada aún recibe `map_unavailable` porque no existe actor/sala; no se reutiliza el hub Glory `i32`. Rust: fmt/check y tests dirigidos PASS; el handshake TCP real, actor de sala, snapshots y movimiento quedan pendientes.
- [x] **297A-43 — Prueba TCP real del upgrade WebSocket:** servidor Axum efímero en `127.0.0.1:0` y cliente `tokio-tungstenite` verifican upgrade real, `join` válido con `map_unavailable`, replay `unauthorized`, primer mensaje inválido `invalid_message`, cierre y capacidad global HTTP 409. Los recursos del servidor de test se apagan con graceful shutdown y `JoinHandle`; fmt/check, 4 tests TCP, tests dirigidos y diff-check PASS. No incluye actor de sala ni movimiento.
- [x] **297A-44 — Actor de sala server-authoritative:** `GameRoomState` crea una sala single-instance bajo demanda con cap 8, TTL configurable de sala vacía, actor Tokio de propietario único, backpressure bounded, teardown prioritario, mapa inmutable con hash verificado y spatial index con presupuesto de referencias. El handler enlaza tickets single-use con `joined`, snapshots filtrados por interés, heartbeat, intents `move` validados por secuencia/rate limit y cierre seguro; `GAME_MAP_ID` carga el snapshot publicado y los tests pueden inyectar un fixture válido. Gate técnico PASS: fmt/check, actor, handler, contrato realtime, 7 tests TCP, TTL/recreación, capacidad, colisión y diff-check.
- [x] **297A-45 — Cliente realtime autenticado del Bosque:** `game-playable` conserva fallback offline público y, para cuentas autenticadas, solicita ticket server-side, conecta `/api/game/ws`, envía intents/heartbeat, valida snapshots, interpola entidades, usa el `playerId` efímero del servidor para el avatar local y libera socket/timers/listeners al destruir la vista. Gate técnico PASS: type-check, 20 tests frontend, build y diff-check; la validación visual de navegador queda pendiente porque la automatización no produjo una sesión/pestaña válida. No incluye invitados ni reconexión persistente.
- [x] **297A-46 — Harness de medición realtime 1/4/8 clientes:** `tests/game_ws_benchmark.rs` levanta el router WebSocket real con fixture de mapa y reporta p50/p95 de `joined`/primer snapshot, snapshots, mensajes y bytes de payload por escenario. Gate PASS: con `CARGO_TARGET_DIR=C:/tmp/glory-target/game_ws_benchmark_check`, `cargo check --test game_ws_benchmark` y `cargo test --test game_ws_benchmark -- --ignored --nocapture` pasan; 1/4/8 clientes completan 1/1 test en 6.56 s. La monitorización externa observó pico de 56.34 MiB working set y 1.031 s de CPU acumulada; los bytes siguen siendo payload JSON, no tráfico físico. El harness continúa fuera de la suite normal y no sustituye una prueba distribuida.
- [x] **297A-47 — Identidad temporal de invitados:** `POST /api/game/ticket` acepta cuenta autenticada con sesión/CSRF o invitado temporal server-side. La cookie `guest_game` es opaca, HMAC, `HttpOnly`, `SameSite=Strict`, TTL 2 h y store acotado a 4096 identidades; el rate limit por IP devuelve 429 y una sesión inválida nunca degrada a invitado. El cliente `game-playable` usa el mismo realtime para cuenta/invitado. Gate técnico: `cargo fmt --check`, `cargo check --tests`, 9 tests unitarios, type-check y 8 tests frontend PASS; 2 pruebas HTTP de invitado PASS. La integración de cuenta queda pendiente de ejecutar contra BD de pruebas migrada.
- [x] **297A-48 — Perfil persistente de cuenta del juego:** `user_game_profiles` guarda únicamente el nombre visible allowlisted de cuentas autenticadas. `GET/PUT /api/game/profile` usa `AuthUser`, CSRF, JSON estricto, revisión optimista y UPSERT transaccional; invitados reciben 401 y el DTO no expone `user_id`. Gate técnico: `cargo fmt --check`, `cargo check --tests`, 4 tests HTTP PostgreSQL, 2 unitarios y `git diff --check` PASS; la integración dentro de `game-playable` y el catálogo de personajes quedan para el siguiente bloque.

- [ ] GAME-01 restante: cierre visual/operativo pendiente (validación de navegador del fixture con datasets GPU/métricas y full CI `task:check --full` tras el cooldown). La presencia avanzada queda cerrada por `297A-77` (fix de resync tras reconexión y `characterId` en el snapshot realtime con tono por personaje en remotos y local). Las Fases 4 y 8 quedan cerradas por `297A-74` (culling avanzado por distancia, batching por materiales y probe físico de GPU/memoria) y `297A-75` (dos salas concurrentes, métricas agregadas `GET /api/game/metrics` y runbook de rollback). La reclamación invitado→cuenta queda cerrada por `297A-76` (revocación server-side de la identidad temporal + limpieza de la cookie `guest_game` en login/logout; nada se transfiere, el perfil de la cuenta aplica). `297A-72` añade Assets 3D (backend): `game_asset_versions` inmutables por hash (content-addressed bajo `upload_dir/assets/{hash}.glb`), importación de GLB vía multipart con validación de magic/versión/tamaño (16 MiB), numeración secuencial, metadata allowlisted (proxy circle/aabb + scale 0.1..4) editable solo en versiones inactivas, activación única (desactiva las demás y congela la versión por trigger), contrato público de la versión activa (`{assetId}-v{version}` sin storage paths) y auditoría `asset.version.created/updated/activated` transaccional; 6/6 tests HTTP PostgreSQL PASS (más 9/9 de regresión de catálogo admin/público). `297A-71` persiste el borrador del mapa: tabla `game_map_drafts` (un borrador por mapa con revisión optimista), `GET/PUT /api/admin/game/maps/:map_id/draft` con `AdminUser`/CSRF, validación completa del documento (mismo camino que publicar), 409 ante revisión obsoleta, 413/422/404 fail-closed y publicación que elimina el borrador en la misma transacción (la versión publicada pasa a ser la base); el editor carga el borrador si existe (si no, publicación activa → fixture), expone el botón "guardar borrador" y muestra la revisión en el pie; 5/5 tests HTTP PostgreSQL (draft) + 7/7 publish de regresión y 68 tests frontend dirigidos PASS. La vista del editor se dividió además en `game-map-editor-interactions.ts` para mantenerla <300 líneas (gate 297A-70 PASS tras el refactor). `297A-70` añade el preview 3D del borrador al editor: botón "preview 3D" en el toolbar que alterna el canvas 2D por un adaptador Three que reutiliza `buildTerrainMeshData` y los materiales de superficie del runtime (sin segundo motor), sincronizado con el documento en cada cambio y con teardown completo (geometrías, materiales, observer, renderer y contexto WebGL); `buildPreviewChunkData` expone los datos de malla puros y testeables. 3 tests nuevos + asserts del toolbar PASS. `297A-69` añade la creación de terreno al editor: tool `terrain` con `terrainChunkAt` (mundo → chunk local), `canCreateChunk` (fail-closed: chunk existente, cuota maxChunks, índices negativos que exigirían reindexar y huecos no contiguos) y `addTerrainChunk` que crea un chunk plano y expande `maxX/maxZ` dentro de `maxWorldWidth/Depth`; el canvas sombrea las celdas vacías contiguas. 7 tests nuevos + asserts del toolbar PASS. `297A-68` añade la superficie "camino" (2) al pincel: `TERRAIN_SURFACE_VALUES.path`, `isAllowedSurface` fail-closed para la vista, sombreado propio en el canvas y tests del camino; el runtime ya mapeaba 2→material medio (297A-33), así que el circuito pintar→publicar→jugar traduce el camino sin cambios en la escena. Se corrigieron además 4 tests de superficie de 297A-66 que asumían suelo en la celda (0,0) del fixture, que el contrato define como agua. `297A-67` añade el pincel de altura al editor: `TERRAIN_HEIGHT_VALUES` discretos allowlisted (0–4), tool `height`, `terrainVertexAt` (mundo → vértice de la malla (chunkSize+1)², fail-closed fuera de bounds) y `paintHeight` que pinta el vértice compartido en TODOS los chunks que lo contienen (bordes y esquinas sincronizados) con commit solo al cambiar (arrastre limpio); el canvas sombrea las celdas por altura y marca los vértices, y el toolbar expone botón "altura" + select de nivel. El pincel no crea terreno ni redimensiona bounds; sin caminos ni tipos de superficie adicionales; la representación 3D del relieve llega con Assets 3D. `297A-49` carga el perfil antes de WebGL/realtime y `297A-50` añade catálogo base/selección allowlisted sin consultas en el loop de render. `297A-51` cierra Fase 6: decisión invitado→cuenta documentada (nada se transfiere; el perfil de la cuenta aplica) y rehidratación del juego ante login/logout/cambio de cuenta, con 25 tests frontend dirigidos. El build del bloque queda condicionado a un error TypeScript preexistente en `notifications-popover.ts` (archivo sin commitear de otro agente). `297A-52` añade la gestión admin del catálogo de personajes en el backend: `POST/PUT /api/admin/game/characters` con `AdminUser`/CSRF, validación allowlisted, 409/404 y desactivación que bloquea nuevas selecciones; 10/10 tests HTTP PostgreSQL PASS. `297A-53` añade el panel admin de UI (tab "juego"): listado completo activas/inactivas vía `GET /api/admin/game/characters`, alta, edición y desactivación/reactivación; 8/8 tests HTTP y 13 tests frontend dirigidos PASS. `297A-54` añade el editor de personaje del jugador en la app Bosque (botón "personaje" → modal con catálogo activo y nombre visible; guardado con revisión optimista; invitados sin persistencia): 39 tests frontend dirigidos PASS. `297A-55` añade la auditoría persistente de cambios sensibles del catálogo: `game_audit_events` registra crear/actualizar/desactivar en la misma transacción y `GET /api/admin/game/audit/characters` los lista acotados con `AdminUser`; 15/15 tests HTTP PostgreSQL PASS. `297A-56` añade el panel UI de auditoría: sección "actividad del catálogo" en el tab "juego" del Admin con los últimos 10 eventos, aislada de la lista si falla; 16 tests frontend dirigidos PASS. `297A-57` cierra la reconexión persistente: `join_player` reemplaza la conexión previa del mismo subject (sin duplicar jugadores; la vieja se cierra sola con el código 4001 y el cliente no reintenta para evitar ping-pong) y el cliente reintenta con backoff 1s→30s + jitter con estado `reconnecting`, cancelación al destruir y error de transporte no fatal (error→close 1006 reintenta); 5/5 unit de `game_room`, 7/7 tests TCP, 25 tests frontend dirigidos, fmt/check/clippy y diff-check PASS. `297A-58` audita la publicación de mapas: `game_map_repo.publish` pasa a transacción y `map.published` se registra en `game_audit_events` en la misma transacción (nunca evento huérfano), con `GET /api/admin/game/audit/maps` acotado (1..=100, filtro `entityId`) y `AdminUser`/CSRF; 13/13 tests HTTP PostgreSQL PASS. `297A-60` añade el catálogo de assets del juego (backend): `game_assets` con seed (terreno/árbol/roca/agua), CRUD admin allowlisted y catálogo público activo, con auditoría transaccional de cambios (`asset.created`/`asset.updated`) y listado admin acotado; 9/9 tests HTTP PostgreSQL PASS. `297A-61` añade el panel UI del catálogo de assets: lista completa activas/inactivas, alta/edición con categoría del contrato del mapa, sección "actividad de assets" aislada y pares acción-entidad de assets en el validador; 9 tests frontend dirigidos PASS. `297A-62` mueve la configuración del Bosque DENTRO de la ventana del juego: toolbar real de `game-playable` con el comando `game:settings` (adminOnly, oculto en vivo para no-admin) que abre un modal B&W con las secciones organizadas "personajes" y "assets" (alta/edición/activar-desactivar + actividad aislada); el tab "juego" del Admin desaparece y `admin-juego.ts` se elimina (la lógica vive en `game-settings.ts`); sin preview de modelos hasta Assets 3D. `297A-63` corrige la UX: la configuración ya no es un modal — `game:settings` dispara un evento sobre la ventana enfocada del Bosque y la app alterna su contenido (destruye el runtime y monta `createGameSettingsPanel` con TABS personajes/assets/actividad en la misma ventana; "volver al Bosque" rehidrata); carga bajo demanda por tab. `297A-64` añade el tab "mapa" con el Editor de mapa 2D dentro de la misma ventana: canvas top-down (grid por cellSize, instancias por categoría, spawns, selección), paleta de assets activos del catálogo, command stack con undo/redo (colocar/mover/duplicar/borrar instancias y spawns con ids generados), validación local con `validateMapVersion` y publicación atómica (`POST /api/admin/game/maps` con `expectedVersion` + conflicto 409 visible); `GameMapAdminService` valida estrictamente el envelope y el documento; el runtime aún consume el fixture. `297A-65` conecta el runtime al mapa publicado: `resolvePlayableMap` carga la publicación activa con fallback fail-closed al fixture (404 sin aviso; fallo de red con aviso) y `game-playable` resuelve el mapa en `hydrate()` antes de montar WebGL/realtime (la escena usa documento/mundo publicado, la simulación sus colliders y el spawn el primero de la publicación); al volver al Bosque tras publicar, el circuito editar→publicar→jugar queda cerrado; 7 tests frontend dirigidos PASS. El runtime aún no da representación visual 3D a instancias del catálogo (Assets 3D). `297A-66` añade el pincel de superficie al editor: tool `paint` con suelo/agua (enteros allowlisted del contrato), `terrainCellAt` (mundo → chunk local + índice, fail-closed fuera de chunks) y `paintSurface` con commit solo al cambiar (arrastre limpio); botón "pintar" + select de superficie en el toolbar y sombreado de celdas > 0 en el canvas; 7 tests nuevos del core + asserts de toolbar PASS. Sin altura (vértices compartidos entre chunks) ni creación de terreno.
- [x] **297A-73 — Panel de versiones de Assets 3D (frontend):** servicio admin (`listVersions`/`import`/`updateMetadata`/`activate` + lectura binaria del GLB vía fetch directo), preview 3D aislado con GLTFLoader (`game-asset-preview.ts`), panel de versiones en `game-settings` (listado, importar GLB, metadata, activar, preview) y endpoint backend `GET /api/admin/game/assets/:asset_id/versions/:version/file` para servir el GLB a admin. Gate `task:check -- 297A-73` PASS, type-check, 142 tests frontend (juego completo) y build PASS.
- [x] **297A-74 — Cierre Fase 4: culling avanzado, batching por materiales y medición física de GPU/memoria:** (a) `MapChunkCache.select` acepta `maxDistance` (radio circular de visibilidad en unidades de mundo) que recorta chunks/instancias fuera del radio aunque caigan dentro de la ventana rectangular, con rechazo de radio inválido; (b) `GamePlayableVisualCache` fusiona en un solo `InstancedMesh` los meshes del prototipo que comparten geometría+material (`groupMeshesByMaterial` pura y exportada, `count = instancias × meshes fusionados`, matrices locales por bloque) y expone `batchDrawCallCount()`/`batchSourceMeshCount()` para medir el ahorro; (c) `game-gpu-probe.ts` lee identidad de GPU (`WEBGL_debug_renderer_info`), mide tiempo de frame GPU con `EXT_disjoint_timer_query` (nanosegundos → ms, asíncrono con `readFrameMs()`) y estima bytes de texturas/geometrías de la escena, con contexto inyectable y teardown; la escena activa el culling por distancia (`STREAM_MAX_DISTANCE`) y publica `data-gpu-*`, `data-batch-*` y `data-gpu-frame-ms`. Gate `task:check -- 297A-74` PASS, type-check, 196 tests frontend del juego y build PASS. La validación visual del fixture con los datasets GPU queda pendiente de una sesión con backend actualizado (el binario del puerto 3000 no expone las rutas del juego).
- [x] **297A-75 — Fase 8: dos salas concurrentes, métricas agregadas y runbook de rollback:** (a) `GameRoomState` pasa a registro multi-sala claveado por `map.map_version()` con `register_map`/`join_on`: cada mapa tiene su propio actor con cap de 8 y TTL independiente, los jugadores de una sala no aparecen en la otra y la capacidad se mantiene por sala (tests unit: dos salas aisladas con snapshots de una sola entidad cada una y rechazo del 9.º solo en la sala llena); (b) métricas agregadas del realtime en `GameRoomMetrics` (joins, joins_rejected, disconnects, rooms_created, snapshots_sent, backpressure_evictions, rate_limited, sequence_rejected, active_players) contadas por el actor con contadores atómicos y expuestas en `GET /api/game/metrics` (público agregado, sin identidades ni coordenadas, en OpenAPI/Orval); test TCP real verifica conteos tras join + ticks y la ausencia de campos privados; (c) runbook `Agente/documentacion/operacion/runbook-rollback-juego-2026-08-05.md` con rollback de mapa (re-publicación de la versión buena, sin mutar snapshots inmutables) y de asset (re-activación de versión anterior), verificación, no-hacer y emergencia SQL documentada. Gate `task:check -- 297A-75` PASS (incremental local-light; full diferido por cooldown), 9/9 unit de `game_room`, 8/8 TCP, 51 tests backend de regresión y 665 tests frontend PASS.
- [x] **297A-76 — Reclamación invitado→cuenta (limpieza de identidad temporal):** `GameTicketStore::revoke_guest` elimina la entrada del store de la cookie invitada (firma + entrada vigente requeridas; revocación fallida no toca la identidad), el login exitoso expira la cookie `guest_game` (Max-Age=0) y el logout también; el handler del ticket revoca server-side la cookie invitada cuando viaja con una sesión autenticada (la cuenta es la autoridad; nunca se fusiona ni degrada). Tests unit (revocar cookie válida la invalida, revocar inválida/secret incorrecto fail-closed) y HTTP (cookie invitada vigente → login → deja de resolver; subject de cuenta distinto del invitado). Gate `task:check -- 297A-76` PASS, 11/11 unit de `game_ticket`, 6/6 HTTP de tickets y 8/8 TCP de regresión PASS.
- [x] **297A-77 — Presencia avanzada: personaje visible en remotos y resync tras reconexión:** el snapshot realtime lleva `characterId` del catálogo (bounded a 64 y validado fail-closed en el contrato Rust/TS). El personaje se resuelve en la capa HTTP al emitir el ticket (`GameProfileRepository::get` contra el perfil de la cuenta; invitados sin perfil viajan sin personaje) y viaja server-side en el ticket (`PendingTicket`/`GameTicketClaims`; el token firmado nunca lo expone); el room lo almacena por jugador (`join_with_character`, default `forest-scout` si no hay perfil o el id es inválido) y lo incluye en el initial y en cada snapshot. El frontend aplica el tono (`ink`/`middle`/`paper` → material compartido) en `createFigure`, recrea la figura si el personaje cambia, y el jugador local offline conserva su personaje a través de `createWorldState`/`normalizeState`/`simulateTick` (default `forest-scout` en fixtures sintéticos). Fix de resync: al recibir `joined` el cliente resetea `lastSnapshotSequence` y limpia los snapshots previos para que el primer snapshot de la sala nueva (cuyo contador puede reiniciarse por TTL) no se descarte como replay. Gate `task:check -- 297A-77` PASS, 12/12 unit de `game_ticket` (+carácter), 19 unit de room/realtime/ws-handler, 8/8 TCP, 54 tests backend HTTP de regresión, 668 tests frontend y type-check PASS.

**Gate/salida:** el plan GAME-01 queda aprobado y cada fase tiene su propio ID, gate `task:check`, auditoría SOLID/rendimiento/escalabilidad/seguridad/observabilidad, pruebas de navegador y evidencia de carga antes de iniciar la siguiente.

### 018A-96 — GAME-02: Extraer `glory-render` como motor reutilizable (planificado)

**Depende de:** GAME-01/Fase 8 estabilizada y de un segundo caso real que justifique cada abstracción. Plan: `Agente/planes/plan-glory-render-motor-juegos-2026-08-01.md`.

- [ ] Auditar `frontend/src/features/game-core/` y clasificar qué pertenece al motor agnóstico, al adaptador Three, al OS, al backend o a Bosque.
- [ ] Crear `glory-render/` dentro de este workspace como repositorio Git independiente, con `core`, contratos, adaptador Three, fixtures, CI, SemVer y quality gate propios.
- [ ] Migrar Bosque a exports públicos sin copiar lógica; fijar integración por submódulo/commit o artefacto reproducible.
- [ ] Crear un segundo juego mínimo de conformidad que pruebe fixtures, lifecycle, renderer fake, límites y compatibilidad sin depender de wandori.us.
- [ ] Publicar una versión estable solo después de comparar rendimiento, bundle, memoria, teardown, seguridad y rollback en ambos consumidores.

**Gate/salida:** dos juegos consumen `glory-render` sin imports específicos de wandori.us/Bosque; el repositorio anidado tiene CI, documentación, release reproducible y rollback.

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
