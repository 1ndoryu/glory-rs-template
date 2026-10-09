# Plan — Burbujas estructuradas Marketplace (09AA-19, 2026-10-09)

## Objetivo
Que el backend deje de adivinar la conversación desde texto plano: el
flotante manda la lista ya separada `[{lado, texto}]` y el backend la
guarda tal cual. Fin de la familia wilmery/edickson/edgarluis.

## Evidencia base (snapshot real del chat edgarluis, 2026-10-09)
- Cada burbuja trae dueño propio: `Mensaje enviado 12:59 am por
  Edgarluis: Hola. ¿Sigue...` vs `1:10 am por Tú: Hola, Edgarluis...`.
- El tip de seguridad y el `Meta podría usar...` vienen firmados como
  Edgarluis y Tú aunque son del sistema → van como `sistema`, nunca
  como lado.
- La URL sale como nodos link (`http ://wa.me/...` + `wa.me, wa.me`):
  de ahí nace la URL pegada al concatenar sin separador.
- El mensaje del cliente SÍ está en el DOM; se perdió al aplanarlo.

## Alcance / no alcance
- SÍ: lector por-burbuja en el float (lab), payload estructurado,
  backend que lo acepta, espejo en vivo, fixture + tests, retest desde
  cero en el hilo real.
- NO: partir `marketplace_texto.rs` (09AA-18), decisión firma-clave
  (09AA-16b), canonical `thread_id` con punto final (08AA-30, solo se
  documenta si estorba), deploy prod (todo es local).

## Dependencias
- Lab `opencode-propio-dev` (sin git): se implementa y verifica allí;
  se promociona a su app SOLO en ventana explícita con ella (regla
  `lab-opencode`: diff-first + manifest, ella reinicia).
- Ella: 5 min en Facebook (chat edgarluis abierto) para el retest vivo.

## Fases
- **F0 — Contrato primero (nuevo, antes de todo):** tipos
  `{v:1, hilo_hint, burbujas:[{lado:cliente|duena|sistema|desconocido,
  texto}]}` + fixture anonimizada del snapshot real + validador
  backend (enum, utf-8, longitudes; `v` desconocida → fallback a texto
  plano + log). F1 programa contra la fixture, no contra Facebook vivo.
- **F1+F2 — Float lee burbujas + espejo (fusionados, lab):** por cada
  burbuja saca `por Quién:` de su propio nombre → `Cliente` / `Dueña`
  (patrones `es|en`, `Tú|You`, case-insensitive; sin match →
  `desconocido` + métrica `parse_miss`); textos del sistema (tip, `Meta
  podría...`, `inició esta conversación`) → `sistema`; pestaña en fondo
  → `desconocido` explícito (jamás adivinar). Nodos link se unen con
  separador. Tope: 50 burbujas / 20k chars, últimas primero; debounce
  solo-al-pedir-borrador. Doble selector + feature-flag al texto plano
  viejo por si FB cambia el DOM. Sin espejo verde no existe F1:
  `lo que se ve == lo que llega`.
- **F3 — Backend acepta estructurado (endurecido):** acepta la lista
  (convive con el texto plano viejo vía discriminador de versión),
  guarda `excerpt_texto` burbuja por burbuja con marcas
  `Cliente:`/`Dueña:`, descarta `sistema`. Regla explícita: si >30%
  `desconocido` → rechaza y pide reintento en foreground, no guarda
  basura. Test matriz: estructurado v1 / texto viejo / v desconocida /
  todo-desconocido / payload gigante.
- **F4 — Retest desde cero:** primero verificación sintética (sin
  depender de sus 5 min); luego borrar las 3 filas de
  `mp_respuestas_cache` (2026-10-09: 0 corregidas, nada que perder),
  borrador fresco desde su Facebook, verificar guardado = mensaje del
  cliente + borrador propio, sin sistema ni URL pegada.
- **F5 — Cierre:** gate (fmt+clippy+test) + re-análisis sin nuevos,
  docs, commit+push `origin main`, promoción lab en su ventana.

## Endurecimiento v2 (segundo reto, 2026-10-09)
- **Privacidad:** el float filtra `sistema` en origen y nunca lo envía;
  `texto` = solo cuerpo (sin `por Quién:` ni hora); `hilo_hint` = hash
  opaco, nunca nombre/título; prohibido loggear `texto` (solo `v`,
  conteos, `parse_miss`); test "sin PII en wire".
- **Hint no-clave:** `hilo_hint` = URL canónica + avisoId; backend lo
  usa solo para log, jamás como PK ni parte de firma/dedup; rechaza si
  cambia intra-petición; test con 2 hilos.
- **Firma-v2:** `hash(v + burbujas normalizadas en orden cronológico
  asc)`; lookup legacy v1 durante transición; el float envía últimas N
  pero ordenadas asc; test mixto viejo/nuevo.
- **`sistema` por defecto `desconocido`:** match exacto normalizado a
  lista versionada + preferencia a marcador DOM; test adverso: cliente
  que pega el tip palabra por palabra debe conservarse.
- **Backend-primero + handshake:** backend acepta v1 y viejo antes de
  promocionar el float; fallback automático a texto plano ante
  `400/unknown-version`; kill-switch en backend; matriz 2×2 en F3.
- **Truncado seguro:** filtrar sistema → últimas N útiles completas
  (jamás partir burbuja; cap 2k/burbuja con `…[truncada]`); test
  gigante con sistema arriba + cliente abajo.
- **DoD en dos niveles:** assert por contenido exacto + sha256 (no
  `length`); espejo con diff automático; gate del float (`tsc` + test)
  en F1; `parse_miss` con umbral en backend; `Idempotency-Key` en
  `/borrador`; sintético = cierre técnico, vivo = confirmación.

## Verificación por fase
- F1/F2: espejo muestra Cliente/Dueña/sistema correctos en el chat
  edgarluis real del lab.
- F3: test con fixture verde + `/borrador` estructurado sintético.
- F4: `excerpt_texto` == `Hola. ¿Sigue estando disponible?` + `Tú:
  Hola, Edgarluis, ...` exactos; `length` predicho de antemano.

## F6 — Globo reutilizable + legible (pedido por ella 2026-10-09)
Base: `mpPanel` en `marketplace-float.ts:231-254`; estilos inline
`:239-248`; anclaje `:220-230,258-285`; borradores `:325-363`;
debug `:426-440`.
- **F6a — Componente SOLID:** separar `crearGlobo / anclar /
  aplicarEstilos / pintarContenido / pintarEstado` como funciones
  testeables + contrato `Globo{title, items, estado}` reutilizable por
  futuros globos (hoy todo mezclado en `FLOAT_SCRIPT:124-381`).
- **F6b — Anclaje arriba del chat:** anclar al composer/caja
  (`contenteditable` de `mpRoot`) o borde inferior del root, no al
  header; re-ancla con `ResizeObserver` del root.
- **F6c — Sin línea d visible:** flag `mostrarDebug=false` por defecto
  (omite `.mp-d`; el debug sigue en consola del lab).
- **F6d — Compacto:** fuente borrador y título −2px, panel ~260px,
  padding menor, como variante paramétrica.
- **F6e — Dos borradores:** `[plantilla rápida, IA arriba]` en orden
  fijo, ambos como texto seleccionable (`user-select:text`) + botón
  Copiar por item (se mantiene click-to-copy). Decisión de ella
  2026-10-09: la plantilla NO se cambia, funciona bien tal cual
  (`Hola, buenos días. Sí, sigue disponible...` + Contacto).
- **F6f — Estado en vivo:** `.mp-s[role=status][aria-live=polite]` con
  `generando… / fallo + Reintentar / regenerando… / listo`, alimentado
  por `want/inflight/error` + timeout 90s.
- **F6g — Legible desde fuera:** `role/aria-label` propios en el globo
  para que el navegador de opencode lo lea en el snapshot (hoy emite
  cero ARIA: el snapshot externo no lo ve).
- Verificación: test de `mpPlace`, de seleccionabilidad/ARIA, de
  estados visibles y de orden plantilla+IA; espejo F2 muestra lo mismo
  que el globo.
Hilo edgarluis real limpio (cliente + propio, sin sistema, sin URL
pegada), gate 0E sin nuevos, commit+push, lab promocionado en su
ventana, 09AA-19 archivada.
- Estado 2026-10-09 (lab `opencode-propio-dev`, SIN commit ni
  reinicios por pedido de ella): F6a–F6g implementados en
  `marketplace-float.ts` (V7) + espejo puro en main + tests escritos
  (`mpPlace`, seleccionabilidad/ARIA, estados, orden IA-arriba con
  plantilla intacta). Pendiente: `bun test` (lo corre ella), espejo F2
  vs globo en lab, promoción en su ventana.

## F7 — Vínculo exacto por ID de aviso (pedido por ella 2026-10-09)
Nota: VEF0 NO es código, es precio en 0 (`mpPrice` casa VEF+0). La idea
del "código del encabezado" muere aquí. Hoy el emparejado es solo por
título (`ficha_por_titulo`/`puntaje_titulo` en `marketplace.rs:603-656`
vs PG `inmuebles WHERE publicado=TRUE`) y `avisoId` viaja siempre
`null` (piloto); no existe `marketplace_id` en catálogo.
- **F7a — Campo + rama exacta (backend):** migración `ALTER TABLE
  inmuebles ADD COLUMN marketplace_id TEXT UNIQUE NULL` + `COLUMNAS` +
  `find_by_marketplace_id` (`repositories/inmueble.rs`); rama
  prioritaria en `claves_cache` (`handlers/marketplace.rs:60-88`): si
  hay ID → `SELECT ... WHERE marketplace_id=$1`, si no → fallback por
  título intacto (degrada a `SIN_FICHA`, nunca bloquea).
- **F7b — El float extrae el ID (lab):** probe en vivo sobre
  `mpRoot(header)`: `querySelectorAll('a[href*="/marketplace/item/"]')`
  → `match(/\/item\/(\d+)/)`; se envía `avisoUrl`/`avisoId` (tocar
  `MpFloatWindow`, `isFloatWindow`, `NucleoVentana`,
  `buildBorradorRequest`, `firmaAviso` — el ID entra a la firma).
  Verificar en lab: ID extraído == URL manual del aviso.
- **F7c — Admin vincula y verifica (front):** campo `marketplace_id`
  en `modal-inmueble.tsx` + badge `verificado` (viene de
  `aviso_conocido`) en `tabla-inmuebles.tsx`; vista de hilos huérfanos
  (sin vínculo) desde `chats-marketplace.tsx` para vincular a mano.
- **F7d — Verificación:** dos inmuebles similares misma zona → cada
  chat trae su precio/ficha correctos; cambio desde admin refleja al
  siguiente borrador; matriz ID válido / ID inexistente / sin ID.
- DoD F7: ningún borrador con `SIN_FICHA` teniendo el aviso vinculado.
