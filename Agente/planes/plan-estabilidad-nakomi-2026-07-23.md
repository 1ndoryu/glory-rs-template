# 237A-3 — Estabilidad integral de Nakomi Studio

> **Fecha:** 2026-07-23  
> **Estado:** Watchdog y persistencia desplegados; alertas externas, Realtime y finanzas pendientes
> **Prioridad:** Crítica  
> **Responsable técnico:** agente principal  
> **Delegación:** las tareas mecánicas y de bajo riesgo se asignan a un subagente con criterios de aceptación explícitos.  
> **Objetivo inmediato:** impedir que el runtime vuelva a cerrarse por un falso freeze y recuperar un chat persistente, trazable y realmente realtime.

## 1. Decisiones confirmadas por el usuario

1. `1ndoryu/glory-rs-template` es el repositorio contenedor:
   - `main` debe ser un template genérico vacío de lógica de proyectos.
   - Las demás ramas son proyectos concretos, por ejemplo `glory-rust-nakomi`, `glory-rs-rest` y `kamples`.
2. `1ndoryu/glory-rs-framework` es el núcleo compartido entre templates.
3. Los repositorios no deben fusionarse ni intercambiar responsabilidades.
4. La restauración de `glory-rs-template/main` debe planificarse, pero se ejecutará después de estabilizar Nakomi.
5. El agente principal trabaja las partes arquitectónicas y difíciles.
6. Un subagente ejecuta únicamente tareas fáciles o mecánicas siguiendo este plan.
7. Los mensajes no deben desaparecer con el tiempo.
8. La prioridad máxima es que el freeze deje de ocurrir.

## 2. Hallazgos confirmados

### 2.1 Topología Git y causa del deploy desactualizado

- El remoto raíz `origin` apunta correctamente a `glory-rs-template`.
- El remoto raíz llamado `framework` apunta a un repositorio legado `glory-rs.git`; no es `glory-rs-framework`.
- La rama local `glory-rust-nakomi` rastrea por error `framework/glory-rust-nakomi`.
- El HEAD local inicial era `3f6d0c47`.
- `origin/glory-rust-nakomi` estaba en `5ff27edf`, siete commits por detrás del HEAD local inicial.
- Coolify clona correctamente `glory-rs-template/glory-rust-nakomi`; por tanto reconstruye frontend y backend desde la rama de proyecto desactualizada.
- El núcleo real está en `./glory-rs`, como repositorio Git anidado ignorado, y su `origin` sí apunta a `glory-rs-framework`.
- Local y producción no fijan el mismo commit del framework:
  - local usa una rama feature;
  - el Dockerfile usa `master` por defecto;
  - la configuración declara `main`, pero el argumento no queda aplicado de forma inequívoca.

### 2.2 Freeze y watchdog

- `rt_heartbeat` inicia en `0`.
- La tarea que produce el pulso espera antes del primer heartbeat.
- El watchdog interpreta `0` como timestamp Unix válido y calcula una inactividad de décadas.
- Ejecuta `exit(1)` aunque nunca haya existido un pulso válido.
- El mismo patrón ya se registró el 10 de junio de 2026.
- La caída local del 23 de julio fue provocada por esta decisión inválida del watchdog.
- Todavía debe investigarse por qué el pulser no produjo el primer pulso antes del límite, pero esa investigación no justifica matar el proceso con `last_pulse=0`.

### 2.3 Sonido duplicado y chat

- Al reconectar, el backend reenvía historial como si fueran eventos normales.
- El frontend deduplica visualmente por ID, pero reproduce el sonido fuera de la decisión de inserción.
- Varias pestañas reciben eventos por WebSocket y por `BroadcastChannel`.
- Un mensaje histórico puede volver a sonar sin aparecer de nuevo.
- El sonido es una señal útil de replay/reconexión, pero no una causa demostrada del freeze.

### 2.4 Identidad visitante corrupta

- Frontend acepta cualquier valor truthy de `localStorage`, incluido el literal `undefined`.
- Backend acepta `visitor_id` como string sin validar UUID ni valores reservados.
- El valor queda persistido y puede mezclar sesiones, perfiles y rate limits.

### 2.5 Mensajes que “se borran”

No existe un TTL que borre periódicamente `chat_messages`, pero hay dos defectos que hacen desaparecer el historial de la UI:

1. Un job cierra todas las sesiones tras 24 horas de inactividad.
2. Todas las consultas visibles excluyen sesiones `closed`.

Esto también afecta chats de pedidos. Al buscar de nuevo una sesión cerrada por orden, el sistema puede crear otra sesión y fragmentar el historial.

Además:

- La consulta de mensajes usa `ORDER BY created_at ASC LIMIT`.
- Cuando una conversación supera 100 mensajes, el frontend recibe los mensajes más antiguos y deja de mostrar los nuevos.
- La reconexión del widget restaura los 50 mensajes más antiguos.
- `/reset` sí elimina físicamente los mensajes; debe restringirse a sesiones anónimas o cambiarse por archivado seguro.

### 2.6 Chat de pedidos y notificaciones

- `useOrderChat` abre el WebSocket visitor genérico.
- El backend ignora el contexto de orden para resolver la sesión WS.
- El realtime se conecta a una conversación diferente y el flujo depende de polling REST cada cinco segundos.
- El badge de Mensajes no está implementado en el sidebar.
- `ChatBell` calcula unread, pero no se monta.
- El provider WebSocket de notificaciones tampoco se monta.
- Fuera del panel no hay campana ni indicador.
- El aviso de 20 minutos solo crea notificaciones in-app, puede repetirse cada cinco minutos y usa un deeplink incorrecto.
- No existe integración WhatsApp.

### 2.7 Pagos y reembolsos

- HEAD crea cuenta y orden después del webhook, pero producción no contiene esos commits.
- La persistencia post-pago no es transaccional.
- La idempotencia del webhook usa check-then-process y puede duplicar efectos bajo concurrencia.
- El monto de Stripe se ignora y se recalcula desde el CMS.
- El reembolso marca `approved` antes de ejecutar Stripe.
- Si Stripe falla, el caso puede quedar varado.
- Si falta la clave Stripe, el flujo puede simular éxito.
- Orden, pago y reembolso se actualizan sin una transacción común.
- El cliente ya dispone en HEAD de un modal correcto sin monto.
- Persiste un `prompt()` administrativo conceptualmente incorrecto.

### 2.8 Estado del trabajo anterior

- El commit `acebc117` incorporó piezas útiles, pero declaró completas tareas rotas, parciales o inexistentes.
- HEAD no pasa actualmente el type-check:
  - `useOrderChat` usa un campo inexistente de `AuthUser`;
  - `ModalAsignar` recibe un prop que no consume.
- La centralización de plantillas email y la alineación izquierda de notificaciones sí están correctamente implementadas.

## 3. Arquitectura objetivo

### 3.1 Repositorios y ramas

```text
glory-rs-template
├── main                    template genérico, sin lógica de proyecto
├── glory-rust-nakomi       Nakomi Studio
├── glory-rs-rest           proyecto Rest
└── kamples                 proyecto Kamples

glory-rs-framework
├── main                    núcleo estable compartido
├── tags/releases           versiones reproducibles
└── feature/*               desarrollo del framework
```

Reglas:

- Ninguna rama de proyecto se fusiona hacia `glory-rs-template/main`.
- Ninguna lógica Nakomi entra en `glory-rs-framework`.
- Cada rama proyecto debe fijar una revisión concreta del framework.
- Local, CI y Coolify deben compilar el mismo SHA del proyecto y el mismo SHA del framework.
- El deploy aborta si cualquiera de los dos SHAs no coincide con lo esperado.

### 3.2 Chat

```text
Evento persistido
    ├── transacción BD
    ├── secuencia/ID único
    ├── publicación realtime
    ├── unread persistente
    └── outbox para email/WhatsApp
```

- PostgreSQL es la fuente de verdad.
- El WebSocket transporta eventos, no crea un historial paralelo.
- Los chats de pedidos se conservan indefinidamente.
- Los chats anónimos pueden cerrarse por inactividad, pero continúan visibles en un archivo.
- El cliente carga los últimos mensajes y pagina hacia atrás con cursor.
- El sonido solo se reproduce si un evento live nuevo fue insertado en la vista.

### 3.3 Pagos

```text
Stripe webhook
    ├── inbox idempotente
    ├── transacción local
    │   ├── usuario
    │   ├── orden
    │   ├── fases
    │   ├── pago
    │   ├── asignación
    │   └── outbox
    └── efectos externos posteriores al commit
```

- Ninguna cuenta ni orden antes del cobro confirmado.
- Un PaymentIntent solo puede producir una orden.
- El monto/currency del evento debe coincidir con el snapshot del checkout.

### 3.4 Reembolso

- Cliente: envía únicamente motivo.
- Backend: calcula todos los pagos reembolsables.
- Admin: revisa, conversa, aprueba o rechaza.
- Freelancer/empleado: no decide.
- Aprobación y ejecución Stripe son estados separados.
- Cada operación Stripe usa una clave idempotente estable.
- La orden solo se cancela cuando todos los ítems financieros se completan.
- Política inicial recomendada: devolución de todo lo efectivamente pagado al método original.
- Wallet queda bloqueado hasta decidir expresamente si sustituye o complementa Stripe.

## 4. Fases de ejecución

## Fase A — 237A-3 — Plan, evidencia y salvaguardas

**Responsable:** agente principal  
**Estado:** En progreso

Tareas:

- Crear este plan.
- Preservar el cambio existente en `roadmap.md`.
- Consolidar incidentes y planes anteriores en una única documentación operativa.
- Registrar SHAs actuales de proyecto y framework.
- No cambiar de rama ni resetear el worktree actual.
- No desplegar mientras HEAD no compile y la procedencia no sea verificable.

Criterios de aceptación:

- Existe un único plan vigente.
- Freeze, deploy, chat, retención, notificaciones, pagos y reembolsos tienen causa/estado explícitos.
- Cada fase tiene propietario y pruebas.

## Fase B — 237A-4 — Watchdog y observabilidad del runtime

**Responsable:** agente principal  
**Dificultad:** Alta  
**Prioridad:** Primera implementación
**Estado:** Implementado y validado localmente; deploy pendiente

Resultado 2026-07-23:

- Watchdog agnóstico implementado en `glory-rs-framework`, rama `codex/237A-runtime-watchdog`, commit `fce94c1`.
- Integración del proyecto en commit `3ac24da9`.
- Eliminados los dos watchdog legacy basados en epoch que coexistían en `main.rs`.
- Framework: 15 tests aprobados; proyecto: `cargo check`, `clippy -D warnings` y 208 tests aprobados.

Diseño:

1. Extraer la decisión del watchdog a una unidad testeable.
2. Introducir un estado de readiness explícito:
   - `Starting`;
   - `Ready(last_pulse)`;
   - `Stale(duration)`.
3. `last_pulse=0` o `Starting` nunca mata el proceso.
4. Sustituir duraciones basadas en epoch por tiempo monotónico.
5. El pulser debe publicar readiness al iniciar, sin sleeps independientes usados como sincronización.
6. Separar:
   - fallo de readiness;
   - pulso válido que se estancó;
   - error del propio watchdog.
7. En Windows no intentar `/proc/self/task`; registrar diagnóstico compatible con la plataforma.
8. Conservar restart automático solo para un pulso previamente válido que exceda el umbral.

Pruebas:

- `Starting` durante más del timeout no llama `exit`.
- Primer pulso establece baseline.
- Pulso válido realmente stale sí solicita recovery.
- Reloj regresivo no dispara recovery.
- Suspensión/reanudación no se interpreta como freeze sin evidencia adicional.
- Prueba local de más de 130 segundos con:
  - idle;
  - chat;
  - apertura de reembolso;
  - dos pestañas.

Observabilidad:

- Motivo estructurado de cada transición.
- Edad monotónica del último pulso.
- Estado del pool DB.
- sesiones WS activas;
- última operación/evento procesado;
- build SHA del proyecto y framework.

## Fase C — 237A-5 — Persistencia e identidad de chat

**Responsable:** agente principal  
**Dificultad:** Alta
**Estado:** Núcleo crítico implementado y validado localmente; migración/deploy pendientes

Resultado 2026-07-23:

- Chats cerrados visibles como historial de solo lectura.
- Cleanup limitado a sesiones anónimas; pedidos y usuarios autenticados no expiran.
- Consulta corregida para devolver los últimos N y restaurarlos en orden cronológico.
- `/reset` archiva sin borrar mensajes y se rechaza en sesiones vinculadas.
- `visitor_id` inválido se rota en frontend y se rechaza en backend.
- Sesión de orden consolidada con migración e índice único parcial; creación/reapertura atómica.
- Lectura, escritura y WS de staff autorizados por participante; admin conserva supervisión.
- Queda pendiente sustituir el crecimiento de `limit` por cursor compuesto; no bloquea la corrección de desaparición.

Tareas:

1. Validar `visitor_id` en frontend y backend.
2. Rotar valores vacíos, `undefined`, `null` o no UUID.
3. Auditar sesiones contaminadas antes de consolidar o borrar.
4. Separar políticas:
   - pedido: persistencia indefinida;
   - anónimo/preventa: cierre configurable;
   - cerrado: visible en archivo.
5. Evitar que el cleanup de 24 horas cierre sesiones de orden.
6. `find_session_by_order` debe recuperar la misma sesión aunque esté archivada y reabrirla de forma explícita cuando corresponda.
7. Sustituir `ORDER BY ASC LIMIT` por consulta de últimos N:
   - subquery descendente;
   - reorden ascendente para render;
   - cursor `before_created_at/before_id`.
8. Evitar sesiones duplicadas por orden mediante constraint o reconciliación previa.
9. Cambiar `/reset`:
   - solo permitido en chat anónimo;
   - no elimina historial contractual;
   - cierre/archivado auditable.

Pruebas:

- Una sesión de orden de más de 24 horas sigue visible.
- Una sesión anónima cerrada aparece en Historial.
- Conversación con 150 mensajes muestra primero los 100 más recientes.
- Paginar añade mensajes antiguos sin perder nuevos.
- Reconexión restaura los últimos 50.
- `visitor_id=undefined` es rechazado y reemplazado.
- Una orden no obtiene dos sesiones.

## Fase D — 237A-6 — Contrato realtime y sonido

**Responsable:** agente principal  
**Dificultad:** Alta
**Estado:** Autorización WS completada; contrato de eventos/sonido queda pendiente

Plan de ejecución actualizado: `Agente/planes/plan-cierre-bloques-dificiles-nakomi-2026-07-23.md`.

Tareas:

1. Crear un endpoint/protocolo WS de sesión de orden inequívoco.
2. Autorizar por participantes:
   - cliente dueño;
   - admin supervisor;
   - freelancer asignado.
3. Sincronizar responsable del chat al reasignar la orden.
4. Etiquetar eventos:
   - `live`;
   - `history`;
   - `broadcast`;
   - `self`.
5. Sonar solo cuando:
   - origen sea `live`;
   - no sea `self`;
   - el ID se inserte por primera vez.
6. Deduplicar entre pestañas por `message_id`.
7. Mantener polling únicamente como recuperación degradada, no como realtime primario.

Pruebas:

- Cliente y admin ven el mensaje sin refetch manual.
- Dos pestañas producen un solo sonido.
- Reconectar no reproduce historial.
- Reasignación transfiere realtime al freelancer sin retirar supervisión del admin.
- Cierre WS registra código, motivo y contador de reconexiones.

## Fase E — 237A-7 — Centro global de notificaciones

**Responsable arquitectónico:** agente principal  
**Implementación mecánica delegable:** subagente

> **Corrección de alcance 2026-07-23:** esta fase vuelve a ser prioritaria. Cada
> mensaje de cliente debe producir inmediatamente notificación persistente,
> correo y WhatsApp. La integración WhatsApp reutilizará el `wacli` operativo de
> `glorytemplate` mediante un gateway interno firmado y colas idempotentes; no se
> instalará un segundo cliente ni se compartirán stores entre contenedores.

Arquitectura:

- Un provider autenticado global para sitio público y panel.
- Unread persistido por conversación/usuario.
- Badge en:
  - sidebar desktop;
  - navegación móvil;
  - header público autenticado;
  - campana del panel.
- Marcar leído requiere abrir la conversación correcta.
- Los eventos externos salen de outbox.

Alertas:

- Nueva conversación:
  - in-app inmediata;
  - email a `andoryyu@gmail.com`;
  - WhatsApp al número configurado.
- Nuevo pedido:
  - conservar email existente;
  - añadir WhatsApp;
  - verificar asignación y entrega.
- Mensaje de cliente sin respuesta durante 20 minutos:
  - una alerta por ciclo sin respuesta;
  - destinatarios: admin y freelancer asignado;
  - se rearma tras una respuesta o un mensaje posterior;
  - sin repetición cada cinco minutos.

Delegable:

- Montaje visual del badge.
- Reutilización de `NotificationBell`.
- Corrección de deeplink `seccion=mensajes`.
- Estados vacíos/loading/error.
- Tests de render y copy.

No delegable:

- Persistencia de unread.
- Semántica de respuesta.
- Outbox, idempotencia y retry.
- Integración/proveedor WhatsApp.

## Fase F — 237A-8 — Checkout y webhook idempotentes

**Responsable:** agente principal  
**Dificultad:** Muy alta

Preflight:

- Buscar PaymentIntent duplicados.
- Buscar órdenes duplicadas por evento.
- No crear constraints hasta reconciliar cualquier duplicado.

Implementación:

1. Convertir `stripe_processed_events` en inbox reclamable.
2. Claim atómico mediante `INSERT ... ON CONFLICT DO NOTHING RETURNING`.
3. Estados `processing/completed/failed`, locks y reintentos.
4. Constraint único para `stripe_payment_intent_id`.
5. Persistencia transaccional de usuario, orden, fases, pago, asignación y outbox.
6. Snapshot firmado de precio/plan/currency.
7. Validación contra monto/currency reales de Stripe.
8. Admin primario configurable, no `admins.first()`.

Pruebas:

- Dos webhooks concurrentes solo crean una orden.
- Fallo en una fase hace rollback completo.
- Reintento después del commit es idempotente.
- Cambio de precio CMS no cambia una compra ya iniciada.
- La ausencia de email/notificación no revierte el pago persistido.

## Fase G — 237A-9 — Reembolso multi-pago reintentable

**Responsable:** agente principal  
**Dificultad:** Muy alta

Migraciones:

- Índice único parcial para un refund activo por orden.
- `order_refund_items`, un registro por pago.
- Campos de retry/error/lock en `order_refunds`.
- Idempotencia de wallet si se mantiene.

Flujo:

1. Cliente crea caso con motivo.
2. Backend bloquea orden/pagos y calcula el total reembolsable.
3. Admin revisa y conversa.
4. Admin aprueba o rechaza.
5. Aprobar crea trabajo; no llama Stripe desde el request.
6. Worker reclama con `FOR UPDATE SKIP LOCKED`.
7. Por pago:
   - `held`: cancelar PaymentIntent;
   - `released`: Stripe Refund;
   - clave `order-refund:{refund_id}:{payment_id}`.
8. Fallo parcial queda reintentable.
9. Solo al completar todos:
   - pagos `refunded`;
   - caso `completed`;
   - orden `cancelled`.

Guardas:

- Falta de secret Stripe siempre es error.
- Cancelación directa rechaza órdenes con fondos `held/released` y dirige al flujo financiero.
- Admin no crea solicitudes como cliente.
- Wallet no acredita mientras exista un flujo Stripe no reconciliado.

## Fase H — 237A-10 — Asignación, cancelación y conversación

**Responsable arquitectónico:** agente principal  
**UI mecánica delegable:** subagente

Tareas:

- `PRIMARY_ORDER_ADMIN_ID` o configuración equivalente.
- Admin primario sin límite.
- Reasignación permitida en `in_progress`.
- Operación transaccional:
  - orden;
  - chat;
  - auditoría;
  - notificaciones.
- Admin siempre conserva rol supervisor.
- Cancelación respeta settlement financiero.
- Confirmación antes de desasignar.

Delegable:

- Título/cabecera de `ModalAsignar`.
- Uso real de `orderNumber`.
- Copy “Freelancer”.
- Eliminación del `prompt()` admin.
- Estados visuales del refund.

## Fase I — 237A-11 — CMS, media y orden visual

**Responsable:** subagente  
**Revisión:** agente principal  
**Dificultad:** Baja/Media

Instrucciones:

1. Leer primero variables CSS y componentes UI compartidos.
2. No crear tokens o clases locales duplicadas.
3. Mantener alineación izquierda existente de notificaciones.
4. No duplicar plantillas email; `email_templates.rs` sigue siendo fuente única.
5. Tags:
   - permitir comas;
   - persistir también al guardar, no depender solo de `blur`;
   - probar múltiples tags en Blog, Proyecto y Servicio.
6. Imágenes:
   - aumentar 10% el ancho solicitado al optimizador/srcset;
   - no cambiar solo aspect ratio;
   - aplicar a `galeriaHeroContenedor` y `proyectoGaleriaItem`;
   - comprobar peso y nitidez.
7. Verificar 320 px y 1024 px.
8. Ejecutar type-check y tests frontend asignados.

Criterios de aceptación:

- Sin nuevos errores TypeScript.
- Sin CSS inline.
- Sin clases ad-hoc duplicadas.
- Comas/tags sobreviven edición, guardado y recarga.
- Las solicitudes de imagen son realmente 10% mayores.

## Fase J — 237A-12 — Procedencia reproducible y deploy

**Responsable:** agente principal  
**Dificultad:** Alta

Proyecto:

- Corregir upstream de `glory-rust-nakomi` hacia `origin/glory-rust-nakomi`.
- Publicar la rama proyecto en `glory-rs-template`, no en el remoto legado.
- No cambiar el `repoUrl` de `studio`: ya apunta al repositorio correcto.

Framework:

- Fijar `glory-rs-framework` por SHA/tag.
- Preferencia: submodule si también se consumen frontend, scripts o assets.
- Alternativa: dependencia Cargo `git + rev` si solo se consumen crates.
- Resolver primero la divergencia `main/master`.
- Local, CI y Docker deben usar la misma revisión.

Stamping:

- SHA proyecto.
- SHA framework.
- fecha UTC.
- repo/rama.
- labels OCI.
- endpoint `/healthz`.
- variable frontend visible para diagnóstico.

Preflight obligatorio:

- Verificar que el SHA remoto esperado existe en `origin`.
- Verificar que framework SHA/tag existe.
- Abortar deploy si la rama rastrea un remoto distinto a `origin`.
- Compilar frontend y backend desde el mismo checkout.

Deploy:

1. Commit y push explícitos.
2. `coolify-manager deploy --name studio --update`.
3. No usar `--skip-backup` si hay migraciones.
4. `coolify-manager health --name studio`.
5. Comparar SHA esperado con `/healthz` y labels.
6. Verificar assets Vite.
7. Revisar logs de watchdog/chat/pagos.
8. Si health falla: recovery mediante `coolify-manager`, nunca SSH directo.

## Fase K — 237A-13 — Restaurar `glory-rs-template/main`

**Responsable:** agente principal  
**Momento:** Después de estabilizar y desplegar Nakomi  
**No mezclar con commits de Nakomi**

Evidencia:

- Existe una rama local genérica `main@aae2b174`.
- El árbol no contiene referencias de proyectos conocidos.
- `origin/main` no existe.
- `origin/HEAD` apunta incorrectamente a `glory-rs-rest`.

Procedimiento seguro:

1. Confirmar que no hay worktrees con cambios no preservados.
2. Crear refs/tags de rescate para todos los tips actuales.
3. Verificar nuevamente el árbol de `aae2b174`:
   - sin Nakomi;
   - sin Rest;
   - sin Kamples;
   - sin secretos;
   - documentación genérica;
   - self-check operativo.
4. Publicar sin force:
   - `main:main` hacia `origin`;
   - no fusionar ninguna rama proyecto.
5. Cambiar default branch de GitHub a `main`.
6. Configurar `origin/HEAD -> origin/main`.
7. Corregir upstreams:
   - `main -> origin/main`;
   - cada rama proyecto -> su rama homónima en `origin`.
8. Renombrar o retirar el remoto raíz legado `framework` solo después de verificar que ninguna rama depende de él.
9. Documentar cómo crear una nueva rama proyecto desde `main`.
10. Añadir una protección automática:
    - `main` no acepta archivos/configs específicos de proyectos;
    - CI comprueba neutralidad del template.

Criterios de aceptación:

- GitHub muestra `main` como rama por defecto.
- `main` es un template genérico compilable.
- Las ramas proyecto conservan su historia.
- Ningún proyecto despliega desde el repositorio del framework.
- No se reescribe historia existente.

## 5. Instrucciones para el subagente de partes fáciles

El subagente debe:

1. Trabajar solo en archivos expresamente asignados.
2. Leer estilos/tokens/componentes base antes de editar UI.
3. No modificar watchdog, runtime, transacciones, SQL financiero, WebSocket backend, migraciones de pagos ni configuración de deploy.
4. No inventar políticas funcionales.
5. No ejecutar deploy.
6. No hacer commit independiente salvo instrucción expresa.
7. Entregar:
   - archivos modificados;
   - diff resumido;
   - pruebas ejecutadas;
   - riesgos o decisiones detectadas.
8. Detenerse si una tarea aparentemente visual requiere cambiar un contrato backend.

Asignaciones previstas:

- Corregir errores TypeScript mecánicos.
- Completar cabecera/copy del modal de asignación.
- Eliminar `prompt()` admin después de que el agente principal defina el contrato.
- Montar badges usando el provider diseñado por el agente principal.
- Ajustar deeplinks.
- Tags/comas.
- Resoluciones de imágenes.
- Tests de render/copy.
- Documentación OpenAPI mecánica.

## 6. Validación por bloque

### Runtime/Rust

```powershell
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
```

### Frontend

```powershell
npm --prefix frontend run type-check
```

Además:

- flujo renderizado;
- realtime con dos navegadores/pestañas;
- historial de más de 100 mensajes;
- responsive 320 px y 1024 px;
- estados vacíos/error/retry.

### Cierre de bloque

```powershell
npm run self-check -- -TareaId <ID>
```

Reglas:

- Una única ronda pesada al cierre de cada bloque coherente.
- Los errores preexistentes descubiertos deben corregirse antes del commit.
- `git add` siempre explícito por archivo.
- Commit y push por bloque.
- Releer `roadmap.md` después de cada commit.

## 7. Pruebas E2E obligatorias antes de producción

1. Backend idle durante más de 130 segundos no se cierra.
2. Abrir chat, enviar mensajes y reconectar no congela el runtime.
3. Dos pestañas no duplican sonido.
4. `visitor_id=undefined` no llega al backend.
5. Chat de pedido permanece tras 24 horas.
6. Conversación con más de 100 mensajes conserva los nuevos.
7. Cliente y admin reciben mensajes realtime.
8. Badge aparece fuera y dentro del panel.
9. Alerta de 20 minutos se emite una vez.
10. Pedido nuevo asigna al admin primario.
11. Dos webhooks concurrentes crean una sola orden.
12. Fallo parcial de persistencia revierte todo.
13. Reembolso Stripe fallido queda reintentable.
14. Empleado no puede aprobar reembolso.
15. SHA proyecto/framework local, imagen y `/healthz` coinciden.

## 8. Documentación y prevención

Al completar:

- Consolidar freeze/chat en un único postmortem actualizado.
- Marcar documentos históricos como superseded, sin borrarlos.
- Documentar política de retención de chats.
- Documentar contratos de checkout/reembolso.
- Documentar topología de repositorios y ramas.
- Documentar stamping y preflight de deploy.

Prevenciones candidatas para Glory Sentinel:

- watchdog que compara epoch con valor inicial `0`;
- `prompt/alert/confirm` en flujos financieros;
- query de chat `ORDER BY ASC LIMIT` sin cursor;
- sesión contractual cerrada por cleanup genérico;
- webhook check-then-process;
- PaymentIntent sin constraint único;
- rama proyecto rastreando un remoto distinto a `origin`;
- build sin SHA proyecto/framework.

## 9. Orden de ejecución aprobado

1. Fase B — watchdog.
2. Fase C — persistencia/identidad.
3. Fase D — realtime/sonido.
4. Fase E — notificaciones.
5. Fase J parcial — stamping y framework reproducible.
6. Fase F — checkout/webhook.
7. Fase G — reembolsos.
8. Fase H — asignación/cancelación.
9. Fase I — tareas fáciles delegadas.
10. Fase J final — deploy y verificación.
11. Fase K — restauración de `main`.

## 10. Estado inicial

- [x] Investigación profunda.
- [x] Causa del falso freeze identificada.
- [x] Causa del deploy stale identificada.
- [x] Causa de desaparición de mensajes identificada.
- [x] Auditoría de pagos/reembolsos completada.
- [x] Auditoría del trabajo anterior completada.
- [x] Plan maestro creado.
- [x] Watchdog corregido y probado localmente.
- [x] Persistencia de chat corregida y probada localmente.
- [ ] Realtime corregido.
- [ ] Notificaciones globales implementadas.
- [ ] Pagos/reembolsos endurecidos.
- [ ] Rama proyecto sincronizada con `origin`.
- [ ] Producción desplegada con procedencia verificable.
- [ ] `glory-rs-template/main` restaurada.

Estado de producción corregido:

- [x] Rama proyecto sincronizada con `origin`.
- [x] Watchdog y persistencia desplegados y verificados.
- [x] Framework de producción actualizado en `master`.
- [ ] Alertas inmediatas in-app + correo + WhatsApp.
- [ ] CTA de WhatsApp para escalamiento.

## 11. Recorte de alcance solicitado el 2026-07-23

El usuario pidió terminar únicamente lo importante y complicado. Después aclaró
que alertas, WhatsApp, correo y Realtime forman parte de ese núcleo. Por tanto:

- Watchdog y persistencia ya están desplegados.
- El siguiente bloque obligatorio es alertas inmediatas + CTA de escalamiento,
  seguido por contrato Realtime y alerta idempotente de 20 minutos.
- Pagos/reembolsos continúan como bloques difíciles posteriores.
- Permanecen fáciles y delegables: CMS/media y pulido visual.
- Los cambios fáciles ya preparados pero no pertenecientes al bloque crítico se preservan fuera de los commits de producción.
