# Plan: Mejoras a coolify-manager-rs para respuesta a incidentes

> **Fecha:** 2026-07-25 (v2 — revisión completa)
> **Estado:** ✅ Comandos presentes en el binario local verificado el 2026-07-25.
> Este documento conserva propuestas de diseño históricas; no constituye prueba de
> que correo, WhatsApp, Realtime o alertas de Nakomi estén operativos.
>
> **Nota de reconciliación 2026-07-25:** Los comandos descritos en este plan fueron
> implementados según lo documentado en la sección 12 del plan de incidente
> (`plan-incidente-freeze-bad-gateway-nakomi-2026-07-24.md`). Los archivos creados:
> - `src/commands/incident.rs` (investigate + logs)
> - `src/commands/container.rs` (inspect, events, stats)
> - `src/commands/db_stats.rs` (db-stats)
> - `src/commands/env_toggle.rs` (env-toggle)
> - `src/infra/secrets.rs` (redact_text, redact_url_credentials, redact_env_map)
>
> Pendiente de este plan: actualización del README y la skill (secciones 10 y 11),
> y el fix de matching stale en `diagnose` (paso 10 de la sección 9).
> **Origen:** `plan-incidente-freeze-bad-gateway-nakomi-2026-07-24.md`
> **Propósito:** No resolver el incidente, sino crear las herramientas que harán que resolverlo (y futuros similares) sea más fácil y seguro.

---

## 1. Contexto y problema

El incidente de Nakomi expuso **vacíos críticos** en coolify-manager-rs. Durante la investigación se necesitaba:

- Saber qué pasó con el contenedor anterior (deploy, watchdog kill, OOM, crash?)
- Buscar patrones específicos en logs (FREEZE, panic, OOM, constraint errors, pool timeouts)
- Ver restart_count, OOMKilled, tiempo de inicio del contenedor
- Tener un comando unificado que correlacione todas las fuentes de datos
- Togglear feature flags rápidamente sin hacer sync-env completo
- Ver métricas de DB y recursos del contenedor
- Redactar secretos en la salida de `diagnose` (problema de seguridad confirmado)
- Guardar el reporte de investigación localmente para referencia futura
- Manejar fallos de SSH/API gracefully sin quedarse colgado

Actualmente hay que ejecutar 5+ comandos separados y correlacionar manualmente.

---

## 2. Comandos nuevos propuestos

### 2.1 `incident-investigate <name> [--json] [--save] [--timeout 30s]`

Comando unificado que ejecuta en paralelo (con `tokio::join!` + timeout global) todo lo necesario para la Fase 0 del plan de incidente:

1. **Commit desplegado**: `git rev-parse HEAD` dentro del contenedor
2. **Container inspect**: start time, restart count, OOM, exit code, image, resource limits
3. **Container events**: últimos 48h de eventos del contenedor (create/start/die/destroy/oom/kill)
4. **Incident logs**: búsqueda de todos los patrones de incidente con rango temporal
5. **Health check**: HTTP + app + fatal logs
6. **DB stats rápidas**: conexiones activas, queries >5s, lock waits
7. **Coolify API status**: estado del servicio en Coolify

**Flags:**

- `--json`: salida machine-readable
- `--save <path>`: guarda el reporte como archivo local (sin secretos)
- `--timeout`: timeout global para todo el comando (default 30s)

**Comportamiento ante fallos parciales:**

- Cada subtarea se ejecuta con su propio timeout (10s por subtarea)
- Si una subtarea falla, se marca como `error: <razón>` en el reporte, no aborta todo
- El reporte siempre se genera con lo que se pudo recolectar

### 2.2 `container-events <name> [--since 24h] [--until 0h] [--json]`

Historial de eventos de ciclo de vida del contenedor.

- Usa `docker events --filter container=<name> --since=<time> --until=<time>` via SSH
- Filtra por acciones: `create`, `start`, `die`, `destroy`, `oom`, `kill`
- Muestra timestamp + acción + atributos (exit code, signal)
- Fallback a bollard API cuando el socket Docker sea accesible
- **Seguridad:** no imprime env vars ni secretos de los atributos del evento

### 2.3 `container-inspect <name> [--json]`

Inspección detallada del estado del contenedor.

- Ejecuta `docker inspect <container_id>` via SSH y parsea el JSON
- Extrae campos relevantes únicamente (no imprime el JSON completo):
    - `Created`, `StartedAt`, `RestartCount`, `OOMKilled`
    - `State.Status`, `State.ExitCode`, `State.Error`
    - `Image` (digest corto)
    - `HostConfig.Memory`, `HostConfig.CpuShares`, `HostConfig.RestartPolicy`
    - `NetworkSettings.Networks` (nombres, IPs — sin exponer gateways internos)
- **Redacción automática:** env vars del contenedor se redactan antes de imprimir
- **Seguridad:** nunca imprime `Config.Env` completo — solo keys sin valores

### 2.4 `incident-logs <name> [--since 48h] [--until 0h] [--json] [--patterns <p1,p2>]`

Búsqueda de logs con patrones predefinidos de incidente.

Patrones por defecto:

- `RUNTIME FREEZE DETECTED`
- `panic`
- `OOMKilled` / `oom-kill`
- `no unique or exclusion constraint`
- `current transaction is aborted`
- `pool timeout` / `connection pool` / `too many connections`
- `response cycle` error
- `outbox` error
- `FATAL`
- `watchdog` / `freeze_after`
- `SIGKILL` / `SIGTERM`

**Comportamiento:**

- Usa `docker logs --since <time>` via SSH + grep multi-patrón
- Agrupa resultados por patrón y los ordena cronológicamente
- Limita a 200 líneas por patrón para evitar saturación
- Si un patrón no tiene coincidencias, se omite del output (no muestra secciones vacías)
- `--patterns` permite patrones custom además de los defaults
- **No hacer streaming** — ejecución acotada con timeout de 15s

### 2.5 `env-toggle <name> <KEY> <VALUE> [--restart]`

Toggle rápido de un solo env var.

- Usa Coolify API directamente: GET envs → PATCH el key específico
- Flag `--restart` para reiniciar el servicio después del cambio (default: no reinicia)
- **Validaciones de seguridad:**
    - Rechaza keys en la blocklist crítica: `DATABASE_URL`, `SERVICE_PASSWORD_POSTGRES`, `JWT_SECRET`, `COOLIFY_*`
    - Rechaza keys vacías o con caracteres especiales peligrosos
    - Confirma el cambio mostrando valor anterior → valor nuevo (valores sensibles se redactan)
- **Dry-run:** `--dry-run` muestra qué haría sin ejecutar
- Más rápido y seguro que `sync-env push` completo

### 2.6 `db-stats <name> [--json] [--threshold 5]`

Métricas rápidas de PostgreSQL.

SQL pre-escritos que ejecuta vía SSH + `psql` (reutilizando `pg_utils::run_pg_query`):

| Consulta                                | Qué muestra                              |
| --------------------------------------- | ---------------------------------------- |
| `pg_stat_activity` agrupado por `state` | Conexiones activas/idle/otras            |
| `pg_stat_activity WHERE duration > $1`  | Queries lentas (umbral configurable)     |
| `pg_locks` + `pg_stat_activity`         | Lock waits con query bloqueante          |
| `pg_stat_user_tables`                   | Dead tuples, último vacuum/analyze       |
| `pg_stat_database`                      | Transacciones commit/rollback, deadlocks |
| `pg_stat_bgwriter`                      | Checkpoints, buffers escritos            |

**Seguridad:** las queries largas se imprimen truncadas a 200 chars. No imprime parámetros de conexión.

### 2.7 `container-stats <name> [--json]`

Recursos del contenedor en punto de tiempo.

- `docker stats --no-stream --format '{{json .}}'` via SSH
- Parsea: CPU %, memoria uso/límite, red I/O (rx/tx), block I/O (read/write)
- Fallback a bollard API si el socket es accesible
- **Formato legible:** convierte bytes a MB/GB automáticamente

---

## 3. Módulo: redacción de secretos

### Extender `src/infra/secrets.rs` (ya existe con `mask_secret`)

**Problema de seguridad confirmado:** `diagnose --json` imprimió variables de entorno completas con secretos durante el incidente.

El módulo `secrets.rs` ya tiene `mask_secret` (enmascara strings largos). Se extiende con:

```rust
/// Redacta un mapa de variables de entorno, reemplazando valores sensibles
pub fn redact_env_map(env: &HashMap<String, String>) -> HashMap<String, String>

/// Redacta texto multilinea que pueda contener secretos
pub fn redact_text(text: &str) -> String

/// Detecta si una key es sensible
pub fn is_sensitive_key(key: &str) -> bool
```

**Patrones de detección:**

- Keys: `PASSWORD`, `SECRET`, `TOKEN`, `KEY`, `API_KEY`, `JWT`, `DATABASE_URL`, `SMTP_`, `REDIS_URL`, `WEBHOOK`, `CREDENTIALS`
- URLs con credenciales: `postgres://user:pass@host` → `postgres://[REDACTED]@[REDACTED]`
- Strings que parecen tokens: >32 chars alfanuméricos sin espacios

**Aplicación obligatoria en:**

- `diagnose` (todas las secciones, especialmente secciones 4, 5, 8, 10, 12)
- `incident-investigate`
- `container-inspect` (env vars)
- `db-stats` (connection strings en errores)
- Cualquier comando con `--json` que pueda contener datos del contenedor

**Tests:**

- Test snapshot que falla si un output contiene un patrón conocido de secreto
- Test que verifica que `DATABASE_URL` con credenciales se redacta correctamente
- Test que verifica que URLs con `@` se redactan

---

## 4. Mejoras a comandos existentes

### 4.1 `diagnose` → integrar redacción de secretos

- Aplicar `redact_env_map` y `redact_text` a toda la salida
- Secciones críticas: 4 (DB creds), 5 (MySQL details), 8 (logs raw), 10 (Coolify API response), 12 (PG inspection)
- En modo `--json`, redactar antes de serializar
- **Preservar comportamiento existente** — solo cambiar la capa de presentación

### 4.2 `health` → agregar datos del contenedor

- Agregar al `HealthReport` (en `health_manager.rs`):
    ```rust
    pub restart_count: Option<i64>,
    pub oom_killed: Option<bool>,
    pub container_start_time: Option<String>,
    ```
- Obtener estos datos ejecutando `docker inspect` después del health check
- Mantener backward compatibility — los campos nuevos son `Option`
- Actualizar `healthy()` para no depender de los campos nuevos

### 4.3 `logs` → agregar rangos de tiempo

- Agregar flags `--since` y `--until` al struct Clap de `view_logs`
- Convertir formato relativo (`2h`, `48h`) a timestamp absoluto para `docker logs --since/--until`
- Agregar `--pattern` (repetible) para múltiples patrones de grep
- **Validación:** `--until` debe ser posterior a `--since`

### 4.4 `diagnose` → fix matching stale de Coolify API

El incidente reveló que `diagnose` reportó `degraded:unhealthy` y "no encontró contenedores" aunque la app estaba viva. Esto es un bug de matching/estado stale.

- Verificar que la búsqueda de contenedores use el UUID correcto del servicio
- No confiar solo en el estado de la API de Coolify — corroborar con `docker ps`
- Agregar campo `coolify_api_status` vs `docker_actual_status` al reporte de `diagnose`

---

## 5. Estructuras de datos nuevas

Definir en `src/domain/mod.rs` (extender, no reemplazar):

```rust
use serde::Serialize;
use std::collections::HashMap;

/// Reporte unificado de investigación de incidentes
#[derive(Debug, Clone, Serialize)]
pub struct IncidentReport {
    pub site_name: String,
    pub timestamp: String,                    // ISO 8601
    pub investigation_duration_ms: u64,
    pub deployed_commit: Option<String>,
    pub container: Option<ContainerInspectData>,
    pub events: Vec<ContainerEvent>,
    pub log_matches: Vec<LogMatchGroup>,
    pub health: Option<IncidentHealthSummary>,
    pub db_stats: Option<DbStats>,
    pub coolify_status: Option<CoolifyServiceStatus>,
    pub errors: Vec<SubtaskError>,            // subtareas que fallaron
}

/// Detalles del contenedor inspeccionado
#[derive(Debug, Clone, Serialize)]
pub struct ContainerInspectData {
    pub container_id: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub start_time: Option<String>,
    pub restart_count: i64,
    pub oom_killed: bool,
    pub exit_code: Option<i64>,
    pub error_message: Option<String>,
    pub memory_limit_mb: Option<i64>,
    pub cpu_shares: Option<i64>,
    pub restart_policy: String,
}

/// Evento de ciclo de vida del contenedor
#[derive(Debug, Clone, Serialize)]
pub struct ContainerEvent {
    pub timestamp: String,
    pub action: String,  // create, start, die, destroy, oom, kill
    pub exit_code: Option<i64>,
    pub signal: Option<String>,
    pub attributes: HashMap<String, String>,
}

/// Grupo de líneas de log que coincidieron con un patrón
#[derive(Debug, Clone, Serialize)]
pub struct LogMatchGroup {
    pub pattern: String,
    pub matches: Vec<LogLine>,
    pub total_count: usize,
}

/// Línea individual de log
#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub timestamp: Option<String>,
    pub line: String,
}

/// Resumen de health para el reporte de incidente
#[derive(Debug, Clone, Serialize)]
pub struct IncidentHealthSummary {
    pub http_ok: bool,
    pub status_code: Option<u16>,
    pub app_ok: bool,
    pub fatal_log_detected: bool,
    pub restart_count: Option<i64>,
    pub oom_killed: Option<bool>,
}

/// Estadísticas de PostgreSQL
#[derive(Debug, Clone, Serialize)]
pub struct DbStats {
    pub connections_by_state: Vec<ConnectionState>,
    pub long_running_queries: Vec<QueryStat>,
    pub lock_waits: Vec<LockWait>,
    pub deadlocks_total: i64,
    pub top_tables: Vec<TableStats>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionState {
    pub state: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryStat {
    pub pid: i64,
    pub duration_secs: f64,
    pub state: String,
    pub query_preview: String,  // truncado a 200 chars
}

#[derive(Debug, Clone, Serialize)]
pub struct LockWait {
    pub blocked_pid: i64,
    pub blocking_pid: i64,
    pub lock_type: String,
    pub query_preview: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableStats {
    pub table_name: String,
    pub row_estimate: i64,
    pub total_size_bytes: i64,
    pub dead_tuples: i64,
    pub last_vacuum: Option<String>,
    pub last_analyze: Option<String>,
}

/// Estado del servicio en Coolify API
#[derive(Debug, Clone, Serialize)]
pub struct CoolifyServiceStatus {
    pub uuid: String,
    pub name: String,
    pub fqdn: Option<String>,
    pub coolify_status: String,
    pub docker_actual_status: String,
    pub status_mismatch: bool,  // true si Coolify dice X pero Docker dice Y
}

/// Error de una subtarea de investigación
#[derive(Debug, Clone, Serialize)]
pub struct SubtaskError {
    pub task_name: String,
    pub error: String,
    pub duration_ms: u64,
}
```

---

## 6. Archivos a crear y modificar

| Archivo                        | Acción        | Descripción                                                      |
| ------------------------------ | ------------- | ---------------------------------------------------------------- |
| `src/commands/incident.rs`     | **Crear**     | `incident-investigate` + `incident-logs`                         |
| `src/commands/container.rs`    | **Crear**     | `container-events`, `container-inspect`, `container-stats`       |
| `src/commands/db_stats.rs`     | **Crear**     | `db-stats`                                                       |
| `src/commands/env_toggle.rs`   | **Crear**     | `env-toggle`                                                     |
| `src/cli/mod.rs`               | **Modificar** | Agregar nuevos subcomandos al enum Clap                          |
| `src/commands/mod.rs`          | **Modificar** | Registrar nuevos módulos                                         |
| `src/commands/diagnose.rs`     | **Modificar** | Integrar redacción + fix matching stale                          |
| `src/commands/health_check.rs` | **Modificar** | Agregar restart_count y OOM al reporte                           |
| `src/commands/view_logs.rs`    | **Modificar** | Agregar `--since`, `--until`, `--pattern`                        |
| `src/infra/docker_api.rs`      | **Modificar** | Agregar `inspect_container`, `container_stats`                   |
| `src/infra/secrets.rs`         | **Modificar** | Extender con `redact_env_map`, `redact_text`, `is_sensitive_key` |
| `src/domain/mod.rs`            | **Modificar** | Agregar structs de incidente                                     |

---

## 7. Mitigaciones y seguridad

### 7.1 Redacción de secretos (prioridad crítica)

- **Alcance:** TODOS los comandos que imprimen datos del contenedor
- **Implementación:** extender `secrets.rs` existente (no crear módulo nuevo)
- **Test obligatorio:** snapshot test que falla si un output contiene patrón de secreto
- **Cobertura:** env vars, URLs con credenciales, connection strings, tokens en logs

### 7.2 Timeouts y fallos parciales

- `incident-investigate` tiene timeout global configurable (default 30s)
- Cada subtarea tiene timeout individual (default 10s)
- Si una subtarea falla → se registra en `errors[]` del reporte, no aborta todo
- El reporte siempre se genera con lo que se pudo recolectar

### 7.3 Validación de input

- `--since` / `--until`: validar formato y que `until > since`
- `env-toggle`: validar que la key no está en blocklist crítica
- `env-toggle`: rechazar keys vacías o con caracteres peligrosos
- Nombres de sitio: validar que existen en la configuración antes de ejecutar

### 7.4 Preservación de evidencia

- `incident-investigate --save <path>` guarda el reporte localmente
- El reporte NO contiene secretos (redactados antes de guardar)
- Incluye timestamp de generación para trazabilidad
- Formato JSON para poder procesarlo con jq u otras herramientas

### 7.5 Detección de status mismatch

- `incident-investigate` compara el estado de Coolify API con el estado real de Docker
- Si hay discrepancia, se marca como `status_mismatch: true` en el reporte
- Esto previene el problema del incidente Nakomi donde `diagnose` reportó `degraded:unhealthy` pero la app estaba viva

### 7.6 SSH fallback y resiliencia

- Todos los comandos intentan Docker API (bollard) primero, fallback a SSH
- Si SSH falla → error claro con causa, no crash silencioso
- `container-events`: si `docker events --since` no soporta el rango, usar `docker ps --filter` como fallback degradado
- Reutilizar `SshClient` existente con sus timeouts configurados (1800s para largos)

### 7.7 No mutar producción

- `incident-investigate` es 100% de solo lectura
- `incident-logs` es 100% de solo lectura
- `container-inspect` es 100% de solo lectura
- `container-events` es 100% de solo lectura
- `db-stats` es 100% de solo lectura
- `container-stats` es 100% de solo lectura
- `env-toggle` es el único comando que muta — requiere confirmación implícita (sin `--dry-run`)

---

## 8. Consideraciones de arquitectura

- Todos los comandos nuevos usan la infraestructura SSH existente (`SshClient`)
- Container events usa Docker API (bollard) con fallback a SSH
- `incident-investigate` ejecuta subtareas en paralelo con `tokio::join!` + timeout
- Todos los outputs soportan `--json` para salida machine-readable
- Redacción de secretos se aplica en la capa de presentación (antes de imprimir o guardar)
- Los comandos existentes no se rompen — solo se extienden con campos `Option`
- Reutilizar `pg_utils::run_pg_query` para `db-stats`
- Reutilizar `health_manager::run_site_health_check` para el health en `incident-investigate`

---

## 9. Orden de implementación recomendado

| #   | Tarea                                   | Dependencias  | Riesgo |
| --- | --------------------------------------- | ------------- | ------ |
| 1   | Extender `secrets.rs` con redacción     | Ninguna       | Bajo   |
| 2   | `container-inspect` + mejora a `health` | Ninguna       | Bajo   |
| 3   | `incident-logs`                         | Ninguna       | Bajo   |
| 4   | `container-events`                      | Ninguna       | Bajo   |
| 5   | Aplicar redacción a `diagnose`          | Paso 1        | Bajo   |
| 6   | `incident-investigate`                  | Pasos 2, 3, 4 | Medio  |
| 7   | `env-toggle`                            | Ninguna       | Medio  |
| 8   | `db-stats`                              | Ninguna       | Bajo   |
| 9   | `container-stats`                       | Ninguna       | Bajo   |
| 10  | Fix matching stale en `diagnose`        | Paso 5        | Medio  |
| 11  | Actualizar README                       | Todos         | Bajo   |
| 12  | Actualizar skill                        | Todos         | Bajo   |

---

## 10. Plan de actualización del README

### Cuándo: después de implementar todos los comandos

### Secciones a agregar/modificar en `README.md`:

#### Nueva sección: "Investigación de incidentes" (después de "Health & Security")

Documentar:

- `incident-investigate`: uso, flags, ejemplo de output
- `incident-logs`: patrones por defecto, cómo agregar custom
- `container-events`: rango temporal, filtros
- `container-inspect`: qué muestra, qué redacta
- `db-stats`: qué métricas incluye
- `container-stats`: formato de salida
- `env-toggle`: validaciones, dry-run

#### Modificar sección existente: "Referencia de comandos CLI"

Agregar las 7 nuevas entradas en la tabla de comandos agrupadas bajo "Investigación de incidentes".

#### Modificar sección: "Arquitectura"

Mencionar el módulo `secrets.rs` extendido y la redacción automática.

#### Agregar nota de seguridad:

> **Nota de seguridad:** Los comandos de investigación redactan automáticamente secretos (passwords, tokens, API keys, connection strings) en toda la salida. El módulo `secrets.rs` se aplica en `diagnose`, `incident-investigate`, `container-inspect` y cualquier comando con `--json`.

---

## 11. Plan de actualización de la skill

### Cuándo: después de implementar todos los comandos

### Cambios en `.agents/skills/coolify-manager/`:

#### Agregar sección: "5. Investigación de incidentes"

````markdown
### 5a. Investigación rápida de incidente

```powershell
& $cm incident-investigate --name studio --json --save report.json
```
````

Recolecta en paralelo: commit desplegado, estado del contenedor, eventos
de ciclo de vida, logs con patrones de incidente, health check, métricas
de DB, y estado de Coolify API. Todo con secretos redactados.

Usar cuando:

- Un servicio está caído o comportándose extraño
- Se necesita correlacionar qué pasó (deploy, crash, OOM, watchdog)
- Se quiere un reporte completo antes de hacer cambios

### 5b. Búsqueda de logs de incidente

```powershell
& $cm incident-logs --name studio --since 48h
```

Busca patrones predefididos (FREEZE, panic, OOM, constraint errors,
pool timeout, etc.) en los logs del contenedor.

### 5c. Eventos del contenedor

```powershell
& $cm container-events --name studio --since 24h
```

Muestra historial de create/start/die/destroy/oom/kill.

### 5d. Inspección de contenedor

```powershell
& $cm container-inspect --name studio
```

Restart count, OOMKilled, start time, image, resource limits.

### 5e. Métricas de DB

```powershell
& $cm db-stats --name studio
```

Conexiones activas, queries lentas, lock waits, dead tuples.

### 5f. Toggle rápido de env var

```powershell
& $cm env-toggle --name studio CHAT_ALERT_CAPTURE_ENABLED false --restart
```

Cambia una variable de entorno sin hacer sync-env completo.

#### Modificar sección "6. Reglas de seguridad"

Agregar:

> **8. Usar `incident-investigate` antes de mutar.** Antes de deploy, restart o cualquier cambio en producción durante un incidente, ejecutar `incident-investigate` para preservar evidencia.

#### Modificar tabla de troubleshooting

Agregar filas:
| Problema | Causa probable | Solución |
|---|---|---|
| App congelada / Bad Gateway | Watchdog mató el proceso, deploy en curso, o OOM | `incident-investigate` primero para distinguir las 6 causas posibles |
| `diagnose` muestra `degraded` pero app responde | Bug de matching stale en Coolify API | Confiar en `health` + `container-inspect`, no en el campo `status` de `diagnose` solo |

---

## 12. Criterio de cierre

### Implementación

- [ ] Los 7 comandos nuevos funcionan con flag `--json`
- [ ] `diagnose` redacta automáticamente todos los secretos
- [ ] `health` incluye restart_count y OOM
- [ ] `logs` soporta `--since`, `--until`, `--pattern`
- [ ] `incident-investigate` ejecuta en < 30 segundos con timeout
- [ ] `incident-investigate --save` genera archivo local sin secretos
- [ ] Fallos parciales se reportan en el JSON, no abortan
- [ ] `env-toggle` rechaza keys críticas y soporta `--dry-run`
- [ ] Status mismatch entre Coolify API y Docker se detecta

### Testing

- [ ] Test snapshot de redacción pasa (no hay secretos sin redactar)
- [ ] Test que `DATABASE_URL` con credenciales se redacta
- [ ] Test que URLs con `@` se redactan
- [ ] `cargo fmt --check && cargo check && cargo clippy -- -D warnings && cargo test` pasa

### Documentación

- [ ] README actualizado con sección de investigación de incidentes
- [ ] Skill actualizada con ejemplos de uso
- [ ] Cada comando tiene docstring con ejemplo de uso

```

```
