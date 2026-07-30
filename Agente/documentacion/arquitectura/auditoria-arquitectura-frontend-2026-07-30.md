# Auditoría de arquitectura y escalabilidad — Frontend wandori.us

> **Fecha:** 2026-07-30 (actualizado)
> **Alcance:** frontend TypeScript/Vite del OS desktop
> **Resultado:** 3 violaciones de tamaño, 5 problemas de escalabilidad, 1 falla arquitectónica crítica (Finder), 4 fortalezas identificadas
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

## 7. Falla arquitectónica crítica: Finder no es un explorador de archivos

> **Severidad:** bloquea la experiencia base del OS. Todos los problemas de Finder/Galería reportados por el usuario tienen una causa raíz común.

### 7.1 Problemas reportados

1. Crear carpeta dentro de Galería → se crea en el escritorio, no dentro de Galería
2. Clic derecho dentro de Galería → no abre menú contextual
3. Galería parece una app estática, no una carpeta genuina de archivos
4. No se pueden arrastrar archivos de Galería al escritorio
5. No se pueden abrir las imágenes de Galería
6. Al crear una carpeta en el escritorio y hacer clic → abre Galería pero no navega a esa carpeta
7. ¿Es escalable añadir nuevas apps? (ej: calculadora)

### 7.2 Causa raíz: Finder es un preview hardcodeado, no un file browser

**`finder-preview.ts`** contiene datos estáticos:

```typescript
const folderImages = [
  { src: '/legacy-assets/colors/...', label: 'color_01.jpg', ... },
  { src: '/legacy-assets/colors/...', label: 'flores_02.jpg', ... },
  { src: '/legacy-assets/colors/...', label: 'forma_03.png', ... },
];
const folderDocuments = ['El silencio de las máquinas', 'Fragmentos de código'];
```

Estos arrays son **literales de código**, no leen de `workspaceStore`, `default-release.ts`, ni del backend. Finder ignora completamente el modelo de datos del workspace.

### 7.3 El gap conceptual: `type: 'folder'` vs `type: 'app'`

En `default-release.ts`, "Galería" es un nodo `type: 'app'` con `refId: 'finder'`:

```typescript
gallery: { id: 'gallery', type: 'app', refId: 'finder', ... }
```

Esto significa que **no existe ningún nodo `type: 'folder'` real en el release**. Los nodos de tipo `'folder'` solo se crean con `createFolder()` y carecen de `refId`.

Cuando el usuario hace clic en un icono del escritorio, `desktop-shell.ts` decide:

```typescript
const onActivate = node.refId
  ? () => openAppWindow(node.refId!)      // apps → abre la app
  : node.type === 'folder'
    ? () => openAppWindow('finder')         // folders → abre Finder genérico
    : undefined;                            // otros → ignora
```

**Problema:** Abrir Finder genérico no navega a la carpeta clickeada. Finder siempre muestra los mismos datos hardcodeados sin importar qué carpeta se abrió.

### 7.4 Análisis de cada problema

| # | Problema | Causa técnica | Código responsable |
|---|---|---|---|
| 1 | Carpeta se crea en escritorio | `finder:new-folder` hardcodea `createFolder('desktop', ...)` | `command-registration.ts:642` |
| 2 | Clic derecho no funciona | Context menu solo se registra en `workspace` e `icon`, nunca dentro de Finder | `desktop-shell.ts:108,210` |
| 3 | Galería es app estática | Finder preview tiene datos literales, no lee workspaceStore | `finder-preview.ts:10-25` |
| 4 | No se pueden arrastrar archivos | Items de Finder no tienen drag handlers ni conexión con workspace | `finder-preview.ts` (sin drag) |
| 5 | No se abren imágenes | Solo documentos tienen click handler → navigate. Imágenes son `<figure>` sin interacción | `finder-preview.ts:45-57` |
| 6 | Clic en carpeta no navega | `openAppWindow('finder')` es singleton genérico, no pasa nodeId | `desktop-shell.ts:89` |

### 7.5 Diseño propuesto: Finder como file browser real

**Principio:** Finder es el explorador de archivos del OS. Debe leer de `workspaceStore` y mostrar los hijos de un nodo dado. Cada carpeta del escritorio abre una ventana de Finder apuntando a esa carpeta.

#### Cambio 1: Finder lee de workspaceStore

```typescript
// En vez de datos hardcodeados:
export function createFinderPreview(options: FinderPreviewOptions): HTMLElement {
  // Suscribirse a workspaceStore y mostrar hijos de options.folderId
  workspaceStore.subscribe((ws) => {
    const children = Object.values(ws.nodes)
      .filter(n => n.parentId === options.folderId);
    renderChildren(grid, children, options);
  });
}
```

#### Cambio 2: Finder no es singleton

```typescript
AppRegistry.register({
  id: 'finder',
  singleton: false,  // Permitir múltiples ventanas (una por carpeta abierta)
  // ...
});
```

#### Cambio 3: Carpetas abren Finder con contexto

```typescript
// desktop-shell.ts — folder activation
: node.type === 'folder'
  ? () => openFinderWindow(node.id)  // Pasa el nodeId de la carpeta
  : undefined
```

#### Cambio 4: finder:new-folder crea en el contexto actual

```typescript
// Command recibe el folderId del Finder activo
execute: (ctx?: CommandContext): CommandResult => {
  const currentFolderId = getActiveFinderFolderId() ?? 'desktop';
  createFolder(currentFolderId, 'Nueva carpeta');
  return { status: 'success' };
}
```

#### Cambio 5: Finder items son arrastrables

Los hijos renderizados en Finder deben usar `enableIconDrag` o un drag handler que:
- Al arrastrar al escritorio → mueve el nodo a `parentId: 'desktop'`
- Al arrastrar a otra carpeta → mueve el nodo a ese `parentId`

#### Cambio 6: Context menu dentro de Finder

```typescript
grid.addEventListener('contextmenu', (e) => {
  const item = (e.target as HTMLElement).closest('.desktop-finder__item');
  if (item) {
    openContextMenu({ context: 'finder-item', targets: [...], ... });
  } else {
    openContextMenu({ context: 'finder', targets: [{ id: folderId }], ... });
  }
});
```

### 7.6 Implicaciones para el modelo de datos

El workspace model actual (`types.ts`) ya soporta carpetas con hijos via `parentId`. El gap no está en el modelo sino en **ninguna parte del código lee los hijos de una carpeta para renderizarlos**.

`workspaceStore.get().nodes` contiene todos los nodos. `getChildren(parentId)` ya existe en `workspace-store.ts`. Lo que falta es:

1. Un componente `WorkspaceFileBrowser` que reciba un `folderId` y renderice sus hijos
2. Finder use ese componente en vez de datos hardcodeados
3. Que los hijos renderidos sean interactivos (abrir, arrastrar, menú contextual)

### 7.7 Impacto en roadmap

Este gap debería resolverse **dentro de 297A-11** (workspace + overlay) porque:
- 297A-11 ya define el workspace con carpetas y nodos hijos
- Sin Finder funcional, el overlay invitado no tiene forma de ver sus carpetas
- El clipboard (copiar/pegar) ya está implementado pero no tiene dónde mostrar resultados
- La papelera ya lista nodos tombstonados pero no se pueden restaurar visualmente

---

## 8. Escalabilidad de creación de nuevas apps

### 8.1 Proceso actual para añadir una app

Para añadir una calculadora, el proceso sería:

```typescript
// 1. Crear componente (1 archivo)
// frontend/src/features/desktop/apps/calculator/calculator-preview.ts
export function createCalculatorPreview(): HTMLElement { ... }

// 2. Registrar en app-registration.ts (1 bloque)
AppRegistry.register({
  id: 'calculator',
  title: 'Calculadora',
  icon: Calculator,  // de lucide
  iconType: 'application',
  singleton: true,
  requires: 'public',
  render: (ctx: RenderContext): MountedView => {
    return { element: createCalculatorPreview(), destroy: () => {} };
  },
});

// 3. Añadir al default-release.ts (1 nodo)
calculator: {
  id: 'calculator',
  parentId: 'desktop',
  type: 'app',
  label: 'Calculadora',
  refId: 'calculator',
  position: { col: 1, row: 1 },
  requires: 'public',
},
```

**Evaluación:** ✅ Razonablemente simple. Tres archivos, ~30 líneas nuevas. AppDefinition como contrato es sólido.

### 8.2 Problemas de escalabilidad para apps futuras

| Problema | Descripción | Solución |
|---|---|---|
| **Bundle size** | Todas las apps se importan en `app-registration.ts` (side-effect en main.ts). 10 apps = todo el código cargado al inicio. | Lazy loading: `render: async (ctx) => { const m = await import('./calculator'); return m.render(ctx); }` |
| **Singleton vs multi-instancia** | `singleton: true` impide abrir dos Finder distintos (dos carpetas). `singleton: false` permite infinitas ventanas. No hay `maxInstances`. | Añadir `maxInstances?: number` a AppDefinition |
| **Sin parámetros de instancia** | `openAppWindow('finder')` no puede pasar qué carpeta abrir. El render function solo recibe `RenderContext` (signal). | Añadir `params?: Record<string, string>` a RenderContext y openAppWindow |
| **Sin hot registration** | Añadir una app requiere modificar 3 archivos. No hay plugin system ni dynamic discovery. | Futuro: `AppRegistry.registerLazy({ id, load: () => import(...) })` |

### 8.3 Gap crítico: RenderContext no tiene parámetros

El `RenderContext` actual:

```typescript
interface RenderContext {
  signal: AbortSignal;
}
```

No hay forma de pasar parámetros como `folderId`, `articleSlug`, o `productId` al render de una app. Esto bloquea:

- Finder con contexto de carpeta
- Reader con slug de artículo
- Editor con ID de recurso
- Cualquier app que necesite saber qué instancia mostrar

**Solución propuesta:**

```typescript
interface RenderContext {
  signal: AbortSignal;
  params?: Record<string, string>;  // Parámetros de instancia
}
```

Y en `openAppWindow`:

```typescript
export async function openAppWindow(
  appId: string,
  params?: Record<string, string>,
): Promise<void> { ... }
```

Esto permite `openAppWindow('finder', { folderId: 'mi-carpeta-123' })`.

---

## 9. Actualización de recomendaciones

### Prioridad inmediata (297A-11)

| # | Tarea | Impacto | Bloquea |
|---|---|---|---|
| 1 | Añadir `params` a RenderContext y openAppWindow | Crítico | Finder real, Editor, todos los flujos |
| 2 | Reescribir Finder como file browser que lee workspaceStore | Crítico | Experiencia base del OS |
| 3 | Finder no-singleton con folderId como parámetro | Alto | Abrir múltiples carpetas |
| 4 | Context menu dentro de Finder (items + fondo) | Alto | Interacción básica |
| 5 | Drag de items Finder → escritorio/otra carpeta | Medio | Flujo de archivos |
| 6 | Split de los 3 archivos grandes | Medio | Escalabilidad del código |

### Futuro (297A-12+)

| # | Tarea | Impacto |
|---|---|---|
| 7 | Lazy loading de apps | Bundle size |
| 8 | CSS layers | Conflictos de specificity |
| 9 | Store event typing | Overlay remoto |

---

## 10. Referencias

- Manual de arquitectura: `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`
- AGENTS.md §8: Estándares esenciales (límites de tamaño)
- Plan maestro: `Agente/planes/plan-escritorio-persistente-cuentas-admin-apps-2026-07-29.md`
