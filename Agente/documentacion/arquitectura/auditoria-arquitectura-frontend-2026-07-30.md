# Auditoría de arquitectura y escalabilidad — Frontend wandori.us

> **Fecha:** 2026-07-30
> **Alcance:** frontend TypeScript/Vite del OS desktop
> **Resultado:** 3 violaciones de tamaño, 5 problemas de escalabilidad, 4 fortalezas identificadas
> **Plan asociado:** `Agente/planes/plan-refactorizacion-architectura-2026-07-30.md`

## 1. Métricas del codebase

| Capa         | Líneas      | Archivos |
| ------------ | ----------- | -------- |
| Frontend TS  | ~8,150      | 50+      |
| Backend Rust | ~4,610      | 30+      |
| CSS          | ~2,260      | 14       |
| **Total**    | **~15,000** | **95+**  |

### Archivos por tamaño

| Archivo                   | Líneas | Límite AGENTS.md | Estado    |
| ------------------------- | ------ | ---------------- | --------- |
| `command-registration.ts` | 725    | 300              | 🔴 +142%  |
| `workspace-store.ts`      | 430    | 300              | 🔴 +43%   |
| `desktop-shell.ts`        | 419    | 300              | 🔴 +40%   |
| `font-panel.ts`           | 304    | 300              | 🟡 límite |
| `window-manager.ts`       | 273    | 300              | 🟢        |
| `desktop-window.ts`       | 266    | 300              | 🟢        |

---

## 2. Violaciones de tamaño

### 2.1 `command-registration.ts` — 725 líneas

**Problema:** Mezcla 6 dominios distintos en un solo archivo.

| Dominio              | Líneas aprox. | Contenido                                                       |
| -------------------- | ------------- | --------------------------------------------------------------- |
| Window commands      | ~200          | close, minimize, restore, focus, focus-next                     |
| App commands         | ~60           | app:open, app:focus                                             |
| Geometry keyboard    | ~150          | move-up/down/left/right, resize-up/down/left/right              |
| Workspace/clipboard  | ~150          | trash, restore, reset, publish, copy, cut, paste, create-folder |
| App toolbar commands | ~80           | trash:restore-all, trash:empty, finder:new-folder, projects:new |
| Keyboard handler     | ~80           | initKeyboardShortcuts, matchesShortcut                          |

**Impacto:** Cualquier cambio en un dominio toca un archivo masivo. Los comandos de workspace importan `workspace-store`, los de ventana importan `window-manager` — todo en el mismo scope de imports.

**Solución:** Split en 6 módulos bajo `commands/`:

```
frontend/src/features/runtime/commands/
  window-commands.ts      (~200 líneas)
  geometry-commands.ts    (~150 líneas)
  workspace-commands.ts   (~150 líneas)
  app-commands.ts         (~60 líneas)
  toolbar-commands.ts     (~80 líneas)
  keyboard-handler.ts     (~80 líneas)
  index.ts                (re-export + initKeyboardShortcuts)
```

### 2.2 `workspace-store.ts` — 430 líneas

**Problema:** 5 responsabilidades distintas en un solo módulo.

| Responsabilidad       | Líneas aprox. | Contenido                                                                                                                       |
| --------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Merge algorithm       | ~80           | mergeWorkspace, rebaseOverlay                                                                                                   |
| Store + subscriptions | ~60           | releaseStore, overlayStore, workspaceStore, recompute                                                                           |
| Persistence + API     | ~50           | loadOverlay, saveOverlay, fetchWorkspaceRelease, publishWorkspace                                                               |
| Overlay mutations     | ~120          | moveNodePosition, moveNodeToParent, addOverlayNode, tombstoneNode, restoreNode, resetOverlay, reorderDesktopNodes, createFolder |
| Clipboard             | ~80           | getClipboard, setClipboard, clearClipboard, pasteFromClipboard, wouldCreateCycle                                                |

**Impacto:** El clipboard no tiene relación con el merge algorithm. Las mutations de overlay son un dominio distinto del persistence. El cycle validation es lógica pura testeable independientemente.

**Solución:** Split en 4 módulos:

```
frontend/src/features/runtime/workspace/
  types.ts                (ya existe)
  default-release.ts      (ya existe)
  merge.ts                (~80 líneas — merge + rebase algorithm)
  workspace-store.ts      (~120 líneas — stores + subscriptions + persistence)
  overlay-mutations.ts    (~120 líneas — move, tombstone, restore, reorder, createFolder)
  clipboard.ts            (~80 líneas — clipboard state + paste + cycle validation)
  index.ts                (re-export)
```

### 2.3 `desktop-shell.ts` — 419 líneas

**Problema:** 4 responsabilidades acopladas en el shell.

| Responsabilidad    | Líneas aprox. | Contenido                                                       |
| ------------------ | ------------- | --------------------------------------------------------------- |
| Icon grid reactivo | ~120          | createWorkspaceIconGrid, suscripción a workspaceStore, drag     |
| Reactive taskbar   | ~100          | createReactiveTaskbar, suscripción a windowStore                |
| Profile management | ~50           | registerShellWindow, re-registro al cerrar                      |
| Window rendering   | ~150          | windowStore.subscribe, createDesktopWindow loop, ResizeObserver |

**Impacto:** El taskbar es un componente independiente. El icon grid tiene su propio drag/resize. El profile tiene lifecycle especial. Todo acoplado en un solo archivo dificulta testing y evolución.

**Solución:** Split en 3 módulos:

```
frontend/src/features/desktop/
  desktop-shell.ts        (~150 líneas — orquestación, window container)
  workspace-icon-grid.ts  (~120 líneas — grid reactivo + drag)
  reactive-taskbar.ts     (~100 líneas — taskbar subscribe a windowStore)
```

---

## 3. Problemas de escalabilidad

### 3.1 Dependencias circulares potenciales

```
command-registration.ts → window-manager.ts → (indirecto) → app-registry.ts
command-registration.ts → workspace-store.ts → store.ts → (indirecto) → router.ts
desktop-shell.ts → window-manager.ts → route-app-adapter.ts → (indirecto) → desktop-shell.ts
```

Los `dynamic imports` en `command-registration.ts` (`await import('./route-app-adapter')`, `await import('../../router')`) son la válvula de escape actual, pero es un síntoma de acoplamiento circular.

**Riesgo:** Conforme crezcan los módulos, los ciclos se vuelven más difíciles de romper. Los dynamic imports son workarounds, no soluciones.

### 3.2 Store sin tipado de eventos

`workspaceStore`, `releaseStore`, `overlayStore` usan `createStore<T>` genérico. No hay distinción entre:

- "overlay cambió por mutation del usuario" vs "overlay cambió por rebase ante release nuevo"
- "store actualizado por API" vs "store actualizado por localStorage"

**Riesgo futuro:**

- Overlay remoto (297A-13) necesitará distinguir sync local vs remoto
- Analytics necesitará saber qué causó un cambio
- Undo/redo necesitará revertir solo mutations del usuario

### 3.3 CommandRegistry sin namespaces

Los command IDs usan `:` como separador (`window:close`, `workspace:trash`, `trash:empty`), pero no hay agrupación formal. Cuando lleguen los comandos de editor (297A-14: `editor:save`, `editor:undo`, `editor:format`), el `getByContext()` escala linealmente y no permite filtrado por dominio.

**Solución:** Implementar `CommandRegistry.getByPrefix('workspace:')` o agrupar por namespace en el registro.

### 3.4 CSS sin sistema de capas

Los estilos están organizados por feature (`desktop-shell.css`, `desktop-window.css`), pero no hay specificity management. Los overrides de toolbar, window body padding, y finder se pisan unos a otros.

**Riesgo:** Conforme crezcan las apps (297A-14), los conflictos de specificity serán frecuentes. Los fixes ad-hoc (como el padding que afectó a galería pero no a documentos) son sintomáticos.

**Solución:** CSS `@layer base, components, overrides` para gestión explícita de specificity.

### 3.5 Sin lazy loading de apps

Todas las apps se registran en `app-registration.ts` como side-effect en el import de `main.ts`. Conforme crezcan (Finder con gallery, Reader con markdown, Editors, Tienda), el bundle inicial crecerá sin control.

**Solución:** Apps deberían cargarse bajo demanda mediante dynamic `import()` en `AppRegistry.register()` con `load: () => import(...)`.

---

## 4. Fortalezas arquitectónicas

| Aspecto                 | Estado          | Por qué funciona                                                                                                 |
| ----------------------- | --------------- | ---------------------------------------------------------------------------------------------------------------- |
| **AppRegistry**         | ✅ Sólido       | Singleton, tipado, `AppDefinition` como contrato claro con `toolbar`, `requires`, `routePatterns`                |
| **CommandRegistry**     | ✅ Buen diseño  | `isAvailable()` + `contexts` + `execute()` es escalable. Unificado con toolbar via refs                          |
| **Unified menu system** | ✅ Correcto     | CommandRegistry como fuente única. `ToolbarItemRef` permite overrides. `createAppToolbar` lee de CommandRegistry |
| **WindowManager**       | ✅ Funcional    | Store reactivo, boundary clamping, z-index dinámico, estados tipados                                             |
| **Workspace merge**     | ✅ Robusto      | Release + overlay + capability filtering es el patrón correcto para multi-tenant                                 |
| **RouteAppAdapter**     | ✅ Desacoplado  | URL ↔ app mapping sin hardcoding. Interceptor evita doble rendering                                              |
| **CSS variables**       | ✅ Centralizado | `variables.css` con tokens, monocromo consistente. Identidad Mac OS 9 respetada                                  |

---

## 5. Riesgo de avanzar sin refactorizar

| Tarea futura                 | Riesgo si no se refactoriza                                                                                               |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| **297A-12 (móvil)**          | Necesitará su propio command set. `command-registration.ts` de 725 líneas será ingobernable con comandos móviles añadidos |
| **297A-13 (overlay remoto)** | Necesita stores separados para distinguir local vs remoto. `workspace-store.ts` monolítico no permite esto                |
| **297A-14 (editors)**        | Añadirá ~200+ líneas a command-registration. Los editors necesitarán toolbar commands propios                             |
| **297A-15 (comercio)**       | Checkout/entitlements necesitarán sus propios commands y analytics. El archivo seguirá creciendo                          |
| **Testing**                  | Merge algorithm, clipboard y cycle validation son lógica pura testeable, pero no se pueden importar aisladamente          |

---

## 6. Recomendación

Ejecutar los splits (#1-3) como parte de **297A-11** antes de avanzar a 297A-12. Los splits son **mecánicos** (mover código, actualizar imports) sin cambiar comportamiento — bajo riesgo, alto impacto.

Lazy loading (#5) y CSS layers (#7) pueden ejecutarse en paralelo con 297A-12/14.

Store event typing (#6) es necesario antes de 297A-13 (overlay remoto).

---

## 7. Referencias

- Manual de arquitectura: `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`
- AGENTS.md §8: Estándares esenciales (límites de tamaño)
- Plan maestro: `Agente/planes/plan-escritorio-persistente-cuentas-admin-apps-2026-07-29.md`
