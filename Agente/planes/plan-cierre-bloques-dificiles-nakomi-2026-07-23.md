# Plan de cierre de bloques difíciles — Nakomi Studio

> **Fecha:** 2026-07-23  
> **Estado:** Nakomi desplegado 2026-07-26: correo/captura activos y CTA
> configurado; faltan canary de recepción, prueba visual del CTA y gateway
> WhatsApp automático en glorytemplate.
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

Cuando una persona del equipo responda:

1. La IA deja de contestar inmediatamente.
2. Los siguientes mensajes del cliente esperan hasta diez minutos por una
   respuesta humana.
3. Si admin/freelancer responde dentro del plazo, el trabajo IA se cancela.
4. Si nadie responde, la IA cubre la conversación usando todos los mensajes
   acumulados, sin duplicar respuestas.
5. Una pausa manual mediante el botón es absoluta: nunca se reactiva por tiempo.

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

### No está listo (residual post-implementación)

- Gateway WhatsApp en glorytemplate no implementado (worker + endpoint HMAC + wacli). Nakomi ya envía a outbox; falta el receptor.
- Rate limiting en endpoint de claim de tokens de continuación.
- Tests de aceptación E2E con proveedor IA simulado (capture_email tool call real).
- Frontend: `ModalAsignar` tiene error preexistente TS6133 (variable sin usar).
- Re-elección de audio leader si la pestaña líder se cierra (Web Locks no notifica).

### Ya implementado (2026-07-23)

- 🟡 Bloque A: Outbox, worker SMTP y flags desplegados; captura/correo activos,
  pero falta demostrar recepción. Cliente gateway presente; endpoint WordPress pendiente.
- 🟡 Bloque B: CTA, ciclo y número público desplegados; prompt escala también
  conversaciones profundas de proyecto y revela identidad IA; falta prueba visual.
- ✅ Bloque C: Secuencia monotónica (`next_message_sequence`), campo `delivery` (live/history), `from_chat_message` helper, dedupe de sonido por delivery+sender+messageId+audio leader (Web Locks API + localStorage fallback).
- ✅ Bloque D: `ai_mode` (automatic/human_priority/manual_pause), `chat_response_cycles`, `response_cycle_worker` (FOR UPDATE SKIP LOCKED, fallback 10 min, mantiene human_priority), toggle_ai sincroniza ai_mode, staff envía → human_priority.
- ✅ Bloque E: `is_valid_email` (RFC 5322 simplificada), `exec_capture_email` con validación real + `email_normalized` + `email_captured_at` + `continuation_consent_at` + `email_source='chatbot'`, token de continuación (`chat_continuation_tokens`, SHA-256, un uso, 7 días), handler `POST /api/chat/continuation/claim`, template `render_chat_continuation`, `send_chat_continuation`.

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

## 7. Bloque D — Toma humana y fallback IA a los 10 minutos

**Prioridad:** después de alertas inmediatas
**Dificultad:** alta

La alerta externa de cada mensaje ya es inmediata. Por eso se elimina la alerta
adicional de 20 minutos: sería tardía y duplicaría correo/WhatsApp. El timeout se
usa para que la IA cubra al equipo, no para volver a notificar.

Estados:

```text
ai_active
human_priority
manual_pause
waiting_human
ai_fallback_claimed
```

Reglas:

1. Una respuesta de admin/freelancer cambia la sesión a `human_priority`.
2. Si llega un mensaje de cliente:
   - `ai_active`: la IA responde con el timing conversacional normal;
   - `human_priority`: crear ciclo `waiting_human` con deadline a diez minutos;
   - `manual_pause`: no crear trabajo IA;
   - sesión cerrada: rechazar escritura o reabrir según la política existente.
3. Mensajes adicionales del cliente se agregan al ciclo abierto; no crean timers.
4. Una respuesta humana antes del deadline cancela el ciclo en la misma
   transacción que persiste la respuesta.
5. Al vencer:
   - worker reclama el ciclo con `FOR UPDATE SKIP LOCKED`;
   - relee mensajes y estado;
   - si ya hubo respuesta humana, cancela;
   - si continúa pendiente, genera una respuesta IA con el buffer completo.
6. Tras el fallback, la sesión continúa en `human_priority`: el siguiente mensaje
   del cliente abre otra espera de diez minutos.
7. El botón manual usa `manual_pause` y solo otro clic puede volver a
   `ai_active`/`human_priority`.
8. No usar `tokio::sleep(600)` por sesión. El deadline vive en BD y un worker lo
   reclama, para sobrevivir reinicios.

Pruebas:

- A los 9:59 minutos la IA no responde.
- A los 10 minutos responde una sola vez si no hubo humano.
- Respuesta humana a los 9:59 cancela el trabajo.
- Carrera exacta en el deadline produce una sola respuesta: humana o IA.
- Tres mensajes del cliente generan un ciclo y una respuesta combinada.
- Reiniciar el backend no pierde el deadline.
- `manual_pause` permanece pausado después de 10 minutos y tras reinicio.

## 8. Bloque E — Captura de email y continuación de conversación

**Prioridad:** junto al CTA/escalamiento
**Dificultad:** alta

### Estado actual

- `capture_email` existe y guarda en `visitor_profiles`.
- El prompt sugiere pedirlo después de 2–3 intercambios productivos.
- La validación actual solo comprueba que contenga `@`; es insuficiente.
- No existe prueba que demuestre que el modelo llama realmente la tool.
- No existe correo de continuación ni enlace para recuperar una conversación.

### Captura

1. Para usuarios autenticados, usar el email verificado de su cuenta; no volver a
   pedirlo en chat.
2. Para visitante anónimo:
   - pedir nombre en la primera/segunda interacción cuando sea natural;
   - pedir email después de la primera ayuda útil y antes de cerrar/escalar;
   - explicar: “Puedo enviarte un enlace para continuar esta conversación”.
3. Cuando el visitante entregue un email, la IA debe llamar `capture_email`.
4. Validar con un parser de email real, normalizar y limitar longitud.
5. Guardar:
   - email normalizado;
   - `email_captured_at`;
   - `continuation_consent_at`;
   - origen `chat_ai|authenticated_account`;
   - sesión en la que se obtuvo.
6. La tool retorna éxito solo después de releer el perfil persistido.
7. No escribir emails completos en logs.

### Email para continuar

No enviar inmediatamente por cada cierre de WebSocket: móviles y redes producen
desconexiones breves. Crear un ciclo durable:

1. Al desconectar:
   - sesión abierta;
   - email conocido/consentido;
   - último mensaje relevante del cliente o respuesta pendiente;
   - crear deadline con gracia de dos minutos.
2. Si el mismo visitante reconecta antes del deadline, cancelar.
3. Si sigue desconectado, encolar un correo único por ciclo.
4. El correo contiene un enlace firmado de un solo propósito:
   `https://nakomi.studio/chat/continuar?t=<token>`.
5. Guardar únicamente el hash del token.
6. Token:
   - aleatorio criptográficamente;
   - expira en siete días;
   - un solo uso;
   - ligado a sesión y destinatario;
   - revocable al cerrar conversación.
7. Al canjearlo:
   - validar hash, expiración y uso;
   - emitir credencial corta específica de chat;
   - abrir el widget con la misma sesión;
   - nunca exponer `visitor_id` o JWT administrativo en URL.
8. Rate limit por IP/email/sesión.
9. Plantilla central `chat_continuation` y registro en `email_logs`.
10. No enviar si:
    - email no fue consentido;
    - sesión cerrada;
    - cliente volvió;
    - ya se envió en ese ciclo;
    - no existe actividad pendiente.

Pruebas:

- Conversación guiada hace que la IA solicite email y llame la tool.
- Email inválido no se persiste.
- Email capturado sobrevive recarga.
- Desconexión de 30 segundos no envía correo.
- Desconexión superior a dos minutos envía uno.
- Reconexión cancela el trabajo.
- Token válido abre la conversación correcta.
- Token usado/expirado/manipulado se rechaza.
- Ningún log o URL revela email, `visitor_id` o token almacenado.

## 9. Bloques difíciles posteriores

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

## 10. Orden seguro de implementación y despliegue

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
10. Implementar captura verificada de email y continuación segura.
11. Implementar contrato Realtime, toma humana y fallback IA a diez minutos.
12. Endurecer pagos, reembolsos y asignación en commits separados.

Todos los deploys, health, logs y operaciones de producción pasan por `coolify-manager-rs`.

## 11. Rollback y control de incidentes

- `CHAT_ALERT_CAPTURE_ENABLED=false` detiene nuevas filas externas sin borrar la
  outbox existente.
- `CHAT_EMAIL_DELIVERY_ENABLED=false` pausa solo correo.
- `CHAT_WHATSAPP_DELIVERY_ENABLED=false` pausa solo WhatsApp.
- El gateway puede rechazar temporalmente con `503`; Nakomi reintenta.
- Nunca reproducir mensajes históricos al habilitar el sistema.
- Una migración no elimina filas de chat o notificaciones.
- Dead-letter se conserva para inspección y reenvío manual idempotente.

## 12. Criterio de cierre

El bloque no se considera listo solo porque compile. Deben existir evidencias reales de:

- mensaje desde cliente por WS y por REST;
- badge inmediato y persistente;
- correo recibido;
- WhatsApp recibido;
- CTA de WhatsApp abierto por el cliente;
- dos pestañas sin sonido duplicado;
- reconexión sin replay;
- respuesta humana silencia IA y fallback único ocurre a los 10 minutos;
- botón manual no se reactiva solo;
- email capturado y enlace de continuación canjeable de forma segura;
- health y logs limpios tras deploy.

## 13. Guía operativa para agentes implementadores

Esta sección elimina decisiones implícitas. El agente que tome una fase debe
seguir el orden indicado, modificar solo los archivos de su tarjeta y detenerse
ante cualquiera de las condiciones de parada.

### 13.1 Reglas que no se pueden reinterpretar

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

### 13.2 Mapa de archivos — Nakomi Rust

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
| Toma humana | `src/services/chat_timing.rs`, `src/services/chat.rs`, `src/repositories/chat.rs` | Reemplazar timers en memoria por ciclos durables de respuesta. |
| Toggle IA | `src/models/chat.rs`, `src/handlers/chat/ws_staff.rs`, `frontend/src/hooks/useChatWs.ts` | Unificar `enabled`; agregar ACK/rollback y modo manual explícito. |
| Captura email | `src/services/ai_prompts.rs`, `src/services/ai_tools.rs`, `src/repositories/chat.rs` | Validación real, consentimiento y verificación post-write. |
| Continuación | migración, servicio/token, handler dedicado y email template | Token hasheado, un uso, expiración y reconexión a la sesión exacta. |

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

### 13.3 Mapa de archivos — gateway glorytemplate

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

### 13.4 Tarjeta A1 — Migración y dominio de alertas

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

### 13.5 Tarjeta A2 — Worker Nakomi

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

### 13.6 Tarjeta A3 — Gateway firmado

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
5. `event` permitido inicialmente: solo `chat.client_message`.
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

### 13.7 Tarjeta A4 — Notificación visible

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

### 13.8 Tarjeta B — CTA de escalamiento

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

### 13.9 Tarjeta C — Realtime

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

### 13.10 Matriz de pruebas y evidencias

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
| Humano responde | ciclo cancelado | modo humano | ninguno extra | ninguno extra | IA silenciosa |
| Humano tarda 10 min | claim único | estado fallback | alerta inmediata ya existente | alerta inmediata ya existente | una respuesta IA |
| Pausa manual | `manual_pause` | botón confirmado | normal | normal | ninguna IA |
| Email + desconexión | ciclo + token hash | retorno al mismo chat | continuación única | no aplica | reconexión segura |

Cada evidencia debe incluir:

- ID de mensaje;
- claves idempotentes;
- estados de outbox;
- registro `email_logs`;
- resultado enmascarado de `wacli`;
- captura de badge/CTA;
- logs sin secretos.

### 13.11 Condiciones de parada obligatoria

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

### 13.12 Formato de entrega de cada agente

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

## 14. Decisiones cerradas para evitar interpretaciones

### 14.1 Destinatarios y precedencia

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

### 14.2 Estados entre Nakomi y glorytemplate

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

### 14.3 Registro de correo

- `email_logs` registra un resultado final por trabajo.
- Los intentos individuales viven en `chat_alert_outbox.attempts/last_error`.
- No insertar múltiples filas `email_logs` por retry del mismo mensaje.
- `email_logs.status=sent` solo después de aceptación SMTP.

### 14.4 Feature flags

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

### 14.5 Ciclo de escalamiento

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

### 14.6 Modos IA y precedencia humana

Estado actual que debe corregirse:

- Frontend envía `enable`, backend deserializa `enabled`; el botón puede mostrar
  un cambio optimista aunque el servidor haya rechazado el payload.
- El handler WS descarta el JSON inválido sin ACK de error. El toggle actual no
  cancela buffers, timers ni una generación IA que ya está en vuelo.
- `generate_ai_response` exige `assigned_staff_id IS NULL`. Asignar/tomar una
  sesión puede apagar de hecho la IA sin representar la política de diez
  minutos.
- Responder como humano no tiene una transición uniforme: en chat general no
  detiene la IA; en órdenes solo el empleado asignado desactiva la
  intermediación, mientras el admin puede responder sin hacerlo.
- El toggle de órdenes debe comprobar asignación: hoy un empleado no asignado
  puede intentar cambiar el modo. La intervención humana tampoco limpia
  consistentemente `is_escalated`.
- Los timers actuales viven en tareas Tokio y se pierden al reiniciar.

Modelo nuevo:

- `ai_mode = automatic|human_priority|manual_pause`;
- `assigned_staff_id` solo controla routing/autorización;
- `chat_response_cycles` guarda:
  - `id`, `session_id`, `opened_by_message_id`;
  - `first_client_message_at`, `deadline_at`;
  - `status waiting|claimed|answered_human|answered_ai|cancelled`;
  - `claimed_at`, `answered_message_id`;
  - constraint único para un ciclo abierto por sesión.

Precedencia:

1. `manual_pause` gana sobre todos los timers.
2. Respuesta humana gana sobre un worker aún no reclamado.
3. Si worker y humano compiten, ambos bloquean el mismo ciclo:
   - humano primero: `answered_human`, worker aborta;
   - worker primero: marca `claimed`, relee mensajes; la UI indica que IA está
     preparando respuesta;
   - antes de persistir respuesta IA, revalidar que no apareció respuesta humana.
4. Activar manualmente IA cancela ciclos abiertos y pasa a `automatic`.
5. Desactivar manualmente cancela ciclos y pasa a `manual_pause`.
6. Toda respuesta de admin o empleado autorizado, por WS o REST, resuelve el
   escalamiento abierto y cambia a `human_priority` en la misma transacción que
   persiste el mensaje.
7. Autorización: admin/supervisor puede cambiar cualquier sesión; un empleado
   solo la orden que tiene asignada; visitante nunca puede cambiar el modo.
8. Cada cambio de modo incrementa una `ai_generation_epoch`. Buffers, timers y
   generaciones capturan esa época y abortan si cambia. Antes de guardar o
   publicar una respuesta IA se releen época, modo y ciclo; no basta cancelar
   una tarea local.

Contrato toggle:

```json
{"type":"toggle_ai","session_id":"uuid","enabled":false,"request_id":"uuid"}
```

Servidor responde:

```json
{
  "type":"ai_mode_changed",
  "session_id":"uuid",
  "mode":"manual_pause",
  "enabled":false,
  "request_id":"uuid"
}
```

El frontend cambia el estado definitivo solo con ACK. Puede mostrar loading,
pero debe revertir y mostrar toast si hay error/timeout. El backend emite el ACK
solo después de persistir el modo y cancelar ciclos/buffers asociados. Añadir
tests de contrato JSON, autorización, rollback, cambio durante `waiting`, cambio
durante generación IA en vuelo y respuesta humana durante upload/procesamiento.

### 14.7 Captura de email y consentimiento

La instrucción del prompt no demuestra que la tool se ejecute. Deben existir
tests de conversación con un proveedor IA simulado que verifiquen:

1. visitante sin email recibe la pregunta después de una interacción útil;
2. al responder con email, el siguiente tool call es `capture_email`;
3. la tool persiste y relee el valor;
4. el prompt de turnos posteriores contiene “email ya conocido” y no lo repite;
5. visitante que rechaza no vuelve a ser presionado durante esa sesión.

Antes de esos tests deben corregirse los defectos de persistencia ya
confirmados:

- ambos caminos, WS y REST, hacen upsert de `visitor_profiles` antes de ejecutar
  tools; `capture_email` no puede depender de que otro flujo haya creado la fila;
- `save_client_info(name)` actualiza solo el nombre y nunca escribe `email = ""`;
- la escritura devuelve error explícito si falla; no se descarta el resultado;
- logs registran como máximo dominio/hash y nunca la dirección completa;
- unificar el nombre de la variable SMTP documentada con la que consume el
  servicio antes de probar el correo de continuación.

Añadir a `visitor_profiles`:

- `email_normalized`;
- `email_captured_at`;
- `continuation_consent_at`;
- `continuation_declined_at`;
- `email_source`;
- constraint case-insensitive según la política de perfiles.

No inferir consentimiento solo porque aparece una dirección en texto citado,
adjunto o contenido de terceros. La IA debe explicar el propósito y recibir una
respuesta afirmativa o el propio email en respuesta a esa solicitud.
La prueba de continuidad debe abrir el enlace en un navegador limpio, sin el
`localStorage` original: reutilizar el mismo navegador no demuestra recuperación
por email ni entre dispositivos.

### 14.8 Secuencia Realtime

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

## 15. División exacta en tareas y commits

Cada fila es un commit independiente. No mezclar repositorios en un commit.

| Orden | ID sugerido | Repositorio | Resultado | Estado |
|---|---|---|---|---|
| 1 | `237A-7a` | glorytemplate | Esquema outbox saliente + repositorio + tests de idempotencia. | ⏳ Pendiente (gateway glorytemplate) |
| 2 | `237A-7b` | glorytemplate | Endpoint HMAC + nonce + tests de contrato/replay. | ⏳ Pendiente (gateway glorytemplate) |
| 3 | `237A-7c` | glorytemplate | Worker saliente + `WacliService` + canary controlado. | ⏳ Pendiente (gateway glorytemplate) |
| 4 | `237A-7d` | Nakomi | Migración/modelos/repositorio outbox + transacción de mensaje. | ✅ Implementado |
| 5 | `237A-7e` | Nakomi | Worker SMTP/gateway + consulta de estado + métricas. | ✅ Implementado |
| 6 | `237A-7f` | Nakomi frontend | Runtime global, campanas y badges persistentes. | ✅ Implementado (AuthenticatedNotificationRuntime + NotificationBell en HeaderPanel) |
| 7 | `237A-7g` | Nakomi | Ciclo de escalamiento + rich message `contact_cta`. | ✅ Implementado |
| 8 | `237A-7h` | Nakomi frontend | Render CTA + pruebas responsive. | ✅ Implementado (ChatWidget render contact_cta) |
| 9 | `237A-6a` | Nakomi backend | Secuencia, envelope v2, snapshot y reparación de huecos. | ✅ Implementado |
| 10 | `237A-6b` | Nakomi frontend | Realtime v2, dedupe, líder de audio y compatibilidad. | ✅ Implementado |
| 11 | `237A-6c` | Nakomi | Modos IA, ACK del botón y ciclo durable de fallback a 10 minutos. | ✅ Implementado |
| 12 | `237A-7i` | Nakomi | Captura/consentimiento de email y tests de tool call. | 🟡 Implementado parcialmente; `267A-2` separa nombre/email y elimina fallos silenciosos, faltan tests conversacionales/consentimiento explícito |
| 13 | `237A-7j` | Nakomi | Token y correo de continuación tras desconexión real. | ⏳ Piezas backend aisladas; faltan disparador durable, cancelación por reconexión, enlace consumible en frontend y E2E en navegador limpio |

Gates:

- No iniciar `237A-7d` hasta validar `237A-7a..c` y health de `wacli`.
- No iniciar CTA hasta demostrar exactamente una alerta por canal.
- No retirar protocolo v1 hasta validar v2 con widget y chat de pedido.
- No activar fallback de diez minutos hasta probar la carrera humano/worker y
  el botón `manual_pause`.
- No activar correo de continuación hasta probar consentimiento, cancelación por
  reconexión y token de un solo uso.
- Pagos/reembolsos empiezan después de cerrar estos gates.
