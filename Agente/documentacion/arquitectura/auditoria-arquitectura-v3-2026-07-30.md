# Auditoría de arquitectura y escalabilidad v3 — Frontend wandori.us

> **Fecha:** 2026-07-30
> **Alcance:** frontend TypeScript/Vite del OS desktop (post-fixes v2)
> **Resultado:** 12 hallazgos nuevos. 1 alto, 4 medios, 4 bajos, 3 informativos.
> **Contexto:** Aplicados todos los fixes de auditoría v2 (4 críticos, 3 altos, 12 medios, 4 bajos).

## 1. Métricas actualizadas

### TypeScript — Top 10 por tamaño

| Archivo | Líneas | Límite | Estado |
|---|---|---|---|
| `window-manager.ts` | 314 | 300 | 🔴 +5% |
| `font-panel.ts` | 304 | 300 | 🔴 +1% |
| `desktop-menu-bar.ts` | 291 | 300 | 🟡 |
| `finder-preview.ts` | 286 | 300 | 🟡 |
| `icon-drag.ts` | 262 | 300 | 🟢 |
| `app-registration.ts` | 243 | 300 | 🟢 |
| `drag-resize.ts` | 238 | 300 | 🟢 |
| `desktop-window.ts` | 215 | 300 | 🟢 |
| `desktop-shell.ts` | 211 | 300 | 🟢 |
| `workspace-commands.ts` | 205 | 300 | 🟢 |

**Evolución:** v1 tenía 3 archivos sobre 300 (725, 430, 419). Ahora solo 2 (314, 304). Mejora significativa.

### CSS — Todos los archivos

| Archivo | Líneas | Límite | Estado |
|---|---|---|---|
| `components.css` | 330 | 300 | 🔴 +10% |
| `pages.css` | 312 | 300 | 🔴 +4% |
| `layout.css` | 310 | 300 | 🔴 +3% |
| `desktop-shell.css` | 250 | 300 | 🟢 |
| `desktop-window.css` | 147 | 300 | 🟢 |
| `variables.css` | 130 | 300 | 🟢 |
| Otros (8 archivos) | 82-128 | 300 | 🟢 |

**Nota:** Los 3 CSS sobre el límite son archivos legacy del sitio (no del OS). El OS desktop respeta los límites.

### Patrones de código

| Patrón | Cantidad | Evaluación |
|---|---|---|
| `innerHTML` usages | 33 | 🟡 Patrón de re-render completo |
| Type assertions (`as`) | 15 | 🟢 Mayoría seguras |
| `.subscribe()` calls | 18 | 🟢 Infraestructura de eventos |
| Console warn/error | 4 | 🟢 Mínimo, solo errores reales |
| Imports profundos (3+ niveles) | 13 | 🟡 Acoplamiento vertical |
| CSS `@layer` declarado | 1 | 🟡 Solo reset+base envueltos |

---

## 2. Hallazgos

### 2.1 🟠 ALTO — innerHTML como patrón de re-render destruye el DOM

**Archivos afectados:** 15+ (workspace-icon-grid.ts, reactive-taskbar.ts, finder-preview.ts, desktop-menu-bar.ts, sidebar.ts, profile.ts, trash-preview.ts, etc.)

**Problema:** Cada callback de `subscribe()` hace `container.innerHTML = ''` y reconstruye todo el DOM desde cero. Esto:
- Destruye event listeners existentes (memory leak si no se limpian)
- Dispara reflows masivos en cada cambio
- Escala O(n²) para listas grandes
- Imposibilita animaciones de transición entre estados

**Ejemplo típico:**
```typescript
workspaceStore.subscribe((ws) => {
  grid.innerHTML = '';           // ← destruye todo
  for (const child of children) { // ← reconstruye todo
    grid.appendChild(createItem(child));
  }
});
```

**Solución:** Implementar un `reconcileChildren(container, newChildren, getKey, createFn)` que:
1. Reuse nodos existentes por key
2. Solo cree/elimine los que cambiaron
3. Reordene mediante `insertBefore` en vez de recrear

**Esfuerzo:** 2-4 horas (crear utility + migrar los 5 subscribientes principales)

---

### 2.2 🟡 MEDIO — window-manager.ts sobre 300 líneas

**Archivo:** `features/runtime/window-manager.ts` (314 líneas)

**Problema:** Mezcla el store reactivo (`windowStore`) con todas las funciones de mutación (open, close, focus, minimize, restore, maximize, updateBounds, registerShell). El tipo `WindowEntry` tiene 18 campos.

**Solución:** Extraer `window-mutations.ts` con las funciones de mutación. El store y tipos permanecen en `window-manager.ts`.

**Esfuerzo:** 30 min (split mecánico)

---

### 2.3 🟡 MEDIO — font-panel.ts sobre 300 líneas

**Archivo:** `features/settings/font-panel.ts` (304 líneas)

**Problema:** Mezcla UI (3 tabs de configuración), lógica de negocio (Google Fonts loading, debounce save), y persistencia (API calls). La función `loadSavedFonts()` parsea settings del backend.

**Solución:** Extraer `settings-repo.ts` con la lógica de carga/guardado de settings. `font-panel.ts` solo contendría UI.

**Esfuerzo:** 45 min

---

### 2.4 🟡 MEDIO — CSS @layer declarado pero incompleto

**Archivos:** `variables.css`, `reset.css`, `base.css`

**Problema:** Se declaró `@layer base, components, overrides;` y se envolvió reset+base en `@layer base`. Pero los CSS de componentes (`components.css`, `pages.css`, `layout.css`) y del OS (`desktop-*.css`) permanecen sin envolver, lo que los pone en la capa de mayor specificity por defecto. Esto significa que los estilos del OS pueden ser sobreescritos por cualquier regla no-envuelta.

**Solución:** Envolver los CSS legacy en `@layer components` y los desktop-*.css también. Esto requiere envolver cada archivo.

**Esfuerzo:** 1 hora (mecánico pero requiere verificar que no haya regressions visuales)

---

### 2.5 🟡 MEDIO — registerLazy existe pero no se usa

**Archivo:** `features/runtime/app-registry.ts`

**Problema:** Se implementó `registerLazy()` con dynamic import, pero `app-registration.ts` todavía importa estáticamente las7 apps. Finder (286 líneas), font-panel (304 líneas), reader, trash, etc. se cargan todas al inicio.

**Solución:** Migrar apps pesadas a lazy loading:
```typescript
AppRegistry.registerLazy({
  id: 'settings',
  title: 'Configuración',
  icon: Settings,
  singleton: true,
  requires: 'admin',
  load: () => import('../settings/font-panel').then(m => ({ render: m.createFontPanelRender })),
});
```

**Esfuerzo:** 1-2 horas (requiere refactor de cada render function para ser exportable)

---

### 2.6 🔵 BAJO — Type assertions frágiles en desktop-menu-bar.ts

**Archivo:** `features/desktop/components/desktop-menu-bar.ts` (líneas 50-51, 225)

**Problema:** Usa `(menu as HTMLElement & { _onOpen?: () => void })._onOpen` para almacenar callbacks en el DOM. Esto es un pattern no tipado y frágil.

**Solución:** Usar un `WeakMap<HTMLElement, () => void>` para almacenar callbacks de apertura.

**Esfuerzo:** 15 min

---

### 2.7 🔵 BAJO — CSS legacy sin envolver en @layer

**Archivos:** `components.css` (330), `pages.css` (312), `layout.css` (310)

**Problema:** Estos 3 archivos suman 952 líneas de CSS legacy del sitio que no pertenecen al OS desktop. Están sin `@layer`, lo que les da la mayor specificity. Si alguna regla del OS necesita sobreescribir una regla legacy, necesita `!important` o mayor specificity.

**Solución:** Envolver en `@layer components`.

**Esfuerzo:** 30 min (incluido en 2.4)

---

### 2.8 🔵 BAJO — Imports profundos indican árbol de módulos anidado

**Archivos:** `finder-preview.ts` (`../../../../store`), `reader-preview.ts` (`../../../../api/client`), `desktop-window.ts` (`../../../store`)

**Problema:** Los módulos en `features/desktop/apps/finder/` están a 4 niveles de profundidad desde `src/`. Esto genera imports largos y frágiles.

**Solución:** Mover `store.ts`, `api/client.ts` a rutas más accesibles o usar path aliases (`@/store`). Los aliases de Vite (`@/`) ya están disponibles.

**Esfuerzo:** 30 min (configurar en tsconfig + migrar imports)

---

### 2.9 🔵 BAJO — No hay error boundaries en app render

**Archivo:** `features/runtime/route-app-adapter.ts`, `app-registration.ts`

**Problema:** Si una app lanza un error durante `render()`, la ventana queda vacía sin feedback al usuario. No hay try/catch alrededor de `AppRegistry.instantiate()`.

**Solución:** Envolver `instantiate` en try/catch y mostrar un fallback de error en la ventana.

**Esfuerzo:** 15 min

---

### 2.10 ⚪ INFO — Store event typing infraestructura no aprovechada

**Archivo:** `store.ts`, `stores.ts`

**Situación:** Se implementó `StoreSource` ('user'|'api'|'overlay'|'init'|'sync') y `TypedListener`. Pero los 18 `.subscribe()` existentes ignoran el parámetro `source`. La infraestructura está lista para cuando se necesite distinguir el origen (undo/redo, analytics, sync remoto).

**Acción:** Ninguna ahora. Se activará con 297A-13 (overlay remoto) o cuando se implemente undo/redo.

---

### 2.11 ⚪ INFO — CommandRegistry getByPrefix no tiene consumidores

**Archivo:** `features/runtime/command-registry.ts`

**Situación:** Se añadió `getByPrefix('workspace:')` pero ningún componente lo usa aún. Está listo para cuando los editors (297A-14) necesiten filtrar comandos por dominio.

**Acción:** Ninguna ahora.

---

### 2.12 ⚪ INFO — FontConfig tiene 22 campos en un solo store

**Archivo:** `store.ts` — `FontConfig` interface

**Situación:** 22 propiedades en una sola interfaz. Podría agruparse en sub-objetos (`fonts: { menu, titulo, texto }`, `sizes: { ... }`, `layout: { ... }`). Pero como `fontStore.subscribe` se usa para aplicar CSS vars, la estructura plana es funcionalmente correcta.

**Acción:** Considerar si se refactoriza el settings panel.

---

## 3. Prioridad de fixes

| # | Severidad | Fix | Esfuerzo | Bloquea |
|---|---|---|---|---|
| 1 | 🟠 ALTO | Reconcile utility para reemplazar innerHTML | 2-4h | Performance con muchas ventanas |
| 2 | 🟡 MEDIO | Split window-manager.ts (314→2 módulos) | 30 min | Límite de tamaño |
| 3 | 🟡 MEDIO | Split font-panel.ts (304→2 módulos) | 45 min | Límite de tamaño |
| 4 | 🟡 MEDIO | Envolver CSS legacy en @layer components | 1h | Specificity management |
| 5 | 🟡 MEDIO | Migrar apps a registerLazy | 1-2h | Bundle size |
| 6 | 🔵 BAJO | WeakMap para callbacks en menu-bar | 15 min | Type safety |
| 7 | 🔵 BAJO | Error boundary en instantiate | 15 min | UX en errores |
| 8 | 🔵 BAJO | Path aliases para imports profundos | 30 min | DX |

---

## 4. Estado acumulado de las 3 auditorías

| Auditoría | Hallazgos | Completados | Pendientes |
|---|---|---|---|
| v1 | 10 | 10 | 0 |
| v2 | 28 | 28 | 0 |
| **v3** | **12** | **0** | **12** |

### Distribución v3

| Categoría | Cantidad |
|---|---|
| 🟠 Alto | 1 |
| 🟡 Medio | 4 |
| 🔵 Bajo | 4 |
| ⚪ Info | 3 |

---

## 5. Referencias

- Auditoría v1: `Agente/documentacion/arquitectura/auditoria-arquitectura-frontend-2026-07-30.md`
- Auditoría v2: mismo archivo (secciones §7-§9)
- Plan de refactorización: `Agente/planes/plan-refactorizacion-arquitectura-2026-07-30.md`
- Completados: `Agente/completados/tareas-2026-07-30.md`
