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

