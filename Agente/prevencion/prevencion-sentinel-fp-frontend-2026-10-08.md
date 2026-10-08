# FPs de Sentinel en frontend (08AA-22, 2026-10-08)

Reglas que disparan sobre patrones estándar de React/TS del proyecto.
Ninguna requiere cambio de código; si la regla se reescribe, estos son
los casos testigo que deben dejar de marcarse.

## 1. `inline-style-prohibido` — estilo dinámico calculado

- **Caso:** `frontend/src/features/ask/pagina-ask.tsx:207`
  `style={{ width: `${porcentaje}%` }}` donde `porcentaje` es progreso 0–100.
- **Por qué es FP:** el valor es dinámico por definición; no existe clase
  utilitaria para cada porcentaje. La alternativa (100 clases) es peor.
- **Detección esperada:** la regla debería ignorar `width`/`height` con
  interpolación numérica + `%`, o aceptar `style` con una sola propiedad
  numérica.

## 2. `key-index-lista` — lista estática derivada

- **Caso:** `frontend/src/features/chat/message-media.tsx:54,67,77,85`
  `key={i}` sobre `url.split('\n')` (lista derivada, sin ids estables,
  nunca se reordena ni filtra).
- **Por qué es FP:** con listas estáticas derivadas el índice es la key
  recomendada por React; no hay identidad estable que usar.
- **Detección esperada:** ignorar `key={index}` cuando el `.map` va sobre
  una expresión derivada (`split`, `filter` inline) sin `id` disponible.

## 3. `promise-sin-catch` — `React.lazy` y catch interno

- **Casos:**
  - `frontend/src/App.tsx:7,12`: `lazy(() => import(...))` — React exige
    la promesa sin catch; el boundary de error la gestiona.
  - `frontend/src/features/chat/clientes-duena.tsx:55,166`: `alta()` y
    `enviar()` tienen `try/catch` interno con `setError`
    (`hooks/chat/use-clientes.ts:65-98`); el llamador no necesita otro.
- **Detección esperada:** ignorar el argumento de `React.lazy`; seguir la
  cadena y no marcar si el callee captura internamente.

## 4. `mixed-barrel-logic` — shim de compatibilidad marcado

- **Caso:** `frontend/src/domain/ficha-ask.ts` (re-export de
  `data/inmuebles/ficha-ask.ts` para no romper imports existentes).
- **Por qué es FP:** el shim existe para no mezclar; marcarlo pide justo
  lo contrario de lo que ya hace.
- **Detección esperada:** ignorar módulos cuyo contenido sea solo
  re-exports (`export ... from`).

## 5. `large-interface-isp` — DTOs de API

- **Casos:** `frontend/src/data/chat/cliente-admin.ts:10`,
  `cliente-duena.ts:7,46` (contratos de respuesta del backend).
- **Por qué es FP:** el ancho lo fija el servidor; partir el tipo rompe
  el mapeo 1:1 con el JSON.
- **Detección esperada:** ignorar `interface` que modelan respuestas de
  red (o umbral mayor para `data/**`).

## 6. `componente-sin-hook-glory` — ya verificado

- **Casos:** `modal-detalle.tsx`, `modal-nuevo.tsx`, `modal-descargar-fotos.tsx`,
  `lista-mensajes.tsx`, `pagina-inicio.tsx`.
- **Veredicto 08AA-22:** los 5 ya usan hooks (`useTema`, `useAnadirFotos`,
  `useConfigCopy`, `useCopy`, `useColaMejora`, `useDocumentTitle`); el
  informe inicial era erróneo (barrido superficial). Sin acción.

## Referencia

- Roadmap: `08AA-22` (barrido gate 2026-10-08).
- Gate: `22E/469W/0I/7H` analizado el 10/08/2026 19:24:52.
- No archivar: sigue vigente hasta que las reglas se ajusten y el
  re-análisis confirme cero marcas en estos testigos.
