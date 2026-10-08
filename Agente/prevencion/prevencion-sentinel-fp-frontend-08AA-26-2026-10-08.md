# FPs front 08AA-26: lazy-multilínea, catch-interno, alias único y DTO ancha

4 clases de falso positivo corregidas en `glory-sentinel` (rama `fix/08AA-26-reglas-fp`,
commit `748a387`) + 1 TP corregido en el consumidor + 2 veredictos de no-cambio.
Re-gate MN: **0E/443W/3H → 0E/439W/0H** (todas las reglas del alcance a 0).

## 1. `promise-sin-catch` marca `lazy(() =>` + `import().then(...)` en líneas separadas

- **Dónde:** `frontend/src/App.tsx:7,12` (2 warnings).
- **Causa:** la exención `[124A-FP3]` solo miraba `lazy(` en la MISMA línea del `.then`.
  El patrón real es multilínea (`lazy(() =>` + newline + `import().then(...)`).
- **Fix (regla):** si la línea `.then` contiene `import(`, buscar `lazy(` hasta 5 líneas
  atrás (`reactErrorRules.ts`, tag `[08AA-26]`). Testigo Mal: `.then` sin lazy sigue marcando.
- **Tests:** `src/test/suite/promiseSinCatch.test.ts` (5: lazy misma línea, lazy
  multilínea, Mal sin catch, try-catch, disable con justificación).

## 2. `promise-sin-catch` marca `.then()` sobre promesas que nunca rechazan (catch-interno)

- **Dónde:** `frontend/src/features/chat/clientes-duena.tsx:56,167` (2 warnings).
- **Causa:** `c.alta()` / `c.enviar()` capturan TODO dentro (`use-clientes.ts:67-98`,
  try/catch + `setError`) y exponen el error por estado `c.error` (renderizado en la
  vista). La promesa nunca rechaza: `.catch()` sería código muerto. La regla, de un
  solo fichero, no puede saberlo: es FP por diseño, no por bug.
- **Fix:** la regla NO se debilita (`void` no equivale a "manejado"). En su lugar se le
  dio soporte `sentinel-disable-next-line promise-sin-catch` (consistencia con el resto
  de reglas) y el consumidor lo usa con justificación + referencia al catch interno
  (regla 14 MN: disable solo justificado). Patrón documentado para futuros casos.

## 3. `mixed-barrel-logic` marca un único re-export de compatibilidad

- **Dónde:** `frontend/src/domain/ficha-ask.ts:208`
  (`export { calcularCompletitud } from './pasos-ask'`, documentado para no romper imports).
- **Causa:** la regla marcaba CUALQUIER re-export junto a lógica. Un único re-export es
  alias de compatibilidad, no barrel.
- **Fix (regla):** exigir ≥2 sentencias `export {…} from` / `export * from`
  (`portableRules.ts`, tag `[08AA-26]`). Testigos Bien (1 re-export) y Mal (2 re-exports).

## 4. `large-interface-isp` marca DTOs espejo de API (solo datos, 0 métodos)

- **Dónde:** `frontend/src/data/chat/cliente-duena.ts` (3 hints: 11/11/12 campos).
- **Causa:** la regla contaba campos de datos (`countInterfaceFields` ni siquiera cuenta
  métodos) y pedía "dividir el contrato". ISP aplica a contratos de COMPORTAMIENTO: una
  DTO que refleja 1:1 una fila del backend no impone carga de implementación; dividirla
  rompe el contrato con el servidor por cero beneficio.
- **Fix (regla):** exigir ≥1 firma de método en el cuerpo; umbral >10 miembros se mantiene
  (`portableRules.ts`, tag `[08AA-26]`). Testigos Bien (DTO 12 campos) y Mal (11 miembros
  con 2 métodos; el test viejo `Payload` se actualizó a esa forma).

## 5. `inline-style-prohibido` en barra de progreso: ERA TP, se corrigió el consumidor

- **Dónde:** `frontend/src/features/chat/clientes-duena.tsx` NO — era
  `frontend/src/features/ask/pagina-ask.tsx:208` (`style={{ width: ... }}` dinámico).
- **No es FP:** el ancho % realmente no se expresa con clases estáticas. Fix en el
  consumidor con el patrón bendecido por la propia regla ([054A-19]): valor dinámico vía
  CSS var (`style={{ '--progreso': ... }} as CSSProperties` + `w-[var(--progreso)]`).
  Primera consumidora del patrón en el proyecto. `tsc -b` limpio, sentinel 0 en el fichero.

## Veredictos de no-cambio (con testigo)

- **`sqlite-carga-N-consultas` ×1:** TP real en bucle (`for record...` con 4 awaits en
  `glory-rs/backend/src/fixtures/sync.rs:60-130`, fixture del submódulo ajeno). El FP
  temido (`turno.rs`, 2 awaits dependientes) no dispara. Endurecer a "solo en bucle"
  debilitaría la regla y rompería 5 tests existentes: NO se toca.
- **`componente-sin-hook-glory` ×5:** TPs conformes a la regla 8 MN (lógica >5 líneas con
  estado va a hook). Queda como tarea de refactor del consumidor (extraer
  `useHiloMensajes`, `useSesionesWhatsapp`, `usePestanaIA`, `useTarjetaFotoMejora`,
  `useModalDescargarFotos`), NO como fix de regla (regla 14: no silenciar refactor pendiente).

## Detección futura

- `lazy(` + newline + `import().then` → exento (test multilínea).
- `.then` con disable justificado por catch-interno → exento (test disable).
- 1 re-export + lógica → exento (test alias); ≥2 → marca (test barrel).
- Interfaz 0 métodos por ancha que sea → exenta (test DTO); con métodos y >10 miembros → marca.
