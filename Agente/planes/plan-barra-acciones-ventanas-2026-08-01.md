# Plan — Barra de acciones inferior de ventana (regla 018A-1)

- **Epic:** 297A-4 — OS persistente, cuentas, programas y comercio
- **Fecha:** 2026-08-01
- **Estado:** activo
- **Siguiente paso:** decidir el alcance móvil del slot `actions` (Fase 1) y migrar los editores (Fase 2)
- **Gate de tarea:** `npm run task:check -- 018A-1` (o el ID de cada fase)

## Objetivo

Toda ventana del OS coloca sus acciones primarias en una franja inferior fija
(`.desktop-window__actions`), parte del chrome — debajo del body padded, fuera
de su padding y de su scroll — con los botones alineados al final (derecha).
Regla aprobada visualmente por el usuario el 2026-08-01.

## Límites

- No aplicar la franja a páginas públicas (checkout, galería exterior).
- No duplicar la franja en el contenido: una ventana tiene UNA franja.
- El slot es opcional; una app sin acciones no cambia su comportamiento.
- El móvil necesita resolución explícita (Fase 1): hoy la franja es desktop-only
  y Admin móvil perdería su botón de alta si no se decide el mecanismo.

## Dependencias

- Bloquea: migración de editores (Fase 2), inventario de ventanas (Fase 3).
- Depende de: nada (slot `actions` ya implementado en el runtime chain).

## Fases

### Fase 0 — Base implementada (hecha, 018A-1)

- [x] Slot `actions` opcional en `MountedView`/`WindowContent` y cadena completa
      (window-manager → desktop-shell → createDesktopWindow → después del body).
- [x] CSS `.desktop-window__actions`: flex, `justify-content: flex-end`,
      `gap-md`, padding `sm/md`, `border-top`, `--sistema-superficie`,
      `[hidden] { display: none }`.
- [x] Admin: `createAdminWindowView()` rellena la franja por tab
      (articulos/proyectos/productos → "+ nuevo"; fuentes/sitio/estadisticas → oculta).
- [x] Listas con scroll propio (`.admin-lista` `overflow-y: auto`).
- [x] Manual identidad §9 y guía agregar-app actualizados (018A-2).
- [x] Verificación visual en navegador (geometría: franja debajo del body,
      botones a la derecha).

### Fase 1 — Resolver el alcance móvil del slot (decisión de diseño)

Problema: la presentación móvil (MobileAppStack) no renderiza `actions`, pero
Admin/otros con lista + alta perderían el botón de alta en móvil.

- [ ] Decidir mecanismo: (a) MobileAppStack también coloca la franja debajo del
      contenido (mismo slot, sin duplicar lógica) o (b) el contenido móvil
      aporta CTA propio (duplica lógica — evitar).
- [ ] Si (a): implementar slot en mobile-stack, actualizar manual §9 (quitar
      "el móvil la ignora") y guía agregar-app.
- [ ] Validar en viewport 320/390 que la franja móvil no roba altura crítica
      (launcher a pantalla completa) y que el scroll del contenido sigue.

**Gate Fase 1:** type-check + gate + inspección móvil 320/390.

### Fase 2 — Migrar editores a la franja (article/project/product)

Hoy los editores tienen botones (guardar/publicar/cancelar) dentro del contenido.
Con contenido largo, deben quedar fijos en la franja.

- [ ] Inventariar cómo abre cada editor (`openEditor`, `openProjectEditor`,
      `openProductEditor`): ¿ventana propia o reemplazo en Admin?
- [ ] Si el editor es ventana propia: devolver `actions` con guardar/publicar/
      cancelar; el formulario absorbe su scroll.
- [ ] Si reemplaza el contenido de Admin: rellenar la franja del mismo view según
      el estado (lista → alta; editor → guardar/cancelar).
- [ ] Confirmar que el foco/enter del formulario no colisiona con la franja.
- [ ] Verificar estados: nuevo vs edición, guardando (disabled), error visible.

**Gate Fase 2:** type-check + gate + flujo real crear/editar artículo.

### Fase 3 — Inventario de ventanas restantes y consistencia

Revisar cada app que abre ventana y decidir si aporta acciones:

- [ ] Configuración: ¿acciones de guardar/aplicar o todo inline?
- [ ] Cuenta: cerrar sesión → ¿franja o queda donde está (inline)?
- [ ] Galería/Finder: nueva carpeta / subir / acciones de selección → franja.
- [ ] Trash: vaciar papelera → franja.
- [ ] Perfil/About/Reader/Snake: sin acciones → sin franja (oculta).
- [ ] Verificar que ninguna app conserva botones de acción primaria sueltos
      dentro del body (inconsistencia que originó 018A-1).
- [ ] Registrar decisiones en el manual §9/§13 (qué apps usan franja y cuáles no).

**Gate Fase 3:** type-check + gate + barrido visual de ventanas.

### Fase 4 — Prevención automatizable

- [ ] Evaluar regla Sentinel: "botón `.boton` dentro de `.admin-lista`/listas
      admin que debiera estar en la franja" (o equivalente de bajo ruido).
- [ ] Evaluar VarSense: clases `barra-acciones` locales en apps (ya eliminada la
      receta muerta) — detectar recetas locales duplicadas.
- [ ] Si una regla es viable y de bajo ruido: implementarla en el tool, probarla
      contra el caso original y archivar el MD de prevención.

**Gate Fase 4:** fixture de regla nueva pasa + caso original detectado.

## Pruebas obligatorias (toda fase)

- `cd frontend && npx tsc --noEmit`
- `npm run task:check -- {ID}` desde la raíz
- Inspección en navegador de la ventana afectada (geometría de la franja:
  último hijo de `.desktop-window`, debajo del body, botones a la derecha)
- Viewports desktop 1440/1024 y móvil 390/320 cuando aplique

## Definition of Done

- [ ] Toda ventana con acciones primarias usa la franja (o justificación en manual).
- [ ] Móvil no pierde acciones (Fase 1 cerrada).
- [ ] Manual §9/§13 y guía agregar-app reflejan el estado final.
- [ ] Prevención evaluada (Fase 4), regla implementada si es viable.
- [ ] Gate PASS + type-check limpio.
