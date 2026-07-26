# Plan: Diagnóstico y corrección de conectividad VPS1 en producción

> **Contexto:** El panel de infraestructura de `nakomi.studio` solo muestra despliegues de VPS2
> y las métricas CPU/RAM/disco no cargan. El agente anterior (GitHub Copilot) hizo cambios en
> `coolify-manager-rs` y un deploy, pero la sesión terminó sin verificar resultados.
>
> **Problema raíz identificado:** El contenedor Docker de `studio` no podía:
> 1. Resolver `http://coolify:8000` (puerto equivocado; el interno correcto es `http://coolify:8080`)
> 2. Usar `ssh` — binario ausente en la imagen
> 3. Leer rutas SSH (`COOLIFY_VPS1_SSH_KEY_PATH=ruta_windows`) — rutas Windows inválidas en Linux
>
> **Estado actualizado:** ✅ COMPLETADO — subsumido y resuelto por `plan-fixes-permanentes-studio-2026-05-23.md`.
> Todos los diagnósticos y correcciones de este plan se ejecutaron en las fases 1-5
> del plan de fixes permanentes (2026-05-23). SSH VPS1/VPS2 runtime validado,
> red coolify reconectada, build completo desplegado.
> - `COOLIFY_VPS1_BASE_URL=http://coolify:8080` — runtime alcanzable desde contenedor (`401` esperado sin token)
> - `COOLIFY_VPS1_SSH_KEY_PATH=/home/appuser/.ssh/id_ed25519` — forzado por `deploy-service` en compose efectivo
> - Rutas Windows — filtradas por el manager para que no entren al runtime Linux
> - `openssh-client` — agregado al Dockerfile via template `rust-stack.yaml`
> - Conexión a red `coolify` — agregada al template
> - Autoheal systemd — instalado (`cm-autoheal-studio.timer` cada 60s)
> - Build completo con backup `20260523_163630-pre_deploy_service` en curso al momento de esta actualización

---

## Reglas de seguridad

1. **No ejecutar SSH directo.** Toda operación contra producción usa `coolify-manager-rs`:
   - `exec` para comandos dentro del contenedor
   - `health` para health checks
   - `logs` para revisar logs
   - Ningún comando `ssh`, `docker`, `scp` directo
2. **No modificar producción sin aprobación explícita.** Cada paso propuesto se muestra primero.
3. **No mostrar secretos.** Variables con credenciales se muestran como `=present` o `=missing`. Para materializar claves SSH se usa base64, nunca texto plano.
4. **Revertir si algo rompe.** Comando `restore` disponible vía manager.

## Análisis de riesgos y mitigaciones

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| **Sin backup previo** | Alta | Alto — no hay punto de restauración | El agente anterior usó `--skip-backup`. No ejecutar deploy sin antes hacer backup explícito: `deploy-service --name studio` (sin `--skip-backup`). |
| **Fase 2 modifica prod** | Baja | Crítico | Todos los comandos en Fase 2 son **read-only**: `echo`, `printenv`, `curl`, `getent`, `ls`, `ip addr`. Sin pipes a archivos, sin `rm`, sin `sed`. |
| **Exposición de claves SSH en comandos** | Media | Alto — leak de secretos a logs/historial | Usar base64 encoding para transferir claves. No pasar contenido de clave como argumento. Usar `--` para separar opciones de argumentos. |
| **SSH falla por StrictHostKeyChecking** | Alta | Medio — métricas no cargan | El código actual de `infrastructure_metrics.rs` y `docker_stats.rs` usa `-o StrictHostKeyChecking=accept-new`. No debería fallar, pero si falta conectividad de red, no hay fix local. |
| **Claves SSH quedan expuestas en contenedor** | Media | Medio — acceso al contenedor = acceso a servidores | Si se materializan, las claves se guardan en `/root/.ssh/id_rsa` con permisos 600. Revisar que el contenedor no tenga usuarios no autorizados. |
| **coolify:8000 no resuelve** | Media | Alto — VPS1 sigue sin métricas | Verificar conexión a red coolify con `ip addr`. Si falta, reconectar: `coolify-manager exec --target app -- docker network connect coolify <container>`. |
| **Rebuild del contenedor causa downtime** | Baja | Medio | El deploy de coolify-manager hace blue/green: el contenedor viejo sigue sirviendo durante el build. Solo hay ~1s de swap al final. |
| **El exec del manager no funciona si el contenedor está caído** | Baja | Alto — no se puede diagnosticar | Si `exec` falla, recurrir a health público + Coolify API directamente para diagnóstico mínimo. Último recurso: SSH directo al servidor VPS2 (solo con aprobación explícita). |

---

## Fase 1: Pre-diagnóstico (solo HTTP + manager health)

Objetivo: Determinar el estado actual del servicio sin entrar al contenedor.

| Paso | Comando | Qué verifica |
|------|---------|-------------|
| 1.1 | `Invoke-WebRequest nakomi.studio/healthz` | Health público |
| 1.2 | `Invoke-WebRequest nakomi.studio/api/health` | Health API interna |
| 1.3 | `coolify-manager health --name studio` | Health vía Coolify API |
| 1.4 | `Invoke-WebRequest nakomi.studio/api/hosting/infrastructure/servers` | Despliegues visibles (requiere auth) — **solo si hay token** |
| 1.5 | `Invoke-WebRequest nakomi.studio/panel` | Panel responde |

**Criterio de éxito:** Los 4 endpoints responden 200. Si no, hay un problema más grave que aborta el diagnóstico.

---

## Fase 2: Diagnóstico interno (vía `coolify-manager exec`)

Objetivo: Verificar que el runtime post-deploy tiene las correcciones necesarias.

### 2.1 — Presencia de binarios y variables de entorno

**Comando único (sin secretos):**
```powershell
$cmd = @'
set +e
for k in COOLIFY_VPS1_BASE_URL COOLIFY_VPS1_SERVER_IP COOLIFY_VPS1_SERVER_UUID COOLIFY_VPS1_PROJECT_UUID COOLIFY_VPS1_API_TOKEN COOLIFY_VPS1_SSH_KEY_PATH COOLIFY_BASE_URL COOLIFY_SERVER_IP COOLIFY_SERVER_UUID COOLIFY_PROJECT_UUID COOLIFY_API_TOKEN COOLIFY_SSH_KEY_PATH; do
  if [ -n "$(printenv "$k")" ]; then echo "$k=present"; else echo "$k=missing"; fi
done
echo "curl=$(command -v curl || echo missing)"
echo "ssh=$(command -v ssh || echo missing)"
echo "ip=$(command -v ip || echo missing)"
'@
& $cm exec --name studio --target app --command $cmd
```

**Criterio de éxito:** `ssh=present`, todas las `COOLIFY_*` variables `=present`.

### 2.2 — Conectividad a Coolify VPS1

**Comando único:**
```powershell
$cmd = @'
set +e
echo "=== Coolify routes ==="
for url in \
  "$(printenv COOLIFY_VPS1_BASE_URL)/api/v1/services" \
  "$(printenv COOLIFY_BASE_URL)/api/v1/services" \
  "http://coolify:8080/api/v1/services"; do
  [ -z "$url" ] && continue
  [ "$url" = "/api/v1/services" ] && continue
  code=$(curl -m 5 -sS -o /dev/null -w "%{http_code}" "$url" 2>&1)
  echo "$url -> $code"
done
echo "=== DNS resolution ==="
getent hosts coolify 2>/dev/null || echo "coolify: NXDOMAIN"
echo "=== Network interfaces ==="
ip addr show 2>/dev/null || echo "ip not available"
'@
& $cm exec --name studio --target app --command $cmd
```

**Criterio de éxito:**
- `COOLIFY_VPS1_BASE_URL/api/v1/services` responde (401 esperado, significa "reachable, no token needed for this endpoint")
- `coolify:8080` resuelve via `getent` o curl
- `ip addr show` muestra interfaz en la red `coolify`

### 2.3 — Acceso a claves SSH

**Comando único (sin mostrar contenido):**
```powershell
$cmd = @'
set +e
for k in COOLIFY_VPS1_SSH_KEY_PATH COOLIFY_SSH_KEY_PATH; do
  p=$(printenv "$k")
  if [ -n "$p" ]; then
    [ -r "$p" ] && echo "$k readable" || echo "$k not_readable:$p exists=$(test -e "$p" && echo yes || echo no)"
  else
    echo "$k empty"
  fi
done
echo "=== /root/.ssh ==="
ls -la /root/.ssh/ 2>/dev/null || echo "/root/.ssh/ not found"
echo "=== known_hosts ==="
ls -la /root/.ssh/known_hosts 2>/dev/null || echo "no known_hosts"
'@
& $cm exec --name studio --target app --command $cmd
```

**Criterio de éxito:**
- `$k readable` para al menos `COOLIFY_SSH_KEY_PATH`
- `/root/.ssh/` existe (el agente anterior no configuró esto, es probable que falle)
- Si falla: `not_readable` con ruta Windows → las claves no se materializaron

---

## Fase 3: Corrección (solo con aprobación)

Basado en resultados de Fase 2, se determina qué falta:

| Diagnóstico | Acción | Comando/Tool |
|-------------|--------|-------------|
| `ssh=missing` | Rebuild de imagen Docker | `deploy-service --name studio` (sin --skip-build) |
| `coolify:8080` no resuelve | Verificar conexión a red coolify; reconectar si es necesario desde `deploy-service` | manager health/deploy-service |
| SSH key paths son Windows | **Problema más complejo:** hay que materializar claves dentro del contenedor o cambiar a paths Linux | Sub-fase 3A |
| SSH key paths no existen/ilegibles | Subir claves al servidor y apuntar env a paths Linux | Sub-fase 3B |
| Todo OK pero panel no muestra métricas | Problema en el sampler o handler, no en conectividad | Depuración de `infrastructure_metrics.rs` |

### Sub-fase 3A — Materializar claves SSH en el contenedor

Las claves SSH están localmente en la ruta Windows de `~/.ssh/`. El contenedor Linux no puede leer rutas Windows. Soluciones:

**Opción aplicada:** Configurar `coolify-manager-rs` para que en el deploy monte la clave generada en el host VPS1:
- Host: `/root/studio-ssh/id_ed25519`
- Contenedor: `/home/appuser/.ssh/id_ed25519`
- Env runtime: `COOLIFY_VPS1_SSH_KEY_PATH=/home/appuser/.ssh/id_ed25519`
- El mount no usa `:ro` porque el entrypoint necesita ajustar owner/permisos antes de ejecutar como `appuser`.

**Opción 2 (workaround):** Usar `coolify-manager exec` para subir las claves manualmente:
```powershell
$cm exec --name studio --target app --command "mkdir -p /root/.ssh"
# Luego subir clave con scp vía el manager (si existe el comando)
```

**Opción 3 (puenteo):** Cambiar el sampler para que lea las claves desde una variable de entorno directa (el contenido), no desde un archivo. Requiere modificar `infrastructure_metrics.rs`.

### Sub-fase 3B — Alternativa si no se pueden materializar claves

Si la materialización de claves es compleja, como workaround se puede:
1. Generar un par de llaves SSH nuevo específico para el contenedor
2. Agregar la llave pública a `authorized_keys` en el servidor VPS1
3. Usar la llave privada directamente dentro del contenedor

---

## Fase 4: Validación post-corrección

Repetir Fase 2 completa para confirmar:
- `ssh=present`
- `coolify:8000` responde `401`
- Claves SSH legibles
- `ip addr` muestra red coolify

Luego verificar el panel de infraestructura:
```powershell
# GET al endpoint de infraestructura (requiere token de auth)
Invoke-WebRequest -Uri "https://nakomi.studio/api/hosting/infrastructure/servers" -Headers @{Authorization="Bearer $token"}
```

---

## Fase 5: Rollback (si algo sale mal)

Si el deploy rompe algo:
```powershell
$cm redeploy --name studio  # redeploy sin rebuild (última imagen buena)
```
O si hay backup disponible:
```powershell
$cm restore --name studio
```

Siempre verificar health después:
```powershell
$cm health --name studio
```

---

## Resumen de comandos seguros (sin SSH directo)

| Tool | Propósito |
|------|-----------|
| `coolify-manager health --name studio` | Health check vía Coolify |
| `coolify-manager exec --name studio --target app --command '...'` | Ejecutar comando dentro del contenedor |
| `coolify-manager deploy-service --name studio [--skip-build]` | Deploy/rebuild |
| `coolify-manager logs --name studio` | Logs del contenedor |

Ninguno de estos usa SSH directo desde el host local. `exec` usa SSH pero a través del manager.
