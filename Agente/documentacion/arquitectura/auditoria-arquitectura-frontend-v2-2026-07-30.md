# Auditoría de arquitectura frontend — Segunda iteración

> **Fecha:** 2026-07-30
> **Alcance:** frontend TypeScript/Vite del OS desktop (runtime, desktop, store, estilos)
> **Resultado:** 3 problemas críticos, 5 altos, 8 medios, 5 bajos, 5 informativos
> **Auditoría anterior:** `auditoria-arquitectura-frontend-2026-07-30.md` (primera iteración)
> **Plan asociado:** `plan-refactorizacion-arquitectura-2026-07-30.md` (completado parcialmente)

---

## Resumen ejecutivo

La primera auditoría encontró que Finder era un app hardcodeado y no un file browser real. Eso se corrigió (297A-11). Esta segunda iteración revisa **todo** el frontend con el nuevo modelo de workspace implementado.

**Lo que funciona bien (no romper):**
- Workspace overlay model: release + overlay + merge puro es correcto y escalable
- CommandRegistry como fuente única de comandos: bien diseñado
- ResourceTypeRegistry: separación correcta de implementación vs tipos
- Store pub/sub simple: sin dependencias, predecible
- Split de archivos en módulos: todos bajo los límites de líneas
- Menu bar "Aplicaciones" derivando de workspaceStore: principio de fuente única aplicado
- Drag/resize con pointer events: patrón estándar correcto

---

## 1. CRÍTICO — desktop-concept.ts es código muerto activo

**Severidad:** CRÍTICO
**Archivo:** `frontend/src/features/desktop/desktop-concept.ts`
**Líneas:** ~160

`desktop-concept.ts` es la versión original del desktop (297A-2) con datos hardcodeados. Sigue existiendo como archivo completo con imports de componentes que ya no se usan. No se importa desde ningún sitio activo (main.ts usa `desktop-shell.ts`), pero:

1. **Confunde a cualquier agente/humano** que lea el código — parece que hay DOS sistemas de escritorio paralelos
2. **Los imports de componentes** (`createFinderPreview`, `createReaderPreview`, `createFontPanel`) están cableados directamente a implementaciones, rompiendo el patrón AppRegistry
3. **Los tipos están desalineados** — usa `'folder' | 'document' | 'application'` hardcodeado en vez del modelo de workspace

**Fix:** Eliminar el archivo. Es código muerto de la fase de concepto. Todo lo que hace ya lo hace `desktop-shell.ts` + `app-registration.ts`.

---

## 2. CRÍTICO — Finder non-singleton crea ventanas duplicadas al navegar entre carpetas

**Severidad:** CRÍTICO
**Archivos:** `route-app-adapter.ts`, `finder-preview.ts`

Cuando el usuario hace doble clic en una carpeta dentro de Finder, se llama `onOpenApp('finder', { folderId: childId })`. Esto pasa por `openAppWindow` que crea una **nueva ventana Finder** para cada carpeta. El usuario espera que la ventana existente **navegue** a la carpeta (como Windows Explorer), no que se abra una ventana nueva por cada subcarpeta.

El dedup de `_paramKey` previene duplicados del **mismo** folderId, pero no evita abrir 5 ventanas para 5 carpetas distintas.

**Impacto:** Si el usuario navega Galería → julio 2026 → fotos, tiene 3 ventanas Finder abiertas. Esto no es un file browser real — es un launcher de carpetas.

**Fix propuesto:** Finder debería **reutilizar la ventana existente** cuando navega entre carpetas (cambiar el folderId del contenido), no crear nuevas ventanas. Solo abrir ventana nueva si el Finder se abre desde el icon grid o el menu bar.

Opciones:
1. Finder mantiene un `currentFolderId` interno y re-renderiza al navegar (preferred)
2. `openAppWindow` para Finder cierra la ventana anterior antes de abrir la nueva
3. Agregar un parámetro `navigate: true` que indique "reutilizar ventana existente"

---

## 3. CRÍTICO — No hay modelo para "archivos" en el workspace

**Severidad:** CRÍTICO
**Archivos:** `workspace/types.ts`, `workspace/default-release.ts`, `workspace-icon-grid.ts`

El workspace model tiene `type: 'resource'` para artículos, imágenes, etc. Pero **no hay ningún nodo resource en el DEFAULT_RELEASE** y ningún flujo crea recursos automáticamente desde el backend.

Los artículos del blog se cargan desde la API en el menu bar "Archivo", pero **no existen como nodos del workspace**. No se pueden arrastrar al escritorio, no aparecen en Finder, no se pueden mover a carpetas.

El flujo completo sería:
1. Backend tiene artículos/productos/media
2. El workspace release debería tener nodos `type: 'resource'` para cada uno
3. Finder los renderiza cuando navegas a la carpeta que los contiene
4. ResourceTypeRegistry determina qué app abre cada recurso

Pero esto no existe. El workspace actual solo tiene folders y app shortcuts. Los recursos son un concepto declarado pero no implementado.

**Fix:** Implementar el flujo de recursos en el workspace — al menos para artículos. El workspace release del backend debería incluir nodos resource referenciando los artículos publicados.

---

## 4. ALTO — `_paramKey` es frágil para dedup de ventanas

**Severidad:** ALTO
**Archivo:** `route-app-adapter.ts`

```typescript
const paramKey = Object.values(params).join(':');
```

Problemas:
1. `Object.values` no garantiza orden — `{ folderId: 'a', view: 'grid' }` y `{ view: 'grid', folderId: 'a' }` producen keys diferentes
2. Si un valor contiene `:`, dos parámetros distintos pueden colisionar: `{ a: 'x:y' }` vs `{ a: 'x', b: 'y' }`
3. No escala a más de 2 parámetros

**Fix:** Usar `Object.entries(params).sort().map(([k,v]) => `${k}=${v}`).join('&')` para una key determinística y sin colisiones.

---

## 5. ALTO — windowStore subscribe en desktop-shell.ts recrea DOM en cada cambio

**Severidad:** ALTO
**Archivo:** `desktop-shell.ts` (líneas ~100-140)

El `windowStore.subscribe()` en desktop-shell tiene este patrón:
```typescript
windowStore.subscribe((windows) => {
  for (const win of windows) {
    if (!renderedWindows.has(win.instanceId)) {
      // Crear ventana completa con DOM + drag + resize
    }
  }
  // ... actualizar posiciones
});
```

Cada vez que **cualquier** propiedad de **cualquier** ventana cambia (bounds, zIndex, focused), el subscribe se dispara y recorre todas las ventanas para actualizar estilos inline. Para 5+ ventanas, esto es O(n) por cada pixel de drag/resize.

El `enableDragResize` actualiza el DOM directamente durante el arrastre (correcto), pero luego `commitBounds()` dispara `windowStore.set()` → subscribe → re-aplica los mismos bounds al DOM. Es redundante.

**Fix:** 
1. El subscribe solo debería crear/eliminar ventanas, no actualizar bounds en cada tick
2. Las actualizaciones de bounds durante drag deberían ser solo DOM (ya lo son), con commit al final
3. Separar subscribe de creación vs subscribe de actualización

---

## 6. ALTO — workspaceStore.get() snapshot en menú "Aplicaciones" es estático

**Severidad:** ALTO
**Archivo:** `desktop-menu-bar.ts`

`createApplicationsMenu()` hace `workspaceStore.get()` una sola vez cuando se abre el menú por primera vez. Si el usuario crea una carpeta nueva y luego abre el menú, la carpeta no aparece hasta que el menu bar se re-renderice (que no lo hace — es estático).

Igual que el menú "Archivo" que hace `api.get()` una vez, el menú "Aplicaciones" es un snapshot. Esto es aceptable para artículos (cargan al abrir el menú) pero no para el workspace que cambia durante la sesión.

**Fix:** El menú debería re-consultar workspaceStore cada vez que se abre (en `toggleEntry`), o suscribirse a cambios y re-renderizar.

---

## 7. ALTO — Shell windows (Perfil) tienen código path especial en todo el stack

**Severidad:** ALTO
**Archivos:** `window-manager.ts`, `desktop-shell.ts`, `reactive-taskbar.ts`

Perfil es una `registerShellWindow()` — un path especial que bypassa AppRegistry. Esto crea una duplicación conceptual:
- `registerShellWindow` vs `openWindow` — dos APIs para crear ventanas
- Shell windows no tienen `app` ni `controller` — campos opcionales que requieren checks
- Taskbar muestra iconos de shell windows con fallback `win.icon ?? win.app?.icon ?? FileUser`
- `closeWindow` hace `target.controller?.abort()` — shell windows no tienen controller

Perfil debería ser una app normal en AppRegistry (singleton, requires 'public'). La única razón para ser shell window era que se abre al arrancar, pero eso puede hacerse con `openAppWindow('profile')` en main.ts.

**Fix:** Registrar Perfil como app en AppRegistry. Eliminar `registerShellWindow`.

---

## 8. ALTO — Tipo `WorkspaceResourceKind` vs `ResourceKind` incompatibles

**Severidad:** ALTO
**Archivos:** `workspace/types.ts`, `resource-type-registry.ts`

```typescript
// workspace/types.ts
type WorkspaceResourceKind = 'article' | 'about' | 'project' | 'product' | 'image' | 'audio' | 'video' | 'document' | 'generic';

// resource-type-registry.ts  
type ResourceKind = 'article' | 'about' | 'project' | 'product' | 'image' | 'audio' | 'video' | 'document' | 'folder' | 'shortcut' | 'generic';
```

Diferencias:
- `ResourceKind` tiene `'folder'` y `'shortcut'` que `WorkspaceResourceKind` no tiene
- `WorkspaceResourceKind` no tiene `'folder'` ni `'shortcut'`

En `workspace-icon-grid.ts` hay un cast `as ResourceKind` para resolver tipos. Esto funciona runtime pero pierde type safety.

**Fix:** Unificar en un solo tipo. `ResourceKind` debería incluir todos los tipos del workspace, o `WorkspaceResourceKind` debería ser un subset tipado de `ResourceKind`.

---

## 9. MEDIO — Reader está hardcodeado, no carga contenido real

**Severidad:** MEDIO
**Archivo:** `reader-preview.ts`

Reader muestra contenido hardcodeado (un artículo de ejemplo). No usa `ctx.params` para cargar el artículo real. El `RenderContext` tiene `params.resourceId` pero Reader lo ignora.

Esto bloquea: abrir artículos desde Finder, desde el menu "Archivo", desde el workspace.

**Fix:** Reader debería hacer fetch de `/api/articles/{slug}` usando el resourceId/slug de params.

---

## 10. MEDIO — `about` app hace import dinámico de página legacy

**Severidad:** MEDIO
**Archivo:** `app-registration.ts` (About render)

```typescript
render: (ctx: RenderContext): MountedView => {
  const container = document.createElement('div');
  void import('../../pages/about').then(async m => {
    if (ctx.signal.aborted) return;
    container.appendChild(await m.renderAbout());
  });
  return { element: container, destroy: ... };
}
```

About importa `pages/about.ts` que es una página legacy con su propio fetch y rendering. Esto es un puente temporal — About debería ser una app que renderice contenido del workspace (nodo `about` de tipo resource), no una página SPA legacy.

**Fix:** About como app que lee contenido del workspace/backend a través del modelo de recursos.

---

## 11. MEDIO — Overlay mutations importan de workspace-store (circular)

**Severidad:** MEDIO
**Archivos:** `overlay-mutations.ts`, `workspace-store.ts`

```
workspace-store.ts → re-exports from → overlay-mutations.ts
overlay-mutations.ts → imports from → workspace-store.ts (overlayStore, workspaceStore, releaseStore, EMPTY_OVERLAY)
```

Esto es un **ciclo de importación**. TypeScript lo resuelve en tiempo de compilación porque los módulos usan referencias lazy (los stores son objetos, no valores que se evalúan al importar). Pero es un patrón fragil que puede romper con cambios menores.

**Fix:** Extraer los stores (`overlayStore`, `workspaceStore`, `releaseStore`) a un archivo `stores.ts` separado que ambos importen. O mover `EMPTY_OVERLAY` a `types.ts`.

---

## 12. MEDIO — mergeWorkspace hace delete sobre objeto mutado

**Severidad:** MEDIO
**Archivo:** `merge.ts`

```typescript
for (const tombId of tombstoneSet) {
  delete result[tombId];
}
// ... luego
for (const [id, node] of Object.entries(result)) {
  if (node.parentId === id) {
    tombstoneSet.add(node.id);
    delete result[node.id]; // Mutando mientras iteramos
  }
}
```

Eliminar keys de un objeto mientras se itera con `Object.entries()` es seguro en JS (entries se capturan al inicio), pero es confuso y propenso a bugs si alguien refactoriza.

**Fix:** Recolectar IDs a eliminar en un array separado, luego eliminar todos al final.

---

## 13. MEDIO — desktop-context-menu y desktop-app-toolbar__dropdown son menús paralelos

**Severidad:** MEDIO
**Archivos:** `desktop-context-menu.ts`, `desktop-window.ts`

Hay DOS implementaciones de menú:
1. `desktop-context-menu` — clic derecho, position fixed en body, z-index 9999
2. `desktop-app-toolbar__dropdown` — toolbar de app, position absolute, z-index 9999

Ambos renderizan items de CommandRegistry pero con lógica de apertura/cierre, keyboard handling y positioning duplicada.

El manual dice "superficies proyectan CommandRegistry" — correcto. Pero no dice que deben duplicar la implementación de menú.

**Fix:** Extraer un componente `createDropdownMenu(items[], options)` compartido que ambos usen. La diferencia es solo positioning (fixed vs absolute) y fuente de datos (filterByContext vs resolveByIds).

---

## 14. MEDIO — grid del desktop usa position absolute (no CSS grid real)

**Severidad:** MEDIO
**Archivo:** `desktop-shell.css`

```css
.desktop-icon-grid {
  position: absolute;
  top: var(--espacio-xl);
  right: var(--espacio-lg);
  display: grid;
  grid-template-columns: repeat(2, var(--sistema-icono-celda));
}
```

El grid está positioned absolutamente en la esquina superior derecha con solo 2 columnas. Esto no escala:
- Con 8+ iconos, se desborda verticalmente
- No se adapta al tamaño de la pantalla
- En mobile necesitará un layout completamente diferente

El manual dice "Desktop/tablet (>=768): escritorio" y "Móvil (<768): launcher". El grid actual no soporta tablet (podría necesitar 3-4 columnas).

**Fix:** Hacer el grid responsive con CSS grid auto-fill o un layout basado en el tamaño del workspace. Posicionar con `margin-left: auto` en vez de `position: absolute`.

---

## 15. MEDIO — window-manager no soporta maximize/fullscreen

**Severidad:** MEDIO
**Archivo:** `window-manager.ts`

`WindowState = 'open' | 'minimized' | 'maximized'` — el tipo existe pero no hay función `maximizeWindow()`. El estado `'maximized'` nunca se usa. No hay forma de que el usuario maximice una ventana (dobleclic en titlebar, botón, o teclado).

**Fix:** Implementar `maximizeWindow()` que guarda bounds anteriores y expande al workspace completo. Doble-clic en titlebar para toggle.

---

## 16. MEDIO — Sin Ctrl+C/X/V para clipboard del workspace

**Severidad:** MEDIO
**Archivos:** `clipboard.ts`, `commands/workspace-commands.ts`, `keyboard-handler.ts`

El clipboard del workspace existe (`setClipboard`, `pasteFromClipboard`) pero los atajos de teclado Ctrl+C/X/V no están conectados al clipboard del workspace. Solo hay comandos declarados pero la ejecución no filtra por selección actual.

**Fix:** Los comandos `workspace:copy`, `workspace:cut`, `workspace:paste` deberían leer `selectionStore.getSelectedIds()` y operar sobre esos nodos.

---

## 17. MEDIO — CSS variables mezclan español e inglés

**Severidad:** MEDIO
**Archivo:** `variables.css`

```css
--color-fondo: #dcdcdc;        /* español */
--color-texto: #000000;         /* español */
--sistema-fondo: #ffffff;       /* español */
--sistema-superficie: #ffffff;  /* español */
--espacio-xs: 4px;              /* español */
--borde: 1px solid var(--color-borde);  /* español */
```

El manual dice "CSS del proyecto: clases en español camelCase, tokens centralizados". Los tokens están en español — esto es consistente con el manual. Pero hay tokens legacy del sitio original (`--fuente-menu`, `--entrada-size`) que coexisten con tokens del OS (`--sistema-fondo`, `--sistema-texto`).

No es un problema funcional, pero la dualidad dificulta encontrar el token correcto.

**Fix (bajo riesgo):** Consolidar todos los tokens bajo prefijos consistentes: `--app-*` para el sitio legacy, `--os-*` para el OS.

---

## 18. MEDIO — Sin feedback visual al crear carpetas

**Severidad:** MEDIO
**Archivo:** `overlay-mutations.ts` (createFolder)

`createFolder()` crea el nodo en el overlay y devuelve el ID. Pero no hay UI para renombrar la carpeta recién creada (label por defecto: "Nueva carpeta"). El usuario tiene que hacer clic derecho → Renombrar.

**Fix:** Después de crear, entrar en modo inline-edit del label. Esto requiere que Finder/desktop-icon soporte editing inline.

---

## 19. BAJO — `generateWindowId()` usa contador global no persistente

**Severidad:** BAJO
**Archivo:** `window-manager.ts`

`let nextWindowId = 1` se resetea al recargar la página. Los IDs son `win-1`, `win-2`, etc. Esto no causa bugs porque los IDs son solo para la sesión, pero es una limitación si se quiere persistir estado de ventanas.

---

## 20. BAJO — `closeWindow` muta el array original

**Severidad:** BAJO
**Archivo:** `window-manager.ts`

```typescript
const topWindow = remaining.reduce((a, b) => (a.zIndex > b.zIndex ? a : b));
topWindow.focused = true; // Mutando un objeto del array
```

`remaining` contiene referencias a los objetos originalos del array. Mutar `topWindow.focused` directamente funciona porque luego se hace `windowStore.set(remaining)`, pero es confuso — parece que se olvidó de crear una copia.

---

## 21. BAJO — Sin protección contra XSS en labels del workspace

**Severidad:** BAJO
**Archivos:** `finder-preview.ts`, `desktop-icon.ts`, `reactive-taskbar.ts`

Los labels del workspace se insertan con `textContent` (seguro) en la mayoría de sitios. Pero si algún futuro render usa `innerHTML` para labels con formato, sería XSS.

Confirmado: todos los usos actuales usan `textContent` → seguro. Solo documentar como regla.

---

## 22. BAJO — `desktop-concept.ts` tiene imports no usados

**Severidad:** BAJO
**Archivo:** `desktop-concept.ts`

Imports de `createDesktopTaskbar`, `createFinderPreview`, `createReaderPreview`, `createFontPanel` que solo se usan dentro de este archivo muerto. Al eliminar el archivo, estos imports desaparecen.

---

## 23. BAJO — Finder no soporta selección múltiple

**Severidad:** BAJO
**Archivo:** `finder-preview.ts`

Finder usa `selectSingle()` en mousedown pero no implementa Ctrl+clic (`toggleSelect`) ni Shift+clic (`extendSelect`). Las funciones existen en `selection-store.ts` pero no se conectan.

**Fix:** Detectar Ctrl/Shift en el event handler de Finder items.

---

## 24. INFORMATIVO — Analytics dispatcher no tiene backend

**Severidad:** INFO
**Archivo:** `analytics/dispatcher.ts`

El dispatcher de eventos (`app_opened`, `app_closed`, `window_focused`) registra eventos pero no envía a ningún backend. Es un stub.

---

## 25. INFORMATIVO — Sin tests unitarios

**Severidad:** INFO

No hay ningún test para el frontend. `merge.ts` es una función pura perfecta para testing. `CommandRegistry`, `AppRegistry`, `selectionStore` también.

---

## 26. INFORMATIVO — initApp() en main.ts es secuencial y lento

**Severidad:** INFO
**Archivo:** `main.ts`

```typescript
await api.get('/api/auth/me');     // ~200ms
await loadSavedFonts();             // ~50ms  
await fetchWorkspaceRelease();      // ~200ms
// ... crear DOM
```

Tres awaits secuenciales antes de renderizar. `auth/me` y `fetchWorkspaceRelease` podrían ejecutarse en paralelo.

---

## 27. INFORMATIVO — route-app-adapter importa workspace-store dinámicamente solo para Finder title

**Severidad:** INFO
**Archivo:** `route-app-adapter.ts`

```typescript
if (appId === 'finder' && params?.folderId) {
  const { workspaceStore } = await import('./workspace/workspace-store');
  // ...
}
```

El import dinámico está bien para evitar circular deps, pero es un code smell que `route-app-adapter` necesite saber sobre workspaceStore solo para resolver un título. El título debería venir del caller o del AppDefinition.

---

## 28. INFORMATIVO — Sin error boundary

**Severidad:** INFO

Si una app throw en su `render()`, el error no se captura. No hay try/catch en `AppRegistry.instantiate()`. Un error en Finder bloquea todo el OS.

**Fix:** Wrap `app.render(ctx)` en try/catch en `openAppWindow()` y mostrar ventana de error.

---

## Prioridad de fixes

| # | Severidad | Fix | Esfuerzo |
|---|---|---|---|
| 1 | CRÍTICO | Eliminar desktop-concept.ts | 5 min |
| 2 | CRÍTICO | Finder navega en ventana existente (no abre nueva) | 2h |
| 3 | CRÍTICO | Implementar flujo de recursos en workspace | 1 día |
| 4 | ALTO | _paramKey determinístico | 15 min |
| 5 | ALTO | Separar subscripción de creación vs actualización en windowStore | 2h |
| 6 | ALTO | Menú Aplicaciones reactivo (re-consultar al abrir) | 30 min |
| 7 | ALTO | Perfil como app normal (eliminar registerShellWindow) | 1h |
| 8 | ALTO | Unificar WorkspaceResourceKind y ResourceKind | 30 min |
| 9 | MEDIO | Reader carga contenido real desde API | 3h |
| 10 | MEDIO | About como app del workspace | 2h |
| 11 | MEDIO | Romper ciclo overlay-mutations ↔ workspace-store | 1h |
| 12 | MEDIO | mergeWorkspace sin mutar durante iteración | 30 min |
| 13 | MEDIO | Unificar menú dropdown compartido | 3h |
| 14 | MEDIO | Grid responsive | 2h |
| 15 | MEDIO | Implementar maximizeWindow | 1h |
| 16 | MEDIO | Conectar Ctrl+C/X/V al workspace clipboard | 1h |
| 17 | MEDIO | Consolidar CSS tokens | 2h |
| 18 | MEDIO | Inline rename al crear carpeta | 2h |

---

## Checklist de escalabilidad: ¿Cuánto cuesta agregar una nueva app?

### Ejemplo: Agregar una Calculadora

1. Crear `apps/calculator/calculator-preview.ts` — render function
2. Registrar en `app-registration.ts` — AppRegistry.register con id, title, icon, render
3. Agregar nodo en `default-release.ts` — workspace node con type:'app', refId:'calculator'
4. Agregar ruta opcional en routePatterns
5. Agregar toolbar opcional con command IDs
6. CSS opcional

**Total: 2 archivos nuevos, 2 archivos editados, ~30 minutos.** Esto es **escalable y correcto**.

### Ejemplo: Agregar un nuevo tipo de recurso (ej: "video channel")

1. Agregar a `WorkspaceResourceKind` en types.ts
2. Agregar a `ResourceKind` en resource-type-registry.ts  
3. Registrar en `initResourceTypeRegistry()` con appId y actions
4. Implementar render en la app correspondiente

**Total: 2 archivos editados, 1 registro, ~20 minutos.** Escalable.

### Ejemplo: Soporte móvil (<768px)

1. Crear `mobile/launcher.ts` — launcher grid de apps
2. Crear `mobile/mobile-shell.ts` — shell sin ventanas/taskbar
3. Crear breakpoint detector que alterna desktop/mobile shell
4. Mismas apps, solo cambia la presentación

**Bloqueadores actuales:**
- `desktop-shell.ts` tiene window rendering hardcodeado (no swappable)
- No hay `MobileAppStack` ni launcher
- CSS del desktop no tiene media queries responsive
- El icon grid está absolute-positioned (no adaptable)

---

## Conclusión

La arquitectura base es **sólida y escalable** para el caso de escritorio. Los principios de fuente única (workspaceStore, AppRegistry, CommandRegistry) están bien implementados después de las correcciones de 297A-11.

Los problemas críticos son:
1. **Código muerto** (desktop-concept.ts) — limpieza trivial
2. **Finder abre ventana nueva por carpeta** — rompe la metáfora de file browser
3. **Sin flujo de recursos** — el workspace solo tiene shortcuts, no archivos reales

Los problemas altos son refinamientos arquitectónicos que bloquean features futuras (mobile, maximize, clipboard real).

La prioridad recomendada es: eliminar código muerto → fix Finder navigation → implementar recursos → luego los altos.
