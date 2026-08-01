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
