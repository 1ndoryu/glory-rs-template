Objetivo: crear un template de Rust + Ts React + OpenAPI + Codegen + Clippy nivel paranoia, para crear sitios web, un solo repositorio, con una pagina sencilla de ejemplo, con el gitignore configurado y todo listo para empezar a desarrollar, adelante a todo, comandos, documentación, he creado https://github.com/1ndoryu/glory-rs para lo subas alli, se tiene que pdoer crear varias ramas para varios sitios y que no haya problema de cambiar a un sitio a otro cambiando de rama, aplica todo lo que por defecto vendría siendo optimización, seguridad, rendimiento, experiencia de desarrolo, etc, de base de datos tengo corriendo postgres aca localmente.

## Estado: ✅ Template base completado (253A-1)

Ver `Agente/completados/tareas-2026-03-25.md` para detalles.

## Stack implementado

| Capa | Herramienta |
|------|-------------|
| Framework web | Axum 0.7 |
| OpenAPI | utoipa 4 + utoipa-swagger-ui 7 |
| Serialización | serde |
| Base de datos | SQLx 0.8 (PostgreSQL) |
| Migraciones | SQLx migrate |
| Validación | validator 0.20 |
| Variables de entorno | dotenvy |
| Logging | tracing + tracing-subscriber |
| Errores | thiserror 2 |
| Auth | jsonwebtoken + argon2 |
| CORS | tower-http |
| Linter | clippy (deny all + warn pedantic) |
| Frontend | React 18 + TypeScript + Vite |
| State | React Query + Zustand |
| Codegen | Orval 8 (reemplaza openapi-typescript-codegen) |

## Pendientes

- **09AA-14 — Conectar el flotante al núcleo local (pedido por ella
  2026-10-10, EN CURSO):** la ventanita negra cocinaba sola con receta vieja
  (plantilla "sigue disponible + precio" con solo el título, intención
  `desconocida`) y por eso los Logs salían vacíos (solo anotan el backend).
  Hecho: `MP_CLI_MINUTOS=5256000` (10 años, atado a máquina, revocable por
  `jti`) en `.env` gitignored + token CLI emitido (`jti 2a82d043…`, exp 2036)
  + `MP_SAL` 32 bytes + los 4 valores en `.env` y en env de usuario (`setx`:
  `MP_NUCLEO=on`, `MP_MN_TOKEN`, `MP_SAL`, `MP_MN_API`) — surten efecto cuando
  ella reinicie su app (yo nunca la toco). Verificado de extremo a extremo
  replicando al flotante: `/borrador` con firma-v1 → `fuente=ia` con receta
  09AA-2/09AA-4 + evento `borrador.ia` en Logs. Vivo `29028` debe seguir
  encendido al usar Facebook (si está apagado, el flotante vuelve a local en
  silencio). 2026-10-10: a pedido de ella cerré el lab viejo y lo reabrí yo
  con su misma receta (channel propio, MP_NUCLEO=on, su token+sal+API 3110;
  wrapper pid 1320, :5174, 6 electron) — su app viva ni se tocó. Gotcha: un
  `bun run dev` huérfano de ayer (hijo del lanzador muerto) tumbaba el
  arranque nuevo con `exit 255` sin más texto; matar el huérfano antes lo
  arregló. El backend NO verifica la sal (firma-v1 = clave de caché opaca);
  lo que exige es el token (MpAuth). Pendiente: ella prueba Facebook en el
  lab y confirma borrador del núcleo en Logs. Mejora opcional en el lab: marcar en la línea debug qué cocinó
  cada borrador (núcleo vs local). Causa raíz del "sigue la plantilla"
  (01:35): el float abortaba a los 20s (`AbortSignal.timeout`, log
  `http=red`) y la IA tarda 13-33s → caía al motor local; el backend sí
  respondía 200 tarde. Fix SOLO en lab (`marketplace-nucleo.ts`:
  `NUCLEO_TIMEOUT_MS` 20s→90s; el lab no es git, sin commit) + lab
  reiniciado (:5174, esta vez con token 10y, equivalente). Gotchas: los Logs
  son buffer en memoria — reiniciar el vivo los borra (la caché DB
  sobrevive); `bun dev` sin `OPENCODE_CHANNEL` muere con exit 128/255
  (`predev`→`git branch`, el lab no es git). Pendiente: ella Regenera en
  Jorge y espera ~60s; promover el timeout a opencode-propio SOLO cuando
  ella dé ventana (guardar archivos rompe su app viva); después, fix del
  scraper (lee chrome, no burbujas).
- **09AA-7 — Invariante persistencia mensajería (09AA-6 §8.1, siguiente
  bloque ejecutable):** `responder` idempotente + transaccional,
  dead-letter `pending`. NO toca regenerar. Sin empezar hasta cerrar
  09AA-4/09AA-5 (mismo pipeline) o serializar con esas sesiones.
- **09AA-13 — Regenerar: loop + conserva (BLOQUEADA, la testea él):**
  comportamiento actual de `regenerar_todo` intencional, funcionalidad
  sin terminar. No tocar hasta que avise; coordinar con 09AA-4.
- **09AA-8 — Webhook fail-closed + tope media (09AA-6 §8.2).**
- **09AA-9 — Auth con rol en consola staff (09AA-6 §8.3, requiere verificar
  contrato float opencode-propio antes de codificar).**
- **09AA-10 — Dedup + espejo M3 (09AA-6 §8.4).**
- **09AA-11 — Proveedores IA: abstracción + reintento glory coordinado con
  09AA-4 (09AA-6 §8.5).**
- **09AA-12 — Frontend chat: sub-hook + feedback visible (09AA-6 §8.6,
  paralelizable).**
- **09AA-4 — Regenerar es un solo botón + IA vacía con reintento (pedido por
  ella 2026-10-09, ACTIVA):** su prueba de 09AA-3 falló: al abrir la
  conversación sigue el texto viejo. Causas confirmadas: (1) OpenCode Go
  devuelve 200 sin `message` (WARN `IA caída (... respuesta sin texto)`,
  latencia 12s; el relay está sano — replay mínimo `completed` con 118
  tokens; sin `AI incompleta` en el log = no es tope, es vacío del relay);
  (2) el `conserva` de 09AA-3 devuelve lo viejo en `reserva` = literalmente
  «el mensaje cacheado»; (3) cada excerpt nuevo es firma nueva y las filas
  viejas viven 90 días (el panel las lista junto a la fresca). Fix:
  `completar_opencode` a 8000 tokens + 1 reintento ante vacío + WARN con la
  forma cruda (estado/tipos/uso/motivo, sin PII); `regenerar_uno` borra
  primero las filas no-corregidas del hilo (correcciones intactas) y NO
  conserva nada si la IA cae; panel con un solo botón «Regenerar» (fuera
   «Limpiar»; el endpoint `DELETE /chats` queda como API admin). Añadido
   2026-10-09 noche: sesión estable en `x-opencode-session` — el relay exige
   ese header (400 `MissingSessionID` sin él) y premia la estabilidad con
   ruteo afín y prompt caching; antes mandábamos uuid fresco por llamada
   (evita el 400 pero rompe la afinidad y parece abuso). Ahora
   `completar_opencode(..., sesion)` recibe `sha_hex(clave_hilo(thread))` en
   borradores (hash, jamás PII en claro), `"centro-ia"` en el centro IA y
   `"fotos"` en descripción de fotos. Gate: fmt 0 + clippy 0 + test 153/153;
   vivo `9280` binario 23:13 (health OK). Falta: prueba real con JWT
   (`/borrador` → confirmar `fuente=ia`, sin WARN de vacío).
- **09AA-5 — Tab de Logs del puente (pedido por ella 2026-10-09, ACTIVA):**
  en algunas conversaciones sigue saliendo la plantilla P1
  («Hola, buenas noches. Sí, sigue disponible… 43.000$…») en vez de la IA.
  El log del vivo local solo tiene polling del lab: sus pruebas NO llegan a
  local (¿está probando en prod, que aún no tiene 09AA-4?). Fix-observabilidad:
  buffer en memoria (500 eventos) + `GET /api/admin/marketplace/logs`
  (solo admin; hilos como hash-8, jamás PII) con eventos `borrador.cache`,
  `borrador.ia` (fuente+latencia+reintento), `ia.vacia` (WARN forma),
  `ia.reintento_ok`, `regenerar` (borradas+fuente), `regenerar-todo`
  (resumen); panel con tab «Logs»: tabla Hora|Nivel|Estado|Evento|Mensaje,
  filtro por nivel, click → modal con detalle (prioridad: error=alta,
  warn=media, info=baja), auto-refresh 5s + pausa.
- **09AA-3 — Regenerar-todo + dieta del prompt (pedido por ella 2026-10-09,
  CERRADA pendiente de su prueba):** (1) botón «Regenerar todo» (secondary)
  al lado de Recargar, con confirmación y resumen
  (`Regenerados X de N (+Y en reserva, +Z omitidos)`):
  `POST /api/admin/marketplace/regenerar-todo` (solo admin) regenera EN SERIE
  cada fila con borrador (`filas_para_regenerar`: salta `corregida=TRUE` y
  `respuesta=''`, recientes-primero), responde
  `{candidatos, regenerados, en_reserva, omitidos, detalle[]}`; (2) dieta del
  prompt (~-40%: bans fusionados, la forma la impone Rust desde 09AA-2);
  (3) `Regenerar` ya NO borra en reserva: `regenerar_uno` extraído conserva
  el viejo si la IA cae. Gate: fmt 0 + tsc 0 + clippy 0 + test 151/151
  (nuevo `regen_solo_borradores_no_corregidos` con testigo a/b/c). Commit
  `b6308adf` (origin+template); vivo `5612` binario 22:27 (health OK,
  POST sin token → 401, openapi lista `regenerar-todo`). Gotcha: el vivo real
  era el hijo `21024`, no el `cmd 24304`; `23732` PROYECTO TASKS ajeno
  preservado. Respuesta honesta pendiente en el chat: NO hay perilla
  minimal/low (provider solo `max_output_tokens`+`timeout_secs`, relay 400 a
  extras) y la «plantilla» a veces es del puente local (timeout puente 20s
  vs backend 120s) — subirlo queda para el lab de opencode-propio.
- **08AA-39 — Botón Limpiar chats (pedido por ella 2026-10-08, CERRADA
  pendiente de su prueba):** botón «Limpiar» (destructive) al lado de
  «Recargar», con confirmación; `DELETE /api/admin/marketplace/chats`
  (solo admin) → `borrar_todo_cache` (responde `{"borrados": n}`).
  Gate: fmt 0 + tsc 0 + clippy 0 + test 143/143. Commit `463d70ef`
  (origin+template); vivo `21688` binario 21:39 (health OK, sonda DELETE
  sin token → 401 = ruta+guard OK); frontend dev recarga solo el botón.
  Sus 2 filas intactas. Gotchas en completada.
- **09AA-2 — Fix definitivo párrafo de relleno (pedido por ella 2026-10-09,
  CERRADA verificada por ella 2026-10-09):** 4 subagentes confirmaron la raíz: el
  prompt mismo ORDENABA el relleno («avanza la conversación: ofrece fotos o
  pregunta qué busca») y esa orden positiva siempre le ganó al veto; además
  «máximo 3» + 3 roles se leía como «exactamente 3». Fix en 4 puntos:
  `imponer_forma_borrador` en `services/marketplace.rs` (poda determinista
  por intención + final canónico reconstruido, excepción 1 línea solo para
  dato concreto no cubierto), prompt reescrito a 2 bloques + excepción +
  veto a `?/¿` fuera del final, poda conectada en rama IA de
  `generar_borrador`, fallback sin «confirmo». Gate: fmt 0 + clippy 0 +
  test 150/150 (7 tests nuevos con testigos reales). Commit `2697e6b2`
  (origin+template); vivo `24304` binario 22:04 health OK. Verificada por
  ella con Regenerar en el panel (era caché vieja). Sin deploy prod todavía.
- **08AA-38 — 2º párrafo: regla estructural + veto por palabras (reportado
  por ella 2026-10-08, CERRADA sustituida por 09AA-2):** «Sí, la publicación
  sigue vigente» burló la lista de frases de 08AA-37 con un sinónimo. El
  veto por palabras no bastó (el prompt ordenaba el relleno); ver 09AA-2.
  Commit `2df97c16` (origin+template). Sus 2 filas (sicilia/melisa 20:49)
  SÍ se guardaron — el «no apareció» fue mi limpieza 20:44 + timing.
- **08AA-37 — Borrador: nombre mal + 2º párrafo repite (reportado por ella
  2026-10-08 con mensaje pegado, ACTIVA):** con hilo `marializ|...` saludó
  «Ordaz» (tomó apellido/lugar del excerpt) y el 2º párrafo repitió lo del
  1º («Sí, se mantiene publicada en venta al momento»). Fix en
  `handlers/marketplace.rs` (`saludo_y_regla`: nombre del hilo único válido;
  2º párrafo jamás reafirma disponibilidad/precio ni usa «publicada»/
  «estatus»/«ficha»). Gate: fmt 0 + clippy 0 + test 143/143. Commit
  `ced4cc2d` (origin+template). Vivo `29332` con binario 20:43; chats
  limpiados (0) para que pruebe.
- **08AA-36 — Borrador repite "confirmo con la dueña" (reportado por ella
  2026-10-08 con mensaje pegado, ACTIVA):** el prompt ordenaba "di solo que
  está disponible y que lo confirmas con ella" (`marketplace.rs`, testigo:
  fila guardada olear "está disponible y lo confirmo con la dueña"; su
  mensaje lo trae 2 veces). Fix: prohibir anunciar confirmación con la
  dueña (el dato se da una sola vez) + borrar las 7 filas guardadas de
  Riberas para empezar limpio.
- **08AA-35 — Detector diferencias prod↔local + convergencia (CERRADA
  2026-10-09):** `verificar` permanente (`inmueble.mjs`, exit 0/1/2, cero
  escrituras) + pull quirúrgico `sync-pull.mjs --slug` (flag nuevo; parser
  `--dry-run` corregido + drenaje tolerante con relectura ante cascada
  original→mejorada). Forense: único frente `mejorada` de
  `casa-en-venta-en-altos-del-caron`; `push-full` descartado con razón (prod
  tenía el juego completo) y convergencia por pull prod→local aprobado por
  ella. Evidencia: `verificar` total LIMPIO exit 0 (13/259 = 13/259, 259
  pares de bytes OK); gate re-analizado 0E/436W sin nuevos; respaldo
  file-level en `C:\tmp\backup-08AA-35-caron`; prod intacta. Plan archivado
  en `Agente/planes/completados/plan-verificar-sync-push-2026-10-08.md`,
  detalle en `Agente/completados/tareas-2026-10-09.md`.
- **08AA-34 — Migrar /ask a componentes del sistema (hallazgo gate 2026-10-08
  cerrando 08AA-33):** 15 `html-nativo-en-vez-de-componente` en
  `entrada-pregunta.tsx` (8) y `pagina-ask.tsx` (7); son `<button>`/`<input>`
  preexistentes, no del bloque 08AA-33 (ese usó `Boton`/`Dialog`). Cambiar a
  `Boton`/`Input` del sistema + verificar /ask en vivo.
- **08AA-32 — Extraer 5 hooks `componente-sin-hook` (cola de 08AA-26, CERRADA 2026-10-08):**
  `useHiloMensajes`, `useSesionesWhatsapp`, `usePestanaIA`, `useTarjetaFotoMejora`,
  `useModalDescargarFotos`; componentes con solo JSX + helpers puros.
  Evidencia: `tsc -b` 0 errores; sentinel `frontend/src` 0 `sin-hook`
  (85→79 findings, 0 nuevos); gate `0E/435W/0H` (sin `componente-sin-hook`).
- **08AA-30 — Unificar `thread_id` borrador vs releer (hallazgo 2026-10-08
  verificando 08AA-28 en vivo):** `borrador` guarda `thread_id` literal
  (`handlers/marketplace.rs:197`) mientras `releer`/`buscar` usan
  `clave_hilo()` (`:363`): el hilo Yusmelis quedó en 2 filas (`...puerto
  ordaz.` con borrador viejo 320 + `...puerto ordaz` solo-foto nueva 353).
  Decidir forma canónica y migrar/fusionar (ver filas en
  `glory_backend_inmobiliaria.mp_respuestas_cache`).
- **08AA-26 — Reparar reglas Sentinel con FPs documentados (pedido por ella
  2026-10-08 tras 08AA-23; reglas CERRADAS 2026-10-08):** gate `0E/439W/0H`,
  7 reglas del alcance a 0 (`ruta-post`, `path-join`, `key-index`, `promise`,
  `mixed-barrel`, `large-interface`, `inline-style`). Rama
  `fix/08AA-26-reglas-fp` (`748a387`, full sentinel 749 passing) pusheada a
  `origin` (solo la rama; `main` de sentinel intacto). `sentinel.lock.json`
  sigue en 0.7.13 a propósito: `sentinel update --dry-run` instalaría
  topología `versions/` ajena al consumo por checkout compartido y nada del
  gate lee la versión del lock (solo manifiesto) — decisión SEGUIR-POR-FUENTE
  adoptada este turno (reversible; pin por versión solo con pedido explícito). TPs
  honestos sin tocar: `sqlite-carga-N` en bucle `glory-rs/.../sync.rs`
  (submódulo ajeno) y `componente-sin-hook` x5 (refactor 5 hooks
  pendiente). Evidencia: `Agente/completados/tareas-08AA-26-cierre-2026-10-08.md`
  + `prevencion-sentinel-fp-frontend-08AA-26-2026-10-08.md`; commit MN
  `48e0a9d4` en `origin main`. Flujo según
  `area-trabajo/Agente/documentacion/mantenimiento-herramientas-calidad-2026-10-06.md`.
- **08AA-21 — Guardar excerpt crudo junto al limpio (pedido por ella
  2026-10-08, verificado vivo 2026-10-08, commit `e50839a1`):** columna
  `excerpt_crudo TEXT` (migración `...32`, NULL en filas viejas);
  `borrador`/`regenerar`/`releer` guardan el texto tal como llegó además
  del limpio (struct `FotoHilo`: clippy no admite 8 args). Verificado:
  `/borrador` sintético con ruido pegado estilo wilmery → `fuente=ia`
  y `excerpt_texto` = `excerpt_crudo` = texto enviado — confirma que sin
  saltos de línea el filtro no toca nada (pista para el 08AA-8).
  Fila de prueba borrada, `wilmery` intacta.
- **08AA-8 — Excerpt Marketplace sigue sucio con chrome en español
  (reportado por ella 2026-10-08 con captura, hilo andreina):** el 08AA-5
  calibró con el fixture (chrome en inglés) pero el hilo real trae
  `Marketplace` suelto, `View buyer`/`More options` con mayúscula,
  `Mensajes`, `Presionar Enter, Detalles de la conversación`,
  `Ver perfil del comprador`, `Mensaje enviado 11:30 pm por Andreina:`,
  `Escribir mensaje`/`Escribe en …`, títulos con `·` en vez de `-`
  (rompe el dedup) y el mensaje del comprador duplicado con prefijos
  distintos. Testigo exacto en BD: `mp_respuestas_cache`
  `thread_id='andreina|VEF0 casa en venta en riberas del caroní, puerto
  ordaz'`, `length(excerpt_texto)=614`. La respuesta/borrador sí sale
  bien (4 partes + contacto + wa.me). Alcance: extender
  `RUIDO_EXCERPT_*` en `src/services/marketplace.rs` con variantes ES
  (insensible a mayúsculas donde sea seguro) + dedup tolerante a `·`/`-`
  + test con ese excerpt exacto + regenerar la fila y verificar en el
  panel. Pregunta abierta a ella: además de limpio, ¿quiere que el
   excerpt se vea como chat (burbujas por mensaje) o basta el texto
   limpio? Lo segundo es cambio mayor (hoy solo se guarda
   `excerpt_texto` plano).
   Verificado 2026-10-08 (filtro pegado sin saltos): `segmentar_pegado()`
   + extras ES en `RUIDO_EXCERPT_*` (`marketplace_texto.rs`); testigo
   wilmery 554 → `Hola. ¿Sigue estando disponible?` (32) en vivo
   (`/releer` → `actualizado:true`; crudo 554 guardado). Gate 138/138.
   Queda abierta la 08AA-8b y la pregunta del chat.
- **08AA-8b — El excerpt no incluye la respuesta de ella (reportado por
  ella 2026-10-08: respondió en Messenger y su mensaje no aparece):**
  dos causas confirmadas en código. (1) El float sí distingue lados
  (`mpDialogo()` en `marketplace-float.ts:181-214`: izq=`Cliente:`,
  der=`Dueña:`), pero cae a texto plano sin marcas cuando la pestaña
  está en fondo (`getBoundingClientRect` en 0 → `""` → `texto=full`,
  línea 301); el excerpt guardado de andreina no trae ni una marca,
  luego se generó en plano. (2) Aunque el DOM cambie, el servicio
  reutiliza el borrador en caché (`marketplace-service.ts:191-195`:
  "con borrador en caché se reutiliza aunque el texto cambie"); solo
  ventana nueva o Regenerar generan y guardan excerpt fresco en el
  backend. O sea: su respuesta posterior no refresca nada hasta
  Regenerar. Workaround inmediato: pulsar **Regenerar en el float del
  lab** (manda excerpt fresco + genera de nuevo). Alcance del fix:
  política de refresco del excerpt ante mensajes nuevos (distinguiendo
  ruido DOM de mensaje real por firma, sin romper el 08AA-1) +
  rescatar marcas Cliente/Dueña aun con pestaña en fondo si el DOM lo
  permite.
- **08AA-9 — Botón Releer separado (pedido por ella 2026-10-08):** el
  float del lab lleva un segundo botón junto a Regenerar que manda el
  excerpt fresco al backend SIN regenerar el borrador (solo refresca la
  foto del hilo en el panel). Backend: `POST
  /api/admin/marketplace/releer` (`thread_id` + `excerpt`, normaliza con
  `normalizar_excerpt`, `UPDATE mp_respuestas_cache SET excerpt_texto`
  por `thread_id`, sin IA, con tope por minuto). Lab: botón `.mp-rere`
   + `releerNucleoParaVentana()` + strings `reread`. **Estado
  2026-10-08:** implementado y verificado vivo (`actualizado:true`,
  `length(excerpt_texto)` andreina 614→222). Lab pendiente de
  promocionar en ventana explícita.
- **08AA-10 — El borrador no da el precio de Riberas (pedido por ella):**
  causa raíz confirmada: `claves_cache(pool, None)` (piloto: `avisoId`
  siempre null) nunca intenta emparejar el título del hilo contra
  `inmuebles`, y `precio_del_aviso("VEF0 casa...")` da None (el 0 no
  vale como cifra). El catálogo SÍ tiene `Casa en venta en Riberas del
  Caroní` $43.000. Fix en el backend (fuente de verdad, no el DOM):
  `ficha_por_titulo()` (normaliza tildes/caja, directo por includes,
  si no solape ≥3 con ≥1 palabra distintiva no genérica, solo
   publicados; fallo de BD → None sin bloquear) usada por `borrador` y
  `regenerar` cuando no hay ficha por UUID, con sus hashes reales.
  **Estado 2026-10-08:** verificado vivo (`/regenerar` andreina →
  `aviso_conocido:true`, `fuente:reserva`). Texto final con precio
  pendiente: el proveedor IA (OpenCode Go) devuelve vacío hoy
  (`/ia/probar` cuelga; ambiental, sin tocar `ia*.rs`).
- **08AA-11 — Borrador largo y con saltos incoherentes (pedido por
  ella):** el prompt pedía "máximo 6 líneas" y la IA devuelve líneas
  sueltas sin separación. Fix: prompt exige 4 párrafos cortos separados
  por línea en blanco + `formatear_parrafos()` post-IA (colapsa 3+,
  parte `Cualquier cosa escríbeme al TEL` y el enlace wa.me cada uno a
   su propia línea) aplicado tras `asegurar_contacto`, con tests.
  **Estado 2026-10-08:** implementado, gate verde (133 tests);
   verificación visual del texto pendiente del proveedor IA.
- **08AA-15 — Borrador breve 3 párrafos con nombre corto (pedido por
  ella 2026-10-08, en curso):** ejemplo suyo `Hola Karely, buenas
  noches, el Apartamento de Residencias Caroni (nombre corto) esta
  disponible y tiene un valor de $43.000 negociable.` + CTA + contacto.
  Alcance: prompt `BREVE: máximo 3 párrafos cortos` + regla de nombre
  corto (solo tipo + residencia, sin dirección ni zona duplicada) en
  `src/handlers/marketplace.rs`.
  **Estado 2026-10-08:** prompt aplicado en código; verificación viva
  pendiente (el proveedor IA devuelve respuesta sin texto).
- **08AA-6 — Barrido progresivo baseline sentinel (activa 2026-10-08, no
  urgente)**: el tablero marca ~518 en MN (26E/483A preexistentes, verificados
  2026-10-08: ningún hallazgo nuevo de 08AA-1/3/4; el conteo incluso bajó 9).
  Por clases: `sqlx-query-sin-macro/as`, `handler-accede-bd-rs`,
  `html-nativo`, `ruta-post-sin-rate-limit` x21 (FP documentado, no reescribir),
  `todo-prosa` front + 4 recomendaciones `sentinel.config.json` + `varsense
  ausente` (declarar o eximir) + rsa residual sin parche. Abordar por clases,
  sin mezclar con frentes; `dev deriva 3102` es gateway parado normal (no
  arrancar sin QR/autorización) y `sinPush` era caché pre-push.
  Verificado 2026-10-08 ~22:40: re-escaneo gate forzado idéntico 26E/476W/7H
  (cero nuevos); secret-scan limpio en `d7d3b6d4`+`aeee01c6`; 26E = 21
  ruta-post (FP documentado) + 2 broadcast-mutex
   (`services/marketplace.rs:827,844`) + 2 god-object
   (`handlers/chat_staff.rs`, `handlers/chat_tools.rs`) + 1 path-join (FP
   documentado). Lo corregible real: broadcast/god-object por refactor.
- **08AA-6 — Refactor broadcast-mutex + god-object (cerrado 2026-10-08,
  ver `Agente/completados/tareas-2026-10-08.md`)**: fan-out `mpsc`
  + split `chat_staff_config`/`chat_tools_definiciones`. Gate calibrado
  (pares `ruleId|archivo` baseline vs post): errores 26→22 (−2
  broadcast eliminados, −2 god-object degradados a warning), warnings
  +5 (+2 degradados + 3 umbrales marginales por líneas netas: `mod.rs`
  502/500, `services/marketplace.rs` 727/700). Cero errores nuevos.
- **08AA-7 — Partir `handlers/mod.rs` y `services/marketplace.rs`
  (cerrado 2026-10-08, ver `Agente/completados/tareas-2026-10-08.md`)**:
  `superficie.rs` (SPA + SEO + catálogo agente) y `marketplace_vuelo.rs`
  (Singleflight + test, re-exportado). Gate `22E/478W/7H`: caen los 3
  umbrales marginales de 08AA-6; cero hallazgos en archivos nuevos.

## Deploy mn-inmobiliaria.com (239A-1, en curso 2026-09-23)

- Plan: `Agente/planes/plan-deploy-produccion-2026-09-23.md`.
- Monorepo: `INMOBILIARIA/*` → `frontend/`; servir `STATIC_DIR` + fallback SPA;
  CORS prod solo dominio; `Dockerfile.rust` multi-stage (`VITE_API_URL` build-arg).
- Fase 0 cerrada 2026-09-23: commits receta+mejora (INMOBILIARIA `5a113f5`,
  `70a6361`) y suscriptor+plan (`6cd8a90e`, `3ac8ca46`); `.env` a
  `glory_backend_inmobiliaria`; gate local verde (tsc, vite build, fmt, check,
  clippy `-D warnings`, test 15/15).
- Mejora IA queda solo-local (prompt Ultra HD + descarga `=s0` validados);
  cookies fuera del VPS; sync prod→local solo lectura.
- Fase 1 cerrada 2026-09-23 (`39f383de`+`4c9f1d45`): humo local `/`+fallback SPA 200,
  `/api/health` 200, 11 publicados. En prod manda el `Dockerfile.rust` del template del
  manager (no el del repo): 239A-2 (`944bacf8`) cambió `glory-agent` a git-dep pineado
  `f2f19e7` tras el fallo `failed to read /glory-agent/Cargo.toml`.
- Fase 3+4 hechas 2026-09-23: manager recompilado (1.0.0), sitio `inmobiliaria`
  (`as0scgwg44wkkkccgwcwg8w0`) creado. `sync-env push` bloqueado (el manager exige claves
  Stripe que el proyecto no usa; mejora pendiente a la herramienta, sin dummies).
- Incidencia 2026-09-24 CERRADA: `Servidor no disponible` con API sana = `VITE_API_URL`
  sin hornear (bundle con `127.0.0.1:3000`). Fix herramienta (`d908b94` template
  `ARG VITE_API_URL`, `3008e65` push `--only` exime trio Stripe) + push var + rebuild
  6/6 OK; bundle `index-EZwYRSQX.js` con dominio; API `total=11` fotos `218`.
  Detalle en plan (seccion incidencia) y `Agente/completados/tareas-2026-09-24.md`.
  Pendiente del usuario: recargar https://mn-inmobiliaria.com y confirmar lista visible.
- Fase 5 COMPLETADA 2026-09-24: dump data-only + `run-sql --file` (el `import` del
  manager es solo-WP) → 11/218/2; 218 fotos vía `POST /api/admin/fotos/upload` (el
  manager no tiene push a volúmenes); admin rotado a password nueva generada,
  `import@example.com` eliminado.
- Fase 6.3 E2E COMPLETADO 2026-09-24: 11 publicados, fotos HD 200, login admin, receta
  PUT→pública→revertida, solicitud POST→admin→borrada, WhatsApp horneado.
  Estado prod: 11 inmuebles, 218 fotos, 0 solicitudes, 1 user.
- Requiere del usuario: nueva password admin para la rotación de Fase 5.4 (o confirmar
  mantener la actual / que la genere yo).

## Optimizacion PageSpeed movil + SEO profundo (249A-1, completada 2026-09-24)

- Cerrada y desplegada: ver `Agente/completados/tareas-2026-09-24.md` y plan en
  `Agente/planes/completados/plan-optimizacion-pagespeed-seo-2026-09-24.md`.
- Restos abiertos: F8 SEO profundo (h1/alt/canonical/JSON-LD `ItemList` si falta);
  rerun PageSpeed movil (la API devolvia 429 por cuota).
- 249A-3 (2026-09-24, desplegada): preload del heroe muerto eliminado
  (`PresentacionCaja` no se renderiza; 502 KB que competian con JS/CSS) + `cn`
  local (`clsx`+`tailwind-merge`) en vez del paquete `cn`; JS 192.73 KB gzip.
  Ver `Agente/completados/tareas-2026-09-24.md`.
- Requiere del usuario: confirmar que https://mn-inmobiliaria.com muestra la lista
  y correr PageSpeed movil para el puntaje final.
- 249A-4b+249A-4d (2026-09-24, desplegadas commit `c8ae5341`): thumbs de tabla
  320→160 (`min160-<uuid>.jpg`, legacy `thumb-` con backfill+borrado al servir)
  + `llms.txt` y `/.well-known/ai-catalog.json` dinámicos desde el backend
  (11 entries, URNs saneadas, validados con el tester oficial ARD: 0 errores);
  prebuild `generar-llms.mjs` y `public/llms.txt` eliminados; `lighthouse`,
  `@radix-ui/*` y `shadcn` desinstalados del front. Verify prod: `/llms.txt`
  200 6092 B, ai-catalog 200 11 entries, thumb `min160-` 200 jpeg, health 200.
  Ver `Agente/completados/tareas-2026-09-24.md`.
- 249A-4a+249A-4c (2026-09-24, desplegadas commit `80b6ba2e`): fallback
  metric-matched `Söhne-relevo`/`Kräftig→Arial Bold` en `src/index.css`
  (CLS footer 0.195), primera tarjeta `eager`+`fetchpriority=high` resto
  `lazy`, `Agentmap:` en `robots.txt` + `<link rel=ai-catalog>` en
  `index.html` (4/4 navegación agéntica), 5 overlays públicos tras
  `React.lazy` (`modal-filtros 5.16kB, detalle 6.45kB, publicar 7.67kB,
  chat 11.18kB, dialog 106.97kB gzip 35.70kB`). Verify prod: `robots.txt`
  con `Agentmap:`, `/` con `rel=ai-catalog`, bundle nuevo
  `index-D8vNccQi.js` + chunks `inmueble/plantilla-publicidad`.
  Ver `Agente/completados/tareas-2026-09-24.md`.
- 249A-5 (2026-09-24, desplegada commit `6cbe0134`): PSI escritorio
  79/100/100/92 + 3/4 agéntica → `Agentmap:` fuera de `robots.txt`
  (directiva desconocida para Google, SEO 92→100; descubrimiento solo
  vía `<link rel=ai-catalog>`), 5 overlays `lazy` con pestillo de montaje
  (`useMontarAlAbrir`: montan al primer uso, fuera de la ruta crítica,
  conservan animación de cierre), esqueleto de 10 filas `h-[100px]` en
  vez de `Cargando…` + conteos de píldoras con ancho fijo + reserva de
  alto de paginación (CLS 0.484→~0). Verify prod: `robots.txt` sin
  `Agentmap:`, bundle nuevo `index-Br2UKVtj.js` solo con preloads
  estáticos (`inmueble`, `plantilla-publicidad`). Ver
  `Agente/completados/tareas-2026-09-24.md`.
- Restos abiertos de 249A: F8 SEO profundo, rerun PageSpeed escritorio y
  móvil tras 249A-5 (cuota API 429 el 2026-09-24; último manual escritorio
  79/100/100/92 + 3/4, móvil 88/100/100/100 + 3/4), y el sub-chequeo
  agéntico que sigue en 3/4 (pendiente: nombre del chequeo fallido
  expandiendo la sección en pagespeed.web.dev).

## Receta publicitaria en el servidor (229A-2, en curso 2026-09-22)

- El modal público genera la publi con receta automática; lo configurado en
  admin vive solo en `localStorage` del admin y nunca llega al público.
- Backend: columna `inmuebles.receta JSONB` (migración `...12`), struct
  `RecetaPublicidad` validada (formato allowlist, índices >= 0), `PUT`
  la fija, el `GET` público la expone.
- Front (repo `INMOBILIARIA`): `guardar` hace PUT con la receta; el modal
  público usa `i.receta` (servidor manda, local queda de reserva).

## Notas

- Configurar coolify-manager-rs para desplegar proyectos Rust (repo separado)
- Prioridades: 1. Velocidad desarrollo, 2. Decisiones futuras, 3. Rendimiento, 4. Seguridad, 5. Popularidad, 6. Facilidad, 7. Docs, 8. Compatibilidad, 9. Flexibilidad, 10. Escalabilidad

## F5 chat visitante con IA (169A-1, en curso 2026-09-16)

- Plan: `Agente/planes/plan-chat-inmobiliaria-2026-09-16.md`.
- Backend: `glory-agent` por path, migracion `agent_*`, rutas
  `/api/agent/*` (WS + REST + historial), PromptConfig inmobiliario.
- Front (`INMOBILIARIA`, repo hermano): widget en `src/features/publica/`
  montado en `PaginaPublica`, tokens de `disenno.ts` (sin redondeados,
  sin sombras, Söhne).
- Alcance v1: sin tools (la ejecucion de tools es F6 en `glory-agent`).
- Requiere del usuario: fijar `OPENCODE_GO_API_KEY` y `AGENTE_CONTACTO`
  reales en `MN-Inmobiliaria/.env` (sin key el chat persiste y reenvia
  en realtime pero la IA no responde).

## Publicar mi inmueble: solicitudes pendientes (169A-2, en curso 2026-09-16)

- Plan: `Agente/planes/plan-solicitudes-2026-09-16.md`.
- Backend: tabla `solicitudes`, subida publica de fotos (reutiliza
  validacion magic-bytes + 10 MiB), `POST /api/public/solicitudes`,
  `POST /api/public/solicitudes/fotos`, `GET/PATCH /api/admin/solicitudes`
  (JWT). Validacion basica de email/telefono.
- Front (`INMOBILIARIA`, repo hermano): `ModalPublicar` cuadrado con
  borrador persistente; las fotos ya subidas son claves (no pesan).
- Sin UI admin de revision (pendiente explicito para despues).

## Chat con atencion humana + WhatsApp + config admin (169A-3..6, 2026-09-16)

- Plan: `Agente/planes/completados/plan-chat-atencion-2026-09-16.md`
  (cerrado 2026-09-16; diseno contra Nakomi como referencia
  solo-lectura).
- 169A-3 (repo `glory-agent`): loop de tools F6 + gate IA real
  (`ai_enabled` + ciclo `escalated` se hacen cumplir) + `0002_config.sql`
  + persistencia sesiones/config + tests. Nucleo agnostico.
- 169A-4 (este repo) [x] 2026-09-16: 5 tools (`buscar_inmuebles`,
  `detalle_inmueble`, `registrar_contacto`, `datos_contacto`,
  `escalar_a_humano`) + worker WhatsApp (outbox, poll 15 s) + rutas staff
  `/api/admin/agent/*` (JWT) + publicas `/api/agent/info` y
  `/sesiones/:id/contacto` + hub compartido. Sin seeds: claves opcionales
  con defaults en codigo. Gate verde (fmt/check/clippy/test 11 lib OK);
  humo BD rama OK (tools contra esquema real, staff toma hilo, config
  roundtrip, worker marca `failed` sin gateway, 2 tests HTTP OK).
- 169A-5 (repo `INMOBILIARIA`) [x] 2026-09-16, commit `fae846e`: admin
  Mensajes (lista + hilo + responder + tomar/soltar IA) y Config chat
  (prompt_extra, telefonos, kill-switch). `oxlint` 0 errores, `npm run
  build` verde, dev 200. Incluye frente visitante 169A-1 (cliente, canal
  WS+REST, hook, ventana).
- 169A-6 (repo `INMOBILIARIA`) [x] 2026-09-16, commit `fa745e0`: tarjeta
  contacto/escalado en el widget (`pedirInfo` + `enviarContacto`,
  auto-apertura si pide humano) + E2E en vivo contra este backend
  (health/messages/history/info/contacto OK, staff 401 sin token).
- Requiere del usuario: `GLORY_ALERT_GATEWAY_URL` + numero admin
  (`whatsapp_admin`); `OPENCODE_GO_API_KEY` + `AGENTE_CONTACTO` siguen
  pendientes de 169A-1.

## Centro de IA: GloryAPI + OpenCode Go via backend (199A-1/199A-2, en curso 2026-09-19)

- Plan: `Agente/planes/plan-ia-central-2026-09-19.md`.
- 199A-1 (este repo): `src/handlers/ia.rs` con `GET /api/admin/ia/estado`,
  `PUT /api/admin/ia/config`, `POST /api/admin/ia/probar`,
  `POST /api/admin/ia/completar` (JWT); config en `agent_config`
  (`ia_activo`, `ia_hab_*`, `ia_check_*`); claves solo en `.env`
  (`GLORY_API_URL`, `GLORY_API_KEY`, `OPENCODE_GO_API_KEY`).
  Codigo completo 2026-09-19: `fmt` + `check` + `clippy -D warnings` +
  `test --lib` 14/14 (3 nuevos `handlers::ia::pruebas`).
   Pendiente humo HTTP: el dev-server vivo (otra sesion) usa binario anterior;
   reiniciarlo y probar `estado` + `completar` antes de 199A-2.
   Humo CERRADO 2026-10-01 (011A-4, ver `Agente/completados/tareas-2026-10-01.md`):
   sin servidor vivo, se arranco el binario actual y `completar` devolvio
   `ok:true` via OpenCode Go (gloryapi sigue `sin-clave` por falta de key).
- 199A-2 (repo `INMOBILIARIA`): redaccion y copy via `completar` (activo
  manual + fallback); pestana `ia` en Config con estado, habilitar/elegir
  activo y probar por proveedor.
- Requiere del usuario: `GLORY_API_KEY` real en `MN-Inmobiliaria/.env`
  (sin ella GloryAPI queda `sin-clave` y todo va por OpenCode Go).

## Sesion admin de 10 años (199A-3, en curso 2026-09-19)

- El JWT ya duraba 365 dias con `JWT_SECRET` estable en `.env` (gitignored):
  la sesion solo caia por secreto rotado o expiracion real.
- 199A-3: `generate_token` pasa a 10 años. Tras arrancar el servidor nuevo,
  salir y entrar una vez para que el token traiga la expiracion larga
  (los tokens viejos conservan la suya).

## Editar inmueble 405: front mandaba PATCH, contrato es PUT (199A-4, 2026-09-19)

- `actualizarRemoto` enviaba `PATCH /api/admin/inmuebles/:id` pero el router
  solo acepta `PUT` (asi desde 159A-2 + utoipa `put`): 405 preexistente, no
  causado por los reinicios de 199A-1/199A-3.
- Fix en repo `INMOBILIARIA`: `PUT` en `actualizarRemoto` (manda el objeto
  entero = reemplazo, semantica PUT correcta). `fijarPublicado` sigue PATCH
  (su ruta si es `patch`). Verificado con PUT+JWT a id inexistente → 404.

## Fotos 404 al editar: sincronizarFotos borraba antes de re-descargar (199A-5, 2026-09-19)

- Ante CUALQUIER cambio de fotos (anadir, quitar, reordenar, elegir
  principal), `sincronizarFotos` borraba TODOS los originales (fila + archivo
  en disco via `delete_foto`) y luego intentaba re-descargar las conservadas
  del servidor para re-subirlas: 404 inevitable ("No se pudo leer la foto
  para subirla") y el inmueble quedaba sin fotos (perdida real; los datos
  texto si se guardaban porque el PUT va antes).
- Fix en repo `INMOBILIARIA` (commit `c0037dc`): leer los bytes de TODAS las
  deseadas ANTES de borrar; si alguna falla no se ha borrado nada. Servidor
  verificado sano (fotos existentes responden 200, 92 archivos en
  `uploads/`); causa 100 % front, no backend.
- Pendiente usuario: revisar la propiedad editada y re-subirle las fotos
  perdidas; recargar `:5200` con `Ctrl+F5`.

## Principal + mejoradas en el frente público (199A-6, 2026-09-19)

- Dos causas front (repo `INMOBILIARIA`, commit `7e245b2`; backend intacto,
  la API pública ya devuelve originales + mejoradas emparejadas por `orden`):
  1. La galería del detalle público (`modal-detalle-publico.tsx`) mostraba
     `i.fotos` = solo originales; las mejoradas solo aparecían en la portada
     de la tarjeta. Ahora usa `fotosVisiblesDe` (mejorada por posición si
     existe, si no el original; `portadaDe` = primera visible).
  2. Al reordenar/elegir principal, `sincronizarFotos` re-creaba los
     originales con `orden` nuevo pero dejaba las mejoradas con el viejo: el
     pareo por `orden` quedaba roto y la portada mostraba la mejora de otra
     foto. Ahora las mejoradas se descargan antes de borrar y se re-suben
     siguiendo a su original (misma garantía sin-pérdida de 199A-5); la de un
     original eliminado se descarta.
- Pendiente usuario: `Ctrl+F5` en `:5200` y comprobar Townhouse/Orquídea
  (editadas hoy 17:07): si alguna portada sigue mostrando la mejora de otra
  foto (pareo roto anterior, irreparable por contenido), avisar cuál para
  limpiar sus mejoradas y regenerarlas pareadas.

## Pareo roto en Orquídea: reparado por contenido (199A-7, 2026-09-19)

- La usuaria elegía principal en admin (se guardaba bien) pero en público
  seguía viendo otra foto: el pareo `orden` original↔mejorada de Orquídea
  estaba roto por un reorden anterior (las mejoradas conservaban el `orden`
  viejo). Demostrado por hash perceptual (dHash, distancias 0-8 = misma
  escena): mejorada 0→original 8, 1→9, 2→10, 3→0, 4→11, 5→12. La portada
  (mejorada 0) mostraba la mejora del original 8, no de la principal.
- Townhouse comprobada igual: sus 6 parejas correctas, no se tocó.
- Reparación solo-datos vía API (sin código ni reinicio): subidas primero
  las 6 mejoradas con `orden` correcto (`origen=mejorada`, mismos bytes),
  luego borradas las 6 viejas por id, verificado por contenido (las 6
  parejas distancia ≤8). Total Orquídea: 15 originales + 6 mejoradas = 21.
- El fix 199A-6 impide futuros despareos al reordenar desde el front.
- Seguimiento propuesto (no urgente): endpoint `PATCH /api/admin/fotos/:id`
  con `{orden}` para reordenar sin borrar+re-subir (menos churn y sin
  cambiar urls/ids que cachea el store local de mejora).

## Siguiente bloque

(279A-1 cerrada 2026-09-27 ~17:10: ver `Agente/completados/tareas-2026-09-27.md`.)

## 279A-2 — Agente MN pulido: 2 modos, delegación, clientes, observabilidad (plan revisado 2026-09-27)
- Plan: `Agente/planes/plan-whatsapp-2026-09-27.md` (revisión mayor: ya no
  es solo WhatsApp; dos números × dos modos completo/inicial, personalidad,
  congelamiento, `clientes`, tokens, consola de dueña; pista núcleo
  glory-agent para historial real + usage).
- Fases: F0 pista núcleo → F1 tablas+clientes → F2 webhook 2 números →
  F3 delegación → F4 memoria+media → F5 consola dueña.
- Requiere de usuaria (al final): QR de ambos números (2 min por número)
  + storage de fotos. Número A (completo, pruebas): 0412 0825234
  (`584120825234`). Número B real (inicial, agente): 0424 9208855
  (`584249208855`, número vivo: no usar hasta el final); para las pruebas
  se usa el +1 (814) 957-5416 (`18149575416`) como B temporal
  (`wa_numero_b`/`whatsapp_admin` ya configurados en `agent_config`).
  F0/F1/F3/F2 simulado/F5/restos sin QR ya verificados; el gateway real
  existe (ver 289A-1 abajo).
- F1 verificado local 2026-09-27: migración `20260927000014_agente_clientes`
  (`clientes` UNIQUE por teléfono, `canal_sesiones`, `atencion_sesiones`,
  `uso_mensajes` + trigger estima len/4); `registrar_contacto` y
  `POST .../contacto` crean `clientes` sin duplicar (`0412 0825234` →
  `584120825234`); `fmt`+`check`+clippy 0, `cargo test` 20 passed.
- F3 verificado local 2026-09-27: `consultar_agente` (consultando +
  `ai_enabled=false` + ciclo `waiting`) y `escalar_a_humano` (delegada +
  triple freno) con `destino` explícito + `resumen`; worker arma ficha
  comercial (`ficha_para_aviso`) con fallback a texto mínimo; staff
  `POST .../devolver` (nota `staff` + `answered` + `ai_enabled=true` +
  `activa`) y bandeja con `estado_atencion/modo_atencion`;
  `fmt`+`check`+clippy 0, `cargo test` 22 passed, outbox/atención limpios.
- F2+F5 backend verificados local 2026-09-27: `POST /agent/whatsapp/webhook`
  simulado con reparto por `numero_destino` (A→`wa_a`/`completo`,
  B→`wa_b`/`inicial`, `numeros_configurados` desde `agent_config` con
  fallback env), hilo por cliente×canal reutilizado, foto como
  `[foto] {url}`, passthrough `media_url` en outbox; consola dueña
  `GET|POST /agent/clientes`, `PATCH /agent/clientes/:id`,
  `GET /agent/clientes/:id/sesiones`, `POST /agent/enviar`,
  `GET /agent/uso`, `GET /agent/auditoria`; `fmt`+`check`+clippy 0,
  `cargo test` 24 passed.
- F0 verificado local 2026-09-27: núcleo `glory-agent@b235771`
  (historial 30 turnos + `usage` exacto + ventana configurable;
  `sentinel analyze` 0/0/0/0, `db_roundtrip -- --ignored` PASS);
  `Cargo.toml` bump `f2f19e7`→`b235771` + migración
   `20260927000015_agent_usage` (columnas espejo + trigger copia exacto a
   `uso_mensajes`; prueba viva `2|120|35` con `ROLLBACK`); `fmt`+`check`+
   clippy 0, `cargo test` 24 passed. Gotcha: `_sqlx_migrations` traía
   checksum viejo de `...13` (aplicada de un borrador); se sincronizó
   (sha384 del archivo = BD) antes de migrar.
- F5-UI + F4-parcial verificados local 2026-09-28: pestañas
  `Bandeja|Clientes|Uso y auditoría|Config` en `vista-mensajes.tsx`
  (montadas siempre + `hidden` para no perder selección); `clientes-duena`
  (buscar, alta con normalización, ficha, hilos, «Dime y lo envío» con pie
  de foto opcional), `uso-auditoria` (tokens día×remitente + tomas
  humanas); alta/ficha/envío/uso probados en navegador contra `:3000`
  (outbox `manual`/`pending`, `media_url` en payload, trigger
  `tokens_est=10`) y datos de prueba borrados; `tsc` 0 + `self-check`
  (check+clippy+test+front) verde. F4-audio fuera: sin transcripción
  (ver plan).
- Resto sin QR verificado local 2026-09-28: secreto webhook
  `WA_WEBHOOK_SECRETO` (cabecera `X-Gateway-Secret`, 401 sin/falla, 200 con;
  sin configurar acepta todo para el simulado) + storage de fotos decidido
  (disco local `UPLOAD_DIR/whatsapp/<tel>/` con `guardar_archivo`,
  servidas por `GET /uploads/whatsapp/:telefono/:archivo` con
  `clave_whatsapp_valida`, fallback a URL remota) + tope diario LLM
  (`ia_tope_tokens_dia` default 2M, watcher 5 min, aviso `motivo: tope`
  una vez/día, solo alerta) + fix worker (`texto` explícito del payload
  manda; antes habría entregado texto de ficha al cliente en vivo);
  verificado E2E contra `:3000` (401/200, `[foto] /uploads/whatsapp/...`
  200307 bytes servidos `image/jpeg`, traversal 404, aviso tope `pending`
  con texto intacto) y datos de prueba borrados; `check`+clippy 0,
  `cargo test` 27 passed.
- 289A-1 gateway real + vinculación desde admin (código listo 2026-09-28,
  sin vincular físico todavía): `gateway/` (Baileys 6.7.24, sesiones
  `wa_a`=A + `wa_b`=B temporal, inbound→webhook con `numero_destino`,
  fotos→`media/` temporal, `POST /send` con `via`, QR en terminal +
  `gateway/qr-wa_a.png`/`qr-wa_b.png`, arranque
  `gateway/iniciar-pruebas.ps1`); backend `via` en todo outbox `whatsapp`
  (`canal_de`, default `wa_a`) + proxy admin
  `GET /api/admin/agent/whatsapp/sesiones` y
  `GET .../sesiones/:canal/qr` (401 sin JWT, 400 canal malo, 500 gateway
  caído; E2E contra `:3000` OK) + pestaña «WhatsApp» en Mensajes con QR y
  auto-recarga 20 s; `clippy` 0, `cargo test` 27 passed. Front sin `tsc`
  (falta `vite` en `node_modules`): el panel nuevo solo tuvo revisión.
  Queda: usuaria ejecuta `iniciar-pruebas.ps1`, vincula A + B temporal,
  batería real F2/F3, limpieza y deploy.
- 289A-1 alcance 2026-09-28 (decisión usuaria tras bucle A↔B real): **solo A
  es IA; B mudo**. El ping-pong se confirmó en BD (la respuesta `ai` de cada
  lado entraba como `client` del otro). `gateway/src/config.mjs:respondeIA`
  (`SESSION_A_RESPONDE_IA=1`, `SESSION_B_RESPONDE_IA=0`) +
  `sesion.mjs` descarta inbound de vía muda antes de bajar media (outbound
  intacto); anti-eco `eco.mjs` queda como defensa. BD de pruebas a cero.
  Verificado 2026-09-28: B→A real responde la IA (client|1→ai|2, outbox
  `sent via wa_a`) y la vía B queda muda (`[B] vía muda` en log, sin bucle).
- 289A-2 panel + tono IA (verificado en vivo 2026-09-28, commit `998d4036`):
  Responder staff persiste `staff|3` con `insert_message_seq` (adiós 500 por
  clave duplicada), emite WS, encola `whatsapp motivo: manual` con
  `destino+via` del hilo y el worker lo entrega (`sent`); `devolver_a_ia`
  igual; tomar/soltar (`PATCH aiEnabled` true/false) OK. Panel: fecha corta
  en cada mensaje + teléfono del cliente en bandeja y cabecera del hilo.
  Prompt IA reescrito: identidad `Asistente de IA de MN Inmobiliaria`,
  saludo según hora de Venezuela, máx 3 opciones con palabras propias (sin
  copiar títulos ni listas largas), cierre ofreciendo fotos/info.
  `clippy` 0, `cargo test` 27 passed, `tsc` limpio.
  Lección 2026-09-28: el gateway debe correr sin interrupción; matar+relanzar
  con la misma identidad (y peor con solape) bifurca el cifrado y el teléfono
  muestra "esperando mensaje". `mn-arrancar-gateway.ps1` ya rehúsa lanzar un
  segundo (guardia por puerto 3102); reinicio limpio = verificar PID muerto +
  puerto libre + 5 s antes de lanzar.
- 289A-4 logs de tools y cierre de turno (commit `138b71f5`): `ejecutar()` en
  `chat_tools.rs` loguea tool+sesión+ms+chars/error y el webhook el cierre del
  turno (chars+via); lo pedido por usuaria ("todo debe tener logs").
- 289A-5 techo IA 4096 (causa raíz del "no respondió más", verificado crudo y
  E2E 2026-09-28, commits `0050d08` en `glory-agent` + `04491ec3` aquí):
  muse-spark razona ~600-800 tokens y con `max_output_tokens=800` el turno
  post-tool moría `incomplete` sin texto ni calls (800→incomplete+0 items,
  2000→completed+3 `detalle_inmueble`); `ChatApiOptions::standard()` ahora
  4096 + WARN `AI incompleta (motivo)` en `call_provider`. E2E sesión
  `3fd474ee`: buscar(826)+3×detalle→`ai|4` listado 858 chars con tono 289A-2
  →outbox `sent motivo:ia via wa_a`. `Cargo.toml`+`Dockerfile.rust`
  (`GLORY_AGENT_REF`) en `0050d08b47`; clippy 0, tests 27+27+5+1.
  Sesión `739bb63e` quedó `consultando` con el preámbulo parcial: reintentar
  con `devolver_a_ia` tras este fix.
- 289A-6 techo IA 8192 (pedido usuaria "súbelo bastante", commit `e4ad5d7` en
  `glory-agent` + `24281773` aquí, push 2026-09-28): `standard()` 4096→8192 +
  test; `Cargo.toml`+`Dockerfile.rust` en `e4ad5d78`; backend reiniciado
  (health `ok`) con ese binario.
  Gotcha: el backend NO arranca si el cwd no es la raíz del repo (`.env` con
  `DATABASE_URL` vive ahí); lanzar siempre con cwd del repo.
- 289A-7 hilo nuevos-arriba + lado por remitente (commit `dd8fe64e`, `tsc`
  limpio, push 2026-09-28): se revierte el ASC de 289A-3 — lo acordado era
  nuevos arriba, viejos abajo (`sort` DESC por `sequence_num`); `claseLado()`
  en `hilo-mensajes.tsx`: Visitante derecha, IA izquierda, staff derecha
  destacado, sistema centrado; el auto-scroll acompaña al inicio.
- 289A-8 lados finales (commit `cd0fce3a`, `tsc` limpio, push 2026-09-28):
  orden nuevos-arriba confirmado por usuaria (se mantiene); lados
  invertidos: Visitante izquierda, IA + staff a la derecha.
- 289A-9 orden normal de lectura (commit `a944795c`, `tsc` limpio, push
  2026-09-28): ASC por `sequence_num` (el primero arriba, el último abajo)
  + sigue-fondo; los lados de 289A-8 no cambian. Lección: "antiguo/nuevo"
  es ambiguo — pedir siempre anclas concretas (ej. "el de las 06:13
  primero").
- 289A-10 texto plano para `WhatsApp` (commit `0b7e6241`, clippy 0, tests 27,
  push 2026-09-28): el prompt ordena sin `**`/encabezados/tablas, listas
  1. 2. 3., emojis sí; verificado en vivo (cero `**`, emojis 🏡😊).
- 299A-1 batería de escenarios IA (plan
  `Agente/planes/plan-bateria-escenarios-ia-2026-09-29.md`, BD a cero): Fase 0
  auditoría (solicitudes/fotos/notes, descarte audio, media_url saliente),
  Fase 1 diez escenarios que ya deberían funcionar, Fase 2 builds E11–E16
  (visión, audio, envío fotos, captación, citas, bug consultando→activa),
  Fase 3 batería completa con criterio de aceptación.
- 299A-2 correctivo Fase3-v2 (verificado 2026-09-29, H1-H6 cerrados F1-F10):
  `registrar_sin_vincular` (ficha sin re-clavear), filtros exactos
  `habitaciones`/`zona` en BD, `sencilla()` (migración `...18`),
  regla no-afirmar-sin-tool, sin oficina física (delegar a asesor), acuse
  6 s con triple guarda; batería paralela `scripts/bateria-ia.ps1`; tests
  41/41 con BD real + prevención H9. Detalle en
   `Agente/completados/fase3-bateria-resultados-2026-09-29.md` (§Batería v2).
- 299A-3 demo fotos/audio en el hilo + render media (verificado 2026-09-29):
  webhook archiva foto y voz (`guardar_audio` ogg/mp3/m4a con magia, mismo tope
  10 MiB), sirve audio con su mime, front renderiza `<img>`/`— se ve:`/
  `<audio controls>`/link adjunto + proxy `/uploads` en vite dev; demo viva via
  `wa_b` (E11 foto archivada y descrita, E12 `[audio]` archivado 200
  `audio/mpeg`, E13 3 `ia_foto` 200 `image/jpeg`), panel verificado por DOM,
  BD a cero. Detalle en `Agente/completados/tareas-2026-09-29.md` (## 299A-3).
  Gotchas: start-script apuntaba a target viejo, mp3 8.9 MB supera timeout 20 s
  (usar audios pequenos), la IA escala tras 2 notas de voz.
- 299A-4 espejo outbound en el hilo (verificado 2026-09-30): `Herramientas`
  lleva `hub` opcional (`with_hub`, sin romper `new` ni los tests sin hub);
  tras cada enqueue al visitante se persiste el mismo contenido como `ai`
  (`tarjeta_texto` tal cual; foto como `[foto] url — se ve: pie`, que ya
  renderiza `MessageMedia`); `aviso_humano` no se espeja (destino staff, no
  visitante). Detalle en `Agente/completados/tareas-2026-09-29.md` (## 299A-4).

## 279A-3 — /ask cuestionario de ficha (F1 verificado local 2026-09-27; pendiente: respuestas usuaria)
- Plan: `Agente/planes/plan-ask-2026-09-27.md`.
- Página privada con login existente; preguntas por tipo (piso solo
  apartamento), condicionales, dinámicas IA opcional; `precio_minimo`
  privado con frontera en API; tabla % + semáforo en admin; alimenta
  chat web + WhatsApp.
- Dudas resueltas 2026-09-27 (commit `40244fb5`): mínimo insinuable sin
  cifras, piso = apartamento + townhouse, dinámicas desde F1, aviso sin
  bloquear, `/ask` con sesión admin.
- F1 implementado y verificado local (commit `57f8410f`): migración
  `extras` + `precio_minimo`, `GET/PUT /api/admin/.../ficha`, frontera
  pública (`extras={}` sin `precio_minimo`, 422 a array), login `/ask`
  centrado, lista + cuestionario (respuesta → 14% en DB y UI), `tsc` 0,
  `cargo test --lib` 18 passed. Dato de prueba limpiado.
- F2 mejora /ask verificada local (commit `e119b9c4`, `tsc` 0): sin
  `urbanizacion_zona` en `extras`; paso inteligente ubicación+residencia
  (rellena columnas vía PUT, confirma lo existente); numéricos
  condicionales por tipo (m² salvo terreno, parcela en casa/terreno,
  puestos salvo terreno, solo si faltan); orden aleatorio por sesión;
  botón Anterior (deshacer/corrige); No sé en Sí/No; numéricos con unidad,
  stepper y error visible; página centrada angosta con foto de portada +
  descripción; sección Ficha /ask editable en el modal admin (pisa
  `extras`/`precio_minimo`; ubicación/medidas ya se editaban). Flujo
  probado en navegador sin ensuciar datos (ubicación restaurada,
  `extras={}` intacto, guardado admin sin cambios = no-op).
- 279A-4 sin lista /ask verificado local (commit `ecb71411`, `tsc` 0): al
  entrar elige sola una propiedad con algo que preguntar (candidatas en
  aleatorio, ficha una a una, primera con pendientes); sin lista visible,
  solo preguntas; botón Otra propiedad (excluye la actual) + Otra pregunta
  aleatoria al terminar; panel ¡Todo al día! con Revisar de nuevo;
  `estado-ask.ts` separa estado/updaters (hook 117 líneas). Navegador:
  entrada directa a pregunta, Otra propiedad salta de Calas Suites a
  Arivana, recarga cae en Riberas del Caroní, solo lecturas.
- 279A-5 verificado local (`tsc` 0): numéricos nombran el tipo («¿Cuántos
  m² tiene el apartamento?»; terreno: «¿Cuántos m² tiene el terreno?»);
  precio arriba («35.000 € · venta», «Precio a consultar» si es 0);
  lenguaje neutral en UI/comentarios/plan (no es para el dueño): mínimo
  «¿Cuál es el precio mínimo aceptado? (privado, no se publica)».
   Navegador: precio + m² + mínimo confirmados, solo saltos sin guardar.
- 279A-6 verificado local (`tsc` 0): huecos internet («¿Posee internet?»)
  y agua («¿Llega el agua?», `si_no`, públicos) en `SERVICIOS` para los 5
  tipos; dólar de frente (`$ 90.000`, `Precio ($)`, IA `$`, mínimo `$`);
  escalable: receta en `ficha-ask.ts` (dato suelto = 1 entrada, sin backend;
  columna = migración + `COLUMNAS_POR_TIPO`), `destino: 'precioMinimo'` en
  vez de `clave === …` regado, `COLUMNAS_POR_TIPO: Record` (un tipo nuevo
  falla en compilación), `assertNunca` exhaustivo en `pasoRespondido`,
  `responder` y render. Navegador: 11 pendientes, internet + agua salen en
   el flujo, `$` arriba, solo saltos sin guardar.
- 279A-7 verificado local (`tsc` 0): agua con A veces («¿Llega el agua?» =
  `opciones` Sí/No/A veces/No lo sé); `No lo sé` en TODAS (ficha, sí/no,
  opciones con `conNoSe`, mínimo con `claveNoSe`, ubicación, numéricas con
  marcas `*_nose`) que persiste en `extras` (no se vuelve a preguntar) y el
  backend la borra al llegar dato real; `No lo sé` siempre sin fondo
  (seleccionado = semibold + subrayado); amoblado con «En trato» en
  apartamento/townhouse/casa. Navegador: mínimo→9%, puestos→10%,
  agua A veces→30%, resaltados correctos, datos de prueba limpiados
   (`extras={}` en ambas).
- 279A-8 verificado local (`cargo check` + clippy 0 en `inmobiliaria` con la
  BD de rama; en `main` el check no compila por `agent_outbox` ausente en
  `glory_backend`, preexistente — RESUELTO 2026-10-01 en 011A-3 (BD reconstruida, check verde en `main`)): la IA ve todo lo rellenable —
  `detalle_inmueble` devuelve `extras` tal cual (incluidos `no_se`/`a_veces`)
  + `margen_negociable` calculado en SQL; la cifra del mínimo jamás sale
  (frontera 279A-3); prompt + descripción de la tool instruyen insinuar sin
  cifras. SQL probado con psql en transacción con ROLLBACK (margen `t`/`f`,
  cifra filtrada, cero cambios).
- Pendiente: cablear `extras` públicos (`privada: false`) a la ficha
  visible y a la IA — hoy nadie los lee fuera del panel (la pública ni los
  pide, el backend los pela con `'{}'`).
- Requiere de usuaria: responder el cuestionario en `/ask`.

## 011A-5 — F5 consumidor delgado MN (cerrada 2026-10-01: ver `Agente/completados/tareas-2026-10-01.md`)

## 03AA-3 — Asistente Marketplace (REPLANTEADO 2026-10-03)
- Plan: `Agente/planes/plan-asistente-marketplace-separacion-2026-10-03.md`
  (reto aplicado: E0 inventario+corpus, strip con flag, doctrina Meta común
  con 03AA-5, caché con spec, test anti-fuga del mínimo, DoD con números;
  2º reto: freeze+timebox E0, allowlist extras, modelo pineado, matriz fuga,
  caché por terna+TTL, host dueño núcleo, Regenerar sin tope por
  decisión usuaria 2026-10-05; 3er reto: P0 seguridad/PII/rollback/
  concurrentes + 8 contradicciones; 4º reto: E0 checklist ejecutable,
  frase canónica de ficha, schema publicado, rollback por repo, números
  con origen, orden E1→E2→M3→M2→E3→M4→M1; 5º reto: avisoId nullable,
  freeze cifrado, token endpoint con spec, HMAC audit, restore con dueño,
  strip allowlist, CLI 8h, scopes separados, 429 exime a ella, store en
  memoria, doctrina referenciada a 03AA-5).
- E0 ejecutado 2026-10-05 (día 1 de 3, solo lectura): freeze cifrado en
  `%MP_PRIVADO%` + inventario + firma-v1 + anonimizar/verificador + fixture
  sintética + corpus (3 hilos copiados + 1 de visita dictado por ti, ANON_OK).
  E1 hecho: repo local `../plugins-opencode` (rama `main`, sin remoto) con
  núcleo agnóstico (excerpt 5 campos, firma-v1, lector, intención, schema M3)
  31/31 tests verdes. E2 hecho: bridge simétrico + flag `MP_NUCLEO=off`
  (fail-safe) + paridad 7/7 sobre corpus real (el plan pedía 10/10; el corpus
  trae 7 mensajes literales: hilo-04 es dictado no literal y se excluyó).
  M3 hecho 2026-10-06 (commit 81244ec1): migración `mp_tokens_emitidos` +
  `mp_uso_minuto` + `mp_auditoria`, strip allowlist 6 campos, JWT mp 15min con
  `jti`+revocación, cubo 429 (5/min token, 30/min borrador) con `Retry-After`,
  matriz negativa v1, audit con HMAC server-side; gate: fmt+check+clippy limpios,
  82/82 tests (6 nuevos), medida mock p50=0.01ms p95=0.03ms, prueba viva
  (login 200, token 201, borrador 200 fuente=ia, audit 201 HMAC sin PII, 429 OK;
  la viva cazó `RETURNING n` INT4→i64, fixeado con `::BIGINT`).
   Activo: M4 (caché `mp_respuestas_cache` + Regenerar + purga). E3 hecho
2026-10-06: `MpClaims.mid` opcional hex64 + `POST /token/cli` (exp 8h,
`maquina_hash` 64hex, cubo propio 5/min) + binding `X-MP-Maquina` en `MpAuth`
(401 máquina ajena/sin header, 422 hash malo; panel sin `mid` intacto, sin
header 200) + `scripts/mp-cli.mjs` Node sin Electron (fuga 0: solo borrador;
audit `emision`) + test expiración; gate fmt+clippy limpios, 87/87 tests;
 viva (CLI 480min fuente=reserva sin inventar precio; panel 15min 200).
 M4 hecho 2026-10-06: migración `mp_respuestas_cache`
 (firma,precio_hash,catalog_hash,respuesta,valida_hasta +90d,usos,corregida)
 + `hash_ficha()` byte-a-byte tras fetch + `claves_cache()` único cálculo +
 borrador hit (`fuente=cache`) / miss singleflight (una sola generación ante
 10 concurrentes) + `guardar_cache` solo `fuente=ia` + Regenerar
 (`DELETE`+bypass, sin tope por decisión 2026-10-05) + `corregir`
 (`corregida=TRUE`, cubo propio 30/min, matriz también al texto) + purga al
 arrancar siempre + pg_cron diaria solo si `DB_24H=true`; gate fmt+clippy
 limpios, 97/97 tests (10 nuevos; la suite cazó `usos` INT4→i64, fixeado con
 `::BIGINT` igual que M3); viva `M4-VIVA-OK` (miss ia → hit cache mismo texto
 → regenerar fresco → corregir → borrador corregida=true con el texto
 corregido → contacto 422).
Siguiente: C1 (NUEVO 2026-10-07 [07AA-5]: cableado en opencode-propio —
  faltaba, es lo más importante; C1a se prepara sin tocar la app viva,
  C1b en ventana con ella y reinicio por ella) y luego M1 (BLOQUEADO:
  exige C1+E2+M2+M3 verdes en viva + firma de ella + ventana
  congelación escrita por ella; no hay otro frente; sin su firma M1
  prohibido).
  Falta de tu parte para cerrar E0: remoto del repo + fecha de viva 30min.
- M2 hecho 2026-10-06: panel en `../plugins-opencode/src/panel/`
  (`selectores.json` pin v1 por rol/nombre + `registerMode()` asistente>radar +
  `BorradorStore` `mp_borrador:{threadId}` TTL 24h + `PanelController` con
  degradado que bloquea copiar/regenerar + `VistaDom` semántica) 51/51 tests;
  backend `GET /api/admin/marketplace/uso?dias=` (agregado día+evento sin PII,
  solo JWT admin) + test BD viva; gate fmt+clippy limpios, 83/83 tests, viva
  (uso 200, audit copiar 201, copiar 2→3, thread_id jamás en respuesta).
  Visual 1280/390 + teclado/contraste + SPA: checklist manual en README del
  plugin (sin harness de navegador en el repo).
- Deuda aparte (no del cambio, no se mezcla): `handlers::sombra` tiene carrera
  preexistente — sus tests comparten `agent_config` sin sincronizar
  (`adapter_defaults_sin_config` borra `adapter_responde_ia_global` mientras
  otros la escriben; falla 1/2 corridas full, en aislamiento 9/9 verde).

## 07AA-6 — Flujo lab opencode-propio-dev (plan nuevo 2026-10-07)

- Plan: `Agente/planes/plan-flujo-lab-opencode-propio-2026-10-07.md`.
- Su app corre en dev con recarga (guardar rompe); su checkout con cambios
  sin guardar. Lab hermano ya creado + regla `LEEME-LAB.md`.
- Fases: F1 skill `lab-opencode` + F2 promoción con diff-first y manifest +
  F3 piloto C1 de 03AA-3 + F4 prohibiciones. Sin git en el lab (decidido).
- Estado: F1+F2+F3a+F3b hechas en lab (skill + checklist + cableado con
  flag verificado 38/38 tests + typecheck). C1b espera ventana con ella
  (ella reinicia y prueba viva).

## 07AA-8 — Formato de borrador con ficha + contacto y distinguir mis mensajes (cerrada 2026-10-07: ver `Agente/completados/tareas-2026-10-07.md`)

- Respuestas de ella: contacto fijo **0424 9208855** + `https://wa.me/584249208855`;
  etiquetar siempre (ella: `Dueña:`, cliente: `Cliente:`).
- Hecho: prompt orden exacto 4 partes + `asegurar_contacto()` (garantía
  aunque el modelo omita; también en fallback/reserva) + matriz exime esos
  dos exactos + `mpDialogo()` en float V4 (izq=Cliente, der=Dueña).
- Humo: `fuente: ia` con las 4 partes (ficha + dodge crédito + contacto + wa.me).
- Pendiente ventana con ella: promocionar `marketplace-float.ts` V4 del lab a
  su app viva (regla `lab-opencode`: diff-first + manifest, ella reinicia).

## 07AA-7 — Panel admin Marketplace por chat (cerrada 2026-10-07: ver `Agente/completados/tareas-2026-10-07.md`)

- Plan: `Agente/planes/completados/plan-panel-marketplace-chat-2026-10-07.md`.
- Pedido de ella tras el piloto C1-lab: ver en el admin lo generado por
  chat (conversación + borrador + usos + vigencia). Decisión de ella:
  guardar chat + borrador (solo admin, retención 90d).
- Fases hechas: F1 migración `20261007000031` → F2 guardar en
  `guardar`/`reemplazar` (+ tests de hilo/foto) → F3 `GET chats` +
  `GET chats/:thread` → F4 pestaña Marketplace en Mensajes → F5 gate + humo.

## 03AA-4 — WhatsApp un solo número + triage + config + escenarios (cerrada 2026-10-07: ver `Agente/completados/tareas-2026-10-07.md`)
- Plan: `Agente/planes/completados/plan-whatsapp-numero-unico-2026-10-03.md`.
- Un asistente general en el 0412 0825234 (el mismo de la web); se jubila el
  modo dual completo/inicial y el `wa_b` temporal; multi-número por config.
- Triage con regla de oro (por defecto se atiende; cada `no` con motivo),
  matriz público vs autorizado, config todo-controlable, harness de
  escenarios multi-paso sin WhatsApp (stub + vivo).
- 06AA-1 F1 triage + tests (hecha 2026-10-06): capa `Triage` pura
  (`src/services/triage.rs`: `decidir`/`decidir_para` fail-open, enums
  `Procedencia`/`MarcaTransporte`/`Duplicidad`/`Contenido`, `Motivo` con
  código, trato cliente/neutral que nunca silencia, `RegistroDuplicados`
  TTL 180 s tope 500) + wiring en webhook (`decision` en el 2xx, trato
   solo logueado) + stub vivo (`no:eco`, `no:duplicado`, eco sin turno y
   atiende con turno verificados en BD).
  - 06AA-2 F2 política + tono (hecha 2026-10-07): capa `Politica`
    (`src/services/politica.rs`: `Rol` por allowlist
    `agent_config.whatsapp_autorizados` fail-closed a `Publico`, `resolver`
    pura, `leer_autorizados` 1 consulta, `prefijo_contexto`/`texto_para_turno`
    solo al modelo, `REGLA_FRONTERA` al prompt global) + wiring en webhook
    (`rol`/`trato` en el 2xx, tono al turno, frontera al prompt) + stub vivo
    (`atiende:cliente`/`publico`, `atiende:neutral`/`publico`,
    `atiende:cliente`/`autorizado`, tono neutral breve+califica e intacto en
    BD).
  - 06AA-3 F3 partir capas (hecha 2026-10-07): `Transporte`
    (`src/services/transporte.rs`: `EntradaWhatsapp`, `secreto_valido`,
    `numeros_configurados`, `reparto`), `Sesion`
    (`src/services/sesion.rs`: `RepartoWhatsapp`, `repartir_y_vincular`,
    media-ingesta) y `Turno` (`src/services/turno.rs`: `TurnoFondo`,
    acuse 6s, E-fluido) fuera de `handlers/whatsapp.rs` (queda orquesta +
    triage + ruta + pruebas); `sombra.rs` usa `services/`; sin cambio de
    conducta (stub vivo: `atiende:cliente`/`publico`,
    `atiende:neutral`/`publico` con breve+califica,
    `atiende:cliente`/`autorizado`, reutiliza hilo). Próximo: F4 config
    admin.
  - 07AA-1 F4 config admin (hecha 2026-10-06): expone en
    `PUT /api/admin/agent/config` + panel `ventana_retraso_min`,
    `whatsapp_autorizados`, `ia_tope_tokens_dia`, `corte_whatsapp` y tono
    (`whatsapp_acuse/fallback/aviso_asesor_texto`, que el turno lee con
    fallback a constantes); `wa_numero_a/b` ya editables en backend salen
    en el front; fix `config_triage` a columnas `key/value` (estaba
    fail-open silencioso); `GLORY_ALERT_GATEWAY_URL` queda env (infra, no
    UI); `prompt_extra` sigue guardada sin lector (deuda). Próximo: F5
    retirar `wa_b`.
  - 07AA-2 F5 retirar `wa_b` (hecha 2026-10-07): solo A reparte
    (`reparto` sin B + `destino_jubilado`); destino B → 2xx
    `no:canal-jubilado` sin persistir ni turno; destino desconocido sigue
    400; envío manual staff por `wa_b` queda (explícito + auditado,
    gateway B mudo lo frena); gateway intacto. Próximo: F6 escenarios.
  - 07AA-3 F6 harness 8 escenarios (hecha 2026-10-07): receptor
    stubbed (HTTP directo), IA real sin afirmar texto — solo contrato.
    `C:\tmp\probar-f6.mjs` `F6-OK 13/13`: A público, reutiliza hilo
    (secuencia crece), B jubilado, B from_me jubila, desconocido 400,
    eco, duplicado, autorizado temporal (setup/restore allowlist por
    psql; login harness da 401 — ver lecciones). Limpieza 0 filas,
    allowlist restaurada, 3110 cerrado. Próximo: F7 vivo + cierre.
  - 07AA-4 F7 pasada vivo + cierre (hecha 2026-10-07): gate completo
    (`fmt`, clippy 0, `cargo test --lib` 116/116, `tsc` 0) + harness
    `C:\tmp\probar-f6.mjs` `F6-OK 13/13` en pasada viva con sufijo
    nuevo; limpieza 0 filas, allowlist restaurada, 3110 cerrado.
    03AA-4 cerrada, plan archivado.

## 03AA-5 — Detector de captación Marketplace (REPLANTEADO 2026-10-03, bloqueado)
- Plan: `Agente/planes/plan-detector-captacion-2026-10-03.md` (incorpora reto
  hostil: riesgo mínimo —no 0—, C0 corpus+dataset+DDL primero, adenda con
  03AA-3, métricas por fase).
- Bloqueado hasta C0 (corpus 5+2+1, dataset 30–50, DDL) y E1 de 03AA-3
  (repo + núcleo lector; a su vez bloqueado hasta E0). Nada de código antes.

## 05AA-1 — MN adopta `Resolver` del núcleo (cerrada 2026-10-05: ver `Agente/completados/tareas-2026-10-05.md`)
