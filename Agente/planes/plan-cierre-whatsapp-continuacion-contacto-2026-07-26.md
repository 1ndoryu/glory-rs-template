# 267A-3 — Cierre de WhatsApp y continuidad de conversación

> **Fecha:** 2026-07-26  
> **Estado:** En ejecución  
> **Prioridad:** Crítica  
> **Destino administrativo:** email `andoryyu@gmail.com`, WhatsApp `+1 608 466 8134`  
> **Repositorios:** Nakomi (`glory-rs-template`) y gateway reutilizable (`glorytemplate`)

## 1. Resultado obligatorio

1. Cada mensaje nuevo de cliente genera una alerta inmediata por email y otra
   por WhatsApp, con idempotencia y trazabilidad separadas por canal.
2. La IA solicita naturalmente nombre y correo, los conserva sin sobrescribirlos
   y no vuelve a pedir datos conocidos.
3. Cuando el visitante consiente recibir seguimiento y permanece desconectado,
   recibe una sola invitación por email para retomar la conversación.
4. El enlace funciona en un navegador limpio: canjea un token de un uso,
   restaura `visitor_id`/sesión, abre el widget y carga el historial real.
5. Para contacto directo se usa el CTA a WhatsApp. Responder directamente a un
   email no forma parte de este cierre porque SMTP es saliente y no existe un
   proveedor/webhook de correo entrante; no debe simularse como disponible.

## 2. Invariantes de seguridad y estabilidad

- No crear un cliente WhatsApp adicional ni compartir el store de `wacli` entre
  contenedores. Nakomi llama a un gateway interno firmado de `glorytemplate`.
- Firma HMAC-SHA256 sobre método, ruta, timestamp, nonce y hash del body;
  tolerancia temporal limitada, nonce de un uso e idempotency key obligatoria.
- El número de destino se toma de entorno y se normaliza a E.164 sin `+`:
  `16084668134`. Ningún secreto o número completo aparece en logs de error.
- PostgreSQL es la fuente de verdad. No usar timers Tokio por WebSocket ni
  depender de memoria para la continuación; reinicios y reconexiones deben ser
  seguros.
- El token de continuación en claro solo existe en memoria mientras se prepara
  el email. En BD se conserva únicamente SHA-256, con expiración de siete días,
  un solo uso y revocación.
- Una reconexión antes de la entrega cancela el seguimiento pendiente. Un nuevo
  ciclo de desconexión obtiene una idempotency key diferente.
- Todos los I/O fallidos producen estado visible (`pending/failed/dead`) y logs
  útiles; nunca se marca `sent` con solo aceptar una solicitud HTTP.

## 3. Fase A — Contrato WhatsApp en `glorytemplate`

1. Reutilizar `WacliAlertService` y la sesión multiusuario ya instalada.
2. Crear `POST /wp-json/glory/v1/internal/alerts` en un módulo aislado:
   - leer body crudo;
   - validar timestamp y firma con comparación constante;
   - reclamar nonce de forma atómica con TTL;
   - validar versión, evento, destinatario, texto, URL e idempotency key;
   - insertar/upsert en una outbox local antes de responder;
   - retornar `202` solo cuando la fila quedó persistida.
3. Worker local reclama con `FOR UPDATE SKIP LOCKED` o mecanismo equivalente,
   envía con `wacli`, registra el identificador/respuesta real y aplica backoff.
4. Estados mínimos: `pending`, `processing`, `sent`, `failed`, `dead`.
5. Añadir health/consulta de estado para que Nakomi pueda distinguir
   `accepted_by_gateway` de `sent`.
6. Canary: alerta controlada recibida físicamente en `+1 608 466 8134`, una sola
   vez, y evidencia de estado final `sent`.

## 4. Fase B — Activación WhatsApp en Nakomi

1. Configurar por entorno:
   - `CHAT_WHATSAPP_DELIVERY_ENABLED=true`;
   - URL interna del gateway;
   - secreto HMAC compartido;
   - destinatario `16084668134`.
2. Mantener la outbox por mensaje ya existente. El worker llama al gateway con
   timeout corto y reintentos; 4xx de contrato van a `dead`, red/429/5xx reintentan.
3. Añadir reconciliación de estado remoto o callback firmado para convertir
   `accepted_by_gateway` en `sent` únicamente tras entrega confirmada.
4. Probar duplicado, replay, timestamp vencido, gateway caído y recuperación.

## 5. Fase C — Seguimiento durable por email

1. Añadir presencia durable a `chat_sessions` (`visitor_disconnected_at`).
   - última conexión cerrada: guardar timestamp;
   - nueva conexión: ponerlo en `NULL`;
   - varias pestañas: solo la última desconexión inicia el ciclo.
2. El worker de outbox inserta `chat.continuation` cuando:
   - desconectado por al menos dos minutos;
   - sesión activa con mensajes;
   - email normalizado y consentimiento vigentes;
   - no existe fila para esa sesión + timestamp de desconexión.
3. Al procesar la fila, volver a comprobar que sigue desconectado. Generar el
   token en memoria, persistir solo su hash y enviar la plantilla existente.
4. Si SMTP falla, revocar el token no entregado y reintentar con backoff. Si el
   visitante reconectó, cancelar la fila sin enviar.
5. Registrar `email_logs.template = chat_continuation` y conservar trazabilidad.

## 6. Fase D — Restauración frontend

1. URL: `https://nakomi.studio/?chat_continuation=<token>`.
2. `ChatWidget` detecta el parámetro, llama una vez a
   `POST /api/chat/continuation/claim` y nunca lo imprime en logs.
3. En éxito:
   - reemplazar de forma atómica la identidad anónima local con `visitor_id` y
     `session_id` retornados;
   - limpiar mensajes cacheados de otra conversación;
   - retirar el token de la URL mediante `history.replaceState`;
   - abrir el widget y conectar; el backend entrega el historial.
4. En token inválido/expirado/usado, mostrar feedback visible y retirar el token;
   no crear silenciosamente una conversación que parezca la original.

## 7. Pruebas y gates

- Rust: formato del bloque, `SQLX_OFFLINE=true cargo check`, clippy y tests.
- Frontend: `npx tsc --noEmit` y prueba visible desktop/móvil.
- Contratos: firma válida, firma inválida, replay, idempotencia y retries.
- Contacto: nombre→email y email→nombre conservan ambos valores.
- Continuación: desconexión corta/reconexión no envía; desconexión >2 min envía
  una vez; reinicio del worker no duplica; navegador limpio restaura historial.
- Producción: backup, deploy mediante `coolify-manager-rs`, health, cero OOM,
  canary de email de continuación y WhatsApp físicamente recibidos.

## 8. Estado por bloque

| Bloque | Estado |
|---|---|
| Correo inmediato al admin | ✅ Recibido por la usuaria |
| Captura nombre/email sin pérdida | ✅ Código y producción |
| Gateway WhatsApp firmado | ⏳ En auditoría/implementación |
| WhatsApp automático al admin | ⏳ Depende del gateway y canary físico |
| Scheduler durable de continuación | ⏳ Pendiente |
| Restauración frontend desde token | ⏳ Pendiente |
| Correo entrante/reply-to-chat | Fuera de alcance hasta elegir proveedor inbound |
