# Plan detector de diferencias + push local→prod — 2026-10-08 (08AA-35)

> Estado: CERRADO 2026-10-09 (ver "Cierre" al final). Origen: pedido de
> ella 2026-10-08 ("detectar cosas que difieran entre prod y local; todo
> sincronizado; subo a prod la versión local para coherencia").
> Base existente: `scripts/sync-pull.mjs` (prod→local) + `scripts/inmueble.mjs
> push --slug` (un inmueble local→prod). Falta: detector total y push
> completo con certeza.

## Objetivo

Un subcomando de solo-lectura que compare prod vs local y diga con certeza
qué difiere (inmuebles por `slug`, fotos por `origen:orden` + contenido), y
un push local→prod completo que deje ambos lados idénticos, con dry-run,
backup y cero sorpresas.

## No alcance

- Nada de fusión campo a campo ni resolución automática: ante valor distinto
  se reporta y manda local solo con flag explícito (decisión 2).
- Solicitudes/suscriptores/chat (`agent_*`, `clientes`, `mp_*`) no se
  sincronizan: solo catálogo (`inmuebles` + `fotos` + `ficha`). El hilo
  WhatsApp de prueba de hoy (+52, Loma Linda) vive en tablas de chat y no lo
  toca ningún push de catálogo.
- Sin tarea programada (doctrina 08AA-2). Sin SSH/docker/scp contra prod
  (reglas 1 y 19): todo por API + manager.
- `ai_enabled_global=on` NO es parte de este plan: tarea inmediata aparte,
  pendiente de que ella confirme que terminó de reenviar fotos (si se
  reactiva antes, la IA le respondería a ella).

## Estado verificado

- `sync-pull.mjs` 2026-10-08: prod 13/259, local 13/255 → deriva conocida de
  4 fotos; el verificador debe señalarla exacta (testigo de calibración).
- `push --slug` verificado 2026-10-08: el mecanismo de escritura funciona.
- Límites reales del código (2 retos 2026-10-08, verificados en fuente):
  `lib-api.mjs:46-50` timeout 60 s + `Buffer` entero en memoria (sin
  concurrencia); `inmueble.mjs:420-438` solo sube faltantes o borra TODO con
  `--sobrescribir`, y `:406-408` crear-por-slug ROTO (slug inmutable,
  `services/inmueble.rs:24,84` + `repositories/inmueble.rs:195-226` sin
  `SET slug` → el re-find por slug devuelve null); `repositories/
  inmueble.rs:197-214` `COALESCE` (null no borra); `cmdPush` no toca `/ficha`
  (ruta real `handlers/ask.rs:47`); `diffNucleo` (`:96-100`) compara
  `JSON.stringify` crudo; servidor re-codifica a JPG (`:230-243`) → mismo
  visual, distinto sha; `Foto` (`models/inmueble.rs:187-206`) NO trae
  size/hash; sin UNIQUE `(inmueble,origen,orden)` (`add_foto :320-342`,
  `ON CONFLICT` ausente); `DELETE` una `original` mata a su `mejorada`
  hermana y renumera superiores (`:351-388`); `PUT` núcleo muta `extras`
  lateralmente (`:218-223`); `RecetaPublicidad` son índices sobre visibles
  (`models/inmueble.rs:24-26`).

## Diseño (2 retos aplicados; un solo CLI, sin deps nuevas)

- Nada de scripts separados: `inmueble.mjs verificar` + `inmueble.mjs
  push-full`, reuse `login/listar/nucleo/magia` (YAGNI).
- `verificar` en 2 pasadas (el sha masivo de ~500 fotos es inviable):
  P1 solo metadatos: `slug`, núcleo canónico §abajo, set `origen:orden` +
  conteo + `updated_at` padre (NO `size`: no existe sin descargar; descargas
  con JWT por `f.url`, nunca "público sin auth"). P2 sha solo sobre
  candidatas `FOTO-DISTINTA`, concurrencia 4, techo 10 MiB, hasheando bytes
  RE-CODIFICADOS con la misma rutina PIL en ambos lados (pelar EXIF no basta:
  el servidor re-codifica a JPG; si no converge, `mejorada` va a categoría
  propia fuera del gate exit-0, nunca re-subida en bucle).
- Canónico de núcleo fijado antes de F1: trim, números a formato fijo,
  `null≡undefined≡""` explícito, orden de claves fijo. Comparación incluye
  `ficha` (`extras`/`precio_minimo`) y `copy_*` con semántica `COALESCE`
  documentada + mutación lateral de `extras` por `PUT` documentada.
- Pareo en 2 fases (lección 199A-6/199A-7): fase 1 por contenido (hash) para
  FOTOS y para INMUEBLES (un renombre por título = slug nuevo con mismo
  contenido); fase 2 por `orden`/`slug`. Un shift de orden no marca todo
  FOTO-FALTA. Ante `FALTA-PROD` + `FALTA-LOCAL` simultáneos con contenido
  igual: PROHIBIDO crear automático (sería duplicar en prod) → revisión
  manual. F0 + solo-faltantes NO cubren este caso.
- `verificar` aserta duplicados `(origen,orden)` como `DIFIERE` propio y
  `push-full` aborta ese slug antes de subir (la unicidad no existe en BD).
- Salida: `IGUAL | FALTA-PROD | FALTA-LOCAL | DIFIERE(campo) | FOTO-FALTA |
  FOTO-SOBRANTE | FOTO-DISTINTA | POSIBLE-RENOMBRE` + conteos + auditoría de
  rutas llamadas + aserción `cero escrituras fuera de inmuebles/fotos/ficha`.
  Exit 0 = cero DIFIERE/FALTA, con SOBRANTES listados aparte (nunca bloquean
  el 0); exit 1 con difs; exit 2 preflight o cambio mid-flight.
- `push-full`: corre `verificar`; `--dry-run` default; `--si` aplica por
  `slug`; ante fallo en un upload se aborta ESE slug (no se sigue);
  re-verifica al final. Crear FALTA-PROD con `POST nucleo(loc)` DIRECTO,
  jamás `POST {}` + `PUT` (slug roto). Orden de escritura por slug: núcleo
  → fotos → ficha (`PUT /ficha` explícito) → receta AL FINAL (sus índices
  dependen de las visibles); `verificar` marca receta `DIFIERE` si hay fotos
  pendientes.
- Política huérfanas: default `solo-faltantes` (extras en prod = `FOTO-
  SOBRANTE` aparte); borrado solo con `--con-borrado` por slug confirmado,
  confirmado por `foto.id` (no por orden) mostrando hermanas afectadas en el
  dry-run (borrar `original` arrastra `mejorada` + renumera); prohibido
  borrado-total-masivo.
- Anti-carrera: estampar `listar` prod+local con timestamp y re-leer cada
  slug antes de escribirlo; si cambió mid-flight se aborta el slug (exit 2).
- Seguridad: backup local (`pg_dump` → `C:\tmp`) + backup prod vía manager
  con id en log ANTES del primer push real; backup prod con id OBLIGATORIO
  antes de cualquier `--con-borrado` (sin vía "declaración" para borrados).
  JWT en memoria.

## Fases

- F0 — Forense previo (HECHO 2026-10-08) + verificación profunda P1+P2
  (HECHA 2026-10-08, 255 pares, 0 errores, temporales borrados):
  núcleo 16 campos 13/13 slugs 100% igual; `publicado` igual; ficha contenido
  igual (solo difiere `inmueble_id`, distinto por diseño → excluirlo del
  canónico); cero renombres/duplicados. ORIGINALES 128/128 bytes idénticos.
  Único frente: `mejorada` de `casa-en-venta-en-altos-del-caron`: 4 solo en
  prod (0/12/13/14, falta la portada) + las 11 apareadas con bytes distintos
  (regeneradas en momentos distintos por lado); resto 116/116 idénticas.
  Nota P2: 514 descargas con concurrencia 6 tardaron 10 s → el miedo OOM no
  aplica con pool acotado. Dirección recomendada para ese slug: prod→local
  (prod tiene el juego completo 15/15), NO pisar prod. Decisión 1 pendiente.
- F1 — `verificar` + primera pasada (HECHA 2026-10-08, permanente en
  `scripts/inmueble.mjs:cmdVerificar`): reproduce el forense exacto (15 difs,
  exit 1, cero escrituras). DoD cumplido.
- F2 — `push-full` DESCARTADO 2026-10-09 con razón registrada: el forense
  demostró que prod tenía el juego completo (15/15 mejoradas) y local el
  incompleto; empujar local→prod habría destruido la portada y 3 mejoradas
  de prod. Dirección aprobada por ella ("si ok" 2026-10-09): prod→local.
  Ejecutado como pull quirúrgico con `sync-pull.mjs --slug
  casa-en-venta-en-altos-del-caron` (flag `--slug` añadido permanente 2026-
  10-09): núcleo + ficha + publicado + reemplazo total de las 27 fotos con
  bytes de prod. DoD: `verificar` exit 0 por slug y total + espejo OK.
- F3 — Docs: skill `publicar-inmuebles` (verificar + pull quirúrgico) +
  completada + roadmap + commits `08AA-35` + push. Gate sin hallazgos nuevos
  (re-análisis 2026-10-09: 0E/436W, igual que baseline).

## Cierre 2026-10-09 (convergencia total por pull, no por push)

- `verificar --slug casa-en-venta-en-altos-del-caron`: 1/27 = 1/27, 27 pares
  de bytes OK, exit 0. `verificar` total: prod 13/259 = local 13/259, 259
  pares (128 originales + 131 mejoradas) 0 errores, exit 0.
- Gotchas del pull (fijados en código, no solo en papel):
  1. `--dry-run` MENTÍA si iba seguido de otra flag: el parser
     `arr[i+1] ?? 'true'` tomaba `--slug` como valor de `dry-run` y ejecutó
     de verdad (borró 7 fotos locales antes de fallar). Fix permanente: el
     valor solo se consume si no empieza por `--`.
  2. El borrado foto-a-foto con lista stale muere con 404: borrar una
     `original` arrastra a su `mejorada` hermana (+ renumera). Fix
     permanente: drenaje tolerante con relectura hasta vaciar (cota 3×).
  3. Daño del run accidental: PUT núcleo/ficha con valores prod (= estado
     final deseado, sin efecto) + 7 fotos locales borradas (4 originales
     byte-idénticas a prod → recuperadas exactas del pull; 3-4 mejoradas
     locales con bytes distintos → sustituidas por las de prod según la
     dirección aprobada). Respaldo file-level previo al pull real: 16 fotos
     + manifiesto en `C:\tmp\backup-08AA-35-caron` (pg_dump no disponible en
     esta máquina).
- Decisión 2 (publicados) quedó sin objeto: `publicado` igual en los 13
  slugs. Decisión 3 (ventana push) quedó sin objeto: no hubo nada que
  empujar a prod; prod intacta en todo el bloque (cero escrituras en prod).

## Verificación (DoD global)

- [x] Forense F0 hecho (deriva + renombres + duplicados explicados).
- [x] Verificador calibra con testigo + P2 acotada sin OOM/timeout/bucle.
- [x] Tras pull: `verificar` exit 0 por slug y total (13/259 = 13/259, 259
  pares de bytes OK). Humo público sin objeto: prod intacta, nada cambió
  fuera.
- [x] Cero escrituras fuera de catálogo local; respaldo file-level
  `C:\tmp\backup-08AA-35-caron` (16 fotos + manifiesto) ante ausencia de
  pg_dump; prod con cero escrituras en todo el bloque.
- [x] Commit + push del bloque.

## Decisiones que requiere de ella (defaults tras retos) — RESUELTAS 2026-10-09

1. Dirección: aprobada prod→local para `casa-en-venta-en-altos-del-caron`
   ("si ok"). Ejecutado.
2. Publicados en conflicto: sin objeto (13/13 iguales).
3. Ventana: sin objeto (push-full descartado; convergencia por pull).
