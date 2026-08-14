# Plan 138A-5..9 — Constructor de mundo v2: toolkit de edición (2026-08-14)

> **Estado:** ACTIVO — 138A-6 completado; siguiente bloque 138A-7.
> **Rama:** `wandorius` · **Gates:** `npm run gate:check -- 138A-5` …
> `npm run gate:check -- 138A-9`
> **Fuente de contexto:** decisiones de producto 2026-08-13/14 (motor propio,
> estilo Genshin-like low poly, sin indicadores, mundo único cap 32), cierre
> 138A-4 (constructor completo) y veredictos de cierre: supervisor_reviewer
> **APROBADO CON RESERVAS** (fuga de material del agua corregida en `bcc6648c`;
> observaciones menores: `cellSize` sin propagar al preview, campo `style` sin
> consumir, import sin validación cruzada opciones↔mapa, error de lectura de
> import silencioso) y sentinel_inspector **OK CON OBSERVACIONES, ACCION:
> ninguna**.

## 1. Contexto y decisión del usuario (2026-08-14)

El usuario pidió *"planifica todo esto a continuacion"* con la siguiente lista
(se conserva el orden original):

1. Cuando se cambie un valor, se cambie en tiempo real.
2. Dividir el panel de terreno en paneles pequeños, agrupados en un panel
   lateral con iconos (tipo Blender): al hacer clic en un icono salen las
   opciones.
3. Auditar principios SOLID y arquitectura a todo lo relacionado con el juego.
4. Eliminar los árboles del modo suave.
5. Dejar 2 estilos: bloque y suave.
6. Poder cambiar el tamaño de los bloques.
7. Auditar solo el rendimiento.
8. Que los valores no se pierdan al recargar.
9. Poder añadir bloques y sus variantes, quitar árboles y añadir rocas.
10. "Las cosas van más pequeñas".
11. Poder elegir entre 3 modos de cámara: primera persona, libre y 3ª persona.
12. Poder cambiar todos los colores en un panel especializado.
13. Panel para cambiar las texturas o agregarlas.
14. Panel para manejar todos los assets.

## 2. Objetivo

Convertir el constructor de mundo (138A-4) en un **toolkit de edición tipo
estudio**: UI lateral por iconos con subpaneles pequeños, regeneración en vivo,
persistencia local, dos estilos (`bloques`/`suave`), edición de objetos,
panel de colores/texturas/assets, tres modos de cámara y dos auditorías
(SOLID/arquitectura y rendimiento) con evidencia. Se mantiene el flujo canónico
`TerrainOptions → buildMapVersionFromOptions → MapVersion` y `game-core` puro
(sin Three/DOM/red).

## 3. Bloques y alcance

### 138A-5 — UI tipo Blender + tiempo real + persistencia

- **Panel lateral por iconos:** barra vertical de iconos (tipo Blender); cada
  icono abre/cierra un subpanel pequeño con opciones agrupadas (Terreno,
  Mundo/Estilo, Cámara, Objetos, Color, Textura, Assets). Reemplaza la sección
  única "Constructor" del panel actual (`game-world-constructor.ts`), sin
  romper el contrato de `WorldConstructorControls` (los callbacks se
  conservan).
- **Tiempo real:** cada cambio de valor regenera el mundo con debounce
  (~200 ms), cancelando regeneraciones en vuelo (última gana); stats y
  documento actualizados sin perder el estado del panel. Sliders/inputs emiten
  `input`, no `change` a la espera.
- **Persistencia:** las opciones, estilo, cámara y paleta se guardan en
  `localStorage` (clave versionada `wandorius:constructor:v1`) y se restauran
  al recargar; export/import JSON sigue siendo la fuente portable. Sin backend.

**Checklist:**
- [x] `game-world-constructor.ts` se divide en subpaneles por tema sin duplicar
      lógica de controles (helper compartido de campo/slider/select).
- [x] Iconos con accesibilidad (tooltip + teclado) y tokens del OS; un solo
      subpanel abierto a la vez.
- [x] Debounce de regeneración con cancelación; test DOM (N cambios rápidos →
      1 regeneración) y teardown de timers.
- [x] Persistencia `localStorage` versionada con restauración fail-closed y
      limpieza en teardown.
- [x] Gate 138A-5 PASS (pendiente la validación visual del usuario en
      `/forest-playable`, que se hace al probar el bloque).
- [x] Corrección por feedback del usuario (14-ago): el rail de iconos es el
      panel **exterior** (cabecera colapsable "Constructor") y los controles
      del terreno clásico son secciones suyas ("Isla" y "Estilos"), no al
      revés. `mountWorldConstructor` acepta `extraPanels`/`title`; el panel
      clásico queda solo para el modo legacy sin constructor.

### 138A-6 — Dos estilos, sin árboles en suave, cellSize real, escala menor

- **2 estilos:** `bloques` y `suave`; se retira `actual` del contrato
  `TerrainOptions.style` y del comparador. La isla curva queda solo como
  referencia histórica/experimento (sin selector en el constructor).
- **Sin árboles en suave:** el modo suave deja de colocar árboles
  (`maxTrees=0` o retirada del presupuesto); conserva césped y rocas.
- **Tamaño de bloques:** `cellSize` se propaga al preview (bloques y suave),
  agua, ground y pick del comparador (corrige el hallazgo menor del revisor);
  cambiar "Celda" regenera el mundo en tiempo real y el documento escala igual.
- **Escala menor:** escala base de vegetación/props reducida (~0.5× por
  defecto) para que las cosas "vayan más pequeñas"; la interpretación exacta
  se ajusta probando en navegador (decisión abierta documentada en §7).

**Checklist:**
- [x] `TerrainOptions.style` restringido a `'bloques'|'suave'`; consumidores y
      tests actualizados (comparador, panel, serialización).
- [x] Sin árboles en suave (tests de presupuesto/instancias).
- [x] `cellSize` consumido por meshers, agua y pick; test de paridad preview↔
      documento con `cellSize=2`.
- [x] Escala base menor parametrizada (constante en game-core, no mágica en el
      adaptador) con tests de presupuesto.
- [x] Gate 138A-6 PASS + validación visual del usuario en `/forest-playable`.

### 138A-7 — Tres modos de cámara

- **Libre (orbital):** comportamiento actual (arrastre + rueda).
- **Primera persona:** WASD + mouse look desde el punto de vista del jugador,
  con límites del mundo y colisión básica de suelo existente.
- **3ª persona:** cámara que sigue al personaje (distancia/ángulo
  configurables) con colisión contra el terreno.
- Selector en el panel de Cámara y atajo de teclado; el modo se persiste
  (138A-5) y se restaura al recargar; teardown limpio de listeners/RAF.

**Checklist:**
- [ ] `CameraMode` tipado y controlador por modo en la capa de presentación
      (sin lógica de cámara en `game-core`; solo contratos).
- [ ] Tests DOM de cambio de modo y restauración; teardown sin RAF/listeners
      colgados.
- [ ] Gate 138A-7 PASS + validación visual del usuario (los 3 modos).

### 138A-8 — Editor de objetos y paneles de Color, Textura y Assets

- **Editor de objetos:** añadir bloques y variantes (prefabs del toolkit),
  quitar árboles y añadir rocas sobre las instancias del `MapVersion`; las
  ediciones son capa posterior a la generación y se exportan en el JSON
  (absorbe la Fase 4 "retoque fino" diferida de 138A-4; los pinceles del
  editor 2D 297A-64..71 son referencia de operaciones).
- **Panel de Color:** paleta unificada del mundo (terreno, agua, vegetación,
  rocas, bloques y variantes) centralizada en `game-core` (tokens/paleta
  serializable), persistente y aplicable en tiempo real.
- **Panel de Textura:** cambiar o agregar texturas/rampas por material
  (carga local con file input o URL; sin subida a servidor); validación de
  imagen y teardown de object URLs.
- **Panel de Assets:** inventario del manifiesto del mundo (árboles, rocas,
  césped, agua, bloques y variantes) con recuento, visibilidad, cantidad y
  limpieza; sin import de modelos externos en este bloque.

**Checklist:**
- [ ] Operaciones del editor de objetos puras en `game-core`
      (`editMapVersionObjects`/similar) con cuotas fail-closed y tests.
- [ ] Paneles Color/Textura/Assets con tokens del OS, ≤300 líneas cada uno y
      tests DOM; persisten con 138A-5.
- [ ] Texturas agregadas con revocación de object URLs y sin fugas de
      materiales (reutilizar patrón de `game-toon-water.ts`).
- [ ] Gate 138A-8 PASS + validación visual del usuario en `/forest-playable`.

### 138A-9 — Auditorías SOLID/arquitectura y rendimiento

- **Auditoría SOLID/arquitectura** de todo lo relacionado con el juego
  (`game-core`, comparador, paneles, escena, renderer metrics, realtime):
  SRP/OCP/DIP/ISP/LSP, límites de líneas (componentes/CSS ≤300, hooks ≤120,
  utils ≤150), contratos, teardown y deuda acumulada (campo `style` muerto,
  validación cruzada del import, error silencioso de lectura). Documentar
  hallazgos y corregir los materiales.
- **Auditoría de rendimiento dedicada:** benchmark reproducible (generar N
  mundos, medir ms por generación, draw calls, instancias, geometrías/
  materiales vivos antes/después de N regeneraciones y tras `dispose`),
  revisión de presupuestos (chunks ≤1024, instancias ≤10000, octaves),
  memory/GPU con las métricas existentes (`readRendererMetrics` + GPU probe) y
  corrección de hallazgos con tests.

**Checklist:**
- [ ] Informe de auditoría SOLID con hallazgos por módulo y fixes aplicados
      (o deuda documentada con ticket).
- [ ] Informe de auditoría de rendimiento con números y presupuestos
      verificables; sin fugas de material/geometría tras regeneración (test de
      ciclo de vida ampliado a geometrías).
- [ ] Gate 138A-9 PASS + revisión del usuario con evidencia.

## 4. Fuera de alcance

- Backend, realtime, identidad, colisión avanzada, simulación y multijugador
  (cap 32) — siguen pendientes en GAME-01.
- Extraer `glory-render` a repo separado (018A-96 sigue condicionado a un
  segundo consumidor real).
- Importar el pack Synty (Polygon Meadow Forest) como dependencia: queda como
  referencia visual; el motor/toolkit propio se mantiene.
- Subida de texturas/assets a servidor (solo local en este plan).
- Migrar `game-core` a otro framework o renderer.

## 5. Dependencias

- 138A-4 cerrado (cumplido, ver fuente de contexto).
- 138A-5 precede a 138A-6/7/8 (persistencia y UI por iconos son la base).
- 138A-8 reutiliza contratos de `MapVersion` y, como referencia de
  operaciones, el editor 2D de mapa (297A-64..71).
- 138A-9 se apoya en métricas existentes (`readRendererMetrics`, GPU probe) y
  en el test de ciclo de vida del agua (`game-procedural-comparator.test.ts`).

## 6. Definition of Done (por bloque)

- Gate `npm run gate:check -- <ID>` PASS con su propio ID.
- `npx tsc --noEmit` limpio y suite frontend completa PASS (`npm run test:full`).
- Auditorías (SOLID o rendimiento) con evidencia cuando aplique el bloque.
- Validación real del usuario en navegador `/forest-playable` para cambios
  visuales materiales.
- Roadmap/plan/completada actualizados solo con evidencia; sin locks,
  procesos, worktrees ni temporales de la tarea.
- Commit explícito por bloque; push solo con autorización.

## 7. Decisión abierta (asunción del plan)

- **"Las cosas van más pequeñas":** se planifica como escala base de
  vegetación/props reducida (~0.5×) + `cellSize` visible en el preview. Si el
  usuario se refiere a otra cosa (p. ej. tamaño del mundo, cámara o bloques),
  se ajusta la constante antes de implementar 138A-6.
- **Persistencia:** `localStorage` del navegador (herramienta local, sin
  backend). Si se quiere persistencia por cuenta/workspace, es un cambio de
  alcance que se planifica aparte.
- **Retirada de `actual`:** el estilo histórico queda fuera del constructor;
  si el usuario quiere conservarlo como tercer estilo "referencia", se reduce
  el alcance de 138A-6.
