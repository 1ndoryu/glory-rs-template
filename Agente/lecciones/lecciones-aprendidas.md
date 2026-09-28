# Lecciones aprendidas

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
