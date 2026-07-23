# Plan de cierre de bloques difíciles — Nakomi Studio

> **Fecha:** 2026-07-23  
> **Estado:** Planificado; sin implementación en este bloque  
> **Rama de proyecto:** `glory-rust-nakomi`  
> **Framework compartido:** `glory-rs-framework/master`  
> **Objetivo:** cerrar alertas inmediatas, escalamiento a WhatsApp, contrato Realtime y flujos financieros sin volver a crear ramas temporales ni duplicar infraestructura.

## 1. Resultado que debe percibir el usuario

Cuando un cliente envíe cualquier mensaje:

1. El mensaje se persiste una sola vez.
2. Se crea inmediatamente una notificación no leída para el administrador.
3. Si el administrador está conectado, la notificación aparece por WebSocket.
4. Si se conecta después, la notificación continúa visible y no leída.
5. Se encola un correo a `andoryyu@gmail.com`.
6. Se encola un WhatsApp al número administrativo configurado.
7. Cada canal tiene estado, reintentos y trazabilidad; ningún fallo se silencia.

Cuando la IA escale una conversación:

1. La sesión queda marcada como escalada de forma persistente.
2. El administrador recibe los tres canales anteriores.
3. El cliente recibe dentro del chat un CTA claro para escribir por WhatsApp.
4. Reconectar o abrir otra pestaña no duplica el CTA, la alerta ni el sonido.

## 2. Estado real confirmado

### Ya funciona

- Los mensajes y sesiones de pedido persisten y ya no expiran a las 24 horas.
- Existe una tabla de notificaciones persistentes y un WebSocket de notificaciones.
- El panel contiene `NotificationBell` y consulta el conteo no leído.
- SMTP/Brevo está integrado y existen plantillas centralizadas y trazabilidad en `email_logs`.
- La IA detecta escalamiento mediante `request_human_assistance`/`needs_escalation`.
- En el proyecto WordPress `glorytemplate` existe un envío operativo con `wacli`:
  - `WacliService::enviarTexto()`;
  - destinatario por `WHATSAPP_AGENT_TO`, `WHATSAPP_TO` o `WHATSAPP`;
  - cuenta por `WACLI_ACCOUNT`;
  - ejecución segura con argumentos separados;
  - soporte multi-cuenta y health checks.

### No está listo

- Nakomi Rust no tiene integración WhatsApp.
- El camino WebSocket del visitante guarda el mensaje, pero no crea alertas para el administrador.
- El camino REST compara tipos legacy (`visitor`/`user`) aunque actualmente el remitente es `client`; por ello tampoco garantiza la alerta administrativa.
- El correo de escalamiento solo se envía cuando la IA escala, no por cada mensaje de cliente.
- `useNotificationWs` existe, pero no está montado globalmente.
- Fuera del panel no existe una campana/indicador persistente.
- La tool de escalamiento devuelve texto plano; no genera CTA de WhatsApp.
- Historial y eventos live usan el mismo mensaje WS, por lo que una reconexión puede volver a producir sonido.
- Frontend envía `toggle_ai.enable`, mientras backend espera `enabled`.
- La alerta de 20 minutos no tiene ciclo idempotente y puede repetirse.

## 3. Decisión arquitectónica para WhatsApp

### Decisión

Reutilizar `wacli` mediante un **gateway interno firmado** en `glorytemplate`.

```text
Mensaje cliente
  -> transacción PostgreSQL
     -> mensaje
     -> notificación in-app
     -> outbox email
     -> outbox whatsapp
  -> worker Nakomi
     -> SMTP/Brevo
     -> HTTPS + HMAC al gateway glorytemplate
        -> outbox WordPress
           -> WacliService
              -> WhatsApp administrador
```

### Por qué

- La cuenta y el store autenticado ya viven junto a `glorytemplate`.
- Un segundo `wacli` en Nakomi podría competir por locks o corromper la sesión.
- Compartir bind mounts entre servicios crea acoplamiento operativo.
- Ejecutar SSH, Docker o comandos del host desde Nakomi está prohibido.
- Un contrato HTTPS firmado permite cambiar la implementación interna sin acoplar Rust a PHP o al binario.

### Contrato propuesto

`POST /wp-json/glory/v1/internal/alerts`

Headers:

- `X-Glory-Timestamp`
- `X-Glory-Nonce`
- `X-Glory-Idempotency-Key`
- `X-Glory-Signature`

La firma cubre método, ruta, timestamp, nonce y hash SHA-256 del body. El gateway:

1. rechaza desfases mayores a cinco minutos;
2. registra el nonce para impedir replay;
3. valida una allowlist de tipos;
4. hace `INSERT ... ON DUPLICATE KEY`;
5. responde `202 Accepted`;
6. nunca ejecuta `wacli` dentro del request HTTP;
7. entrega desde un worker con retry y dead-letter.

Payload mínimo:

```json
{
  "event": "chat.client_message",
  "messageId": "uuid",
  "sessionId": "uuid",
  "visitorLabel": "Cliente o visitante",
  "preview": "Texto limitado y sanitizado",
  "panelUrl": "https://nakomi.studio/panel?seccion=mensajes&chat=...",
  "occurredAt": "RFC3339"
}
```

No se envían JWT, contenido completo, secretos ni datos financieros.

## 4. Bloque A — Alertas inmediatas y durables

**Prioridad:** máxima  
**Dificultad:** alta

### Backend Nakomi

1. Crear una outbox PostgreSQL con:
   - clave idempotente única;
   - evento, canal, destinatario y referencia;
   - `pending/processing/sent/failed/dead`;
   - intentos, `available_at`, lock, error y timestamps.
2. Insertar mensaje, notificaciones administrativas y eventos de outbox en la misma transacción.
3. Cubrir todos los ingresos:
   - WebSocket del widget;
   - REST de chat de pedido;
   - acciones y adjuntos que representen un mensaje del cliente.
4. Resolver destinatarios administrativos por usuarios activos/configuración, no por `admins.first()`.
5. Un worker reclama con `FOR UPDATE SKIP LOCKED`.
6. Recuperar trabajos `processing` abandonados.
7. Backoff: 5 s, 30 s, 2 min, 10 min y 30 min; después `dead`.
8. Exponer métricas/logs estructurados de pendientes, fallidos y edad del evento más antiguo.

### Correo

- Una fila por mensaje y destinatario.
- Plantilla única `chat_client_message_admin` en `email_templates.rs`.
- Asunto y cuerpo incluyen remitente, preview limitado y enlace correcto.
- Registrar resultado en `email_logs`.
- Un fallo SMTP mantiene el trabajo reintentable.

### WhatsApp

- El worker Nakomi llama al gateway firmado.
- El gateway encola antes de responder.
- El worker WordPress usa `WacliService::enviarTexto(null, mensaje)`, reutilizando el destinatario configurado.
- La idempotency key original se conserva hasta `wacli`.
- El texto incluye remitente, preview y enlace directo al chat.

### Notificación visible

- Persistir una notificación por administrador y `message_id`.
- Constraint único para impedir duplicados.
- Montar una sola conexión de notificaciones en el nivel autenticado de la aplicación.
- Mostrar indicador en:
  - header público autenticado;
  - header del panel;
  - sidebar, específicamente en “Mensajes”;
  - navegación móvil.
- Abrir la conversación exacta marca esa notificación como leída.
- Polling queda como recuperación; WebSocket es el camino primario.

### Pruebas de aceptación

- Un mensaje WS crea exactamente una alerta por canal.
- Un mensaje REST crea exactamente una alerta por canal.
- Repetir el mismo evento no duplica correo, WhatsApp ni notificación.
- Administrador conectado ve badge/push sin refrescar.
- Administrador desconectado ve el badge al volver.
- SMTP o gateway caídos no pierden el evento.
- Reiniciar el worker recupera trabajos abandonados.
- El mensaje enviado a WhatsApp llega al número administrativo real.

## 5. Bloque B — Escalamiento visible al cliente

**Prioridad:** inmediatamente después del Bloque A  
**Dificultad:** media/alta

1. Crear un tipo rico `contact_cta`.
2. La URL se construye en backend desde `PUBLIC_SUPPORT_WHATSAPP`, normalizada a dígitos.
3. Usar `https://wa.me/<numero>?text=<mensaje-codificado>`.
4. El texto prellenado contiene un código público de conversación, no identificadores internos sensibles.
5. Persistir el CTA como mensaje para que sobreviva a recarga.
6. Constraint/estado de ciclo para mostrar un solo CTA por escalamiento.
7. Render seguro con `<a>`, `target="_blank"` y `rel="noopener noreferrer"`.
8. Fallback de texto si el número no está configurado.
9. Corregir el enlace administrativo legacy `/admin/chat` a la ruta real de Mensajes.
10. Marcar y transmitir `is_escalated` inmediatamente.

Pruebas:

- Petición explícita de humano genera CTA.
- Error del proveedor IA también escala y muestra CTA.
- Reconexión no duplica CTA.
- CTA abre el número correcto en móvil y desktop.
- El CTA funciona a 320 px y 1024 px.

## 6. Bloque C — Contrato Realtime y sonido

**Prioridad:** alta  
**Dificultad:** alta

1. Separar snapshots de historial de eventos live:

```json
{
  "type": "chat.message",
  "delivery": "live",
  "messageId": "uuid",
  "sessionId": "uuid",
  "senderId": "uuid-or-visitor",
  "sequence": 42,
  "occurredAt": "RFC3339",
  "payload": {}
}
```

2. Historial se entrega como `chat.snapshot`, nunca como mensajes live individuales.
3. Sonido solo si:
   - `delivery=live`;
   - remitente distinto al usuario actual;
   - `messageId` no procesado;
   - la pestaña es líder para audio.
4. Dedupe por `messageId` en memoria y almacenamiento breve compartido.
5. `BroadcastChannel` transporta IDs/eventos, pero no vuelve a disparar sonido.
6. Reconexión con backoff, jitter, códigos/motivos y contador observable.
7. Polling solo repara huecos de secuencia.
8. Corregir `toggle_ai` para usar `enabled` en ambos extremos.
9. Reasignar una orden actualiza participantes del realtime sin quitar supervisión al admin.

Pruebas:

- Cliente y admin ven el mensaje sin refetch.
- Dos pestañas producen un solo sonido.
- Reconectar no reproduce historial.
- Un hueco de secuencia se recupera por REST.
- Reasignación mueve el acceso al freelancer correcto.

## 7. Bloque D — Alerta por 20 minutos sin respuesta

**Prioridad:** después de alertas inmediatas  
**Dificultad:** alta

1. Modelar un ciclo de espera:
   - comienza con el primer mensaje de cliente posterior a una respuesta;
   - termina cuando responde staff;
   - tiene una única alerta de vencimiento.
2. Constraint único por `session_id + cycle_id + alert_type`.
3. No crear una alerta nueva cada cinco minutos.
4. Enviar in-app, correo y WhatsApp por la misma outbox.
5. Destinatarios: admin y freelancer asignado; WhatsApp administrativo configurable.
6. Link siempre a la conversación exacta.

Pruebas:

- A los 19 minutos no alerta.
- A los 20 minutos alerta una vez.
- Barridos posteriores no duplican.
- Respuesta de staff cierra el ciclo.
- Un mensaje posterior abre un ciclo nuevo.

## 8. Bloques difíciles posteriores

### Checkout/webhook

- Inbox Stripe reclamable e idempotente.
- Constraint por `PaymentIntent`.
- Cuenta, orden, fases, pago, asignación y outbox en una transacción.
- Validar monto/currency contra snapshot de checkout.
- Ninguna cuenta u orden antes de pago confirmado.

### Reembolsos

- Solicitud del cliente solo contiene motivo.
- Admin decide; freelancer no aprueba.
- Trabajo financiero asíncrono por cada pago.
- Idempotency key por `refund + payment`.
- Fallo parcial reintentable.
- Orden se cancela solo cuando termina la reconciliación financiera.

### Asignación/cancelación

- Admin primario configurable y sin límite.
- Reasignación transaccional de orden, chat, auditoría y alertas.
- Cancelación bloqueada si existen fondos sin reconciliar.

### Procedencia y ramas

- Mantener solamente las ramas habituales.
- `glory-rust-nakomi -> origin/glory-rust-nakomi`.
- `glory-rs-framework -> master`.
- Fijar el SHA del framework usado por Docker/local.
- Restaurar `glory-rs-template/main` en un bloque independiente y sin mezclar Nakomi.

## 9. Orden seguro de implementación y despliegue

1. Preflight de `wacli` usando únicamente herramientas aprobadas:
   - estado autenticado;
   - destinatario configurado;
   - envío canario;
   - worker y cola;
   - sin imprimir secretos.
2. Implementar y desplegar gateway/outbox WordPress primero.
3. Probar idempotencia del gateway con un evento sintético.
4. Implementar outbox y worker Nakomi con producción desactivada por feature flag.
5. Implementar notificación visible y CTA.
6. Ejecutar pruebas locales/integración.
7. Activar alertas para un mensaje canario.
8. Confirmar las tres evidencias:
   - notificación persistida/WS;
   - `email_logs=sent`;
   - outbox WhatsApp `sent` y recepción real.
9. Activar para todos los mensajes nuevos.
10. Implementar contrato Realtime y alerta de 20 minutos.
11. Endurecer pagos, reembolsos y asignación en commits separados.

Todos los deploys, health, logs y operaciones de producción pasan por `coolify-manager-rs`.

## 10. Rollback y control de incidentes

- `CHAT_ALERTS_ENABLED=false` detiene nuevos eventos externos sin borrar la outbox.
- `CHAT_WHATSAPP_ALERTS_ENABLED=false` aísla solo WhatsApp.
- El gateway puede rechazar temporalmente con `503`; Nakomi reintenta.
- Nunca reproducir mensajes históricos al habilitar el sistema.
- Una migración no elimina filas de chat o notificaciones.
- Dead-letter se conserva para inspección y reenvío manual idempotente.

## 11. Criterio de cierre

El bloque no se considera listo solo porque compile. Deben existir evidencias reales de:

- mensaje desde cliente por WS y por REST;
- badge inmediato y persistente;
- correo recibido;
- WhatsApp recibido;
- CTA de WhatsApp abierto por el cliente;
- dos pestañas sin sonido duplicado;
- reconexión sin replay;
- alerta de 20 minutos exactamente una vez;
- health y logs limpios tras deploy.

## 12. Guía operativa para agentes implementadores

Esta sección elimina decisiones implícitas. El agente que tome una fase debe
seguir el orden indicado, modificar solo los archivos de su tarjeta y detenerse
ante cualquiera de las condiciones de parada.

### 12.1 Reglas que no se pueden reinterpretar

1. **“Mensaje de cliente”** significa un mensaje nuevo persistido cuyo
   `sender_type` sea `client` o `visitor`. No incluye:
   - respuestas de IA;
   - mensajes de admin/freelancer;
   - historial enviado durante reconexión;
   - typing, presencia o cambios de estado.
2. **“Inmediato”** significa:
   - notificación in-app en la misma operación lógica;
   - outbox creada antes de responder éxito;
   - worker iniciado en menos de cinco segundos;
   - no significa ejecutar SMTP o `wacli` dentro del request.
3. **Una vez** se garantiza por constraint/idempotency key, no por un `if`
   previo ni por memoria del proceso.
4. PostgreSQL es la fuente de verdad del chat y sus alertas.
5. El gateway WordPress es el único dueño de `wacli`.
6. Ningún agente puede:
   - llamar `wacli`, SSH o Docker desde Nakomi;
   - copiar el store de WhatsApp;
   - crear un segundo proveedor WhatsApp;
   - enviar alertas desde el frontend;
   - marcar un trabajo `sent` antes de recibir confirmación;
   - ocultar errores con `let _ =`, `catch {}` o retornos de éxito.
7. No se reproducen mensajes existentes al crear la outbox. Solo se procesan
   mensajes insertados después de habilitar la migración/feature flag.
8. Se trabaja en las ramas habituales:
   - Nakomi: `glory-rust-nakomi`;
   - framework: `master`;
   - glorytemplate: verificar su rama productiva antes de editar.

### 12.2 Mapa de archivos — Nakomi Rust

| Responsabilidad | Archivo existente o nuevo | Instrucción |
|---|---|---|
| Migración outbox/unread | `migrations/<fecha>_chat_alert_outbox.up.sql` y `.down.sql` | Crear tablas, constraints, índices y rollback simétrico. |
| Modelo outbox | `src/models/chat_alert.rs` | Estados y payload tipados; no usar `serde_json::Value` para todo. |
| Repositorio outbox | `src/repositories/chat_alert.rs` | Enqueue, claim, sent, retry y dead-letter con queries preparadas. |
| Orquestador | `src/services/chat_alert.rs` | Construir evento, destinatarios y claves idempotentes. No enviar red aquí. |
| Worker externo | `src/services/chat_alert_worker.rs` | Procesar email/WhatsApp con timeout y backoff. |
| Cliente gateway | `src/services/whatsapp_gateway.rs` | HMAC, timeout HTTP, respuestas tipadas y redacción de logs. |
| Registro módulos | `src/models/mod.rs`, `src/repositories/mod.rs`, `src/services/mod.rs` | Exportar solo lo necesario. |
| Arranque worker | `src/main.rs` | Un `tokio::spawn` supervisado, con tick acotado y shutdown limpio. |
| Chat REST | `src/handlers/chat/rest_messages.rs` | Sustituir `notify_chat_recipient` por el orquestador común. |
| Chat WS visitante | `src/handlers/chat/ws_visitor_helpers.rs` | Usar el mismo orquestador después de persistir; no duplicar lógica. |
| Adjuntos/acciones | `src/handlers/chat/rest_upload.rs`, `ws_visitor_helpers.rs` | Encolar solo si se creó un mensaje real de cliente. |
| Email | `src/services/email_templates.rs`, `src/services/email.rs` | Una plantilla y un método trazable; nada duplicado en previews. |
| Config | `.env.example`, configuración de arranque | Documentar flags y secretos sin valores reales. |
| WS chat | `src/models/chat.rs`, handlers WS y servicio de chat | Añadir envelope/snapshot sin romper autorización existente. |

También se deben registrar dependencias nuevas en `src/lib.rs` y construir el
estado compartido en `src/handlers/mod.rs`; no crear pools o clientes HTTP
adicionales dentro de cada handler.

El agente debe leer estos archivos antes de editar:

- `src/services/notification.rs`;
- `src/repositories/notification.rs`;
- `src/handlers/chat/rest_messages.rs`;
- `src/handlers/chat/ws_visitor_helpers.rs`;
- `src/services/email.rs`;
- `src/services/email_templates.rs`;
- `src/main.rs`.

### 12.3 Mapa de archivos — gateway glorytemplate

| Responsabilidad | Archivo existente o nuevo | Instrucción |
|---|---|---|
| Esquema | `App/Database/Schema.php` | Subir `DB_VERSION` y crear una outbox saliente, no reutilizar la cola entrante. |
| Endpoint interno | `App/Api/InternalAlertApiController.php` | Ruta nueva, pública solo a nivel HTTP y protegida íntegramente por HMAC. |
| Verificación firma | `App/Services/InternalAlertSignatureService.php` | Canonicalización, skew, nonce y comparación constante. |
| Repositorio | `App/Repository/WhatsApp/WhatsAppOutboundRepository.php` | Insert idempotente, claim y estados. |
| Worker | `App/Services/WhatsAppOutboundWorker.php` | Reclamar lote, usar `WacliService`, retry/dead-letter. |
| Envío real | `App/Services/WacliService.php` | Reutilizar `enviarTexto(null, mensaje)`; no modificar su sesión/store. |
| Cron | `functions.php` | Registrar hook y schedule siguiendo el worker WhatsApp ya existente. |

No modificar para esta integración:

- `WhatsAppWebhookService.php`: procesa mensajes entrantes, no alertas salientes.
- `WhatsAppEventWorker.php`: su cola pertenece al chatbot multiusuario.
- `WacliManagerService.php`: administra cuentas; no es el gateway de Nakomi.

### 12.4 Tarjeta A1 — Migración y dominio de alertas

**Entrada:** mensaje de chat ya validado.
**Salida:** mensaje + notificaciones + outbox persistidos.

Pasos:

1. Crear `chat_alert_outbox` con:
   - `id UUID`;
   - `idempotency_key TEXT UNIQUE`;
   - `event_type`, `channel`, `recipient`;
   - `reference_type`, `reference_id`;
   - payload JSON versionado;
   - estado con `CHECK`;
   - attempts, `available_at`, `locked_at`, `last_error`;
   - created/updated/sent timestamps.
2. Añadir índice de claim:
   `(status, available_at, created_at)`.
3. Añadir unicidad a notificaciones de chat:
   `(user_id, notification_type, reference_type, reference_id)`.
   Antes del constraint, consultar duplicados y reconciliarlos.
4. Definir claves exactas:
   - in-app: `chat:{message_id}:in_app:{admin_id}`;
   - email: `chat:{message_id}:email:{email_normalizado}`;
   - WhatsApp: `chat:{message_id}:whatsapp:admin`.
5. Crear una función de aplicación que reciba `ChatMessage` persistido y
   `ChatSession`; no debe aceptar strings sueltos que permitan alertar un mensaje
   inexistente.
6. Si aún no es viable meter `ChatHub::send_message` y outbox en una sola
   transacción, detenerse y rediseñar el boundary. No aceptar “persistir y luego
   intentar encolar” como cierre.

Pruebas mínimas:

- dos intentos con el mismo `message_id` producen una fila por canal;
- fallo de outbox revierte el mensaje;
- mensaje de IA no genera filas;
- cliente por REST y WS generan el mismo resultado.

### 12.5 Tarjeta A2 — Worker Nakomi

Algoritmo obligatorio:

1. Recuperar `processing` con `locked_at` mayor a cinco minutos.
2. Reclamar máximo 20 filas mediante `FOR UPDATE SKIP LOCKED`.
3. Cambiar a `processing` y aumentar attempts dentro de la transacción de claim.
4. Ejecutar cada envío con timeout:
   - SMTP: 30 segundos;
   - gateway: 10 segundos.
5. Resultado:
   - 2xx/SMTP aceptado: `sent`;
   - error temporal/429/5xx: `pending` con backoff;
   - 4xx de contrato/firma: `dead`, excepto 408/429;
   - configuración ausente: `failed` visible y sin falso éxito.
6. Truncar `last_error`; nunca guardar tokens, firmas o bodies completos.
7. Emitir resumen por lote, no un log ruidoso por tick vacío.

Condición de parada:

- Si no hay forma de distinguir aceptación real de envío simulado, no marcar
  `sent` y no continuar al deploy.

### 12.6 Tarjeta A3 — Gateway firmado

Canonical string exacto:

```text
POST
/wp-json/glory/v1/internal/alerts
<unix_timestamp>
<nonce>
<sha256_hex_body>
```

Firma:

```text
hex(hmac_sha256(shared_secret, canonical_string))
```

Validación:

1. Body crudo máximo 16 KiB.
2. Timestamp entero y desfase máximo 300 segundos.
3. Nonce de 16–128 caracteres, guardado con expiración.
4. Firma comparada con `hash_equals`.
5. `event` permitido: inicialmente solo `chat.client_message` y
   `chat.unanswered_20m`.
6. `idempotency_key` obligatoria y única.
7. Preview sanitizado y limitado; URL restringida a `https://nakomi.studio/`.
8. Responder:
   - `202`: nuevo o duplicado ya aceptado;
   - `400`: payload inválido;
   - `401`: firma/timestamp/nonce inválido;
   - `413`: body demasiado grande;
   - `429`: rate limit;
   - `503`: cola/BD no disponible.

Variables:

- ambos proyectos: `GLORY_INTERNAL_ALERT_SECRET`;
- Nakomi: `GLORY_ALERT_GATEWAY_URL`;
- glorytemplate ya conserva `WACLI_ACCOUNT` y destinatario WhatsApp.

El secreto se genera nuevo. No reutilizar JWT, SMTP, Stripe ni webhook secrets.

### 12.7 Tarjeta A4 — Notificación visible

Estado confirmado:

- `NotificationBell` llama `useNotificationWs`, pero solo se monta en
  `HeaderPanel`.
- Por eso existe realtime dentro del panel, no un provider global para toda la
  sesión autenticada.

Pasos:

1. Extraer la conexión a un componente sin UI
   `AuthenticatedNotificationRuntime`.
2. Montarlo una sola vez bajo el provider de React Query y por encima de las
   rutas públicas/panel.
3. Quitar la llamada directa desde `NotificationBell` para evitar dos sockets.
4. `NotificationBell` queda como consumidor de cache.
5. Montar la campana en `Header.tsx` para desktop autenticado.
6. En móvil, mostrar acción “Notificaciones” con badge dentro del menú.
7. `SidebarPanel.tsx` obtiene el conteo de mensajes no leídos y coloca un punto
   en el tab `mensajes`, tanto desktop como mobile/overflow.
8. Abrir una notificación:
   - navega a `panel?seccion=mensajes&chat=<session_id>`;
   - marca solo esa notificación/sesión como leída;
   - invalida lista y contador.
9. Reconexión WS con backoff cancelable; logout cancela timers y socket.

No pedir permiso de notificaciones del navegador automáticamente al montar. El
permiso debe solicitarse tras una acción explícita del usuario; el badge interno
no depende de ese permiso.

### 12.8 Tarjeta B — CTA de escalamiento

1. Cambiar `exec_request_human` para devolver `RichMessage`:
   - `message_type = "contact_cta"`;
   - metadata con `label`, `href` y `support_code`.
2. El backend construye el `href`; el LLM nunca proporciona URLs.
3. El `support_code` se deriva de una referencia pública persistida, no del
   email/teléfono del cliente ni de secretos.
4. Persistir el CTA mediante `chat_hub.send_rich_message`.
5. Añadir `contact_cta` al renderer de `ChatWidget.tsx`.
6. Usar recetas/tokens existentes en `ChatWidget.css`.
7. Persistir ciclo de escalamiento y usarlo en la clave idempotente.
8. Corregir `toggle_ai` a `enabled` en frontend y backend.

Copy mínimo:

- Texto: “Este caso necesita atención personal.”
- Botón: “Escribir por WhatsApp”.
- Fallback: “El equipo fue notificado y responderá por este chat.”

### 12.9 Tarjeta C — Realtime

Orden de edición:

1. Añadir el nuevo envelope en backend manteniendo temporalmente compatibilidad.
2. Añadir `chat.snapshot` para historial.
3. Adaptar hooks frontend.
4. Implementar dedupe y líder de sonido.
5. Retirar el mensaje legacy solo después de pruebas con ambos clientes.

Regla de sonido en pseudocódigo:

```text
if event.delivery != live: no sonar
if event.senderId == currentIdentity: no sonar
if processedMessageIds.contains(event.messageId): no sonar
insertar mensaje
registrar messageId
si esta pestaña es líder: sonar
```

No reproducir sonido fuera de la misma rama que confirmó la inserción del ID.

### 12.10 Matriz de pruebas y evidencias

| Caso | BD | UI | Correo | WhatsApp | Realtime |
|---|---|---|---|---|---|
| Widget anónimo | mensaje + 3 canales | badge | recibido | recibido | una inserción |
| Cliente autenticado | igual | badge persistente | recibido | recibido | una inserción |
| Chat de orden | sesión única | abre chat exacto | recibido | recibido | cliente/admin |
| Dos pestañas admin | una notif | mismo contador | uno | uno | un sonido |
| Reconexión | sin filas nuevas | conserva unread | ninguno nuevo | ninguno nuevo | sin sonido |
| Gateway 503 | outbox pending | notif visible | independiente | retry | chat no falla |
| SMTP caído | outbox pending | notif visible | retry | independiente | chat no falla |
| Escalación IA | ciclo + CTA | botón visible | alerta | alerta | CTA único |

Cada evidencia debe incluir:

- ID de mensaje;
- claves idempotentes;
- estados de outbox;
- registro `email_logs`;
- resultado enmascarado de `wacli`;
- captura de badge/CTA;
- logs sin secretos.

### 12.11 Condiciones de parada obligatoria

El agente se detiene y documenta antes de editar si:

- la rama activa no es la rama habitual del repositorio;
- hay cambios ajenos sin preservar;
- el `wacli` productivo no está autenticado;
- no se conoce la rama/deploy real de glorytemplate;
- se detectan duplicados que impiden crear constraints;
- el endpoint interno no puede usar HTTPS;
- no se puede configurar un secreto distinto en ambos servicios;
- una migración requiere borrar mensajes/notificaciones;
- el cambio exige SSH directo o compartir stores;
- las pruebas solo pueden demostrar “request enviado”, pero no recepción real.

### 12.12 Formato de entrega de cada agente

El agente entrega siempre:

1. tarea/ID y repositorio;
2. archivos tocados;
3. invariantes preservadas;
4. migraciones y rollback;
5. pruebas ejecutadas y resultados;
6. evidencia funcional;
7. variables nuevas sin valores;
8. riesgos/pendientes;
9. commit y rama habitual;
10. confirmación explícita de que no se desplegó, o health post-deploy si estaba autorizado.

## 13. Decisiones cerradas para evitar interpretaciones

### 13.1 Destinatarios y precedencia

Hay dos números distintos aunque inicialmente puedan coincidir:

- **WhatsApp interno de alertas:** destinatario por defecto ya configurado en
  glorytemplate mediante `WHATSAPP_AGENT_TO`, luego `WHATSAPP_TO`, luego
  `WHATSAPP`. Nakomi no conoce ni envía ese número.
- **WhatsApp público de soporte:** `PUBLIC_SUPPORT_WHATSAPP` en Nakomi. Solo se
  usa para construir el CTA que abre el cliente.

Destinatarios:

1. `PRIMARY_ORDER_ADMIN_ID` identifica al dueño operativo.
2. La notificación in-app se crea para el dueño y para otros admins activos,
   deduplicando IDs.
3. El correo inmediato se envía al email actual del dueño obtenido desde BD.
   `CHAT_ALERT_EMAIL_OVERRIDE` solo existe para canary/staging.
4. WhatsApp externo se envía una vez al destinatario por defecto de
   glorytemplate.
5. Si falta `PRIMARY_ORDER_ADMIN_ID`, la aplicación debe reportar configuración
   crítica y no fingir que las alertas externas están listas.

### 13.2 Estados entre Nakomi y glorytemplate

Un `202 Accepted` solo significa **encolado por el gateway**, no enviado a
WhatsApp.

Estados Nakomi:

```text
pending -> processing -> accepted_by_gateway -> sent
                              |                  |
                              v                  v
                            retry              dead
```

El gateway devuelve `gateway_job_id`. Nakomi consulta con firma:

`GET /wp-json/glory/v1/internal/alerts/{idempotency_key}`

Respuesta:

```json
{
  "idempotencyKey": "chat:...",
  "status": "pending|processing|sent|failed|dead",
  "attempts": 1,
  "sentAt": null
}
```

Nakomi marca `sent` únicamente cuando el gateway reporta `sent`. El worker
WordPress marca `sent` solo si `WacliService` retorna `exitCode=0`. Esto demuestra
aceptación por el cliente local; la recepción humana final se confirma en el
canary de producción.

Los reintentos conservan `idempotency_key`, pero generan timestamp, nonce y
firma nuevos.

### 13.3 Registro de correo

- `email_logs` registra un resultado final por trabajo.
- Los intentos individuales viven en `chat_alert_outbox.attempts/last_error`.
- No insertar múltiples filas `email_logs` por retry del mismo mensaje.
- `email_logs.status=sent` solo después de aceptación SMTP.

### 13.4 Feature flags

```text
CHAT_ALERT_CAPTURE_ENABLED
CHAT_EMAIL_DELIVERY_ENABLED
CHAT_WHATSAPP_DELIVERY_ENABLED
CHAT_REALTIME_V2_ENABLED
```

- `CAPTURE=false`: no crea outbox externa; sí mantiene el chat normal. Se usa
  durante la instalación inicial para impedir backfill accidental.
- Canal delivery `false`: las filas nuevas quedan `paused`, no `failed`.
- Al reactivar un canal pausado, un operador debe elegir explícitamente:
  - reanudar filas con menos de una hora;
  - archivar las antiguas sin enviar.
- Realtime v2 se activa después de que backend y frontend compatibles estén
  desplegados.

### 13.5 Ciclo de escalamiento

Crear `chat_escalations`:

- `id UUID`;
- `session_id`;
- `opened_by_message_id`;
- `status open|resolved`;
- `reason`;
- `cta_message_id`;
- `opened_at`, `resolved_at`, `resolved_by`.

Reglas:

1. Índice único parcial: una escalación `open` por sesión.
2. La tool o fallback IA abre/reutiliza el ciclo abierto.
3. El CTA usa `escalation_id` como idempotency key.
4. Una respuesta humana de admin/freelancer resuelve el ciclo.
5. Una respuesta IA no lo resuelve.
6. Tras resolverlo, una nueva detección puede abrir otro ciclo y CTA.
7. `chat_sessions.is_escalated` refleja si existe un ciclo abierto; no se
   actualiza de forma independiente.

### 13.6 Semántica de 20 minutos

- El ciclo comienza con el **primer mensaje de cliente** después de la última
  respuesta humana.
- Mensajes adicionales del cliente pertenecen al mismo ciclo y no reinician el
  reloj.
- Una respuesta IA no cuenta como atención humana.
- Una respuesta de admin/freelancer cierra el ciclo.
- Sesiones cerradas no generan alertas.
- Sesiones escaladas sí generan alerta si nadie humano respondió.
- El barrido toma filas vencidas con lock; no recalcula solo desde
  `MAX(created_at)` en cada ejecución.

### 13.7 Secuencia Realtime

1. Añadir `next_message_sequence BIGINT` a `chat_sessions`.
2. Al insertar un mensaje:
   - bloquear/actualizar la sesión;
   - incrementar y obtener la secuencia;
   - insertar mensaje con esa secuencia en la misma transacción.
3. Constraint único `(session_id, sequence)`.
4. Snapshot incluye `lastSequence`.
5. Cliente conserva `lastSequence` por sesión.
6. Si recibe una secuencia mayor que `lastSequence + 1`, llama REST con
   `after_sequence=<lastSequence>`.
7. Durante compatibilidad, backend emite v2 solo a clientes que negocien
   `protocol=2`; v1 continúa hasta completar rollout.

Para audio, una pestaña toma liderazgo con Web Locks API. Si no está disponible,
se usa lease en `localStorage` con `ownerId` y expiración corta. La pestaña no
líder inserta mensajes, pero nunca reproduce sonido.

## 14. División exacta en tareas y commits

Cada fila es un commit independiente. No mezclar repositorios en un commit.

| Orden | ID sugerido | Repositorio | Resultado |
|---|---|---|---|
| 1 | `237A-7a` | glorytemplate | Esquema outbox saliente + repositorio + tests de idempotencia. |
| 2 | `237A-7b` | glorytemplate | Endpoint HMAC + nonce + tests de contrato/replay. |
| 3 | `237A-7c` | glorytemplate | Worker saliente + `WacliService` + canary controlado. |
| 4 | `237A-7d` | Nakomi | Migración/modelos/repositorio outbox + transacción de mensaje. |
| 5 | `237A-7e` | Nakomi | Worker SMTP/gateway + consulta de estado + métricas. |
| 6 | `237A-7f` | Nakomi frontend | Runtime global, campanas y badges persistentes. |
| 7 | `237A-7g` | Nakomi | Ciclo de escalamiento + rich message `contact_cta`. |
| 8 | `237A-7h` | Nakomi frontend | Render CTA + pruebas responsive. |
| 9 | `237A-6a` | Nakomi backend | Secuencia, envelope v2, snapshot y reparación de huecos. |
| 10 | `237A-6b` | Nakomi frontend | Realtime v2, dedupe, líder de audio y compatibilidad. |
| 11 | `237A-7i` | Nakomi | Ciclo de 20 minutos y entrega por los tres canales. |

Gates:

- No iniciar `237A-7d` hasta validar `237A-7a..c` y health de `wacli`.
- No iniciar CTA hasta demostrar exactamente una alerta por canal.
- No retirar protocolo v1 hasta validar v2 con widget y chat de pedido.
- No desplegar alertas de 20 minutos hasta validar que el flujo inmediato no
  duplica entregas.
- Pagos/reembolsos empiezan después de cerrar estos gates.
