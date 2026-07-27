# Plan: Deploy Producción Nakomi — 27 julio 2026

> **Objetivo:** Desplegar todos los commits del bloque 277A a producción.
> **Servicio:** `nakomi.studio` (Coolify service `do8k4w8swccwwogoc0os0ck0`)
> **Rama:** `glory-rust-nakomi`
> **Autorización requerida:** ✅ Pendiente del usuario

---

## 1. Pre-deploy: diagnóstico

Antes de ejecutar, verificar estado actual:

```powershell
# 1. Health actual
coolify-manager-rs health --name studio

# 2. Commit desplegado vs local
coolify-manager-rs exec --name studio -- cat .git/HEAD
git rev-parse HEAD

# 3. Último log
coolify-manager-rs logs --name studio --tail 20
```

**Si producción ya está desactualizada**, hay 15+ commits pendientes. Si está al día, solo los del 27 julio.

---

## 2. Contenido del deploy (por commit)

### Grupo A — Fixes menores (bajo riesgo, sin migración SQL)

| Commit | Descripción | Archivos | Riesgo |
|--------|-------------|----------|--------|
| `c2351ab1` | Badge notifs, sidebar, dedup, sesiones cerradas, rate limit | 4 archivos Rust | 🟢 Bajo |
| `a944de64` | Watchdog doble señal (main.rs integration) | 1 archivo | 🟢 Bajo |
| `4d95fb4` | Watchdog doble señal (framework glory-rs) | 1 archivo | 🟢 Bajo |
| `de66cf5a` | Errores TS preexistentes (ModalAsignar, SidebarPanel) | 2 archivos TS | 🟢 Bajo |

**Validación:** `cargo check` + `npx tsc --noEmit` ya pasan.

### Grupo B — Pagos/reembolsos (RIESGO MEDIO — incluye migración SQL)

| Commit | Descripción | Archivos | Riesgo |
|--------|-------------|----------|--------|
| `922061d9` | Pagos/reembolsos F+G: idempotency, retry backoff, constraint, validación monto | 9 archivos (+449/-151) | 🟡 Medio |

**Contenido:**
- **Migración SQL:** `20260727000000_payment_refund_hardening.up/down.sql`
  - `refund_status` enum: añade `processing` y `failed`
  - Tabla `order_refunds`: columnas `retry_count`, `next_retry_at`, `last_error`, `idempotency_key`
  - Constraint único `(order_id, status)` donde status IN requested/processing/approved
  - Trigger `updated_at` automático
- **RefundService:** retry con backoff cap 72h, recovery de Processing stuck
- **PaymentService:** Idempotency-Key en todos los llamados Stripe, validación monto
- **Background worker:** retry cada 15 min en `main.rs`

**Riesgos:**
1. Migración SQL añade columnas a tabla existente — compatible con datos existentes
2. Enum `refund_status` se expande — no rompe valores existentes
3. Constraint único puede fallar si hay refunds duplicados ya en la tabla
4. Stripe API: Idempotency-Key es nueva — Stripe la acepta sin problemas

**Mitigación:** La migración es additive (no borra nada). Si falla el constraint, se puede hacer `ON CONFLICT DO NOTHING`.

### Grupo C — SEO fixes (bajo riesgo, sin migración SQL)

| Commit | Descripción | Archivos | Riesgo |
|--------|-------------|----------|--------|
| `a837fdcf` | SEO audit 404: ruta duplicada | 1 archivo Rust | 🟢 Bajo |
| `2ca61834` | hreflang, dateModified, FAQ schema hosting/VPS | 5 archivos TS | 🟢 Bajo |
| `d90ab450` | SEO audit fixes + banner 50% + breadcrumbs | 11 archivos | 🟢 Bajo |
| `5203adcf` | Roadmap actualizado | 1 archivo MD | ⚪ N/A |

---

## 3. Orden de ejecución recomendado

### Opción 1: Todo en un solo deploy (RECOMENDADO)

```powershell
# Pull + rebuild + health check
coolify-manager-rs deploy --name studio --update
# Verificar
coolify-manager-rs health --name studio
```

**Ventaja:** Un solo ciclo de deploy. Todos los cambios son compatibles entre sí.
**Desventaja:** Si algo falla, rollback es más amplio.

### Opción 2: Deploy escalonado (si se quiere minimizar riesgo)

```powershell
# Paso 1: Grupo A (fixes menores) — merge parcial no trivial, saltar
# Paso 2: Grupo B + C juntos — todo lo del 27 julio
coolify-manager-rs deploy --name studio --update
```

**Nota:** Los commits están entrelzados en la historia git. No se pueden deployar parcialmente sin cherry-pick complejo. **Recomendado: deploy completo.**

---

## 4. Verificación post-deploy

### Checklist obligatorio

```powershell
# 1. Health check
coolify-manager-rs health --name studio

# 2. Logs sin errores
coolify-manager-rs logs --name studio --tail 50

# 3. Verificar migración SQL aplicada
coolify-manager-rs exec --name studio -- psql $DATABASE_URL -c "\d order_refunds"

# 4. Verificar watchdog activo
coolify-manager-rs logs --name studio --grep "watchdog\|probe"

# 5. Test endpoint SEO
curl -s https://nakomi.studio/api/admin/seo/audit | head -c 500

# 6. Test endpoint first-order-discount
curl -s https://nakomi.studio/api/orders/first-order-discount

# 7. Test frontend (abrir en navegador)
# - https://nakomi.studio/ (Inicio — JSON-LD visible en view-source)
# - https://nakomi.studio/servicios (organizationSchema)
# - https://nakomi.studio/proyectos (título largo)
# - https://nakomi.studio/blog/{slug} (dateModified en schema)
# - https://nakomi.studio/soluciones/hosting-wordpress (FAQ schema)
# - https://nakomi.studio/soluciones/vps (FAQ schema)
```

### Verificaciones específicas por grupo

| Grupo | Verificación | Comando |
|-------|-------------|---------|
| A | Badge notifs visible en Header admin | Navegar panel admin |
| A | Rate limit funciona | Hacer 6+ claims rápidos |
| B | Refund status enum tiene 4 valores | `SELECT enum_range(null::refund_status)` |
| B | Stripe idempotency funciona | Crear orden de prueba |
| C | SEO audit sin 404 | `GET /api/admin/seo/audit` |
| C | FAQ schema válido | Google Rich Results Test |
| C | Banner 50% aparece | Checkout como usuario nuevo |

---

## 5. Rollback

### Si la migración SQL falla

```powershell
# Rollback migración
coolify-manager-rs exec --name studio -- sqlx migrate revert

# O restaurar backup
coolify-manager-rs restore --name studio
```

### Si el deploy rompe algo funcional

```powershell
# Rollback al commit anterior
coolify-manager-rs deploy --name studio --rollback

# O restaurar backup completo
coolify-manager-rs restore --name studio
```

### Si solo SEO tiene problemas

No requiere rollback — los cambios son cosmeticos y no afectan funcionalidad core.

---

## 6. Riesgos del deploy

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| Migración SQL falla por datos inconsistentes | Baja | Alto | Backup previo + rollback enum |
| Constraint único rechaza refunds existentes | Media | Medio | Verificar datos antes de migrar |
| Watchdog mata el servicio por HTTP probe | Baja | Alto | Grace period 60s + umbral 120s |
| Stripe rechaza Idempotency-Key | Muy baja | Bajo | Documentado como supported |
| Banner 50% no aparece para usuarios nuevos | Baja | Bajo | Endpoint + query con `enabled` condicional |
| SEO audit muestra datos incorrectos | Baja | Bajo | Audit es informativo, no funcional |

---

## 7. Tiempo estimado

| Fase | Duración |
|------|----------|
| Pre-deploy diagnóstico | 2 min |
| Deploy (pull + build + restart) | 8-12 min (Rust build) |
| Verificación post-deploy | 5 min |
| **Total** | **15-20 min** |

---

## 8. Pendientes que NO se deployan

Los siguientes NO están commiteados y NO se incluyen:

1. **Breadcrumb schema en páginas de detalle** — ya implementado en commits anteriores
2. **Tests manuales SEO** — requieren ejecución en navegador
3. **Verificación canary continuación email** — requiere prueba manual
4. **Configuración correo hosting** — bloqueado en decisión de proveedor

---

## 9. Acción inmediata

**El usuario debe confirmar:**

1. ✅ ¿Deploy completo (todos los grupos A+B+C) o solo Grupo A?
2. ✅ ¿Ahora o en horario específico?
3. ✅ ¿Backup previo obligatorio o skip-backup (cambios de código puros)?

**Comando a ejecutar tras autorización:**

```powershell
# Con backup (recomendado para migración SQL)
coolify-manager-rs deploy --name studio --update

# Sin backup (solo si no hay migración SQL)
coolify-manager-rs deploy --name studio --update --skip-backup
```

**Recomendación:** Con backup, dado que incluye migración SQL de pagos/reembolsos.
