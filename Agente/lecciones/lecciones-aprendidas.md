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

## 018A-18 — Una sola autoridad de sesión

- Cuando la cookie opaca ya cubre login, CSRF, revocación y capacidades, conservar un fallback Bearer solo amplía la superficie de ataque y hace ambiguo el contrato; debe retirarse junto con su secreto y dependencia.
- La regresión mínima debe enviar un Bearer legacy al router de producción y comprobar `401`, además de mantener los casos de cookie/CSRF existentes.
- El retiro de JWT no autoriza a eliminar `/uploads`: los descargables privados y las imágenes públicas necesitan primero un contrato de asset autorizado y una migración de URLs.

## 018A-19 — El contrato generado debe reflejar la autoridad real

- Retirar JWT del runtime no basta: Swagger/utoipa y los clientes generados pueden seguir publicando Bearer como si fuera válido.
- La seguridad de sesión se documenta como `ApiKey::Cookie("session_id")`; CSRF queda explícito como header de mutación, sin inventar una segunda autoridad.

## 018A-20 — Las rutas OpenAPI deben probarse contra el router real

- Una anotación utoipa puede compilar aunque apunte a una ruta pública; comparar el path anotado con `.route()` evita que el cliente generado omita el prefijo `/admin`.
- Los campos `serde_json::Value` de DTOs expuestos necesitan `#[schema(value_type = Object)]`; de lo contrario Orval falla con referencias `JsonValue` inexistentes.

## 018A-21 — Los enums anidados también son parte del contrato

- Al añadir un request con un enum de actualización (`ProjectUrlUpdate`), incluir el enum en `components(schemas(...))`; compilar Rust no garantiza que Orval encuentre todas las referencias.

## 018A-22 — Las respuestas de terceros necesitan DTO propio

- Checkout no debe publicar `serde_json::Value` como contrato: un DTO estable conserva la forma pública aunque Stripe agregue campos internos.
- El precio, la disponibilidad y la entrega siguen siendo decisiones server-side; tipar la respuesta no autoriza al navegador a conceder acceso.

## 018A-23 — Agrupar endpoints por dominio reduce drift

- Notificaciones, analytics y settings deben aparecer en el mismo contrato que sus servicios frontend; dejar uno fuera obliga a reintroducir `fetch` y tipos manuales.
- Las respuestas públicas pueden documentarse sin exponer metadata privada; la autorización sigue en `AuthUser`/`AdminUser`, no en el schema.

## 018A-24 — Revisar prefijos al anotar rutas anidadas

- `ApiDoc` se sirve bajo `/api`; las anotaciones admin de workspace sin ese prefijo producían URLs documentadas imposibles aunque Axum respondiera correctamente.
- Las sesiones listadas pueden exponerse como DTO serializable sin tokens; el esquema debe mostrar solo metadata operativa.

## 018A-25 — Documentar multipart sin delegar confianza al cliente

- Media puede publicar filtros, estados y respuesta de upload aunque el cuerpo multipart permanezca en el adaptador manual; el tipo/extensión siempre los decide el backend.
- Papelera y restore deben conservar operaciones separadas en OpenAPI para que una app futura no confunda soft delete con borrado permanente.

## 018A-26 — Las apps internas no necesitan rutas legacy

- Una app administrativa debe registrarse una sola vez en `AppRegistry`; conservar una ruta de página sin ventana crea un segundo punto de entrada y permite que el shell pierda capacidades, foco y analítica.
- Los comandos de toolbar que crean contenido deben declarar `adminOnly` y abrir el editor por `openAppWindow`; navegar a `/admin` acopla una acción concreta a un panel monolítico.
- Retirar la ruta no implica borrar el módulo que renderiza la app: el contenido puede seguir siendo reutilizable mientras la presentación y la autorización viven en el runtime.

## 018A-27 — Documentar los límites de integraciones server-side

- Descargas privadas y webhooks también son parte del contrato: documentar el grant, headers y estados evita que el cliente invente una ruta pública o una autorización alternativa.
- Un endpoint de descarga binaria puede describirse sin registrar storage keys ni modelar el token como credencial reutilizable; OpenAPI debe mostrar solo el boundary observable.
- Los webhooks externos usan cuerpo crudo y firma en header; su documentación no debe generar un cliente de usuario ni sustituir la verificación HMAC del backend.

## 018A-28 — El storage privado no debe ser una ruta pública

- Un filtro SQL en el listado no protege un archivo si `ServeDir` permite adivinar su nombre; la autorización debe repetirse en el handler que abre los bytes.
- El mismo confinamiento canónico de path sirve para media y descargas, pero la decisión de visibilidad debe vivir en el envelope (`active/public/clean`) y no en el navegador.
- Mantener temporalmente el nombre `file_path` como URL de preview permite migrar consumidores sin filtrar la storage key; el contrato DTO público/admin separado debe ser el cleanup siguiente.

## 018A-29 — Separar storage y contrato HTTP

- Un modelo que contiene la storage key no debe ser la respuesta de un handler: aunque se reescriba el valor antes de serializar, el contrato sigue siendo ambiguo y puede filtrar campos privados en una ruta futura.
- DTOs explícitos (`public`, `admin`, `upload`) permiten que cada boundary declare sus capacidades y que OpenAPI/TypeScript detecten regresiones de nombres como `file_path`.

## 018A-30 — El roadmap también es un contrato

- Cuando una implementación cambia un shape o el orden de fases, las referencias históricas activas deben actualizarse en la misma tarea; una línea obsoleta puede hacer que el siguiente agente reintroduzca un contrato retirado.

## 018A-31 — Validar CSS dinámico antes de eliminarlo

- Un selector reportado como huérfano puede construirse desde una cadena o plantilla en TypeScript; antes de borrarlo hay que buscar consumidores dinámicos y conservarlos si forman parte del runtime.

## 018A-32 — Generación y autenticación deben compartir boundary

- Generar funciones `fetch` no las hace seguras automáticamente: el mutator debe centralizar cookie, CSRF, base URL y envelope de errores antes de migrar un servicio.
- Los clientes generados ignorados son reproducibles solo si CI ejecuta codegen antes del type-check; el workflow debe validar esa dependencia explícitamente.

## 018A-33 — Adaptar contratos en el boundary, no en cada consumidor

- Migrar servicios completos al cliente generado evita que editores conozcan rutas HTTP. Cuando el contrato usa un parche semántico (`ProjectUrlUpdate`), la conversión debe vivir en el servicio y conservar omitir/limpiar/reemplazar.
- Los errores de catálogo no deben convertirse en `null` silenciosamente: si la API falla, el servicio propaga el resultado; `null` queda reservado para una respuesta exitosa sin elementos.

## 018A-34 — Migrar transporte sin perder efectos de dominio

- Un servicio de auth no es solo HTTP: la limpieza de clipboard/preferencias y la actualización de `authStore` deben permanecer fuera del mutator, después de validar el estado generado.
- Los headers de consentimiento y de seguridad son parte del contrato del servicio; al migrar a Orval se pasan como `RequestInit` y no se duplican en el cliente generado.

## 018A-35 — Los modelos ricos deben adaptarse en un único boundary

- Cuando OpenAPI expresa árboles u overlays como mapas genéricos, la conversión debe quedar en funciones nombradas del servicio. Así el runtime conserva invariantes (`version`, `nodes`, tipos de nodo) y el cliente generado conserva el contrato HTTP sin duplicación.

## 018A-36 — Retirar una abstracción solo después de cerrar consumidores

- La eliminación segura del cliente manual se confirma con búsqueda estática, type-check y tests del mutator; conservar `ApiError` evita romper boundaries de sincronización que no son transporte.

## 018A-37 — La selección incremental necesita dependencias, no solo nombres cambiados

- Ejecutar únicamente los tests modificados deja sin cobertura los tests que importan un módulo fuente cambiado; un grafo local de imports ofrece selección rápida sin convertir cada cambio en suite completa.
- En Windows, `rename`/`unlink` concurrentes pueden devolver `EPERM` aunque otro escritor esté progresando; el reemplazo atómico debe reintentar ambos pasos con límite y paths exactos.

## 018A-38 — Componer contratos sin romper consumidores

- Un DTO grande puede dividirse con `extends` manteniendo el mismo nombre exportado; así se mejora ISP y Sentinel sin introducir mapeos, cambios de serialización ni duplicación de tipos.

## 018A-39 — Toda acción declarada necesita un ejecutor

- Una matriz de recursos puede aparentar paridad aunque solo enumere acciones: cada acción visible debe resolver target, declarar capacidad y abrir/ejecutar una única ruta del runtime.
- Las propiedades pueden empezar como una lectura local segura; no se debe inventar un endpoint ni mostrar `refId` interno hasta que exista un contrato público y una decisión de privacidad.

## 018A-40 — Un warning de clase huérfana exige búsqueda dinámica

- Antes de borrar una utilidad CSS hay que buscarla en TypeScript, HTML y plantillas; nombres interpolados (`badge--${estado}`) no aparecen como literal completo y deben conservarse con evidencia.
- La limpieza incremental de utilidades sin consumidores reduce la deuda sin convertir los falsos positivos de VarSense en cambios visuales riesgosos.

## 018A-41 — Separar catálogos sin duplicar el registry

- Un catálogo de apps puede dividirse por capacidad/dominio mediante módulos de registro con efectos laterales; el entrypoint debe importar cada módulo una sola vez y conservar AppRegistry como única fuente.
- La división estructural es preferible a una suppression de límite: mantiene rutas, lazy loading y teardown intactos, pero evita que nuevas apps vuelvan a inflar el coordinador.

## 018A-42 — El webhook no debe ser el worker de entrega

- Un webhook debe confirmar rápido la autoridad del pago y encolar un evento; una llamada externa lenta o fallida dentro del request puede dejar el evento marcado sin una entrega recuperable.
- La rotación del grant debe actualizar solo el hash persistido y devolver el token raw únicamente al adaptador de correo; así el reintento genera un enlace nuevo sin convertir la cola en un almacén de credenciales.

## 018A-43 — Separar gate mínimo de backlog de tooling

- Un roadmap de calidad puede mantener una visión amplia sin convertir cada regla futura, benchmark o paridad de adapters en una dependencia del producto.
- La fuente canónica debe declarar explícitamente qué checklist desbloquea el trabajo y qué backlog queda diferido; así el agente ejecuta el gate reproducible sin inflar el contexto ni iniciar migraciones upstream innecesarias.

## 018A-44 — Retirar nombres legacy después de extraer la responsabilidad

- Cuando un módulo deja de contener la responsabilidad que dio origen a su nombre, conservarlo como alias perpetúa una arquitectura equivocada y hace que futuras apps vuelvan a depender del boundary antiguo.
- Renombrar el adaptador manteniendo la app y su contrato permite limpiar la deuda sin borrar la compatibilidad funcional ni reintroducir lógica en el shell.

## 018A-45 — Un token huérfano se elimina solo con doble evidencia

- VarSense identifica candidatos, pero la eliminación segura exige una búsqueda global que confirme que el nombre no aparece como consumidor ni en contratos dinámicos.
- Mantener los tokens que sí tienen consumidores, aunque parezcan legacy, evita que una limpieza visual rompa preferencias de perfil, tema o geometría del OS.

## 018A-46 — El modelo SQL no es un DTO público

- Aunque el frontend omita campos, serializar directamente el modelo interno deja el contrato vulnerable a futuras rutas o consumidores que sí los acepten.
- Un DTO por boundary permite que el modelo conserve datos necesarios para checkout/webhook sin filtrar rutas de storage ni identificadores de proveedores a catálogo o artículos.

## 018A-47 — Un endpoint público de settings también necesita contrato

- Devolver un mapa completo de configuración convierte cada clave futura en una exposición pública accidental; la allowlist debe vivir en el repository y crecer solo mediante revisión explícita.
- Nombrar el cliente como `getPublic` mantiene la frontera visible también en el frontend y evita que una futura pantalla confunda configuración pública con secretos o flags administrativos.

## 018A-48 — Los metadatos de orden también son internos

- Un endpoint público puede filtrar correctamente los registros y aun así revelar cómo se organiza el escritorio si serializa el modelo SQL completo; orden y visibilidad deben pertenecer al DTO administrativo.
- Cuando el backend ya filtra/ordena, el frontend público debe renderizar el resultado directamente. Mantener un segundo filtro en el navegador crea dependencia accidental del contrato interno y facilita que vuelva a filtrarse de forma inconsistente.

## 018A-49 — Una lista compartida puede necesitar tres DTOs

- Las notificaciones parecen una lista única, pero `read` depende de la cuenta y `status`/`created_by` dependen de admin; reutilizar el modelo SQL en los tres endpoints mezcla capacidades y expone metadata.
- Separar las listas por boundary permite que el servicio conserve una sola consulta/repositorio, mientras cada handler decide exactamente qué campos puede devolver.

## 018A-50 — Cerrar lo automatizable sin ocultar deuda visual

- Una matriz de paridad puede cerrarse técnicamente aunque queden CSS huérfanos o fachadas manuales que necesitan revisión visual; conviene separarlos como backlog no bloqueante en vez de falsear el criterio de salida.
- El roadmap debe habilitar el siguiente epic solo cuando sus dependencias de contratos estén cerradas y dejar las migraciones de alto riesgo con criterio explícito de reanudación.

## 018A-51 — Suite completa en CI, alcance incremental en local

- Un gate local que ejecuta toda la suite en cada tarea degrada el equipo a medida que crecen los tests; el modo incremental debe seguir siendo la ruta rápida.
- La cobertura completa no debe desaparecer: se activa con una señal explícita de CI, comparte el mismo reporte y falla el gate si cualquier etapa devuelve error.

## 018A-52 — El caché debe incluir el nivel de evidencia

- Dos ejecuciones con los mismos archivos no tienen la misma evidencia si una ejecuta solo type-check y otra ejecuta la suite completa; el modo de validación forma parte del fingerprint.
- Al cambiar el contrato del caché hay que incrementar su versión para invalidar resultados antiguos en vez de asumir que describen el nuevo gate.

## 018A-53 — Un budget útil debe ejecutarse donde existe el artefacto

- Los límites de rendimiento solo son verificables sobre el build final; medir fuentes o cargar `test:full` en cada ciclo local no protege al producto y degrada el equipo.
- Separar CI de local permite exigir build + gzip en integración sin convertir cada tarea en un proceso pesado. El límite vive en configuración y el reporte indica exactamente el asset y bytes que exceden.

## 018A-54 — Documentar recuperación antes de necesitarla

- Un runbook útil debe definir señales de salida, límites y orden de rollback, pero no debe fingir que una operación de producción fue probada cuando solo se revisó el procedimiento.
- Mantener Coolify Manager como único canal evita que una urgencia reintroduzca SSH y deja cualquier hueco como mejora explícita de la herramienta.

## 018A-57 — Una UI de registro no debe habilitar registro

- El formulario puede vivir dentro de Cuenta y reutilizar el servicio generado, pero la autoridad para crear sesiones sigue en el flag server-side y la verificación de correo.
- Las respuestas de recuperación deben conservar el mensaje no enumerable; tokens de verificación/reset requieren un contrato de URL separado antes de entrar al cliente.

## 018A-59 — Buckets con ventanas distintas necesitan almacenes distintos

- Reutilizar un `HashMap` y limpiar todas sus entradas con la ventana del login hace que una llamada de login pueda borrar prematuramente el contador de recuperación.
- Separar los buckets conserva ventanas independientes y deja claro qué parte es protección local del proceso frente a un futuro limitador distribuido.

## 018A-60 — Auditar sin convertir la auditoría en un almacén de secretos

- Los eventos de auth pueden registrar tipo, éxito, usuario e IP hasheada sin copiar email, contraseña, sesión ni token; el servicio de tokens sigue siendo la única frontera que maneja el secreto crudo.
- Registrar también los fallos de consumo permite detectar replay/abuso, pero el fallo de la propia auditoría debe propagarse para no presentar una acción sensible como completada sin evidencia.

## 018A-61 — Una acción del shell debe tener un solo dueño

- Si taskbar, móvil y titlebar mutan stores directamente, la misma acción puede quedar sin analítica, sin disponibilidad uniforme o con teardown distinto. El contrato debe vivir en `CommandRegistry` y las superficies solo proyectarlo.
- El reencuadre por resize debe ser batch: una sola escritura al store evita N persistencias y mantiene el historial de rutas estable; los cambios ambientales usan `source='sync'`.

## 018A-62/63/64 — ID de tarea y gate: dos lecciones de proceso

- **El ID de tarea se asigna desde git log, no desde memoria.** Se usó `018A-14/15` creyéndolos siguientes cuando la secuencia real ya llegaba a `018A-61`; hubo que corregir comentarios en migración, servicio y handler. Antes de escribir un `[ID]`, comprobar el máximo usado: `git log --oneline | Select-String '018A-(\d+)' | % { [int]$Matches[1] } | Measure-Object -Maximum`.
- **El quality gate exige el ID en `roadmap.md`, `Agente/planes/` o `Agente/completados/`** (`preflight.mjs`). Si la tarea aún no figura, `npm run task:check -- {ID}` falla con "no existe". Registrar la tarea en el roadmap (pendiente) → gate → archivar en completados → quitar del roadmap. El roadmap debe volver a quedar idéntico a HEAD si las tareas se cierran en el mismo bloque.
- **`ON CONFLICT` contra índice parcial exige repetir el predicado `WHERE`** en el arbiter (42P10): `ON CONFLICT (col) WHERE col IS NOT NULL DO NOTHING`. Un índice UNIQUE parcial no matchea un `ON CONFLICT (col)` sin predicado.
- **Desajuste utoipa ↔ cliente generado ↔ servicio manual** causa fallos silenciosos del frontend pese a HTTP correcto (login 204 mostrado como "credenciales incorrectas"). Alinear el contrato y aceptar el status real en `unwrapGeneratedResponse`; regenerar el cliente después.

## 018A-65 — Especificidad de superficie rompe el flex de los botones

- La regla `.desktop-window .boton { display: inline-block }` (0,2,0) sobreescribe cualquier `display: flex/inline-flex` puesto en una clase `.boton` (0,1,0): icono+texto quedan inline con alineación por baseline (SVG arriba, texto abajo).
- Solución reutilizable: receta compartida `.boton-con-icono` definida con los mismos selectores de superficie y DESPUÉS en el archivo, para ganar por orden de fuente y recuperar `display: inline-flex; align-items: center; gap`.
- Antes de escribir `display`/`align-items`/`gap` en un componente `.boton`, comprobar que no lo anula una regla de superficie; si lo anula, la capacidad debe vivir en la receta del sistema, no duplicada en el componente.
