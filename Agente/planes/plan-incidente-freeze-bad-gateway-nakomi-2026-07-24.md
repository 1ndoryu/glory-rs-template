# Plan de incidente: freezes y Bad Gateway de Nakomi

> **Fecha:** 2026-07-24
> **Estado:** activo; investigación interrumpida por límite de tokens
> **Rama correcta:** `glory-rust-nakomi`
> **Servicio Coolify:** `studio`
> **UUID esperado:** `do8k4w8swccwwogoc0os0ck0`
> **Producción:** `https://nakomi.studio`
> **Objetivo:** estabilizar primero, corregir la raíz después y reactivar las
> funciones nuevas únicamente mediante canary verificable.

## 1. Regla principal para el siguiente agente

No asumir que “Bad Gateway” es un problema de Traefik. En este incidente puede
ser la ventana durante la cual el backend fue terminado o reemplazado. Antes de
editar, distinguir:

1. deploy/recreación de contenedor;
2. `exit(1)` provocado por el watchdog;
3. OOM kill;
4. crash/panic normal;
5. backend vivo pero sin responder;
6. fallo exclusivo del proxy.

Toda operación de producción debe usar `coolify-manager-rs`. No usar SSH,
`docker`, Coolify UI ni `curl` directo al servidor. No ejecutar
`restart --all`.

Durante el incidente no restaurar la rama `main` del template ni mezclar la
separación template/framework. Eso es otra tarea.

## 2. Hechos confirmados

### 2.1 Lo que NO causó el runtime

Los commits inmediatamente anteriores:

- `12782494`;
- `fcc2ea90`;
- `fcb8942a`;

solo cambiaron documentación Markdown. El commit `fcb8942a` modificó dos planes
y no hizo deploy. Esos cambios por sí mismos no pueden modificar el backend,
frontend, base de datos ni contenedores.

### 2.2 Cambio ejecutable posterior

Después se creó `325d8189`:

`237A-3+237A-6+237A-7: implementar bloques A/B/C/D/E del chat Nakomi`

Magnitud:

- 44 archivos;
- aproximadamente 2.492 líneas añadidas;
- dos migraciones;
- workers permanentes;
- alertas email/WhatsApp;
- cambios Realtime;
- ciclos de respuesta humana/IA;
- captura de email y tokens de continuación;
- cambios frontend y backend en un solo bloque.

Este commit es el primer candidato que debe auditarse. No revertirlo a ciegas:
las migraciones pueden estar registradas en `_sqlx_migrations`; eliminar sus
archivos podría impedir que una versión anterior arranque por una migración
aplicada que ya no existe en el binario.

### 2.3 Estado observado de producción

En la revisión de 2026-07-24:

- `coolify-manager health --name studio` devolvió
  `http_ok=true app_ok=true fatal_logs=false`;
- el contenedor app observado había iniciado a
  `2026-07-24T11:06:29Z`;
- tenía `restart_count=0`;
- `OOMKilled=false`;
- el heartbeat nuevo avanzaba normalmente después del arranque;
- los logs comenzaban con un inicio completo del servidor a las 11:06;
- el contenedor PostgreSQL seguía levantado desde semanas antes.

Interpretación limitada: el contenedor actual fue creado/reemplazado a las
11:06 y no se había reiniciado dentro de ese mismo contenedor. Esto no demuestra
qué pasó con el contenedor anterior. Hay que recuperar los eventos y logs
anteriores para saber si fue deploy, watchdog o crash.

El diagnóstico integral reportó simultáneamente estado Coolify
`degraded:unhealthy` y no encontró contenedores, aunque `health`, `logs` y la
inspección exacta sí encontraron la app viva. Esto sugiere un problema de
matching/estado stale dentro de `coolify-manager-rs`; no usar ese campo aislado
como prueba de caída.

### 2.4 Watchdog que convierte stalls en Bad Gateway

Commit relacionado: `3ac24da9`, framework `898ba81`.

Configuración confirmada:

- pulso Tokio: 5 segundos;
- chequeo desde thread del SO: 5 segundos;
- `freeze_after`: 30 segundos;
- si una secuencia válida deja de avanzar durante el umbral:
  `std::process::exit(1)`.

El watchdog no origina necesariamente el stall, pero convierte cualquier
starvation del runtime de 30 segundos en una terminación inmediata. Mientras
Docker/Coolify recupera el backend, Traefik puede responder Bad Gateway.

No “solucionar” esto desactivándolo para siempre: sin watchdog el backend puede
permanecer congelado. Primero corregir la carga/fallo y después exigir dos
señales antes de terminar el proceso.

### 2.5 Error SQL confirmado en notificaciones

Migración:

`migrations/20260723100000_chat_alert_system.up.sql`

crea este índice parcial:

```sql
CREATE UNIQUE INDEX uq_notifications_dedup
ON notifications (user_id, notification_type, reference_type, reference_id)
WHERE reference_type IS NOT NULL AND reference_id IS NOT NULL;
```

Pero `NotificationRepository::create_tx` usa:

```sql
ON CONFLICT (user_id, notification_type, reference_type, reference_id)
DO NOTHING
```

PostgreSQL no puede inferir automáticamente ese índice parcial sin repetir su
predicado. La sentencia falla. Como ocurre dentro de una transacción, toda la
transacción queda abortada.

Además, `send_message_with_alerts` descarta el error con `let _ =`. Luego intenta
continuar y finalmente el commit falla. Resultado posible:

- mensaje del cliente no persistido;
- no hay broadcast;
- no hay notificación;
- no hay outbox;
- el error útil queda oculto;
- el contenido aun puede enviarse al canal de timing/IA;
- el cliente puede reintentar o reconectar;
- Realtime, historial e IA divergen.

Corrección exacta:

```sql
ON CONFLICT (user_id, notification_type, reference_type, reference_id)
WHERE reference_type IS NOT NULL AND reference_id IS NOT NULL
DO NOTHING
```

No basta corregir SQL. También hay que propagar cualquier error de
`create_tx`/`insert_tx`; nunca usar `let _ =` dentro de una transacción crítica.

### 2.6 Error SQL confirmado en ciclos de respuesta

La migración crea `uq_chat_response_cycles_open` como índice único parcial:

```sql
CREATE UNIQUE INDEX uq_chat_response_cycles_open
ON chat_response_cycles (session_id)
WHERE status = 'waiting';
```

Pero `ResponseCycleRepository::create_if_needed` usa:

```sql
ON CONFLICT ON CONSTRAINT uq_chat_response_cycles_open DO NOTHING
```

Un índice parcial no es una constraint y no se puede referenciar con
`ON CONFLICT ON CONSTRAINT`.

Corrección exacta:

```sql
ON CONFLICT (session_id)
WHERE status = 'waiting'
DO NOTHING
```

El caller también descarta este error. Debe registrarlo y devolver un resultado
útil; si falla el ciclo, no fingir que la espera humana quedó programada.

### 2.7 Carga nueva habilitada por defecto

En `src/services/chat_alert.rs`:

- `CHAT_ALERT_CAPTURE_ENABLED`;
- `CHAT_EMAIL_DELIVERY_ENABLED`;
- `CHAT_WHATSAPP_DELIVERY_ENABLED`;

usan `unwrap_or(true)`. Si las variables no existen, todas las rutas nuevas se
consideran activas.

El worker de outbox:

- despierta cada 5 segundos;
- reclama hasta 20 filas;
- procesa el lote serialmente;
- permite hasta 30 segundos por SMTP;
- permite hasta 10 segundos por gateway;
- consulta conteos y antigüedad tras cada lote.

La red no mantiene la transacción abierta, pero el worker añade carga constante
a un pool PostgreSQL configurado con solo 10 conexiones.

Cada mensaje cliente además consulta admins/emails/sesión, abre una transacción,
crea N notificaciones y outbox y vuelve a consultar sesión/admin/conteos.

### 2.8 Amplificadores secundarios

- `67e8a0e5`: al conectar un admin por WS se pasó de sesiones activas a
  `list_all_sessions()`, incluidas cerradas. Cada reconexión posterior a una
  caída vuelve a cargar/enriquecer todo.
- `acebc117`: `unanswered_messages_loop` hace `notified.clear()` en cada tick;
  puede repetir notificaciones cada cinco minutos.
- `find_session_by_id` no selecciona correctamente el nuevo `ai_mode` y depende
  de un default vacío; la política `human_priority/manual_pause` puede no
  observarse. Es principalmente un fallo funcional.
- El worker de fallback persiste el mensaje directamente pero no lo publica por
  WS; el usuario puede verlo solo después de refetch/reconexión.
- Los errores del flujo WS se silencian, pero el `TimingEvent` puede enviarse de
  todos modos.

## 3. Hipótesis causal actual

La explicación que mejor concuerda con el empeoramiento es:

1. `325d8189` activa por defecto una ruta de mensajes transaccional defectuosa,
   dos workers y más consultas/fan-out;
2. errores silenciosos producen retries, reconexiones y estados divergentes;
3. la carga/reconexión puede amplificar la presión sobre pool/runtime;
4. el watchdog de 30 segundos transforma cualquier starvation total en
   `exit(1)`;
5. durante la recuperación aparece Bad Gateway.

Esto es una hipótesis fuerte, no una causa cerrada. Falta recuperar evidencia
del contenedor anterior.

## 4. Orden obligatorio de trabajo

### Fase 0 — Preservar evidencia

Antes de deploy/restart:

1. Confirmar HEAD local/remoto y commit desplegado real.
2. Guardar health, hora de inicio, restart count, OOM y estado.
3. Obtener eventos del contenedor anterior mediante `coolify-manager-rs`.
4. Recuperar logs que contengan:
   - `RUNTIME FREEZE DETECTED`;
   - panic;
   - OOM;
   - `no unique or exclusion constraint`;
   - `current transaction is aborted`;
   - errores de `response cycle`;
   - errores de outbox;
   - tiempos de consultas/pool timeout.
5. Correlacionar cada Bad Gateway con deploy, exit, die, start y health.

No quedarse esperando logs en streaming. Usar rangos acotados, timestamps y
salida a un reporte local sin secretos.

### Fase 1 — Mitigación inmediata y reversible

Objetivo: preservar mensajes y reducir carga antes de arreglar funciones nuevas.

1. Desactivar temporalmente `CHAT_ALERT_CAPTURE_ENABLED`.
2. Mantener el guardado básico de chat: con el flag apagado,
   `send_message_with_alerts` debe delegar a `ChatRepository::save_message`.
3. No depender solo de apagar email/WhatsApp: el error ocurre al crear la
   notificación in-app, antes de outbox.
4. Aplicar env únicamente mediante `coolify-manager-rs`. Si `sync-env` no puede
   actualizar una variable sin reconstruir un archivo con secretos, detenerse y
   corregir la herramienta; no editar compose ni usar SSH.
5. Hacer redeploy/restart solo del servicio `studio`.
6. Verificar:
   - HTTP 200;
   - app health;
   - mensaje cliente persiste;
   - mensaje aparece tras refetch;
   - heartbeat avanza al menos dos minutos;
   - no aumenta el contador de exits.

Si no es posible aplicar el flag de forma segura, implementar primero un
hotfix de código con captura desactivada por defecto y desplegarlo.

### Fase 2 — Hotfix SQL y errores silenciosos

Hacer un commit pequeño, no mezclar UI:

1. Corregir `NotificationRepository::create_tx` con el predicado del índice.
2. Corregir `ResponseCycleRepository::create_if_needed` con conflict target
   parcial, nunca `ON CONSTRAINT`.
3. Sustituir todos los `let _ =` críticos por `?` o manejo explícito.
4. Si falla notificación/outbox:
   - rollback completo;
   - log con `session_id`, `message_id` si existe y etapa;
   - respuesta visible de error al cliente;
   - no enviar `TimingEvent`;
   - no emitir broadcast ni conteos.
5. Si se decide que una alerta no debe impedir guardar el mensaje, rediseñar:
   - mensaje en transacción principal;
   - evento outbox durable en la misma transacción;
   - notificación derivada por worker;
   - nunca capturar errores y seguir dentro de una transacción abortada.
6. Añadir tests PostgreSQL reales:
   - primer insert;
   - duplicado;
   - referencias nulas;
   - dos requests concurrentes;
   - un ciclo waiting por sesión;
   - nuevo ciclo después de cancelar/responder;
   - rollback inducido;
   - mensaje no llega a IA si no se persistió.

### Fase 3 — Limitar carga

1. Cambiar feature flags nuevos a fail-closed:
   - ausente = desactivado;
   - solo `"true"`/`"1"` = activado.
2. Activarlos por separado:
   captura → email → WhatsApp.
3. Worker outbox:
   - límite de concurrencia explícito;
   - no procesar 20 SMTP seriales con timeout acumulado;
   - backoff con `next_attempt_at`;
   - máximo de intentos;
   - no consultar agregados tras cada lote en nivel INFO;
   - métricas baratas y periódicas.
4. Medir uso del pool:
   - conexiones ocupadas;
   - acquire timeout;
   - queries lentas;
   - backlog outbox;
   - tiempo por lote.
5. Paginar `list_all_sessions()` o volver a sesiones activas + carga bajo
   demanda. Nunca hidratar todas las conversaciones cerradas en cada WS.
6. Corregir `unanswered_messages_loop` para que la deduplicación sea durable,
   no un `HashSet` limpiado en cada tick.

### Fase 4 — Rediseñar watchdog sin ocultar freezes

No eliminar la recuperación. Evitar que una sola señal produzca un exit.

Diseño recomendado:

1. Señal A: heartbeat Tokio sin avanzar.
2. Señal B: probe loopback HTTP con timeout corto.
3. Solo considerar freeze si ambas fallan consecutivamente.
4. Umbral provisional de incidente: 90–120 segundos, no 30.
5. Antes del exit registrar:
   - secuencia y duración;
   - pool size/idle;
   - número de WS/sesiones;
   - backlog de workers;
   - memoria;
   - motivo de la última tarea lenta.
6. El callback no debe bloquear al intentar volcar stacks.
7. Añadir periodo de gracia de arranque.
8. Añadir test de:
   - scheduler momentáneamente lento;
   - thread monitor suspendido;
   - HTTP sano con heartbeat retrasado;
   - HTTP y heartbeat fallando;
   - una sola acción de recovery.

Canary primero. Si el watchdog deja de matar pero la app sigue congelándose, la
raíz continúa presente.

### Fase 5 — Reactivación canary

Orden:

1. `CHAT_ALERT_CAPTURE_ENABLED=true`, delivery externo false.
2. Probar 20–50 mensajes controlados y concurrencia moderada.
3. Confirmar exactamente una notificación in-app por mensaje.
4. Activar email a un destinatario canary.
5. Confirmar exactamente un email por mensaje.
6. Activar WhatsApp con gateway/secret/health verificados.
7. Confirmar exactamente un WhatsApp por mensaje.
8. Mantener métricas 30–60 minutos antes de ampliar.

Ante cualquier freeze, rollback de flags, no rollback destructivo de base de
datos.

## 5. Matriz mínima de pruebas

| Caso | Resultado requerido |
|---|---|
| Cliente envía mensaje | Persiste una vez y recibe ACK |
| Error de notificación | No hay éxito falso ni evento IA |
| Dos mensajes simultáneos | Ambos persisten; sin deadlock |
| Notificación duplicada | `ON CONFLICT` no aborta TX |
| Dos aperturas de ciclo | Solo un `waiting` |
| Admin responde | IA/ciclo se cancelan atómicamente |
| Captura apagada | Chat básico sigue funcionando |
| Email apagado | No hay intento SMTP |
| Gateway apagado | No hay intento HTTP WhatsApp |
| Reconexión admin | No carga todo el archivo histórico |
| Runtime ocupado < umbral | Watchdog no mata |
| Runtime y HTTP congelados | Recovery ocurre una vez |
| Recovery | Ventana Bad Gateway medida y acotada |

## 6. Validación antes de producción

Una sola ronda al cierre:

```text
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
frontend: npx tsc --noEmit
```

Además:

- tests PostgreSQL con migraciones reales;
- prueba Realtime con dos pestañas;
- prueba de error inducido;
- prueba de reconnect;
- prueba de carga acotada;
- revisar que ningún error crítico se silencie;
- revisar que los flags estén false antes del primer deploy.

No considerar suficiente que compile.

## 7. Deploy y rollback

Deploy obligatorio:

1. preflight del binario de `coolify-manager-rs`;
2. backup si se modifica información o migraciones;
3. `deploy --name studio --update`;
4. comprobar progreso sin espera ciega;
5. `health --name studio`;
6. si falla, `redeploy --name studio`;
7. volver a health y logs.

No usar `deploy-service` para Rust salvo la excepción documentada
`--skip-compose-sync`.

Rollback:

- primero desactivar flags;
- después volver al último binario estable compatible con migraciones;
- conservar archivos de migraciones ya aplicadas;
- nunca ejecutar migraciones down durante el incidente sin backup y análisis de
  datos.

## 8. Pendientes de investigación

El siguiente agente debe cerrar explícitamente:

- [ ] commit exacto desplegado en el contenedor de las 11:06;
- [ ] quién/qué inició esa recreación;
- [ ] eventos `die/start/destroy/create` de las últimas 24–48 horas;
- [ ] logs del contenedor anterior;
- [ ] cantidad real de Bad Gateways y duración;
- [ ] evidencia de watchdog vs deploy vs crash;
- [ ] outbox por estado y antigüedad;
- [ ] errores SQL exactos en producción;
- [ ] pool exhaustion y queries lentas;
- [ ] número de conexiones WS/reconexiones;
- [ ] sesiones totales/cerradas cargadas por admin;
- [ ] volumen de notificaciones repetidas;
- [ ] comportamiento bajo un mensaje real de cliente;
- [ ] valor efectivo de cada feature flag;
- [ ] si `ai_mode` se carga correctamente;
- [ ] si el fallback se publica en vivo;
- [ ] si el host watchdog y el watchdog interno pueden competir.

## 9. Problema de seguridad descubierto durante el diagnóstico

`coolify-manager diagnose --json` imprimió el compose completo con variables de
entorno sin redacción. No copiar esa salida a issues, commits ni chats.

Después de estabilizar disponibilidad:

1. modificar `coolify-manager-rs` para redactar automáticamente claves, tokens,
   passwords y URLs con credenciales;
2. añadir test snapshot que falle si aparece un patrón secreto;
3. tratar las credenciales impresas como potencialmente expuestas y rotarlas de
   forma ordenada;
4. verificar el servicio después de cada rotación;
5. no guardar valores secretos en este plan.

## 10. Criterio de cierre

No cerrar por observar HTTP 200 una vez. Se requiere:

- 24 horas sin freeze ni Bad Gateway atribuible al backend;
- cero transacciones abortadas silenciosamente;
- mensajes persistidos y ACK consistentes;
- no divergencia entre chat, historial e IA;
- watchdog sin falsos positivos;
- alertas reactivadas por canal con idempotencia;
- métricas que permitan explicar cualquier próximo incidente;
- health de Coolify y health real reconciliados;
- documentación de causa raíz con evidencia temporal.

## 11. Primer bloque recomendado para delegar

Entregar a un agente el siguiente alcance exacto:

> “Preserva evidencia de producción sin mutar nada. Reproduce en PostgreSQL los
> dos errores `ON CONFLICT`. Implementa solo el hotfix SQL y propagación de
> errores, añade tests concurrentes y deja los tres flags fail-closed. No
> modifiques UI, prompts, WhatsApp ni email. Valida todo localmente. Antes de
> deploy, presenta diff, resultados y plan de rollback compatible con las
> migraciones aplicadas.”

Ese bloque reduce riesgo y evita que un agente menor intente arreglar watchdog,
Realtime, email y WhatsApp al mismo tiempo.
