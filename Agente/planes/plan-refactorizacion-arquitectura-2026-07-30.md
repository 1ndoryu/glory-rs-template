# Plan de refactorización arquitectónica — Frontend wandori.us

> **Fecha:** 2026-07-30
> **Estado:** activo
> **Auditoría:** `Agente/documentacion/arquitectura/auditoria-arquitectura-frontend-2026-07-30.md`
> **Depende de:** 297A-11 parcial (workspace overlay implementado)
> **Bloquea:** 297A-12 (móvil), 297A-14 (editors), testing unitario

## 1. Objetivo

Reducir la deuda técnica del frontend antes de expandir a móvil, editors y comercio. Los 3 archivos que exceden el límite de 300 líneas (AGENTS.md §8) se dividen en módulos por responsabilidad. Sin cambios de comportamiento — solo reorganización.

## 2. Dependencias

- Ninguna tarea del roadmap se modifica.
- Los splits son mecánicos (mover código, actualizar imports, crear barrel exports).
- TypeScript check debe pasar después de cada fase.
- No se añade funcionalidad nueva.

## 3. Fases

### Fase 1: Split de `command-registration.ts` (725 → 6 módulos)

**Gate:** `npx tsc --noEmit` pasa. Todos los comandos siguen registrados.

- [ ] Crear directorio `frontend/src/features/runtime/commands/`.
- [ ] Extraer `window-commands.ts` — comandos `window:close`, `window:minimize`, `window:restore`, `window:focus`, `window:focus-next`.
- [ ] Extraer `geometry-commands.ts` — comandos `window:move-*`, `window:resize-*`, constantes `KB_STEP`.
- [ ] Extraer `workspace-commands.ts` — comandos `workspace:trash`, `workspace:restore`, `workspace:reset`, `workspace:publish`, `workspace:copy`, `workspace:cut`, `workspace:paste`, `workspace:create-folder`. Incluye `resolveWorkspaceNodeId()`.
- [ ] Extraer `app-commands.ts` — comandos `app:open`, `app:focus`.
- [ ] Extraer `toolbar-commands.ts` — comandos `trash:restore-all`, `trash:empty`, `finder:new-folder`, `projects:new`.
- [ ] Extraer `keyboard-handler.ts` — `initKeyboardShortcuts()`, `matchesShortcut()`.
- [ ] Crear `commands/index.ts` que importa todos los módulos como side-effects y re-exporta `initKeyboardShortcuts`.
- [ ] Actualizar `main.ts` para importar desde `commands/index.ts`.
- [ ] Eliminar `command-registration.ts` original.
- [ ] Verificar que no hay imports rotos.

### Fase 2: Split de `workspace-store.ts` (430 → 4 módulos)

**Gate:** `npx tsc --noEmit` pasa. Workspace merge, overlay mutations y clipboard funcionan.

- [ ] Extraer `merge.ts` — `mergeWorkspace()`, `rebaseOverlay()`. Exporta funciones puras.
- [ ] Extraer `overlay-mutations.ts` — `moveNodePosition`, `moveNodeToParent`, `addOverlayNode`, `tombstoneNode`, `restoreNode`, `resetOverlay`, `reorderDesktopNodes`, `createFolder`, `getTombstonedNodes`, `getChildren`. Importa `overlayStore`, `workspaceStore` de `workspace-store.ts`.
- [ ] Extraer `clipboard.ts` — `ClipboardMode`, `ClipboardEntry`, `getClipboard`, `setClipboard`, `clearClipboard`, `pasteFromClipboard`, `wouldCreateCycle`. Importa `workspaceStore`, `moveNodeToParent`, `addOverlayNode` de otros módulos.
- [ ] Reducir `workspace-store.ts` a stores + subscriptions + persistence + API (~120 líneas).
- [ ] Crear `workspace/index.ts` barrel export.
- [ ] Actualizar todos los imports externos (command-registration, desktop-shell, app-registration, drag-resize, icon-drag, desktop-context-menu).
- [ ] Eliminar exports obsoletos del store original.

### Fase 3: Split de `desktop-shell.ts` (419 → 3 módulos)

**Gate:** `npx tsc --noEmit` pasa. Shell renderiza correctamente.

- [ ] Extraer `workspace-icon-grid.ts` — `createWorkspaceIconGrid()`, `resolveNodeIcon()`, `resolveNodeIconType()`, `SHELL_ICON_MAP`. Importa workspace-store, app-registry, icon-drag, selection-store.
- [ ] Extraer `reactive-taskbar.ts` — `createReactiveTaskbar()`. Importa window-manager, lucide icons.
- [ ] Reducir `desktop-shell.ts` a orquestación pura: `createDesktopShell()`, profile registration, window rendering loop, ResizeObserver (~150 líneas).
- [ ] Verificar que el profile, taskbar y windows siguen funcionando.

### Fase 4: Validación completa

- [ ] `npx tsc --noEmit` — cero errores.
- [ ] `npm run task:check -- 297A-11` — quality gate pasa.
- [ ] Verificar manualmente: abrir/cerrar ventanas, minimizar, taskbar, icon grid, context menu, clipboard, crear carpeta, drag de iconos.
- [ ] Verificar que no hay imports circulares nuevos.

## 4. Criterio de salida

- Ningún archivo TS supera 300 líneas (excepto `font-panel.ts` en el límite).
- No hay imports circulares.
- `npm run task:check` pasa.
- Todas las funcionalidades del OS siguen operativas.
- Los módulos extraídos son importables independientemente para testing futuro.

## 5. Riesgos y mitigación

| Riesgo | Mitigación |
|---|---|
| Imports rotos al mover código | Barrel exports en `index.ts` + TypeScript check después de cada fase |
| Circular imports al separar módulos | Los módulos extraídos importan del store (no al revés). El store no importa de mutations/clipboard |
| Regresión funcional | Validación manual de cada flujo después de Fase 3 |

## 6. Archivos afectados (estimado)

| Fase | Archivos creados | Archivos modificados | Archivos eliminados |
|---|---|---|---|
| 1 | 7 (`commands/*.ts`) | 1 (`main.ts`) | 1 (`command-registration.ts`) |
| 2 | 4 (`workspace/merge.ts`, `overlay-mutations.ts`, `clipboard.ts`, `index.ts`) | 8+ (todos los que importan workspace-store) | 0 |
| 3 | 2 (`workspace-icon-grid.ts`, `reactive-taskbar.ts`) | 1 (`desktop-shell.ts`) | 0 |

## 7. Qué NO se hace en este plan

- Lazy loading de apps (pendiente 297A-14).
- CSS layers (pendiente 297A-14/16).
- Store event typing (pendiente 297A-13).
- Command namespaces formales (pendiente 297A-14).
- Tests unitarios (pendiente configurar vitest).
