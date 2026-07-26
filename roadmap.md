Objetivo: Nakomi Studio — sitio web de agencia creativa. Migrado de WordPress a Rust (Axum) + React SPA.
Rama: glory-rust-nakomi

## Stack

| Capa          | Herramienta                    |
| ------------- | ------------------------------ |
| Framework web | Axum 0.7                       |
| OpenAPI       | utoipa 4 + utoipa-swagger-ui 7 |
| Base de datos | SQLx 0.8 (PostgreSQL)          |
| Validación    | validator 0.18                 |
| Auth          | jsonwebtoken + argon2          |
| Frontend      | React 18 + TypeScript + Vite   |
| State         | React Query + Zustand          |
| Codegen       | Orval 8                        |
| Deploy        | coolify-manager-rs             |

# Nakomi Studio — Roadmap

## Notas de infraestructura

- **nakomi.studio**: VPS1 (66.94.100.241), Coolify service `do8k4w8swccwwogoc0os0ck0`
- **VPS2 Coolify**: Configurado en settings.json
- **Deploy**: Siempre via coolify-manager-rs, nunca desde Coolify UI (ver doc de persistencia volúmenes)
- **Volúmenes**: Documentado en `Agente/documentacion/hosting/coolify-volumenes-persistencia-2026-04-12.md`
- **Admin contactos**: correo `andoryyu@gmail.com`, whatsapp `+1 (608) 466-8134`

## Contexto

Proyecto migrado de WordPress a Rust (Axum) + React SPA. El frontend React se integra en frontend/src/. El backend Rust sirve API + SPA.

---

## Tareas de producto — Correo para Hosting (bloqueado en decisión de proveedor)

Ver análisis completo en `Agente/documentacion/hosting/producto-correo-proveedores-2026-05-26.md`.

**Decisión pendiente (bloqueante):** Elegir proveedor — MXroute ($59/año, más barato, sin API) vs Migadu ($9/mes, API REST). Esto define la arquitectura de provisioning.

- **265A-11 — Fase 1: Aliases/reenvíos gratis con Cloudflare Email Routing.**
  - Configurar MX/SPF/DKIM/DMARC del dominio del cliente apuntando a Cloudflare.
  - Solo reenvío a Gmail/Outlook del cliente (sin IMAP/SMTP).
  - Incluir 3 alias en plan Pro, 5 alias en Avanzado.
  - Sin costo operativo para Nakomi.
  - Backend: `POST /api/hosting/{id}/aliases`, `DELETE /api/hosting/{id}/aliases/{alias}`.
  - Frontend: TabCorreo con lista de aliases y estado DNS.
  - ~8-10h estimado.

- **265A-12 — Fase 2: Buzones IMAP (MXroute o Migadu).**
  - Contratar proveedor y configurar cuenta reseller.
  - Implementar provisioning: crear/suspender/eliminar mailbox vía API (Migadu) o automatización panel (MXroute).
  - Modelos BD: `mail_domains`, `mailboxes`, `mail_events`.
  - Backend: CRUD de buzones, reset password, DNS automático.
  - Frontend: TabCorreo completo con indicadores de estado.
  - Billing: Stripe add-on a $1.50/buzón/mes.
  - ~20-26h estimado.

- **265A-13 — Incluir 1 buzón IMAP gratis en plan Avanzado.**
  - Modificar `hosting_plan_configs` (nuevo campo `included_mailboxes`).
  - Actualizar pricing en frontend y catálogo.
  - Stripe: nuevo price para el add-on.
  - ~3-4h estimado.

---

## Estado interno reciente

- `275A-3`: hotfix del listado de backups para WordPress/Coolify. El endpoint fallaba con 500 porque `alpine:3.20` usa BusyBox y no soporta `ls --time-style=long-iso`; ahora el listing usa `ls --full-time`, comprueba la existencia del volumen antes de montarlo y el parser acepta timestamps `HH:MM:SS +0000`. Validado con test unitario nuevo y smoke SSH contra el VPS del hosting de prueba.
- `20CA`: reorganización del roadmap (20 julio 2026). Sus afirmaciones de
  completado requieren aceptación funcional; no equivalen a una entrega
  confirmada en producción.
- `237A-3`: el código de los bloques A-E está en la rama y el despliegue del
  25 julio pasó health, pero los canales externos de alertas permanecen
  desactivados o sin evidencia de entrega. No tratar esos bloques como cerrados.
- `257A`: limpieza de target Cargo (2026-07-25). Límite 15 GB, tarea oculta, sin consola visible.
- Herramientas de incidente en coolify-manager-rs implementadas (2026-07-25): `incident-investigate`, `incident-logs`, `container-inspect/events/stats`, `db-stats`, `env-toggle`, redacción de secretos.

##

## 237A-3 — Estabilidad integral de Nakomi Studio

### Estado funcional verificado — 2026-07-26

Esta sección prevalece sobre etiquetas históricas de “implementado” en planes
anteriores. **Código presente, una migración aplicada o un health 200 no prueban
que la experiencia solicitada funcione para la administradora.**

#### Hecho y desplegado

- Se corrigió el deadlock de desconexión WebSocket que bloqueaba workers Tokio y
  podía producir freeze/Bad Gateway.
- Se corrigió la pérdida del mensaje pendiente de IA durante una reconexión.
- El servicio respondió health 200 tras el último deploy, sin reinicios ni OOM
  en la comprobación posterior.
- La limpieza local de `C:\tmp\glory-target` quedó limitada a 15 GB mediante
  tarea automática oculta.
- `257A-9` está desplegado: toggle o intervención humana incrementan una época
  durable, invalidan respuestas IA en vuelo y cancelan ciclos de fallback.
- `257A-10` está desplegado: Claudia se identifica honestamente como asistente
  de IA y prioriza el escalamiento cuando el cliente quiere profundizar en su
  proyecto, estrategia, alcance o propuesta personalizada.

#### Pendiente crítico — alertas y contacto

1. **Correo inmediato por cada mensaje de cliente:** código, outbox, worker SMTP
   y configuración están desplegados. `CHAT_ALERT_CAPTURE_ENABLED=true` y
   `CHAT_EMAIL_DELIVERY_ENABLED=true`; el arranque confirma SMTP y worker. Falta
   el canary final: mensaje real, correo recibido y estado `sent` en
   `email_logs`/outbox. Hasta esa evidencia no se marca cerrado.
2. **WhatsApp inmediato por cada mensaje y pedido:** el cliente Nakomi existe,
   pero el gateway firmado/worker `wacli` de `glorytemplate` sigue pendiente de
   implementación y canary. Mientras tanto no hay entrega WhatsApp verificable.
   `accepted_by_gateway` tampoco equivale a recepción humana.
3. **Notificación visible y punto rojo:** los componentes y WebSocket global
   existen, pero el contador del sidebar no hace carga inicial fuera del panel;
   solo consume cache/eventos. Falta corregirlo y probar mensaje nuevo en panel,
   fuera del panel, móvil y tras reconexión.
4. **CTA “Escribir por WhatsApp” de la IA:** código y
   `PUBLIC_SUPPORT_WHATSAPP=16084668134` están desplegados. El prompt prioriza
   este CTA para conversaciones profundas de proyecto y ya no oculta que es IA.
   Falta prueba visible móvil/desktop y confirmar que abre el número correcto.

#### Pendiente crítico — comportamiento de chat/IA

5. Repetir prueba real de dos mensajes consecutivos y reconexión/reload; el
   hotfix está desplegado, pero necesita aceptación funcional.
6. Verificar botón de detener IA, precedencia de respuesta humana y fallback
   solo después de 10 minutos. No declarar completo sin carrera humano/worker
   y prueba visible.
7. Verificar que la IA capture email con consentimiento, no lo vuelva a pedir,
   y que el correo de continuación recupere la conversación desde un navegador
   limpio. El código existe; la prueba extremo a extremo no está realizada.
8. Verificar retención: ningún mensaje debe desaparecer por cierre de sesión,
   paginación o reconexión. Falta prueba de conversación antigua y de más de
   100 mensajes.

#### Pendientes de producto previamente solicitados

9. Reembolso: eliminar el `prompt()` administrativo, revisar transición Stripe
   transaccional y conversación admin-cliente.
10. Pagos/órdenes: impedir cuentas sin compra efectiva, comprobar cobros de
    servicios y endurecer idempotencia/transacciones de webhook.
11. Pedidos: confirmar en UI real auto-asignación ilimitada a admin,
    reasignación/cancelación y etiqueta “Freelancer asignado”.
12. CMS: permitir comas para tags; ajustar 10% de resolución de las galerías.
13. Ramas: finalizar restauración/normalización de `main` y migrar consumidores
    de `master`/`dev-launcher-centralizado` a `main` solo tras estabilizar
    Nakomi, según el plan específico.

Planes activos:

- Maestro: `Agente/planes/plan-estabilidad-nakomi-2026-07-23.md`.
- Cierre ejecutable de alertas, WhatsApp, Realtime y bloques difíciles:
  `Agente/planes/plan-cierre-bloques-dificiles-nakomi-2026-07-23.md`.

**Estado histórico (2026-07-23):** Bloques A, B, C, D, E declarados
implementados en backend + frontend. El despliegue posterior está hecho, pero
la aceptación funcional de alertas, correo, WhatsApp, Realtime y flujos IA sigue
pendiente; consultar “Estado funcional verificado” arriba.

Este problema debe ser resuelto por un agente inteligente, todas estas tareas necesita un plan, separado o unido lo que sea mejor, primero investiga en profundida y luego plantea como solucionar todo y yo autorizare o no:

1. Este problema lleva mucho tiempo, uno en el que el sitio de congela, y cae, lo que se ha logrado hacer es que se puede restaurar automáticamente, hay muchos md sueltos y comentarios sobre este problema, realmente no se porque exactamente, pero la pista es el chat, al escribir, vuelve a sonar el sonido de chat como si hubiera respondido y al sonar de nuevo (sin recibir ningún mensaje) se cae el sitio, por supuesto esta pista puede ser útil o despistar. Hay que conciliar todos los detalles, incidentes, md, comentarios en uno solo para entender el contexto.

2. Acabo de darme cuenta que lo de solicitar reembolso es estupido, abre un modal del navegador, debería ser un modal normal, y no debe especificarse el monto, hay que revisar todo el proceso de reembolso para ver si esta funcionando como se espera.

Debo determinar que se espera: no lo se exactamente solo se que debe ser mejor. El cliente solicita el reembolso y el admin (no empleado) eligira si cederlo, tambien tiene que tener la capacidad de conversar con el cliente.

3. Por cierto despues de pedir un reembolso paso esto y fue local, claramente el problema es grave.

2026-07-23T13:21:53.862612Z  INFO glory_backend::handlers::chat::ws_visitor: Chat visitor autenticado user_id=62e40c38-41fe-4d48-86e9-3e6ee0c1dd2e role=admin effective_role=admin impersonator=None
2026-07-23T13:21:53.864930Z  INFO glory_backend::handlers::chat::ws_visitor: WS session obtenida/creada session_id=c2781720-5d91-463f-a1d9-460ced4880d7 visitor_id=undefined
[hb-logger] last_pulse=0 stale=1784812970s
[hb-logger] last_pulse=0 stale=1784812985s

[rt-watchdog] ⚠️  RUNTIME FREEZE DETECTED: sin pulso en 1784812990s
[rt-watchdog] Volcando stacks del kernel...

[rt-watchdog] No se pudo leer /proc/self/task

[rt-watchdog] Forzando exit(1) para restart de Docker...
9:23:10 a.m. [vite] http proxy error: /api/img/assets/Proyectos%20portadas/TaskPortada.jpg?w=1200&q=72&fmt=webp
Error: read ECONNRESET
    at TCP.onStreamRead (node:internal/stream_base_commons:216:20)
9:23:10 a.m. [vite] http proxy error: /api/img/assets/Proyectos%20portadas/GuillermoPortada.jpg?w=1200&q=72&fmt=webp
Error: read ECONNRESET
    at TCP.onStreamRead (node:internal/stream_base_commons:216:20)
9:23:10 a.m. [vite] http proxy error: /api/img/assets/random/85a51ba9a4233272662e744b48f97d67.jpg?w=150&q=80&fmt=webp
Error: read ECONNRESET
    at TCP.onStreamRead (node:internal/stream_base_commons:216:20)
9:23:10 a.m. [vite] http proxy error: /api/img/assets/random/85a51ba9a4233272662e744b48f97d67.jpg?w=1024&q=80&fmt=webp
Error: read ECONNRESET
    at TCP.onStreamRead (node:internal/stream_base_commons:216:20)
error: process didn't exit successfully: `C:\tmp\glory-target\glory_backend_glory_rust_nakomi\debug\glory-backend.exe` (exit code: 1)
[backend] Proceso terminado con codigo 1
[frontend] Proceso terminado con codigo null
[cargo-target-watch] Proceso terminado con codigo null
PS C:\Users\Owner\OneDrive\Documentos\glory-rust-template> 9:23:18 a.m. [vite] http proxy error: /api/img/assets/Proyectos%20portadas/TaskPortada.jpg?w=1200&q=72&fmt=webp
AggregateError [ECONNREFUSED]:
    at internalConnectMultiple (node:net:1134:18)
    at afterConnectMultiple (node:net:1715:7)
9:23:48 a.m. [vite] http proxy error: /api/img/assets/Proyectos%20portadas/GuillermoPortada.jpg?w=1200&q=72&fmt=webp
AggregateError [ECONNREFUSED]:

3. Ya lo habia comentado antes y no se hizo caso, cuando un cliente escribe un mensaje yo no me entero de nada, ni siquiera hay una notificación, no hay un punto rojo en los mensajes, lo de las notificaciones tambien debería estar del lado cuando se esta fuera del panel para ver cuando algo o llegue un mensaje

4. El codigo o front no se esta actualizando en producción con cd "c:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs" ; .\target\release\coolify-manager.exe deploy --name studio --update --skip-backup esto es grave y cambia el panorama completo porque no se si realmente los problemas anteriores (bueno algunos si los vi en local) pero el punto es que producción no esta actualizado, no se si solo el front o incluye al backend, esto cambia la forma de ver la tarea 1, pues hay que ver desde cuando el backend no esta actualizado.

5. Voy a comentarte lo que le pide a otro agente anterior un poco tonto, hay que revisar si hizo todo bien

"Ve un problema, automaticamente cuando se haga un pedido, tiene que asigarse a mi, no importa que ya tenga pedidos asignados, no hay limite para el administrador

el modal para asignar un empleado se ve mal no se porque no es ve como los otros modales

hay un problema, no veo que despues de que tenga una orden asignada no pueda cancelar el pedido, o cambiar el empleado

Donde dice "Empleado asignado" debería de decir "Freelancer asignado"

otro problema grave es la cuestion de que el chat en los pedidos no funciona en tiempo real, no hubo una notificación a mi cuando probe enviar un mensaje como cliente, tambien debería llegar un correo cuando un mensaje pasa 20 minutos sin responderse, y debería mostrar un punto rojo cuando hay mensajes nuevos en el boton de sidebar de mensajes

hay un problema visual con las notificaciones, el texto esta centrado, no debería

otra cosa es que veo que los correos estan duplicados en el codigo para los envio y preview ¿porque? me parece mal a nivel codigo, deberia estar centralizado en plantillas, a demás de que se esta duplicando codigo innecesario

--------------

## 20/07

Ha pasado algo de tiempo con el proyecto inactivo, necesito confirmar varias cosas.

Comprobar que en nakomi los pagos funcionen: comprobe, que ya no hay el problema de antes sobre de que sin pago se creaban las ordenes, bien, ya no se crean ordenes sin pagos, pero, se crean cuentas sin ordenes, eso no debería de pasar, que no se creen cuentas al menso que se haya hecho el pago del servicio; tambien neecesitamos comprobar que los pagos de servicios funcionan como esperan, no he tenido mi primer pago de servicio asi que no puedo saber aun si realmente funciona.

Algunos detalles más

Comprobar que el chat funciona bien, que el bot redirige al whatsapp, y que cada vez que haya un conversación me llegue un correo y un whatsapp, mi correo es andoryyu@gmail.com y mi whatsapp es +1 (608) 466-8134, esto es importante ya no quiero que las cosas sucedan a ciega, tambien debe llegarme un whatsapp y un correo cuando se haga un pedido, lo de los correo creo que ya funcionaba pero hay verificar que siga funcionando.

Subir un poco la resolucion a galeriaHeroContenedor y a proyectoGaleriaItem, un 10% mas

En el gestor de contenido Nakomi no puedo agregar comas, lo que impide pues crear varios tag y cosas, mal ahi"
