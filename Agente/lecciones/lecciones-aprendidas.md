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

## 2026-07-29 — Documentación canónica y planes ejecutables
- Mezclar visión, arquitectura, identidad, auditoría y tareas en varios planes crea fuentes de verdad competidoras aunque cada documento sea correcto por separado.
- El roadmap debe responder únicamente qué sigue y qué lo bloquea; los contratos viven en manuales y la secuencia detallada en un solo plan maestro.
- Un plan ejecutable necesita checklists, dependencias, gates y criterio de salida por bloque; una lista narrativa de fases no basta para trabajar una por una.
- Los estados visuales, editoriales, comerciales y de papelera deben documentarse como ejes distintos antes de diseñar API o UI; un `status` único produce contradicciones y fugas.
- Archivar planes históricos evita que decisiones superadas —Admin monolítico, uploads públicos, fuentes pixel o productos ligados a artículos— reaparezcan durante la implementación.

## 297A-6 — Un gate full debe incluir sus inputs y excluir sus artefactos

Un analizador instalado dentro del workspace puede terminar analizándose a sí mismo si sus carpetas de herramientas/reportes no están excluidas. A la vez, un cache full no es seguro si solo hashea el diff visible: debe depender de todo el árbol versionado, configs y versiones fijadas. Ambas condiciones se validan antes de aceptar un PASS cacheado.

## 297A-24 — El chrome del shell no es una ruta runtime

`Perfil` se registra como `shell-profile` en `windowStore`, no en `AppRegistry`, y por diseño no tiene URL pública. El sincronizador de URL debe proyectar únicamente apps runtime; si enfocar una entrada shell sin ruta fuerza `/`, la reconciliación puede interpretar una acción visual como navegación fuera del OS y cerrar todas las apps. La guardia debe considerar la superficie activa y cualquier app runtime abierta, mientras que el cierre masivo queda reservado a una navegación documental explícita. ## 297A-14 — El 404 silencioso de la sintaxis de rutas

- axum 0.7.9 documenta `{id}`, pero el parámetro real lo decide matchit resuelto por `Cargo.lock`: este proyecto tiene matchit 0.7.3, que parsea `:param` (estilo axum 0.6). `{id}` se registra como segmento literal y devuelve 404 sin error de compilación ni warning.
- Un contrato de rutas nunca debe asumirse por la doc del framework: verificar empíricamente la sintaxis con un router mínimo + `oneshot` contra el build real y leer el README/parser de la versión exacta de matchit en el lock.
- `utoipa::path` conserva `{id}` (templating OpenAPI) y convive con `:param` en el routing; no «corregirlo».
- Cuando un test HTTP devuelve 404 donde el contrato exige 401/403, sospechar de la ruta antes del middleware: el router de producción y un router mínimo deben coincidir.
- Regla Sentinel candidata: detectar `{` en strings de `.route()`.

## 018A-4 — Suite selectiva segura y procesos acotados

- Un selector incremental no debe inferir dependencias desde cualquier `--changed HEAD`: solo tests modificados pueden ejecutarse selectivamente; código, configuración, borrados, renombres y untracked requieren suite completa.
- El contrato de suite completa debe permanecer explícito (`test`/`test:full`), mientras el modo local selectivo se ofrece como comando separado para no convertir un PASS parcial en una garantía global.
- Limitar workers y captura de salida evita que varios agentes saturen CPU/memoria; el gate debe fallar rápido ante locks duplicados y dejar el detalle en artifacts, no en stdout/contexto.

## 018A-5 — Commit condicional y migración de reglas

- El quality gate no debe ordenar commit a ciegas: diagnósticos, bloques intermedios y trabajo compartido pueden documentarse sin commit; el recordatorio debe indicar commit/push solo cuando el bloque sea entregable.
- Antes de mover una regla del proyecto al core, conservar el bridge durante una fase, añadir fixture y filtrar el duplicado en el adapter; así se puede comparar sin duplicar ruido al usuario.
- Un comando combinado (`all`) es más seguro que dos procesos si comparte provider y snapshot; cambiar el contrato requiere mantener `scan` y `orphan-classes` para no romper consumidores existentes.

## 018A-6 — Gate mínimo antes del roadmap de producto

- Una herramienta de calidad puede seguir mejorando indefinidamente; para no bloquear el producto hay que separar explícitamente el gate mínimo reproducible del backlog de benchmarks, paridad y releases.
- Si el gate mínimo pasa y no hay errores de infraestructura, las mejoras diferidas solo se reactivan cuando una tarea concreta las necesita o aparece una regresión medible.

## 317A-5 — Restaurar antes del router sin cerrar la raíz

- La restauración de ventanas debe ocurrir antes de inicializar el router para conservar foco y deep links, pero la primera reconciliación de `/` no puede interpretar el escritorio como navegación documental y cerrar el estado restaurado.
- Una opción explícita de inicialización (`preserveRootOnInit`) mantiene esa excepción solo una vez; las navegaciones posteriores siguen limpiando el runtime cuando corresponde.
- La evidencia mínima útil combina navegador real en desktop/tablet y móvil con una suite completa y un gate único; no basta con tests unitarios del serializador.

## 018A-8 — Instrumentar foco desde una sola frontera

- Emitir eventos de foco desde cada botón o comando crea duplicados y deja fuera los clicks directos del shell; el store/sincronizador de foco es la frontera única.
- Las rutas protegidas deben medirse solo después de validar capacidad y parámetros, evitando que la analítica revele la existencia de recursos privados.

## 018A-9 — Reintentos seguros de comercio y analytics

- La idempotencia debe existir en dos fronteras: la orden local y el proveedor de pago; una sola no evita cobros o grants duplicados.
- Un webhook repetido no debe depender de memoria: `provider_event_id`, entitlement por orden y outbox con `dedupe_key` permiten reanudar sin duplicar efectos.
- Los enlaces de descarga se envían en claro solo una vez; la base conserva únicamente el hash y el endpoint vuelve a comprobar expiración y confinamiento de path.
- Un batch de analytics necesita `event_id` antes de reintentar; de lo contrario una caída de red infla las métricas aunque el inserto sea multi-fila.
- La auditoría de login debe hashear IP y omitir email/credenciales; registrarla después de validar la entrada evita convertir el log en una fuente de secretos.

## 018A-11 — Notificaciones derivadas del release

- Una novedad básica puede derivarse del `version` público del workspace sin crear una segunda entidad de publicación.
- El estado leído local debe usar un ID estable y una lista acotada; la sincronización por cuenta se deja para cuando exista registro verificado y overlay remoto.
- La campana solo despacha la apertura de la app; mantener fuente y presentación separadas evita duplicar lógica en desktop y móvil.

## 018A-12 — Consentimiento debe existir en dos fronteras

- Bloquear el tracker en el navegador no basta: el backend también debe rechazar lotes sin un header explícito de consentimiento.
- IP y user-agent se anonimizan en el boundary antes del repository; una migración de privacidad no debe intentar restaurar datos que fueron eliminados.
- Una purga parametrizada y acotada permite operar retención sin SQL manual ni intervalos interpolados.

## 018A-13 — Notificaciones persistentes sin duplicar publicación

- El release y su aviso deben confirmarse en la misma transacción para no mostrar una novedad de un escritorio que no llegó a publicarse.
- `notification_reads` es un overlay por usuario; los avisos públicos siguen siendo una lista server-side y el navegador solo conserva fallback offline.
- Un índice único parcial por release evita spam incluso si el endpoint de publicación se reintenta.
- El panel admin debe reutilizar la app pública y sus servicios; añadir otra ventana de publicación crea dos fuentes de estado.

## 018A-14 — Comercio como apps sin duplicar checkout

- Tienda debe consumir el mismo `ProductService` que los artículos; el frontend no debe inventar precios ni decidir disponibilidad.
- Pedidos y Descargas pueden existir como programas desde el principio con estados vacíos honestos; no se debe simular historial antes de tener un endpoint autorizado.
- La migración del release público debe ser aditiva para conservar posiciones que el admin ya haya publicado.

## 018A-16 — Registro verificado y recuperación sin enumeración

- El correo no se considera verificado por tener contraseña: `email_verified_at` y tokens opacos de un solo uso deben vivir en la base y consumirse atómicamente.
- Recuperación responde igual exista o no el email; el token se persiste solo como hash, expira pronto y revocar sesiones después del cambio evita reutilización de una sesión robada.
- Mantener `registration_enabled=false` permite desplegar contratos y migraciones sin abrir el alta pública antes de tener correo real, UI y pruebas E2E.

## 018A-17 — OpenAPI regenerable sin servidor

- Un comando de codegen no debe exigir una base de datos ni dejar un servidor vivo: `--emit-openapi` puede serializar `ApiDoc` antes del bootstrap de configuración/pool.
- Los alias de `serde_json::Value` y tipos plenamente calificados en atributos utoipa producen referencias OpenAPI inválidas; los campos dinámicos deben declarar `value_type` y las respuestas usar nombres de esquema estables.
- En Windows, invocar `npm.cmd` con `spawnSync` puede devolver `EINVAL`; ejecutar el binario Orval con `process.execPath` evita shell, quoting y advertencias de seguridad.
