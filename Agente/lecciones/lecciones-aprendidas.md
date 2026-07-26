# Lecciones Aprendidas

## Backups Coolify — Alpine usa BusyBox, no GNU coreutils
- `alpine:3.20` no soporta `ls --time-style=long-iso`; BusyBox sí soporta `ls --full-time`. Si el parser de backups depende del formato de `ls`, validar primero el binario real del contenedor remoto en vez de asumir opciones GNU.
- `docker run -v nombre:/ruta ...` crea el volumen si no existe. Para leer backups, primero hacer `docker volume inspect` o puedes fabricar un volumen vacío y confundir “sin backups” con “volumen recién creado”.
- No redirigir stderr a `/dev/null` en el path diagnóstico de un listing remoto. Si el comando falla y el stderr queda vacío, el backend pierde la única pista para diferenciar incompatibilidad de shell, volumen ausente o error SSH.

## Datos reales de cliente — no van en fixtures TOML
- Los fixtures sirven para demo/dev, no para dar de alta clientes reales en produccion. Si la carga debe tocar solo cuenta, hostings existentes y facturas, crear un bootstrap admin idempotente y ejecutarlo explicitamente.
- Las fechas de dominios cobrables deben modelar la renovacion real (`due_at`), no la fecha de preview local; para GoDaddy usar el vencimiento/renovacion comunicado por el cliente.

## Dominios — pago no equivale a registro inmediato
- Un checkout de dominio no debe llamar a Contabo `order_domain` si faltan handles WHOIS reales del cliente; registrar con datos incompletos crea deuda legal/operativa.
- El boundary seguro es separar pago (`domain_orders.paid_pending_registration`) de registro final, y mostrar ese estado en panel hasta completar contactos/nameservers válidos.

## Hosting Coolify — bootstrap route estable
- En hostings compose de Coolify, el preview `sslip.io` no debe depender del FQDN implícito ni del `server_uuid` expuesto al frontend.
- El host bootstrap debe derivarse del nombre persistido del servicio (`coolify_site_name`) y el compose debe generar labels Traefik explícitas; si no, el panel puede mostrar una URL válida en apariencia que responde `404 page not found`.
- Los hostings provisionados antes del cambio necesitan refresh para heredar las nuevas labels, aunque el código ya esté corregido.

## Hosting custom domains — ownership antes de routing
- Guardar un dominio custom no debe activar Traefik/SSL inmediatamente. Primero hay que generar un TXT de ownership y bloquear el routing hasta que el backend confirme ese TXT.
- El frontend también debe reflejar esa frontera: mientras el dominio esté `pending_verification` o `verified`, el enlace principal debe seguir apuntando al bootstrap temporal y el SSL debe mostrarse como pendiente.
- Si el dominio cambia y el anterior estaba activo, retirar primero la ruta vieja en Coolify evita dejar un host huérfano que ya no coincide con la BD.

## Hosting WordPress — preinstalado significa cerrar el wizard
- Que el contenedor WordPress responda 200 o muestre `/wp-admin/install.php` no alcanza para vender "WordPress preinstalado". El provisioning debe completar el wizard y verificar que `wp-login.php` ya responde como login real.
- Si el auto-install falla pero el stack existe, no ocultarlo como éxito completo: registrar `wordpress_ready` + `wordpress_install_error` permite distinguir entre infraestructura levantada y bootstrap funcional.
- Cuando una función de provisioning mezcla create/start con bootstrap HTTP posterior, el primer endurecimiento debe ser extraer helpers privados antes de seguir agregando lógica; así se evita romper la regla de 100 líneas justo en el path crítico.

## CLOSE_WAIT en Axum/Hyper — el watchdog puede ser el verdadero killer
- Tras 5 intentos de fix (v1→v4.1), el rate de CLOSE_WAIT bajó de ~50/hora a ~2.5/hora, pero el sitio seguía cayendo cada 6h.
- Con solo 15 CLOSE_WAIT, el event loop NO se satura — 15 conexiones zombie no matan un servidor.
- **El watchdog HTTP (`spawn_http_watchdog`) que hace probes a `/healthz` cada 30s y llama `std::process::exit(1)` tras 3 fallos es el probable killer.** Sus propias conexiones de healthcheck pueden quedar en CLOSE_WAIT, y al fallar los probes → exit(1) → contenedor muere.
- **Lección:** Antes de intentar 5 fixes de CLOSE_WAIT, aislar si el watchdog es la causa desactivándolo (`GLORY_HTTP_WATCHDOG=false`). El diagnóstico correcto es desactivar componentes uno a uno, no iterar sobre el mismo síntoma.
- `hyper = "1"` sin features explícitas NO implementa `header_read_timeout()` — es cfg-gated y compila como stub silencioso. Siempre poner `features = ["http1", "http2", "server"]`.
- `socket2` 0.5.x no tiene `with_retries()` en `TcpKeepalive` — solo `with_time()` y `with_interval()`.
- `Box::leak` es necesario para pasar el hyper-util builder a `tokio::spawn` (requiere `'static`).
- **GracefulShutdown** de hyper-util reduce significativamente CLOSE_WAIT pero no elimina el problema si hay otro componente (watchdog) que mata el proceso.

## tokio::spawn sin tracking — el killer invisible (096A-7)
- **6 intentos de fix para CLOSE_WAIT fueron un diagnóstico erróneo.** El sitio caía por starvation del pool DB, no por conexiones zombie.
- `tokio::spawn(session_timing_loop(...))` creaba tareas huérfanas que retenían conexiones DB (max=10) y hacían HTTP a APIs de IA (hasta 90s cada una).
- Con ~10 visitors concurrentes en chat, el pool se agotaba → servidor colgaba. Los 15 CLOSE_WAIT eran síntoma, no causa.
- **Patrón peligroso:** `tokio::spawn` sin `tokio::time::timeout` wrapping. Invisible en tests (1 usuario), solo se manifiesta con tráfico real.
- **Fix:** Semaphore de concurrencia (max 3 AI simultáneas) + timeout global 600s en timing loop + timeout 60s en generate_context_summary.
- **Prevención:** Regla Sentinel `spawn-sin-timeout-rs` en `Agente/prevencion/prevencion-spawn-sin-timeout-2026-06-10.md`.
- **Lección fundamental:** Antes de 6 fixes iterativos sobre un síntoma, instrumentar: contar tareas activas, conexiones DB en uso, FDs abiertos. Sin métricas, cada fix es una hipótesis a ciegas.

## Hosting Coolify — compose de rescate para hosting administrado
- En stacks compose creados por Coolify, no mezclar `pids_limit` propio con `deploy.resources.limits.pids` inyectado por la plataforma. Docker Compose aborta con `can't set distinct values on 'pids_limit' and 'deploy.resources.limits.pids'` y el servicio queda `exited` aunque la API de creación haya respondido éxito.
- Las imágenes pineadas en `dockerfile_inline` también vencen: `lscr.io/linuxserver/openssh-server:9.9_p2-r0-ls190` ya no existe. Para sidecars SSH/SFTP, validar periódicamente la tag real del registry o el provisioning fallará durante el build aunque el YAML sea correcto.
- Si rescatas un servicio con `docker compose up -d` directo en `/data/coolify/services/{uuid}`, el contenedor puede quedar sano pero la URL pública en timeout. Antes de culpar a Traefik/Caddy, inspeccionar `caddy_ingress_network` en las labels y conectar `coolify-proxy` a esa red si el proxy no quedó cableado por Coolify.

## Rust — Tests con env vars
- `std::env::set_var` / `remove_var` NO son thread-safe. Rust ejecuta tests en paralelo.
- Tests que modifican las mismas env vars compiten entre sí y fallan intermitentemente.
- **Solución:** `static ENV_LOCK: Mutex<()> = Mutex::new(());` y adquirir el guard al inicio de cada test.

## Rust — clippy too_many_lines
- Límite de 100 líneas por función. Extraer helpers agresivamente.
- Patrón efectivo: extraer loops internos, bloques de setup, y operaciones I/O a funciones separadas.
- Las funciones helper pueden ser `async fn` privadas en el mismo archivo.

## SQLx — query_as! vs query_as
- `query_as!` (macro) requiere TODOS los campos del struct en SELECT/RETURNING.
- `#[sqlx(default)]` solo funciona con `query_as` (runtime), no con la macro.
- Tras modificar queries con `sqlx::query!` o `query_as!`, SIEMPRE ejecutar `cargo sqlx prepare`.
- Si la query nueva depende de una columna recién agregada, primero correr `cargo sqlx migrate run` contra la base local de `DATABASE_URL`; si no, `cargo sqlx prepare` falla aunque el código esté bien.
- Si la base local principal tiene checksums viejos de migraciones ya aplicadas, no fuerces esa BD para regenerar `.sqlx/`: crea una base temporal limpia, corre ahí `cargo sqlx migrate run` y luego `cargo sqlx prepare`.

## Hosting runtime — persistir antes de cambiar el dispatch
- Si el dominio de hosting ya tiene una fachada de runtime pero las operaciones críticas siguen despachando por `HOSTING_RUNTIME_PROVIDER`, cambiar el provider global rompe el control de despliegues legacy.
- El siguiente corte correcto no es implementar primero el runtime nuevo: primero persistir `runtime_kind` + `deployment_id` por suscripción y hacer que start/stop/restart/delete/update lean esa identidad guardada.
- `server_uuid` puede seguir como compatibilidad, pero no debe seguir siendo la única fuente de verdad para operaciones de runtime.
- Después de persistir esa identidad, hay que llevar la misma regla al inventario y observabilidad. Si el panel o los loops background siguen resolviendo por provider global, el cambio a `lightweight` rompe Coolify legacy aunque el CRUD principal ya esté corregido.
- Cuando conviven `normal-*` en `lightweight` y WordPress en Coolify, el alta comercial tampoco puede seguir leyendo `HOSTING_RUNTIME_PROVIDER`: el runtime debe fijarse por plan al crear la suscripción y el provisioning posterior debe obedecer el `runtime_kind` persistido.

## Hosting runtime lightweight — un provider no existe hasta que reconfigura
- Que `provision_hosting()` cree el sitio no alcanza: si refresh, rotación de credenciales o dominio custom siguen atados a `CoolifyConfig`, el runtime nuevo queda operativo solo en el alta inicial.
- El corte mínimo vendible para `normal-*` exige cuatro superficies reales y coherentes entre backend y manager: inventario, control, provisioning y reconfigure.
- En el runtime lightweight actual, `access_user` no debe mutarse en caliente: cambiarlo sin rehacer ownership/jail del host compartido deja el sitio en un estado ambiguo aunque el panel diga éxito.

## Hosting runtime lightweight — restore debe cerrar el contrato de credenciales
- Un backup lightweight no queda realmente operativo cuando solo empaqueta `/srv/hosting/{site}`. El restore tiene que rehidratar Caddy/SSH, arrancar el compose y devolver por JSON cualquier password SFTP regenerada.
- Si el backend no persiste esa password nueva en `hosting_subscriptions` al restaurar, el runtime revive pero el panel queda con credenciales obsoletas y el recovery se percibe como fallo parcial.

## PowerShell + cargo
- cargo escribe progreso en stderr. PowerShell interpreta stderr como error.
- `2>&1 | ForEach-Object { $_.ToString() }` y luego `$LASTEXITCODE` es el patrón correcto.
- Comandos largos o ambiguos no se deben "esperar" por intuicion: si no tienen criterio de fin claro, se ejecutan en background/async y se validan con una senal puntual (puerto, health, proceso, archivo generado, ultimas lineas del log).
- Si `npx` o una CLI puede pedir confirmacion, usar `-y` o modo no interactivo desde el inicio; si no, el flujo parece colgado aunque en realidad esta esperando input.
- Si se ejecuta un binario release por path fijo, validar que el build actualizo ese mismo archivo. `CARGO_TARGET_DIR` puede mandar `cargo build --release` a otro target y dejar viejo `target/release/*.exe`; usar `--target-dir target` o ejecutar el binario del target real.
- En apps Tauri/Rust, `current_exe()` puede apuntar a `C:\tmp\glory-target` por `CARGO_TARGET_DIR`; resolver configs desde ruta explícita/env/ancestros del cwd/`CARGO_MANIFEST_DIR` antes que junto al exe.
- Si `clap --help` desborda la pila en Windows por un enum grande, envolver el entrypoint en un thread con stack explícito mantiene el CLI diagnosticable sin tocar cada subcomando.

## Coolify Rust — recovery no-build
- Antes de `docker compose up -d --no-build --force-recreate --no-deps app`, comprobar que la imagen del servicio existe con `docker compose config` + `docker image inspect`.
- Si la imagen fue podada, el recovery no-build debe abortar y pedir reconstrucción. Recrear sin imagen puede eliminar el contenedor anterior y convertir una incidencia recuperable en 503.
- Si `sync-env` dice que Coolify tiene una env pero `printenv` no la ve dentro de `app`, revisar el `docker-compose.yml` efectivo en `/data/coolify/services/{uuid}`. En stacks Rust, `deploy-service` recrea desde ese archivo local y debe inyectar ahí las envs runtime antes del swap.
- Si una env `*_SSH_KEY_PATH` viene de Windows, no debe entrar al compose Linux. Filtrar rutas host-locales y fijar un path Linux controlado en el compose efectivo evita que Coolify reintroduzca `C:/Users/...` tras cada recreación.
- Si el entrypoint debe hacer `chown/chmod` sobre una clave privada montada antes de ejecutar como `appuser`, el bind mount no puede ser `:ro`; con `600 root:root` la validación como root pasa pero el proceso real no puede leer la clave.
- En samplers multi-VPS, no basta con arreglar la clave principal (`COOLIFY_VPS1_SSH_KEY_PATH`): el target default usa `COOLIFY_SSH_KEY_PATH`. Si la clave real vive en el host (ej. `/root/.ssh/vps2_backup`), copiarla al directorio SSH montado y fijar un path Linux evita que reaparezca una ruta Windows.
- Para `POST /api/v1/services` de stacks compose, el payload debe incluir `instant_deploy: true` si el cliente operativo ya lo usa. Sin ese flag, Coolify puede responder `500 Internal Server Error` aunque el compose sea válido, y el error queda engañosamente asociado al provisioning en lugar del contrato HTTP.
- Si el compose se genera por concatenación/manual string building y contiene `dockerfile_inline`, no alcanza con tests de `contains(...)`: hay que parsear el YAML completo en tests (`serde_yaml`) o un bloque mal indentado llega a producción y Coolify revienta con un `Unable to parse at line ...` opaco.
- En `coolify-manager-rs deploy-service`, la fase `[3/6] Construyendo imagen nueva...` puede quedarse silenciosa hasta imprimir `Build completado en ...`. No intervenir manualmente con `docker compose up` mientras ese comando siga vivo: el swap posterior puede chocar con nombres de contenedor. Si el build ya terminó y el manager quedó muerto, recién ahí verificar imagen + bind mount y hacer recovery manual.
- En `studio`, el backend no alcanzó `COOLIFY_BASE_URL=http://173.249.50.44:8000` desde dentro del contenedor aunque sí alcanzó `http://coolify:8080`. Para provisioning/compras reales, la app debe hablar con Coolify por el alias interno del stack, no por la IP pública del host.

## Checkout de prueba — limpiar antes del smoke comercial
- Antes de abrir una compra real, no basta con borrar el usuario test en BD: también hay que vaciar `GLORY_TEST_CHECKOUT_EMAILS` para impedir bypass nuevos y retirar cualquier hosting/VPS de prueba que siga activo en infraestructura.

## Checkout de prueba — no mezclar escrow real con bypass
- Los pagos sintéticos `test_bypass_*` no deben mostrarse como “retenidos” al usuario ni intentar capturarse contra Stripe al completar la orden. El backend debe exponer `bypassed` en el historial y la UI debe etiquetarlos como `Sin cobro`.
- Si un checkout bypass salta Stripe pero el flujo real dispara side-effects críticos (provisioning, activación de infraestructura, emails), el bypass debe reutilizar ese mismo helper. Si no, la cuenta de prueba queda “activa” en BD pero sin recursos reales.

## Chatbot — salida y adjuntos
- El prompt no basta para evitar Markdown visible: limpiar la respuesta al boundary antes de persistir/enviar evita `**Texto**`, headers o listas cuando el modelo se sale del estilo de chat.
- Las descripciones de imágenes no pueden quedar solo en `chat_attachments`; el historial IA se construye desde `chat_messages`, así que la metadata del mensaje debe incluir `ai_description`.
- Si el upload depende de `sessionId` WS, el frontend debe bloquear el adjunto hasta recibirlo. Ignorar el upload en silencio se percibe como “el bot no ve imágenes”.

## Node/Vite — dependencias por rama
- Cambiar de rama no actualiza `frontend/node_modules`: Git cambia `package.json`/`package-lock.json`, pero el árbol instalado queda como estado local compartido.
- Vite puede detectar el lockfile nuevo y reoptimizar, pero no instala paquetes ausentes. El launcher compartido `glory-rs/scripts/dev.mjs` debe sincronizar una vez por huella de lockfile antes de arrancar Vite.
- Recalcular la huella después de `npm install`, porque npm puede ajustar `package-lock.json` y dejar un marker obsoleto si se guarda la huella previa.
- Si un script es agnóstico del flujo local (`dev`, limpieza de target, reparación de dependencias), debe vivir en `glory-rs/scripts/`; la raíz solo debe conservar wrappers o scripts específicos inevitables.

## Code Sentinel — sentinel-disable-file
- Al crear sentinel-disable-file comments, SIEMPRE usar el ID exacto de la regla, no un alias inventado.
- `button-nativo` ≠ `html-nativo-en-vez-de-componente`. El sentinel solo reconoce el ID registrado en ruleRegistry.ts.
- La función `tieneSentinelDisable()` solo verifica `sentinel-disable-next-line`, NO `sentinel-disable-file`.
- Cada regla del sentinel debe implementar su propia verificación de `sentinel-disable-file` explícitamente si necesita soporte file-level.
- [104A-4] Se añadió soporte sentinel-disable-file a: html-nativo-en-vez-de-componente, componente-sin-hook, usestate-excesivo.
- [225A-1] `limite-lineas` usa niveles escalonados (`limite-lineas-nivel-2/3/4`) para que desactivar el primer aviso no esconda archivos gigantes; cada nivel grave necesita su propio disable-file.
- La suite de `code-sentinel` está forzada por `.mocharc.json` a cargar `out/test/suite/*.test.js`; para validar una regla aislada hay que usar `mocha --no-config --ui tdd --require out/test/registerMocks.js <archivo>`.
- Las reglas CSS que miran selectores no pueden usar regex ciega sobre el bloque completo: primero hay que strippear comentarios y distinguir clases del sistema (`menuContextualBoton`, `botonBase`, etc.) o aparecen falsos positivos masivos.
- `inline-style-prohibido` debe aceptar `style={{ '--mi-var': valor }}` también cuando el objeto está en una sola línea; si no, barras de progreso y layouts con CSS vars vuelven a romper el reporte.
- Para validar una versión local de Sentinel contra otro repo sin reinstalar la extensión del editor, usar `CODE_SENTINEL_TARGET_WORKSPACE=<ruta> npm test` en `.agent/code-sentinel`; ese host de pruebas puede regenerar `.sentinel-report.md` de forma reproducible.
- `modal-estructura-no-canonica` no puede limitarse a `form/div` internos: también debe inspeccionar `className` sobre el propio `<Modal>`, porque clases como `usuariosModal` o `checkoutModal` redefinen el contenedor compartido y si no se miran ahí el reporte queda ciego justo en el punto de entrada.
- Para detectar especificaciones de diseño CSS, no depender de nombres `boton/button`: cruzar rol interactivo local (`Trigger`, `Lista`, `Opcion`, `__dropdown`) con varias propiedades visuales (`background`, `border`, tipografia, padding, transition/animation) y cubrirlo con fixture core vs CLI.

## VarSense — CLI editor-agnostico
- Si `tsc` emite en el mismo `dist/` que esbuild, puede sobrescribir el CLI bundlereado con una version que conserva aliases `@/`. Despues del type-check, regenerar el bundle o separar outDirs.
- El smoke valido de una CLI editor-agnostica no es importar funciones desde tests: hay que ejecutar `node dist/cli/index.js ...` contra un workspace temporal para confirmar que Node puro no carga `vscode` ni aliases sin resolver.
- Si el entrypoint ya tiene shebang, no agregar otro con `banner` de esbuild; el segundo shebang queda en linea 2 y rompe `require()`/ejecucion.
- Los snapshots de equivalencia deben normalizar rutas relativas y rangos 0-indexed; los codigos de salida se prueban aparte porque un fixture con errores debe devolver `1` aunque su JSON sea el esperado.
- Un LSP no queda validado por probar solo el mapper de diagnostics: hay que levantar el binario compilado por stdio, enviar `initialize` y `textDocument/didOpen`, y verificar `textDocument/publishDiagnostics`.
- El LSP no debe importar defaults desde el entrypoint CLI. En un bundle esbuild, `require.main === module` puede hacer que el codigo de CLI imprima `Uso:` por stdio y rompa cualquier cliente LSP. Mover defaults/config a `core/config` y cubrirlo con smoke real.
- Los guards de `src/core/**` deben permitir solo el adaptador boundary (`vscodeAdapter.ts`) y fallar sobre cualquier import directo de `vscode`; asi se protege la arquitectura editor-agnostica sin bloquear la compatibilidad VS Code existente.
- La integracion Zed debe ser un launcher fino: registrar `language_servers`, resolver `varsense-lsp` por entorno/PATH/dist local y no copiar reglas ni empaquetar el servidor dentro de la extension.
- En manifests Zed reales, la tabla estable observada es `language_servers`; algunos docs muestran `language-servers` en ejemplos multi-lenguaje, asi que validar contra ejemplos oficiales antes de copiar sintaxis.

## Coolify — deploy vs restart
- `POST /api/v1/services/{uuid}/restart` solo reinicia containers existentes con la misma imagen.
- `GET /api/v1/deploy?uuid={uuid}&force=true` trigger un rebuild completo (git pull + docker build).
- Para cambios de código, SIEMPRE usar deploy, no restart.
- coolify-manager.exe `deploy --name` es para WordPress themes, no para apps Rust. Usar API directa.
- coolify-manager.exe `restart` no siempre reinicia los contenedores de apps Docker Compose; `redeploy` (API) es más fiable para forzar recreación.
- En `studio` (nakomi.studio), los endpoints admin firmados por JWT usan el env runtime `SERVICE_PASSWORD_64_JWTSECRET`; `JWT_SECRET` listado en Coolify no autenticó `/api/admin/blog` durante la verificación real.
- Si el backend introduce una env runtime nueva en stacks Rust, agregarla el mismo bloque al allowlist de `coolify-manager-rs sync-env`; si no, `--only` la reporta como bloqueada/ausente y producción queda sin el comportamiento aunque el código compile.

## Coolify Rust — health por IP Docker
- Un healthcheck a `localhost` dentro del contenedor no prueba el camino real de Traefik. Puede responder localmente mientras la IP Docker del contenedor se cuelga y el proxy muestra `no available server`.
- En stacks Rust, el health válido debe probar `http://$(hostname -i):3000/...` o un probe host/proxy → IP del contenedor.
- Si el probe host→IP falla tras un swap, la recuperación segura es recrear solo `app` con `--no-build --force-recreate --no-deps`, verificando antes que el compose mantiene `/data/uploads/{sitio}:/app/uploads`.
- El default Docker-safe para `HOST` es `0.0.0.0`; `127.0.0.1` debe quedar solo como override local explícito.
- En runtime slim, no asumir `awk`/utils extra en healthchecks. Si el proceso acepta TCP pero no responde HTTP, usar `/healthz` liviano, self-probe interno con reinicio y mover trabajo CPU-bound (`image` decode/resize/encode) a `spawn_blocking` con concurrencia limitada.
- Si el proceso queda vivo pero HTTP se congela, el autoheal útil no es el que reinicia primero: primero debe capturar snapshot host-level (`docker inspect`, `ps -T`, `/proc/<pid>/task/*`, `ss`) y guardarlo en un bind mount persistente antes de tocar el contenedor. Sin esa captura, cada reinicio borra la única evidencia del freeze real.

## Rust/Axum — Timeouts HTTP obligatorios para APIs externas
- **NUNCA crear `reqwest::Client::new()` sin `.timeout()` en código de producción.** Una API externa que se cuelga bloquea la tarea async indefinidamente. Si la tarea retiene una conexión del pool de BD (SQLx), agota el pool y congela toda la aplicación (deadlock de pool). Síntomas: proceso vivo pero 503, tcp backlog lleno, threads dormidos.
- Usar `reqwest::Client::builder().timeout(Duration::from_secs(30)).build()` como mínimo.
- Para cadenas de retry (Groq 3 keys × 3 modelos + Gemini 6 modelos = 24 intentos), agregar timeout global con `tokio::time::timeout(Duration::from_secs(90), ...)` además del per-request timeout.
- [124A-1] Este bug causó un 503 en producción ~1h después de deploy. Los logs no mostraron crash porque no hubo panic — el pool simplemente se agotó en silencio.

## Rust/Axum — SPA fallback con status correcto
- `ServeDir::not_found_service(ServeFile::new(index.html))` puede devolver el HTML correcto del SPA con status HTTP 404 en rutas directas como `/servicios` o `/soluciones/hosting`.
- Para rutas React válidas que deben indexarse o validarse por HTTP, montar rutas Axum explícitas que sirvan `index.html` con status 200. Dejar rutas eliminadas intencionalmente fuera de esa lista para conservar 404 real.
- Después de un deploy SPA, validar códigos HTTP con `curl`, no solo render visual en navegador.

## UI del panel — bases compartidas
- Si una variante visual ya es la buena (`hostingCardIcono` en este caso), promover ese estilo a la clase base compartida y dejar las variantes futuras como overrides mínimos con composición de clases, no como recetas duplicadas.
- Si un `MenuContextual` necesita una composición nueva (grid de apps, launcher, etc.), agregar una variante semántica al componente base antes de pasar `triggerClassName`/`panelClassName` locales. Sentinel lo detecta como parche visual.

## Panel CMS — menus contextuales en cards
- Si una card clickeable del CMS contiene un `MenuContextual`, no puede usar `overflow: hidden` ni depender solo de `:hover` del card para mostrar el wrapper del menú. En cards bajas o con paneles que salen del contenedor, la acción destructiva queda inaccesible aunque el endpoint responda correctamente.

## CSS validator — variables resueltas
- Algunos diagnósticos de CSS siguen reportando “hardcodeado” aunque la regla ya use `var(--token)` y el build pase. Cuando ocurra, corroborar con `npm --prefix frontend run build` antes de perseguir falsos positivos del validador.

## Imágenes responsive — `sizes` omitido no puede caer siempre en `100vw`
- Si un `img` con `srcset` usa anchos descriptivos y el componente no recibe `sizes`, el navegador asume `100vw` y sobre-descarga variantes grandes en avatars, logos y cards pequeñas.
- Medir el ancho real renderizado con `ResizeObserver` y derivar un `sizes` en píxeles evita ese sesgo sin obligar a propagar `sizes` manual en todos los callers.
- Si el backend ya soporta más buckets que el frontend, el cuello de botella real está en la generación del `srcset`, no en el proxy.

## Hero/carrusel above-the-fold — a veces hace falta ancho fijo
- Cuando el objetivo es una URL exacta de optimización (`w=1200&q=80`) para controlar peso en un bloque hero/carrusel, el cálculo responsive por ancho medido + DPR puede seguir sobredescargando.
- En esos casos conviene un modo explícito sin `srcSet` responsive y con ancho fijo de proxy, en vez de pelear contra buckets automáticos.

## Navegación pública — una sola fuente para catálogos vivos
- Si una página pública (`/servicios`, `/proyectos`, etc.) consume un catálogo vivo desde API/CMS, el header y cualquier submenú que liste esos items debe usar la misma fuente.
- Mantener un dataset estático paralelo solo para navegación termina desalineando títulos, slugs y rutas aunque la página principal ya esté correcta.

## Admin deletes — dependencias reales
- Si una entidad admin pide “eliminar” pero tiene FKs sin cascade repartidas en varias tablas, no implementar hard delete ciego. Primero exponer al panel un preflight de dependencias con mensaje explícito y usar suspensión como fallback operativo.

## CMS público — no maquillar vacíos con demo data
- Si una vista pública depende del CMS/API, distinguir entre “todavía no cargó” y “la API devolvió vacío”. Reutilizar fallback demo cuando el backend responde `[]` oculta desincronizaciones reales y hace que home/listados muestren contenido fantasma.
- El router tampoco debe pasar data estática legacy a detalles CMS como placeholder inicial. Aunque la API reemplace el contenido rápido, el usuario percibe un flash de proyecto/servicio viejo; usar loading/404 hasta tener el recurso real.

## Herramientas operativas — demo explícito o API local real
- Si una GUI controla infraestructura, abrirla en navegador no autoriza datos demo silenciosos. El navegador debe consumir una API local real o fallar visible; el modo demo solo debe existir con una bandera explícita.
- Menús dentro de tablas con `overflow` deben renderizarse por portal/fixed para no quedar recortados por el scroll horizontal.
- Si un listado base parece lento, separar el fetch estructural de probes derivados. En 105A-28 `list_sites` tardaba milisegundos; lo lento eran health-checks SSH secuenciales encadenados después del render.
- Cachear lecturas caras en dos límites ayuda: cliente GUI para navegación entre vistas y `gui-api` para compartir resultados entre componentes/procesos web. Los refrescos manuales deben usar `force=true` y reemplazar la caché con datos frescos.

## Checkout de órdenes — IDs visuales no son contrato
- Si el catálogo frontend usa IDs compuestos para UI/traducciones (`web-basico`, `apps-medio`) pero el backend persiste slugs canónicos (`basico`, `medio`), normalizar en el cliente API antes del POST. Un `404` en creación puede ser un `NotFound` de dominio, no una ruta faltante.
- Los alias de `service_slug` también envejecen: si el backend vuelve a usar `diseno-web` como canónico y el cliente lo sigue remapeando a `diseno-de-sitios-web`, reaparece un `404` aunque el `plan_slug` ya esté bien normalizado.
- [095A-19] Si el CMS puede cambiar el slug público activo, la compatibilidad legacy debe vivir en backend y probar primero el slug recibido. El frontend no debe decidir el canónico porque se desactualiza con cada sync de contenido.

## Catálogo público — no vender servicios fantasma
- Si la compra depende del catálogo real del backend, el detalle/listado público no debe caer a datasets estáticos que incluyan servicios ya no publicados. Aunque `apiCreateOrder()` normalice slugs, seguir mostrando `ecommerce`/`seo`/`marketing-digital` cuando la API solo expone 4 servicios termina reproduciendo 404 de negocio igualmente.

## Catalogo CMS — no fusionar servicios reales para acomodar fallbacks
- Si el negocio distingue `ecommerce` y `marketing-digital`, el CMS debe modelarlos como servicios separados aunque el frontend heredado tenga un fallback incompleto. Fusionarlos para “encajar” con el dataset estatico termina rompiendo el catalogo real y obliga a deshacer la carga luego.
- La correccion segura es: arreglar primero la propuesta fuente y resincronizar el CMS; el fallback del frontend se corrige despues como tarea aparte, nunca al reves.

## Empty states — no mezclar jerarquías en la misma sección
- Si una sección ya tiene un estado vacío completo con icono, título y texto, las tabs internas no deberían degradarse a un párrafo desnudo. Reutilizar un bloque común evita que el vacío “parcial” se vea como un render roto.

## Inputs del sistema — no reestilar por costumbre
- Si una sección usa `Input` base y el override local no cambia comportamiento ni semántica, eliminar la clase local en vez de mantener CSS duplicado. Cada wrapper visual extra encarece futuras limpiezas sin aportar contrato nuevo.

## Fixtures TOML — tracking no sustituye existencia real
- Si `_glory_fixtures` conserva `content_hash` y `db_id` pero la fila real ya no existe, el sync no puede hacer `skip` ciego. Primero debe verificar existencia física y reinsertar si falta.
- Cuando una migración agrega columnas `NOT NULL` a una tabla fixture-managed, actualizar ese mismo día todos los `content/*.toml` de la tabla. Un solo campo faltante (`users.username` en este caso) rompe en cascada todos los fixtures dependientes y termina pareciendo un bug del seed en vez de un drift del fixture.

## Code Sentinel — sentinel-disable-next-line formato
- `sentinel-disable-next-line {rule-id}` DEBE estar en la línea inmediatamente anterior a la violación.
- En Rust, usar `// sentinel-disable-next-line {rule-id}` como comentario single-line.
- NUNCA ponerlo dentro de un comentario multilínea `/* ... */` que ocupe varias líneas, porque el checker compara `lineas[i-1]` y la línea anterior real sería el cierre `*/`, no el disable.
- Patrón correcto: explicación en `/* ... */` arriba, y `// sentinel-disable-next-line ...` en la línea justo antes del código.

## Stripe live mode - verificacion local de SetupIntent
- Si el entorno local apunta a llaves `pk_live` y `sk_live`, Stripe bloquea crear tarjetas de prueba por REST con numeros crudos aunque el flujo real con Stripe.js si sea valido.
- Para validar un flujo nuevo de tarjetas guardadas en ese contexto, separar: backend y contrato por API local, compilacion del modal con Stripe.js, y justificar que la confirmacion completa requiere navegador o llaves test.

## Extensiones VS Code — Memory leaks y rendimiento (AUDIT1)
- Regex con flag `/g` NO crear dentro de loops: compilar una vez a nivel de módulo, resetear `.lastIndex = 0` antes de reusar.
- `Promise.all()` sobre arrays dinámicos (archivos encontrados) siempre necesita throttling con lotes.
- Sets/Maps module-scoped son memory leaks si no se limpian en `deactivate()` o al cerrar documentos.
- Funciones que extraen "todos los tokens" de "todos los archivos" sin filtro son bombas de RAM. Siempre acotar semánticamente (ej: solo class/className, no todo identificador).
- WebviewPanels sin singleton ni `onDidDispose` crean múltiples instancias que nunca se liberan.
- Concatenación de strings en loop para reportes es O(n²) — usar `array.push()` + `.join()`.
- Event handlers que actualizan "todos los documentos abiertos" necesitan debounce cuando el emisor puede disparar muchas veces en ráfaga.

## Checkout publico - no crear dos PaymentIntent para la misma orden
- Si el flujo publico ya creo la orden y recibio `client_secret`, el checkout siguiente debe reutilizarlo. Volver a llamar `/api/orders/{id}/pay` desde la siguiente pantalla duplica intents y complica la conciliacion del pago cancelado.
- Cuando el panel persiste la tab activa en `sessionStorage`, cualquier flujo que redirija a `/panel` como fallback debe fijar primero la seccion correcta o el usuario puede aterrizar lejos de la orden recien creada.

## Hosting publico - no reutilizar el funnel de ordenes
- Si un producto ya tiene backend propio de suscripcion (`/api/hosting/subscribe`), la UI publica no debe seguir entrando por `/api/orders` aunque visualmente reuse cards o modales de compra.
- Un `404` en checkout puede venir de un contrato de dominio equivocado: en este caso `service_slug = hosting` no existia en el catalogo de ordenes, asi que la solucion correcta fue mover el flujo al endpoint real de hosting, no inventar aliases en orders.

## Infra admin - proveedor no equivale a deployment

## Hosting runtime — abrir el seam antes de tocar SQLx
- Si el dominio de hosting está acoplado a un proveedor concreto, el primer corte correcto no es una migración masiva de columnas ni renombrar toda la persistencia legacy. Primero hay que extraer una fachada de runtime y recablear altas, control, bajas y listado al nuevo boundary.
- Exponer `runtime_kind` y `deployment_id` derivados de los campos legacy permite preparar frontend y handlers sin disparar todavía cambios de `sqlx::query!` ni regeneración de `.sqlx/` en el mismo bloque.

- Si el panel pide “despliegues reales”, la fuente correcta es la capa de orquestacion (Coolify, Kubernetes, etc.), no la API del proveedor de VPS.
- Contabo responde “que servidores existen”; Coolify responde “que servicios estan desplegados”. Mezclar ambas capas permite cerrar tareas en falso y oculta orfandades reales entre deployment y suscripcion.
- Para recursos de infraestructura, el dashboard tampoco debe abrir SSH en render. Meter un sampler de baja frecuencia + snapshots DB evita carga, timeouts y variaciones raras de UI; hasta que exista una muestra, mostrar `null`/guiones es la opcion correcta.

## Infraestructura — `docker stats` no define límites reales
- Si un contenedor Docker no tiene `NanoCpus` ni `Memory` configurados, `docker stats` puede mostrar la RAM total del host como `MemLimit`; eso no equivale a un cap aplicado.
- Los límites runtime reales deben salir de `docker inspect` (`HostConfig.NanoCpus`, `CpuQuota`/`CpuPeriod`, `Memory`) y persistirse como `null` cuando valen `0`.
- En el panel de hosting, los recursos del plan comercial y los límites runtime detectados deben mostrarse separados. Mezclarlos hace que un deployment legacy parezca capado cuando en realidad está ilimitado.

## Hosting Coolify — CPU burst debe seguir el runtime real
- El contenedor al que se le cambia CPU no se deduce por `plan`; se resuelve desde la identidad runtime del stack. En Coolify, `com.docker.compose.project` puede ser `deployment_uuid` aunque el sitio tenga `coolify_site_name` distinto, así que el executor debe probar `deployment_uuid` primero y dejar el slug solo como fallback.
- Un balancer runtime no debe decidir contra el baseline del plan sino contra el `site_cpu_limit_cores` observado por el sampler. Si no recuerda el último target pedido, reintenta el mismo `docker update --cpus` en cada ciclo hasta que llegue la siguiente muestra.
- Si `docker update --cpus` falla, el loop no debe memorizar igual `last_requested_target`; hacerlo bloquea reintentos posteriores aunque el contenedor siga en el cap viejo.

## Hosting Coolify — `cpu_quota=-1` deslimita aunque `NanoCpus` quede stale
- `docker update --cpus 0` no limpia el cap CPU en contenedores ya limitados; la sonda de campo en `hosting-0fa1d5da` dejó `NanoCpus=500000000` intacto.
- `docker update --cpu-quota -1` sí devuelve CPU efectivo ilimitado, pero `docker inspect` puede seguir mostrando `HostConfig.NanoCpus` con el valor viejo.
- El sampler debe priorizar `CpuQuota < 0` como `sin limite`; si sigue confiando primero en `NanoCpus`, el backend cree falsamente que el sitio continúa capado y el modo de contención nunca se libera solo.

## Hosting Coolify — `docker inspect --format` no separa con tabs reales
- En el sampler de infraestructura, `docker inspect --format '{{...}}\t{{...}}'` devuelve `\t` literales, no tabs reales. Si el parser divide solo por `\t` reales, todas las columnas de runtime limits quedan truncadas y `site_cpu_limit_cores` se persiste en `null` aunque Docker tenga caps válidos.
- El síntoma engañoso es que `docker stats` sí muestra CPU/RAM actuales y la muestra del deployment parece fresca, pero los límites runtime siguen vacíos para todos los sitios del servidor.
- La defensa correcta es normalizar `\t` literales antes del parseo o emitir la línea con `printf`; no asumir que el formato de Docker interpreta escapes como lo hace `printf`.

## Commit-por-tarea — no acumular cambios
- Si el protocolo dice "un commit por tarea", cumplirlo inmediatamente después de validar, no al "final de la sesión" ni "cuando haya tiempo". Acumular 3+ tareas sin commit significa que un solo error en git rompe todo el trabajo.

## Wallet demo local — seed antes que frontend
- Si `cliente@test.com` o `empleado@test.com` muestran wallet vacía pero `src/services/seed.rs` ya define movimientos y retiros, el problema suele ser de entorno local sin reseed reciente, no de UI. Reejecutar `POST /api/admin/seed` y verificar `/api/wallet`, `/api/wallet/transactions` y `/api/wallet/withdrawals` antes de modificar componentes.
- El push es parte del cierre de la tarea. Si no se hizo push, la tarea no está cerrada.
- Refuerzo agregado al protocolo (104A-19): prohibición explícita de acumular 2+ tareas sin commit+push.

## Consistencia visual — leer antes de crear
- Antes de crear o modificar CSS, leer primero `variables.css`, los componentes atómicos en `ui/` y los patrones de componentes similares. Cada clase ad-hoc que duplica un token existente es deuda visual que se acumula.

## Axum detrás de proxy — el rate limit debe asumir tráfico SPA real
- Con Coolify/Traefik, la IP `peer` no basta para bucketear usuarios; usar los headers `forwarded`/`x-forwarded-for` evita `429` cruzados cuando varios clientes salen por el mismo proxy.
- Si la SPA hace polling y varias requests concurrentes por vista, los límites de auth/API no pueden calibrarse como si el sitio fuera navegación manual de una sola petición por segundo. En producción, los umbrales tienen que absorber concurrencia normal antes de parecer abuso.

## Coolify API — un token Sanctum mal serializado rompe toda la API
- Si Coolify 4.1.0 devuelve `500` incluso en `/api/v1/version` y el HTML menciona `in_array(): Argument #2 ($haystack) must be of type array, null given`, revisar `personal_access_tokens.abilities`.
- El valor `[*]` guardado como `text` no equivale a JSON válido `["]*["]`; Sanctum lo castea a `null` y cae en `tokenCan()`. Corregir la fila a `["*"]` restaura `/api/v1/version` y `/api/v1/services` sin redeploy.

## Panel de despliegues — runtime caído no equivale a cero inventario
- Si `list_services()` falla para un runtime externo, el panel admin no debe responder `[]` como si no existieran hostings. Un fallback mínimo desde `hosting_subscriptions` mantiene la tabla utilizable y deja claro que cayó el proveedor, no el inventario persistido.
- Badge siempre en grises (sin color semántico) fue una decisión de diseño Nakomi. Footers, headers, cards deben compartir un patrón unificado.

## Modales — semantica compartida primero
- Si un modal necesita copy neutral, usar `.modalTexto` en `Modal.css` antes de inventar `.algoModalTexto` o `.algoModalDescripcion`. Las clases locales solo deben conservar layout o estado.
- En analyzers CSS por bloques, `sentinel-disable-next-line` debe anclarse a la linea real del selector y no al inicio de un comentario previo; si no, la supresion parece rota aunque el helper este bien.
- La estructura comun del modal tambien es contrato: cuerpo y campos deben salir de `.modalFormulario` / `.modalCampo` o de `ModalBody` / `ModalField`. Clases como `.algoFormCrear` o `.algoCampo` vuelven a introducir especificaciones de diseno prohibidas.

## Checkout publico — cortar roles invalidos antes del request
- Si el backend solo permite crear ordenes como `client` o `admin`, el modal publico no debe decidir solo por `logueado`. En local es frecuente quedar con sesion `employee` por pruebas del panel y eso reproduce `403` evitables.
- Para validar ese caso sin tocar backend, basta simular `auth_user` en `localStorage`, recargar la SPA y comprobar que el guard del frontend muestra el mensaje en vez de entrar al estado de procesamiento.

## Ordenes por fases — el CMS manda
- Si un plan ya define `service_plan_phases`, la creacion de la orden no debe reescribir esos titulos/descripciones con placeholders genericos. Hacerlo rompe el contrato con el CMS y da la falsa impresion de que el empleado debe “definir” las fases a mano.
- Si el checkout ofrece `payment_mode = phased`, un plan sin fases debe rechazarse en dos boundaries: al guardar el plan desde admin y al crear la orden, para que la inconsistencia no llegue a producción.

## CMS de servicios — editar plan no puede reciclar IDs
- Si una orden ya referencia `service_plans.id`, el CMS no puede hacer `DELETE + INSERT` para guardar planes. Hay que preservar el ID existente y tratar `service_plan_phases` como el hijo reemplazable.
- Para permitir cambios de slug/nombre sin romper órdenes históricas, el frontend admin debe enviar `id` opcional por plan y el backend usarlo como llave estable antes de caer al slug.

## Chat de órdenes — sender_type también es contrato
- Si el sistema persiste nuevos tipos semánticos de mensaje (`ai_intermediary`, por ejemplo), el esquema de `chat_messages.sender_type` debe crecer junto con el código. Dejarlo en `VARCHAR(10)` hace que la IA genere bien pero no pueda guardar la respuesta.
- En rutas de chat con `tokio::spawn`, ignorar el resultado de `send_message().await` convierte un fallo de persistencia en un "la IA no responde" imposible de diagnosticar desde la UI. Al menos hay que registrar el error explícitamente.

## Chatbot de cuenta — el prompt no autoriza
- Si el bot puede consultar pedidos, pagos, hosting o reportes, el prompt solo debe orientar la conversación. La frontera real es un contexto firmado (`user_id`, rol real, rol operativo, impersonator) pasado a cada tool y validado en backend.
- `visitor_id` persistido sin owner key mezcla conversaciones cuando alguien hace logout/login o impersona en el mismo navegador. Separar storage por identidad/rol efectivo evita fugas entre cuentas.
- Los logs de tools deben incluir nombre, sesión y status, pero nunca argumentos crudos: pueden contener email, reportes o detalles de pago.

## Panel interno — storage solo no alcanza para deep links
- Si una vista interna del panel debe sobrevivir recargas, compartirse por URL o abrirse desde una notificación, `localStorage`/`sessionStorage` y custom events no bastan. La URL debe ser una fuente observable de verdad y los hooks dueños del detalle deben hidratarse desde `location.search`.
- Al sincronizar estado profundo (`order`, `hostingId`, `chat`) a la URL, no borrar el query param en el primer render si ese mismo param todavía está intentando hidratar el estado. El orden correcto es: leer URL, seleccionar recurso, luego persistir el estado ya resuelto.

## Sesiones impersonadas — no degradar a 500
- Si un JWT con `impersonator` sobrevive a un reseed local y el admin original ya no existe, volver a `admin` no debe caer en `500`. El backend debe responder `401/403` con mensaje accionable y el frontend debe limpiar la sesión persistida para cortar el bucle.

## Tokens visuales — activo no equivale a neutro
- `--bg-item-active` no sirve como borde base. Los bordes genéricos del sistema deben usar `--border-default`; reservar el token activo evita que estados normales parezcan seleccionados y simplifica los barridos visuales.

## Upstreams opcionales - no esconderlos tras 500 internos
- Si una integración externa opcional falla (Contabo, por ejemplo), no devolver `Internal` genérico desde el handler. Clasificar y exponer un `message` accionable evita perseguir fantasmas de backend cuando el bloqueo real es `invalid_grant`, parseo o indisponibilidad del proveedor.
- Cuando una credencial legacy es ambigua (`PASSWORD_CONTABO`), documentar y soportar una variable explícita (`CONTABO_API_PASSWORD`) reduce drift entre proyectos y evita repetir el mismo diagnóstico en cada repo.

## Auditorías vagas - convertirlas en backlog ejecutable
- Si el roadmap trae una tarea tipo “hay que revisar” o “presiento que falta mucho”, cerrarla solo con lectura no alcanza. Hay que salir de la auditoría con un documento, un plan activo y subtareas concretas reinsertadas en el roadmap.
- En hosting, el valor real de la revisión estuvo en separar claramente: provisioning, ciclo de cobro/suspensión, datos reales del servidor y dominios. Sin ese corte, todo queda escondido en una sola tarea imposible de cerrar.

## CMS de servicios - update no puede delegar el contrato a SQLx
- Si el panel edita `services`, el handler de update debe llamar `validate()` igual que create y mapear conflictos/constraints frecuentes a `409/422`. Dejar que `sqlx::Error` suba crudo convierte errores previsibles del CMS en `500` opacos.
- Cuando el slice ya tiene una version mas robusta en otro dominio cercano, conviene copiar el patron completo. En este caso, `UpdateServiceParams<'_>` + `query_as::<_, ServiceRecord>(...).bind(...)` dejo services alineado con projects y evito pelear con una macro mas fragil para updates parciales.

## Catalogos publicos - la shell compartida va antes que dos CSS parecidas
- Si servicios y proyectos tienen la misma pagina a nivel de hero, contenedor y espaciado, ese layout debe vivir en un componente compartido. Mantener dos islands con wrappers casi iguales solo garantiza drift visual y correcciones duplicadas.
- Antes de “arreglar el padding” de una clase legacy, buscar todos sus consumidores. `serviciosContenedor` y `proyectosContenedor` ya se usaban fuera del catalogo publico, asi que la solucion segura fue crear clases nuevas `catalogPage*` y mover el layout comun a una shell dedicada.

## CMS de servicios - un 409 util no sirve si el frontend lo esconde
- Si el backend devuelve `ErrorResponse { message }`, los hooks del panel no deben quedarse con `err.message` de Axios. Hay que extraer `response.data.message`; si no, una mejora real del contrato HTTP termina viendose como otro error generico de red.
- Cuando el formulario ya tiene en memoria el conjunto completo de slugs, conviene hacer preflight local antes del submit. El servidor sigue siendo la verdad final, pero el usuario recibe feedback inmediato y se evita un roundtrip inutil para conflictos obvios.

## CMS de servicios - guardar servicio y planes en dos pasos exige preflight

## Coolify huérfanos - borrar por UUID no alcanza
- Si un panel lista despliegues "huérfanos" cruzando Coolify contra BD, el borrado no puede validar solo `server_uuid`. Algunas filas legacy siguen vinculadas por `coolify_site_name`, y tratarlas como huérfanas rompe la coherencia entre panel e infraestructura.
- Antes de borrar un stack desde Coolify, cruzar siempre por UUID y por nombre de servicio; si cualquiera coincide con una suscripción real, devolver conflicto y forzar el flujo de eliminación desde la suscripción.
- Si el editor primero guarda el servicio y despues hace `PUT /plans`, cualquier validacion tardia de planes produce sensacion de guardado parcial. Antes del primer request hay que validar los invariantes minimos del segundo paso con el mismo contrato del backend.
- En este slice, los invariantes minimos que no deben salir del cliente son: slug y nombre obligatorios por plan, slug unico dentro del servicio, al menos una fase por plan y titulo obligatorio por fase.

## CMS de servicios - no mezclar guardado de metadatos con persistencia de planes
- Si el usuario solo edita imagen, SEO o datos generales, el editor no debe volver a persistir `service_plans`. Un segundo request redundante puede fallar por deuda historica de planes y dejar la falsa impresion de que el campo principal “no guarda”.
- Cuando una pantalla guarda recursos relacionados en dos endpoints, hay que detectar qué slice cambió realmente y disparar solo ese endpoint. La comparación contra el snapshot cargado evita roundtrips innecesarios y reduce errores cruzados.

## Editor inline vs panel - un fix no existe si el control path real sigue duplicado
- Cuando una misma UI reutiliza el mismo modal pero lo monta desde dos owners distintos, no basta con corregir uno. Si el stack apunta a otro provider, hay que seguir ese call site exacto antes de dar el bug por cerrado.
- Si dos flows necesitan decidir lo mismo sobre persistencia (`guardar planes o no`), extraer un helper compartido es más seguro que copiar la condición. En este caso, el drift entre `SubTabServicios` y `AdminEditorProvider` mantuvo vivo el 422 aunque el panel ya estaba corregido.

## Servicios publicos - un fallback estatico puede ocultar deuda real del CMS
- Si la vista pública cae a un dataset estático cuando la API devuelve planes vacíos, el sitio puede aparentar estar “bien” mientras el CMS refleja la realidad de la BD. Esa divergencia confunde el diagnóstico y retrasa la corrección del origen de datos real.
- En este repo, `SeccionPlanesServicio` usa `obtenerPlanesServicio(slug)` como fallback. Mientras exista, cualquier auditoría de catálogo debe distinguir entre “planes reales del backend” y “planes heredados del frontend”.

## Sync admin de servicios - preferir el slug activo y no tocar media vacia
- Si un servicio tiene aliases legacy en la misma BD, el sync por API no puede elegir el primer slug que coincida. Debe priorizar el registro `is_active = true`; en Nakomi, `diseno-de-sitios-web` es el servicio público activo y `diseno-web` quedó como legacy inactivo.
- Cuando la propuesta no trae `image_url` o `gallery`, el sync debe omitir esos campos o preservar la media existente. Enviar media vacía desde el cargador pisa información del CMS justo en la parte que el usuario quiere seguir gestionando manualmente.

## Capabilities del CMS - mas texto, mismo estado de publicacion
- Si el usuario pide ampliar las Capabilities de servicios, el cambio real vive en `skills.descripcion` del catalogo CMS. La validacion mas fiable es medir por API que esas descripciones crecieron y no solo confiar en una lectura visual indirecta.
- Cuando un servicio ya fue archivado manualmente, cualquier sync de copy debe respetar ese estado en la fuente reusable (`status = archived`, `is_active = false`) o lo reactivara en el siguiente empuje aunque el texto sea el unico objetivo del cambio.

## SSH Guard — bloqueo del lado del cliente con perfil PowerShell
- En vez de restringir keys en el servidor (riesgo de lockout), bloquear `ssh`/`scp` desde el perfil PowerShell del usuario. Es más seguro, reversible al instante y no afecta a Coolify ni a coolify-manager-rs.
- coolify-manager-rs usa `russh` (cliente SSH nativo en Rust), NO `ssh.exe`. Por eso el guard en PowerShell no lo afecta — son caminos completamente separados.
- El perfil vive en `$PROFILE` (CurrentUserAllHosts = `~\OneDrive\Documentos\PowerShell\profile.ps1`). Se carga automáticamente en cada sesión PowerShell.
- Bypass de emergencia: `ssh-unsafe` y `scp-unsafe` ejecutan los comandos reales sin filtro. Útil para diagnóstico puntual.
- IPs bloqueadas: `66.94.100.241` (VPS1) y `173.249.50.44` (VPS2). Para agregar más, editar `$Script:BlockedSSHHosts` en el perfil.
- **Lección del lockout anterior (jun 2026):** nunca usar `$(date)` en nombres de backup SSH — el shell remoto puede interpretar el `$` y romper el authorized_keys. Usar nombre fijo.

## Chat system integral — Bloques A/B/C/D/E (jul 2026)

### `#[sqlx(default)]` vs `#[sqlx(default = "expr")]`
- `#[sqlx(default = "String::from(\"automatic\")")]` no compila en sqlx 0.8. Usar `#[sqlx(default)]` simple y dejar que `Default::default()` del tipo maneje el valor (String → "", Option → None, bool → false, i64 → 0).
- Si se necesita un default distinto al del tipo, implementar un setter separado o usar la query de inserción.

### Construcciones manuales de structs con campos nuevos
- Al añadir un campo a un struct derive(FromRow) con #[sqlx(default)], las queries SQL siguen funcionando.
- PERO las construcciones manuales del struct (en tests, helpers, etc.) requieren TODOS los campos explícitamente.
- Buscar con `grep -n 'StructName {'` todos los sitios que construyen el struct manualmente.
- Ejemplo: añadir `sequence_num: Option<i64>` a ChatMessage rompió 3 construcciones en ai_chat.rs tests.

### Worker background no debe crear su propio hub/cliente
- Si un worker necesita enviar mensajes por WS, debe reutilizar el hub existente del AppState.
- Pero los workers se spawnean ANTES de que AppState esté completamente construido.
- Solución: simplificar el worker para que solo persista en BD. El WS broadcast ocurrirá en el próximo reconnect/fetch del cliente.
- Aceptar este trade-off si el delay es razonable (10 min de inactividad → el cliente probablemente se desconectó).

### Audio leader election entre pestañas
- Web Locks API (`navigator.locks.request`) es la forma más fiable de elegir un líder entre pestañas.
- Con `ifAvailable: true`, retorna `null` si el lock no está disponible (otra pestaña lo tiene).
- Limitación: si la pestaña líder se cierra, el lock se libera pero las demás no reciben notificación. Necesitan re-intentar.
- Fallback: localStorage lease con ownerId + timestamp de expiración. Más propenso a carrera pero funciona en todos los navegadores.
- `processedMessageIds` debe ser un Set global (módulo), no local al componente, para persistir entre re-renders.

### Tests con env vars compartidos
- `std::env::remove_var` / `set_var` no son thread-safe. Tests en paralelo pueden interferir.
- Si un test depende de una env var, limpiarla al inicio del test con remove_var.
- Ideal: usar un Mutex estático (como ENV_LOCK en test_checkout.rs) para serializar acceso a env vars en tests.
- El patrón `checkout_bypass_is_configured()` lee env vars → un test que no setea la var puede fallar si otro test la seteó.

### Migraciones con columnas nuevas en visitor_profiles
- Las columnas `email_normalized`, `email_captured_at`, `continuation_consent_at`, `continuation_declined_at`, `email_source` ya existen en la migración de alertas.
- No crear migración duplicada; usar la existente o una nueva que solo añada lo que falta.
- Las columnas con #[sqlx(default)] en el modelo no requieren que TODAS las queries las seleccionen.

### Email templates: patrón consistente
- Todas las plantillas usan helpers: `section_title()`, `paragraph()`, `summary_table()`, `cta_button()`, `email_layout()`.
- `html_escape()` es obligatorio para todo input dinámico.
- Las funciones de envío en email.rs siguen el patrón: construir subject, llamar template, enviar, loguear en email_logs.
- `EmailLogRepository::insert` registra categoría, context, object_id, status y error.

### Datos de contacto: no reutilizar una escritura con valores centinela
- Actualizar solo el nombre con una función que también escribe email y pasar `""`
  puede destruir silenciosamente un contacto válido.
- Nombre, email y consentimiento necesitan contratos explícitos; la captura
  conversacional de email debe persistir su metadata de forma atómica.
- Los logs de captura deben confirmar el resultado sin imprimir la dirección.

### Gateways firmados en WordPress: canonicalizar headers y probar recepción real
- `WP_REST_Request::get_headers()` puede transformar `X-Glory-Timestamp` en
  `x_glory_timestamp`. Un verificador debe aceptar la forma canónica del framework
  y la forma HTTP, o usar `get_header()` para cada campo.
- Validar que dos implementaciones calculan el mismo HMAC no prueba el endpoint:
  la canary debe cubrir normalización de headers, nonce, persistencia, worker y
  recepción física.
- Cambiar variables en Coolify no modifica el entorno de un contenedor existente;
  un restart conserva el entorno original. Los servicios que leen variables de
  proceso requieren recreación/redeploy, o un mecanismo explícito que regenere
  su `.env` y recargue el runtime.
- Si systemd y WP-Cron consumen la misma outbox, `SKIP LOCKED` protege una fila,
  pero no impide dos envíos distintos simultáneos. Un lock asesor global serializa
  el cliente externo compartido sin depender del scheduler que ganó la carrera.
