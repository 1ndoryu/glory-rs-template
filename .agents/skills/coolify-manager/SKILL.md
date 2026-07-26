---
name: coolify-manager
description: "Gestionar deploys, health checks, backups, logs, investigación de incidentes y operaciones en servidores Coolify usando coolify-manager-rs. Cubre flujo completo: deploy WordPress, deploy Rust, recuperación post-fallo, backup/restore, sync de envs, diagnóstico de bind mounts, rollback automático (E11) e investigación de incidentes. Usar cuando: necesitas hacer deploy, verificar health, restaurar un servicio caído, hacer backup/restore, revisar logs de contenedores, ejecutar comandos en runtime, diagnosticar problemas de conectividad o bind mounts, o investigar incidentes de producción (freezes, crashes, OOM, Bad Gateway)."
argument-hint: "nombre del sitio (studio, nakomi, glory-rest, kamples) o 'help'"
---

# Coolify Manager — Guía Completa de Uso

## Cuándo usar esta skill

- Hacer deploy de código (WordPress o Rust) a producción
- Verificar health de un sitio después de deploy
- Restaurar un servicio caído (rollback automático E11 o manual)
- Backup/restore de base de datos y uploads
- Revisar logs de contenedores
- Ejecutar comandos dentro de un contenedor en producción
- Sincronizar variables de entorno entre local y Coolify
- Diagnosticar problemas de bind mounts, DNS collision o compose drift
- **Investigar incidentes de producción** (freezes, OOM, crashes, Bad Gateway, restart loops)

## Estructura general

```
┌─────────────────────────────────────────────────────────────┐
│ coolify-manager-rs  (CLI Rust)                              │
│                                                             │
│  $cm = "C:\Users\Owner\...\target\release\coolify-manager.exe"
│                                                             │
│  Comandos principales:                                      │
│  deploy ── health ── redeploy ── backup ── restore          │
│  logs ── exec ── restart ── git-status ── sync-env          │
│  deploy-websocket                                           │
│                                                             │
│  Comandos de investigación de incidentes:                   │
│  incident investigate ── incident logs                      │
│  container inspect ── container events ── container stats   │
│  db-stats ── env-toggle                                     │
│                                                             │
│  Protecciones integradas:                                   │
│  • Pre-validación compose/conexión                          │
│  • Backup automático pre-write                              │
│  • Post-verify env/volúmenes tras deploy                    │
│  • Rollback automático si health falla (E11)                │
│  • CM_GUARD_v1 marca escritura concurrente                  │
│  • Redacción automática de secretos en diagnóstico          │
└─────────────────────────────────────────────────────────────┘
```

## 1. Encontrar y usar el binario

### Path absoluto universal (funciona desde cualquier proyecto)

```powershell
$cm = "C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs\target\release\coolify-manager.exe"
```

### Verificar que existe y compilar si es necesario

```powershell
if (-not (Test-Path $cm)) {
    Push-Location "C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs"
    cargo build --release --target-dir target
    Pop-Location
}
```

### Atajo rápido (desde cualquier terminal)

```powershell
function cm { & "C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs\target\release\coolify-manager.exe" @args }
cm health --name studio
```

---

## 2. Comandos principales

### 2a. Deploy de código

#### WordPress (rápido, ~1-2 min)

```powershell
& $cm deploy --name nakomi --update --skip-backup
```

#### Rust (lento, ~8-12 min — el build toma tiempo)

```powershell
& $cm deploy --name studio --update --skip-backup
```

> **Importante:** `deploy --update` funciona para WordPress Y Rust. Para Rust, Coolify clona el repo y construye la imagen Docker. Si devuelve 503 durante el build, es normal — esperar. `deploy-service` NO funciona para Rust (falla en build Docker paso 3/6).

#### Flujo deploy obligatorio

```
deploy → health → si falla → redeploy → health
```

#### `--skip-backup` — ¿cuándo usarlo?

| Situación | ¿Skip? |
|---|---|
| Solo cambios de código (PHP/JS/CSS/TS/Rust) | ✅ Sí |
| Migraciones de base de datos | ❌ No |
| Cambios en uploads o assets de usuario | ❌ No |
| Dudas sobre riesgo de regresión | ❌ No |

El backup pre-write transfiere ~2-3 min extra. Omitir solo cuando hay certeza de que no hay datos que perder.

---

### 2b. Health check

```powershell
& $cm health --name studio
```

Verifica:
- Estado HTTP (200 OK)
- Que el contenedor está corriendo
- Que los bind mounts están presentes
- Que la app responde en su health endpoint interno

**Siempre post-deploy.** Si falla, ejecutar `redeploy` inmediatamente.

---

### 2c. Redeploy (forzar sin cambio de código)

```powershell
& $cm redeploy --name studio
```

Usar cuando:
- El deploy inicial falló
- El health check falla y hay que reiniciar servicios
- Se necesita refrescar el contenedor sin rebuild

> **Nota:** Para proyectos Rust legacy (`glory-rest`) que no pasaron por `deploy --update`, el flujo de recuperación es SSH directo: `cd /data/coolify/services/{uuid} && docker compose build && docker compose up -d`. coolify-manager-rs no puede rebuildearlos automáticamente porque su compose no sigue el estándar `rust_app/rust_db`.

---

### 2d. Logs

```powershell
& $cm logs --name studio
& $cm logs --name studio --since 2h --pattern "error|panic|oom"
```

> **Limitación conocida:** `logs` apunta al primer contenedor del stack, que puede no ser el `app`. Para logs específicos de la app, usar `incident logs` o SSH directo.

---

### 2e. Exec (ejecutar comando en contenedor)

```powershell
& $cm exec --name studio --target app --command "printenv GLORY_TEST_CHECKOUT_EMAILS"
```

> **Advertencia:** `exec` busca el contenedor por UUID+nombre. Si el UUID no existe, cae a búsqueda solo por nombre, lo que puede conectar al contenedor INCORRECTO. Siempre verificar la salida antes de actuar.

---

### 2f. Backup y Restore

```powershell
& $cm backup --name studio
& $cm restore --name studio
```

---

### 2g. Git status (tema remoto)

```powershell
& $cm git-status --name nakomi
```

---

### 2h. Deploy WebSocket

```powershell
& $cm deploy-websocket --name studio
```

---

### 2i. Restart

```powershell
& $cm restart --name studio
```

> **⚠️ REGLA CRÍTICA:** No usar `restart --all` si hay servicios Rust en el mismo VPS.

---

## 3. Investigación de incidentes

Comandos diseñados para diagnosticar incidentes en producción. Todos operan via SSH de solo lectura (excepto `env-toggle`) y redactan automáticamente secretos.

### 3a. Diagnóstico unificado (`incident investigate`)

**Primer paso ante cualquier incidente.** Ejecuta un panel completo: health, inspección del contenedor, eventos recientes, stats de recursos, logs con patrones de incidente y métricas PostgreSQL.

```powershell
& $cm incident investigate --name studio
& $cm incident investigate --name studio --since 4h --json
```

Secciones del reporte:
- Health check HTTP
- Inspección del contenedor (estado, restart_count, OOM, exit_code, límites de memoria/CPU, política de reinicio)
- Eventos del ciclo de vida (create, start, die, destroy, oom, kill) — **crítico para saber qué pasó con el contenedor anterior**
- Stats de recursos (CPU, memoria, red, disco, PIDs)
- Logs con patrones de incidente (FREEZE DETECTED, panic, OOM, constraint violation, transaction aborted, pool timeout)
- Métricas PostgreSQL (conexiones por estado, queries largas, locks, deadlocks, tablas por tamaño)

### 3b. Búsqueda de patrones en logs (`incident logs`)

Busca patrones de incidente específicos en los logs del contenedor.

```powershell
& $cm incident logs --name studio
& $cm incident logs --name studio --since 2h --pattern "panic|oom"
```

Patrones por defecto: `FREEZE DETECTED`, `panic`, `OOM`, `no unique or exclusion constraint`, `current transaction is aborted`, `connection pool`/`pool timeout`.

### 3c. Inspección del contenedor (`container inspect`)

```powershell
& $cm container inspect --name studio
& $cm container inspect --name studio --json
```

Muestra: estado, restart_count, OOM killed, exit code, error message, límites de memoria/CPU, política de reinicio. **No imprime env vars ni secretos.**

### 3d. Eventos del contenedor (`container events`)

```powershell
& $cm container events --name studio
& $cm container events --name studio --since 48h
```

Lista eventos de ciclo de vida. **Esencial para saber si el contenedor fue recreado por deploy, watchdog, OOM o crash.**

### 3e. Stats de recursos (`container stats`)

```powershell
& $cm container stats --name studio
```

CPU, memoria, red, disco y PIDs en tiempo real.

### 3f. Métricas PostgreSQL (`db-stats`)

```powershell
& $cm db-stats --name studio
& $cm db-stats --name studio --threshold 10 --json
```

Conexiones por estado, queries activas largas, lock waits, deadlocks, tablas por tamaño y dead tuples.

### 3g. Toggle rápido de env vars (`env-toggle`)

Para mitigación de incidentes: activar/desactivar feature flags sin sync-env completo.

```powershell
& $cm env-toggle --name studio --key CHAT_ALERT_CAPTURE_ENABLED --value false
& $cm env-toggle --name studio --key CHAT_EMAIL_DELIVERY_ENABLED --value false
```

> **Seguridad:** Bloquea automáticamente keys sensibles (SECRET, PASSWORD, TOKEN, KEY, DATABASE_URL, etc.). Para variables críticas usar `sync-env`.

---

## 4. Variables de entorno (sync-env)

```powershell
& $cm sync-env --only GLORY_TEST_CHECKOUT_EMAILS --env-file .env
```

---

## 5. Recuperación de incidentes

### 5a. Rollback automático (E11)

Si un deploy falla el health check, coolify-manager-rs ejecuta rollback automático.

### 5b. Bind mount perdido (incidente studio 4 mayo 2026)

Si la app está viva pero no ve archivos de usuario:
1. Verificar bind path: `ssh root@66.94.100.241 "ls -la /data/uploads/studio/"`
2. Verificar si el contenedor usa named volume en vez de bind mount
3. Corregir compose on-disk y hacer `docker compose up -d --no-build --force-recreate --no-deps app`

### 5c. DNS collision (`postgres` hostname)

Si DATABASE_URL da `28P01 password authentication failed`:
1. Verificar colisión: `docker run --rm --network {uuid} --network coolify busybox nslookup postgres`
2. Fix: usar `postgres-{uuid}` en DATABASE_URL

### 5d. Coolify API devuelve 500/422

Ver sección de troubleshooting del README.

---

## 6. Mapeo de sitios

| Sitio | Tipo | UUID Coolify | Dominio | Bind mount uploads |
|---|---|---|---|---|
| studio | Rust | `do8k4w8swccwwogoc0os0ck0` | nakomi.studio | `/data/uploads/studio` |
| nakomi | WordPress | `u00gc8ss4csc4cckkg4g00ks` | task.nakomi.studio | N/A |
| glory-rest | Rust | (legacy, no estandarizado) | N/A | N/A |

---

## 7. Reglas de seguridad

1. **NO hacer deploy directo con SSH.** Toda operación en producción pasa por coolify-manager-rs.
2. **NO usar `restart --all`** si hay servicios Rust en el mismo VPS.
3. **NO usar `deploy-service` para Rust** — salvo excepción `--skip-compose-sync`.
4. **NO levantar compose por SSH sin verificar bind mounts primero.**
5. **Siempre verificar health post-deploy.** Si falla: redeploy inmediato.
6. **Backup antes de cambios con riesgo de datos.** `--skip-backup` solo para cambios de código puro.
7. **Ante incidente: usar `incident investigate` primero** para obtener diagnóstico completo antes de actuar.
8. **Redacción automática:** todos los comandos de investigación redactan secretos automáticamente. No copiar salida sin verificar.

---

## 8. Troubleshooting rápido

| Problema | Causa probable | Solución |
|---|---|---|
| `deploy --update` devuelve 503 | Build Rust en progreso (8-12 min) | Esperar, verificar con `health` |
| Health check falla post-deploy | App no arrancó o bind mount roto | `redeploy` primero; si persiste, logs + exec |
| Logs no muestran app Rust | `logs` apunta a WP por defecto | Usar `incident logs` o SSH + `docker compose logs app` |
| App viva pero no ve uploads | Bind mount reemplazado por named volume | Verificar compose on-disk, corregir, `docker compose up -d` |
| `28P01 password authentication failed` | DNS collision con `postgres` de coolify-db | Usar `postgres-{uuid}` en DATABASE_URL |
| `restart --all` deja Rust caído | Coolify no puede rebuild imagen local | `redeploy` sitio por sitio |
| Freeze / Bad Gateway | Watchdog mató el proceso tras 30s de starvation | `incident investigate --since 4h` → revisar eventos + logs + stats |
| OOM killed | Memoria agotada | `container inspect` → verificar OOM, `db-stats` → pool exhaustion |

---

## 9. Referencias

- **Repositorio:** github.com/1ndoryu/coolify-manager-rs (rama `main`)
- **Workspace folder:** `c:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs`
- **Protocolo:** Reglas 1 y 19 — prohíben deploy directo SSH, obligan coolify-manager-rs
- **Plan de incidente Nakomi:** `Agente/planes/plan-incidente-freeze-bad-gateway-nakomi-2026-07-24.md`
