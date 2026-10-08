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

- **Casos (gate 08AA-23, 5 marcas):**
  - `features/chat/hilo-mensajes.tsx:12` — `useState` + 3 `useRef` + `useEffect`.
  - `features/chat/sesiones-whatsapp.tsx:13` — 3 `useState` + `useCallback` + 2 `useEffect`.
  - `features/configuracion/pestana-ia.tsx:7` — hook propio `useConfigIA` + `useState` + `useEffect`.
  - `features/imagenes/tarjeta-foto-mejora.tsx:15` — 3 `useState`.
  - `features/inmuebles/modal-descargar-fotos.tsx:8` — 4 `useState`.
- **Veredicto 08AA-22/23:** los 5 usan hooks (verificado por grep de
  `use[A-Z]` en cada archivo; la lista inicial de 08AA-22 citaba otros
  ficheros por barrido superficial). Sin acción.
- **Detección esperada:** la regla no debería marcar un componente que
  llama hooks de React o hooks propios del proyecto.

## Referencia

- Roadmap: `08AA-22` (barrido gate 2026-10-08).
- Gate: `22E/469W/0I/7H` analizado el 10/08/2026 19:24:52.
- No archivar: sigue vigente hasta que las reglas se ajusten y el
  re-análisis confirme cero marcas en estos testigos.
