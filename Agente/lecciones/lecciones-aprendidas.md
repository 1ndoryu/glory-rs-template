# Lecciones aprendidas

## 2026-10-08 - Vista local-first + renumerado servidor = sombra stale (08AA-4)
- El visor prefiere la IndexedDB local (`mejoradaDeLista ?? mejoradasServidor`):
  tras reparar el pareo en el servidor (renumerar `orden`), el navegador siguió
  mostrando pares cruzados + dupes borradas porque sus entradas locales tenían
  URLs viejas por `orden`. Regla: toda reparación de `orden` en servidor exige
  convergencia local→servidor en `importarMejoradasServidor` (rellenar, corregir
  URL distinta, limpiar huérfana; dataURL en curso no se pisan).
- Verificar galerías en el MISMO perfil de navegador con estado real (otra
  pestaña limpia no reproduce el bug). Método calibrado sin screenshot:
  `document.images` + `getBoundingClientRect` distingue grande (630×477) de
  tira (95×95); `indexedDB.open('inmobiliaria')` → tabla `fotos-mejora`
  confirma el pareo efectivo.
- sqlx 0.8: `&mut Transaction` NO implementa `Executor`; usar `tx.as_mut()`
  (`&mut PgConnection`). `&mut *tx` falla E0277, `&mut **tx` falla E0614.
- Dos agentes en el mismo checkout con cambios sin commit se pisan: ante
  `git status` con ficheros ajenos a mitad de tarea, commitear conjunto con
  IDs combinados (`08AA-3+08AA-4`) y documentar la concurrencia en la
  completada; nunca `git add` por archivo completo si mezcla frentes.

## 2026-10-07 - Harness sin login: setup de config por psql, no JWT en scripts
- El login admin del harness dio 401 con el password default de M3 (el hash
  de esta BD ya no coincide): no cazar ni rotar passwords para un test. El
  setup de `agent_config` (allowlist, flags) se hace por `psql`
  (`INSERT ... ON CONFLICT`) con fila de respaldo (`f6_prev_*`, borrada al
  final) y el harness solo ejerce el contrato HTTP. La escritura de config
  por API ya la cubre su propia fase (F4); no duplicarla en cada harness.
- `F6_SUF` (env) coordina el sufijo entre el setup psql y el `.mjs`: sin
  env, cada lado inventa sufijo distinto y el escenario autorizado falla.
## 2026-09-30 - Groq 403 es red, no keys; Opencode Go no transcribe audio
- `403 {"error":{"message":"Forbidden"}}` de Groq hasta en `/models` y en el
  login web con keys válidas = IP/red bloqueada, no keys revocadas: con VPN
  todo pasó a 200 sin tocar keys. Ante un 403 global, probar otra red/VPN
  antes de rotar claves.
- Opencode Go (zen) no tiene vía de STT: ni `input_audio` en Responses ni
  `audio_url` en chat ni endpoint `/audio/transcriptions` ni modelos
  whisper/gemini en el catálogo. Si un proveedor "IA" pela el adjunto en
  silencio (200 con "no audio attached"), el probe debe mirar el contenido
  de la respuesta, no solo el HTTP.
- `main.rs` carga `.env` vía `dotenvy`, pero `cargo test --lib` no pasa por
  `main`: los tests vivos que necesiten secretos los reciben por entorno
  explícito + flag opt-in (`GROQ_LIVE_TEST=1`) y se omiten sin él.

## 2026-05-08 — Core editor-agnostico en extensiones
- Para extraer un core real no basta cambiar tipos: hay que eliminar imports indirectos de servicios del editor, como `configService`, `vscode.workspace` o registries que lean settings globales.
- Si una regla aun necesita workspace/watchers, aislarla como callback/adaptador permite avanzar el core sin romper el provider existente.
- Los reportes y scanners deben recibir datos y providers como parametros; escribir archivos, abrir documentos y escuchar watchers pertenece al adaptador, no al core.
- Las pruebas unitarias con mocks de VS Code no garantizan que una CLI arranque en Node puro; despues de compilar hay que ejecutar el JS real y buscar imports indirectos de `vscode`.

## 2026-05-10 — LSP y lint como cierre de arquitectura
- Un LSP fino debe importar core y adaptadores de transporte, no la CLI; si CLI y LSP comparten defaults, moverlos a `core/config.ts` evita drift silencioso.
- Smoke stdio real debe buscar `textDocument/publishDiagnostics` y un `ruleId` esperado; compilar no prueba que el entrypoint LSP no este ejecutando codigo CLI.
- Activar lint tarde puede revelar errores de regex antiguos. Corregir escapes redundantes es bajo riesgo; patrones Unicode compuestos intencionales necesitan excepcion local documentada.
- Si se agregan fixtures `.tsx` fuera de `src`, `tsconfig.json` debe declarar `include` explicito; si no, `tsc` intenta compilar fixtures fuera de `rootDir` y crashea antes de ejecutar tests reales.

## 2026-09-16 - E2E con servidor detached y terminal sin estado

- Los jobs de PowerShell no sobreviven entre llamadas de terminal sin
  estado: los builds largos van en sincrono con timeout amplio y los
  servidores detached con `Start-Process` + archivo de log + probe de
  readiness (`/api/health`), con cleanup (`Stop-Process` + borrar logs)
  en el mismo bloque.
- Tras una llamada ambigua que pudo lanzar un proceso, la siguiente
  accion es una comprobacion discriminante (`Get-Process`, puerto, log),
  no relanzar: evita duplicar servidores en el mismo puerto.
- Un E2E honesto en degradado (sin clave IA: `reply:null`, mensaje
  persistido, sin escalado espurio) vale mas que un E2E simulado; deja
  por escrito que comportamientos quedan pendientes de credenciales.

## 2026-09-25 - Lectura obsoleta y edit fail-closed como detector
- El `read` puede devolver contenido obsoleto (en 259A-1: 162 lineas con
  `Images`/`reintento` requerido vs 122 reales con `ImageIcon`/`useMemo`).
  El `edit` que no encuentra `oldString` es fail-closed y actua como
  detector: ante un fallo de match, no reintentar variantes a ciegas;
  confirmar con bytes crudos, `git status`/`git diff` y releer el archivo.
## 2026-09-28 - Consola dueña F5: automatización de navegador y gotchas locales
- Los inputs controlados de React no responden a `fill` sintético: hay que
  usar el setter nativo (`Object.getOwnPropertyDescriptor(...,'value').set`
  + evento `input` burbujeante). El error `Illegal invocation` casi siempre
  es selector nulo (pestaña equivocada), no sintaxis.
- `agent_outbox` no tiene columnas `destino`/`canal`: todo vive en `payload`
  (`destino`, `texto`, `media_url`, `motivo`) + `kind`/`status`. El vínculo
  cliente↔sesión vive en `canal_sesiones` (`telefono`, `canal`, `modo`);
  `agent_sessions` no tiene `cliente_id` (borrar por `canal_sesiones`
  arrastra por `ON DELETE CASCADE` mensajes, uso, atención y canal).
- El frontend usa `apiFetch` con base directa a `:3000`: el proxy
  `vite /api→:3122` no afecta a la app; no tocarlo para depurar la API.
- Cambiar de pestaña desmonta el hook y pierde selección/borrador: montar
  las pestañas siempre y ocultar con `hidden` lo evita (vigilar polling
  en segundo plano).
- `curl.exe -d '{...}'` en PowerShell deforma comillas: para JSON usar
  `Invoke-WebRequest` con hashtable → `ConvertTo-Json`.
- `cargo test` no puede reemplazar el exe mientras el servidor de pruebas
  corre desde ese mismo path: detener el proceso (`Stop-Process`) antes
  del self-check, o falla con `os error 5`.
- `_sqlx_migrations` puede traer checksum de un borrador (`...15`): se
  sincroniza con `UPDATE ... SET checksum=decode(sha384 archivo,'hex')`
  antes de migrar, no borrando la fila.
## 2026-09-28 - Webhook secreto, storage WA, tope LLM (resto sin QR)
- `uso_mensajes` no tiene columna `remitente`: es `sender` (el endpoint de
  uso la expone como alias `remitente`). El watcher de tope falló dos
  ciclos con `no existe la columna «remitente»` en log: el fallo ruidoso
  cada 5 min lo delató; sin ese log habría parecido "tope que no salta".
- El tope diario debe contar solo tokens LLM exactos (`tokens_in/out`);
  la estima de cliente no es coste. Verificarlo E2E exige sembrar
  `tokens_in/out` (el tráfico simulado deja 0) y esperar el ciclo real
  del watcher (~5 min): no hay atajo sin falsear el intervalo.
- El worker ignoraba el `texto` explícito del payload y armaba ficha
  siempre: los envíos manuales habrían llegado con texto de escalación
  al ir en vivo. Regla: payload con `texto` manda; ficha solo sin él.
- Hijos del tool de terminal mueren al cerrar la llamada (servidor de
  verificación): para esperas largas (watcher 5 min), una sola llamada
  con arranque+espera+chequeo+stop dentro; `Start-Process` entre llamadas
  no es fiable.
- `check:front` (`tsc -b`) falla en este entorno por `node_modules`
  incompleto (`vite/client`, `node` ausentes): ajeno al bloque (solo se
  tocó `src/*.rs`); se registra como limitación, no se reinstala dentro
  del bloque backend.
- Messenger duplica cada mensaje en el DOM (texto visible + `aria-label`):
  un extractor por texto plano recibe todo 2x más etiquetas de UI
  (`View buyer`, `More options`, `Presionar Enter` x11) y tarjetas del
  sistema. Regla: normalizar en el backend que recibe el texto (dedup +
  allowlist de ruido calibrada con HTML real), nunca confiar en que el
  extractor ya deduplicó.
- Sentinel `todo-prosa-sin-marcador` salta con la palabra "todo" en
  cualquier comentario (08AA-5 añadió 2 warnings con "si todo era ruido"):
  redactar comentarios sin ella ("si solo había ruido").
- Gate "sin nuevos" exige diff calibrado por pares `ruleId|archivo`
  baseline-vs-post con testigo (08AA-6: 509 pares base, solo 1+4 difieren):
  comparar totales no atribuye; los pares distinguen eliminados,
  degradados, reubicados y umbrales marginales por líneas netas.
- `Set-Content -NoNewline` con array concatena todo en una línea y rompe
  comparaciones posteriores (08AA-6: diff de 510 pares inservible dos veces);
  para archivos de pares usar escritura con saltos o memoria.
- Test que barre un canal compartido (`reencolar_fallidos`) es flaky con
  hilos paralelos en la misma BD (08AA-6: 127/128): canal único por corrida
  (`...-{sid.simple()}`), no `sleep` ni serialización global.
