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

> **Última verificación:** 2026-07-27 — todo verificado contra código fuente real.
> **Rama:** `glory-rust-nakomi`
> **Producción:** https://nakomi.studio
> **Admin:** `andoryyu@gmail.com`, WhatsApp `+1 (608) 466-8134`

## Notas de infraestructura

- **nakomi.studio**: VPS1 (66.94.100.241), Coolify service `do8k4w8swccwwogoc0os0ck0`
- **VPS2 Coolify**: Configurado en settings.json
- **Deploy**: Siempre via coolify-manager-rs, nunca desde Coolify UI
- **Volúmenes**: Documentado en `Agente/documentacion/hosting/coolify-volumenes-persistencia-2026-04-12.md`

---

## ✅ Verificado completo — código corregido y desplegado (hasta 26 julio)

Lo siguiente está **confirmado en el código fuente actual** (no solo declarado):

| Componente                      | Verificado | Detalle                                                                                                                                  |
| ------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Watchdog freeze                 | ✅         | `last_pulse=0` no mata. Secuencia monotónica. `src/main.rs` usa `spawn_runtime_watchdog`                                                 |
| SQL ON CONFLICT notificaciones  | ✅         | `notification.rs:54` incluye `WHERE reference_type IS NOT NULL AND reference_id IS NOT NULL`                                             |
| SQL ON CONFLICT response cycles | ✅         | `response_cycle.rs:118` usa `ON CONFLICT (session_id) WHERE status = 'waiting' DO NOTHING`                                               |
| Feature flags fail-closed       | ✅         | `chat_alert.rs` `enabled_flag_value()` solo acepta `"true"`/`"1"`. Ausente = desactivado                                                 |
| Error propagation en alertas    | ✅         | `send_message_with_alerts` usa `?` en create*tx e insert_tx. Sin `let * =` críticos                                                      |
| Persistencia chat               | ✅         | Sesiones de pedido no expiran. Cleanup solo anónimas. `/reset` archiva                                                                   |
| Realtime / sonido               | ✅         | `sequence_num` monotónico, campo `delivery` (live/history), dedupe por audio leader (Web Locks)                                          |
| Alertas email outbox            | ✅         | Outbox `chat_alert_outbox`, worker SMTP, backoff 5s→30s→2m→10m→30m                                                                       |
| Alertas WhatsApp                | ✅         | Gateway HMAC en glorytemplate, outbox separada, worker `wacli`, timer systemd 5s                                                         |
| Captura contacto                | ✅         | Nombre y email separados, no se sobrescriben mutuamente                                                                                  |
| Continuación email              | ✅         | Token SHA-256 un uso, 7 días, scheduler durable, coordinador frontend                                                                    |
| CTA WhatsApp                    | ✅         | `wa.me/<numero>` con texto prellenado, render seguro `<a target="_blank">`                                                               |
| Toma humana / fallback IA       | ✅         | `ai_mode`, ciclos de respuesta, worker `response_cycle_worker` fallback 10 min                                                           |
| Identidad IA transparente       | ✅         | Claudia se identifica como asistente IA, prioriza escalamiento                                                                           |
| ModalAsignar                    | ✅         | `orderNumber` se usa en JSX. Sin error TS6133                                                                                            |
| NotificationBell                | ✅         | Montado en `HeaderPanel`. `AuthenticatedNotificationRuntime` global en `App.tsx`                                                         |
| ChatBell                        | ✅         | Badge de mensajes no leídos por `last_message_at > last_viewed_at`                                                                       |
| Sidebar badge mensajes          | ✅         | Indicador rojo en "Mensajes" del sidebar (desktop + móvil)                                                                               |
| Coolify Manager incident tools  | ✅         | 7 comandos (`incident-investigate`, `container-inspect/events/stats`, `db-stats`, `env-toggle`, `incident-logs`) + redacción de secretos |
| CMS comas/tags                  | ✅         | Permite comas en inputs de tags                                                                                                          |
| Galería resolución +10%         | ✅         | `galeriaHeroContenedor` y `proyectoGaleriaItem`                                                                                          |
| Renombrar Empleado→Freelancer   | ✅         | Global backend + frontend                                                                                                                |
| Auto-asignar admin post-pago    | ✅         | Webhook asigna admin automáticamente                                                                                                     |
| Accounts sin pago               | ✅         | Registro movido a post-pago (Stripe webhook)                                                                                             |

**Evidencia de producción (26 julio):** Health 200, restart_count=0, OOM=false, 69 migraciones. Correo recibido por la usuaria. WhatsApp canary `sent` recibida físicamente.

---

## 🔴 Pendiente crítico (esperando autorización)

### 1. Watchdog: doble señal — IMPLEMENTADO, pendiente deploy

**Estado:** Implementado (277A-6). Doble señal (heartbeat + HTTP probe loopback), umbral 120s, grace period 60s. Tests pasan (8/8). Commits: `a944de64` (main.rs) + `4d95fb4` (framework).
**Pendiente:** Deploy a producción + verificación canary.

### 2. Deploy producción: verificar estado real

**⛔ No ejecutar deploy sin autorización del usuario. Ya se había arreglado previamente.**
**Acción necesaria (solo diagnóstico, sin deploy):**
1. Ejecutar `coolify-manager-rs health --name studio` para obtener estado
2. Comparar commit local (`git rev-parse HEAD`) con el desplegado
3. Reportar al usuario

### 3. Hotfix SQL (ya corregido en código, pendiente de confirmar en producción)

**Nota:** Los bugs ON CONFLICT que estaban en el plan de incidente **ya están corregidos** en el código fuente. Sin embargo, no se puede confirmar que la versión desplegada en producción los incluye.

---

## ✅ Completado 27 julio 2026 (277A)

| Tarea | Fix | Archivos |
|---|---|---|
| Badge notificaciones fuera del panel | NotificationBell lazy-loaded en Header público para admin | `frontend/src/components/layout/Header.tsx` |
| Carga inicial badge sidebar | `useNotifications()` reemplaza query `enabled: false` | `frontend/src/components/panel/SidebarPanel.tsx` |
| Deduplicación unanswered_messages | HashMap<session_id, Instant> con cooldown 30min + cleanup 2h | `src/main.rs` |
| Sesiones cerradas en listado admin | `list_sessions` filtra `status != 'closed'`; método `_archived` separado | `src/repositories/chat.rs` |
| Rate limiting continuation claim | 5 intentos/min por IP con LazyLock<Mutex<HashMap>> | `src/handlers/chat/rest.rs` |
| **Watchdog doble señal** | Heartbeat + HTTP probe loopback, umbral 120s, grace 60s | `glory-rs/backend/src/runtime/watchdog.rs`, `src/main.rs` |
| **Pagos/reembolsos (Fase F+G)** | Idempotency keys, retry con backoff, constraint único, validación monto | 9 archivos (ver 277A-7) |
| **SEO audit 404** | Ruta duplicada `/api/api/admin/seo/audit` → `/admin/seo/audit` | `src/handlers/admin_seo.rs` |
| **SEO: hreflang incorrecto** | Eliminado — apuntaba misma URL para todos los idiomas | `frontend/src/components/seo/SEOHead.tsx` |
| **SEO: dateModified blog** | Pasar `updated_at` del hook al `blogPostSchema()` | `useBlogSingle.ts`, `BlogSingleIsland.tsx` |
| **SEO: FAQ schema hosting/VPS** | FAQPage schema con 4 preguntas por tipo de hosting/VPS | `SolucionHostingIsland.tsx`, `SolucionVpsIsland.tsx` |
| **SEO: descuento 50% backend** | `first_order_discount_percent` ya implementado en `order.rs` | Verificado en código |

---

## ⚠️ Pendiente funcional (medio/alto)

### 4. Pagos/reembolsos — migración pendiente en producción

**Estado:** Código implementado y commiteado (277A-7). **Requiere deploy** para aplicar la migración SQL.
**Incluye:** Idempotency-Key en Stripe, retry de reembolsos con backoff exponencial (1h→4h→16h→64h→72h cap), constraint único de pago activo por orden, validación de monto en checkout webhook, recovery de refunds stuck en Processing (>30min).

### 5. SEO: frontend descuento 50% (banner + badge)

**Estado:** Backend ya implementado (`first_order_discount_percent` en `order.rs`). Falta:
- Banner en checkout: "50% OFF en tu primer servicio"
- Badge en cards de planes del sitio público
- Endpoint API para que el frontend consulte si califica (~30min)
**Esfuerzo:** ~2-3h.

### 6. SEO: breadcrumbSchema en páginas de detalle

**Estado:** Schema definido en `schemas.ts` pero no integrado en ninguna página.
**Páginas:** `/servicios/:slug`, `/proyectos/:slug`, `/blog/:slug`, `/soluciones/hosting`, `/soluciones/vps`
**Esfuerzo:** ~2-3h.

---

## 📋 Pendiente bajo riesgo / delegable

### 11. Test aceptación continuación en navegador limpio

Abrir enlace de continuación en navegador sin sesión, confirmar historial + widget abierto.
**Esfuerzo:** ~30 min (prueba manual).

### 12. Test aceptación: dos mensajes + reconexión

Verificar hotfix de reconexión WebSocket con prueba real.
**Esfuerzo:** ~30 min (prueba manual).

### 13. Test retención >100 mensajes

Conversación con 150+ mensajes, verificar que los más recientes se muestran primero.
**Esfuerzo:** ~30 min (prueba manual).

### 14. Verificar CTA WhatsApp móvil/desktop

Abrir chat, provocar escalamiento, confirmar que el CTA abre `wa.me/16084668134`.
**Esfuerzo:** ~15 min (prueba manual).

---

## 📦 Tareas de producto — Correo para Hosting (bloqueado)

Ver análisis completo en `Agente/documentacion/hosting/producto-correo-proveedores-2026-05-26.md`.

**Decisión pendiente (bloqueante):** Elegir proveedor — MXroute ($59/año, sin API) vs Migadu ($9/mes, API REST).

- **265A-11** — Fase 1: Aliases/reenvíos con Cloudflare Email Routing. ~8-10h.
- **265A-12** — Fase 2: Buzones IMAP (MXroute o Migadu). ~20-26h.
- **265A-13** — Incluir 1 buzón IMAP gratis en plan Avanzado. ~3-4h.

---

## 📝 Planes activos

| Plan                     | Archivo                                                    | Estado verificado                                                                                          |
| ------------------------ | ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Estabilidad Nakomi       | `plan-estabilidad-nakomi-2026-07-23.md`                    | B-E desplegadas. **SQL bugs ya corregidos** (no pendientes). F-K pendientes.                               |
| Cierre bloques difíciles | `plan-cierre-bloques-dificiles-nakomi-2026-07-23.md`       | Bloques A-E implementados.                                                                                 |
| WhatsApp + continuación  | `plan-cierre-whatsapp-continuacion-contacto-2026-07-26.md` | WhatsApp cerrado. Canary continuación pendiente.                                                           |
| Incidente freeze         | `plan-incidente-freeze-bad-gateway-nakomi-2026-07-24.md`   | Causa raíz corregida. Fase 2-5 (SQL hotfix) **ya hechas en código**. Fase 4 (watchdog redesign) pendiente. |
| Coolify Manager mejoras  | `plan-mejoras-coolify-manager-incidencias-2026-07-25.md`   | Implementado. README/skill sin actualizar (pasos 10-11).                                                   |

### Planes sin ejecutar

| Plan                                                      | Estado                             |
| --------------------------------------------------------- | ---------------------------------- |
| `plan-seo-dashboard-blog-descuento-2026-07-25.md`         | Planificado, pendiente aprobación  |
| `plan-migracion-consumidores-glory-rs-main-2026-07-25.md` | Bloqueado hasta estabilizar Nakomi |
| `plan-diagnostico-vps1-2026-05-23.md`                     | Sin ejecutar                       |
| `plan-fixes-pendientes-2026-06-01.md`                     | Sin verificar                      |
| `plan-restauracion-guillermo-2026-07-22.md`               | Sin verificar                      |

---

## 🔄 Correcciones al estado anterior del roadmap

Las siguientes tareas **estaban marcadas como pendientes pero ya están corregidas en código**:

1. **SQL ON CONFLICT notificaciones** — `notification.rs` ya incluye el predicado del índice parcial.
2. **SQL ON CONFLICT response cycles** — `response_cycle.rs` ya usa `ON CONFLICT (session_id) WHERE status = 'waiting'`.
3. **`let _ =` en alertas** — `send_message_with_alerts` ya usa `?` operator.
4. **Feature flags fail-open** — `chat_alert.rs` ya usa fail-closed (`enabled_flag_value`).
5. **ModalAsignar TS6133** — `orderNumber` se usa en JSX.
6. **prompt() admin reembolso** — No encontrado en código fuente.

**Sin embargo**, no se puede confirmar que producción tenga estos fixes hasta verificar el commit desplegado.

---

## ⛔ Restricciones operativas

1. **NO ejecutar deploy sin autorización explícita del usuario.**
2. **Watchdog doble señal** — implementado, pendiente deploy.
3. **Pagos/reembolsos (F+G)** — implementado, pendiente deploy (migración SQL incluida).

## Orden sugerido de ejecución (sin deploy ni watchdog)

1. ✅ **Badge notificaciones fuera del panel** — añadir NotificationBell al Header público.
2. ✅ **Carga inicial badge** — habilitar query o usar useNotifications en sidebar.
3. ✅ **unanswered_messages_loop** — deduplicación durable.
4. ✅ **Rate limiting claim endpoint** — reutilizar patrón existente.
5. ✅ **list_all_sessions** — filtrar sesiones activas.
6. ⏳ **Watchdog doble señal** — esperando autorización.
7. ⏳ **Deploy producción** — ya arreglado, no tocar.
8. ⏳ **Pagos/reembolsos** — esperando autorización.
