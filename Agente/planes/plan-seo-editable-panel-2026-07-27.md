ha# Plan: SEO Editable — Panel con Acciones + DB-backed Settings

> **Fecha:** 2026-07-27
> **Objetivo:** Que todo el contenido SEO estático sea editable desde el panel admin, que el prerender lea de la DB, y que no haya problemas de SEO en contenido estático.
> **Rama:** `glory-rust-nakomi`
> **Dependencia:** Migración `20260727100000_seo_settings` ya creada (pendiente aplicar en producción)

---

## 1. Problema actual

| Componente | Estado | Problema |
|------------|--------|----------|
| `admin_seo.rs` — `static_pages()` | Hardcodeado en Rust | No se puede editar sin redeploy. El audit muestra valores que no necesariamente coinciden con lo que sirve el prerender |
| `prerender.rs` — `resolve_seo_meta()` | Hardcodeado en Rust | Los meta tags de páginas estáticas no se pueden cambiar sin tocar código |
| Panel SEO (`SeccionSeo.tsx`) | Solo lectura | Muestra el audit pero no permite editar nada. Sin botones de acción |
| Tabla `seo_settings` | Creada en migración | Existe pero nadie la lee ni la escribe |

**Resultado:** El usuario ve problemas de SEO en casi todas las páginas estáticas y no puede hacer nada al respecto desde el panel.

---

## 2. Diseño de la solución

### 2.1 Arquitectura

```
┌─────────────────────────────────────────────────────────┐
│                    Panel Admin SEO                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐  │
│  │ Resumen  │  │ Páginas  │  │  Blog    │  │  GEO   │  │
│  └──────────┘  └────┬─────┘  └──────────┘  └────────┘  │
│                     │                                    │
│         ┌───────────┴───────────┐                       │
│         │ Tabla con acciones    │                       │
│         │ ✏️ Editar (estáticas)  │                       │
│         │ 🔗 Ir al CMS (dinámico)│                       │
│         └───────────┬───────────┘                       │
│                     │                                    │
│              ┌──────▼──────┐                            │
│              │ Modal Edición│                            │
│              │ Title        │                            │
│              │ Description  │                            │
│              │ OG Image URL │                            │
│              └──────┬──────┘                            │
└─────────────────────┼───────────────────────────────────┘
                      │ PUT /api/admin/seo/settings/:path
              ┌───────▼───────┐
              │  seo_settings  │ ← tabla PostgreSQL
              │  path (PK)     │
              │  label         │
              │  title         │
              │  description   │
              │  og_image_url  │
              │  json_ld_type  │
              │  updated_at    │
              └───────┬───────┘
                      │
         ┌────────────┼────────────┐
         │                         │
   ┌─────▼──────┐          ┌──────▼──────┐
   │ prerender  │          │ admin_seo   │
   │ (middleware)│          │ (audit)     │
   │ Lee de DB  │          │ Lee de DB   │
   │ con cache  │          │ sin cache   │
   └────────────┘          └─────────────┘
```

### 2.2 Flujo de edición

1. Usuario abre panel SEO → sub-tab "Páginas"
2. Ve tabla con todas las páginas (estáticas + dinámicas)
3. **Páginas estáticas** (page_type === "static"):
   - Columna "Acciones" con icono ✏️ (editar)
   - Click → abre `ModalSeoEdit` con title, description, og_image_url precargados
   - Guardar → `PUT /api/admin/seo/settings/:path` → actualiza DB
   - El audit se refresca automáticamente (invalidate query)
   - El prerender lee de DB (con cache 5min) → los crawlers reciben los nuevos valores
4. **Páginas dinámicas** (service, project):
   - Columna "Acciones" con icono 🔗 (ir al CMS)
   - Click → navega al editor correspondiente en el panel
5. **Blog posts** (ya en sub-tab Blog):
   - Link al editor de blog existente

### 2.3 Reglas de negocio

| Regla | Implementación |
|-------|---------------|
| Solo admin puede editar SEO settings | `require_admin()` en endpoints |
| Título: 30-60 chars (ideal) | Validación frontend + warning en audit |
| Descripción: 70-160 chars (ideal) | Validación frontend + warning en audit |
| OG Image: URL válida o vacía | Validación frontend |
| JSON-LD type: de lista cerrada | Select en modal |
| Cache prerender: 5 min TTL | `RwLock<HashMap<path, (SeoMeta, Instant)>>` |
| Audit siempre lee fresco de DB | Sin cache en audit endpoint |

---

## 3. Archivos a crear/modificar

### Backend (Rust)

| # | Archivo | Acción | Detalle |
|---|---------|--------|---------|
| B1 | `src/repositories/seo_settings.rs` | **Nuevo** | CRUD: `list_all()`, `find_by_path()`, `upsert()` |
| B2 | `src/repositories/mod.rs` | Modificar | Registrar `pub mod seo_settings;` |
| B3 | `src/handlers/admin_seo.rs` | Modificar | Añadir endpoints `GET /admin/seo/settings`, `PUT /admin/seo/settings/:path`. Modificar `seo_audit()` para leer de DB |
| B4 | `src/middleware/prerender.rs` | Modificar | `resolve_seo_meta()` lee páginas estáticas de DB con cache en vez de hardcoded |

### Frontend (TypeScript/React)

| # | Archivo | Acción | Detalle |
|---|---------|--------|---------|
| F1 | `frontend/src/api/admin-seo.ts` | Modificar | Añadir tipos `SeoSetting`, `UpdateSeoSettingBody` + funciones `apiGetSeoSettings()`, `apiUpdateSeoSetting()` |
| F2 | `frontend/src/components/panel/ModalSeoEdit.tsx` | **Nuevo** | Modal de edición con campos: title, description, og_image_url. Validación en vivo (char count, warnings) |
| F3 | `frontend/src/components/panel/ModalSeoEdit.css` | **Nuevo** | Estilos del modal (reutilizar patrón de modales existentes) |
| F4 | `frontend/src/components/panel/SubTabSeoPaginas.tsx` | Modificar | Añadir columna "Acciones" con botones ✏️ (estáticas) y 🔗 (dinámicas) |
| F5 | `frontend/src/components/panel/SeccionSeo.css` | Modificar | Estilos para columna de acciones |

---

## 4. Detalle de implementación

### B1 — Repositorio `seo_settings.rs`

```rust
pub struct SeoSetting {
    pub path: String,
    pub label: String,
    pub title: String,
    pub description: String,
    pub og_image_url: Option<String>,
    pub json_ld_type: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct SeoSettingsRepository;

impl SeoSettingsRepository {
    pub async fn list_all(pool: &PgPool) -> Result<Vec<SeoSetting>, sqlx::Error>;
    pub async fn find_by_path(pool: &PgPool, path: &str) -> Result<Option<SeoSetting>, sqlx::Error>;
    pub async fn upsert(pool: &PgPool, path: &str, title: &str, description: &str, og_image_url: Option<&str>) -> Result<SeoSetting, sqlx::Error>;
}
```

### B3 — Endpoints nuevos en `admin_seo.rs`

```rust
// GET /api/admin/seo/settings — lista todos los settings
async fn list_seo_settings(State(state): State<AppState>, auth: AuthUser)
    -> Result<Json<Vec<SeoSetting>>, AppError>;

// PUT /api/admin/seo/settings/:path — actualiza un setting
async fn update_seo_setting(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<String>,
    Json(body): Json<UpdateSeoSettingBody>,
) -> Result<Json<SeoSetting>, AppError>;
```

Registrar en `routes()`:
```rust
Router::new()
    .route("/admin/seo/audit", get(seo_audit))
    .route("/admin/seo/settings", get(list_seo_settings))
    .route("/admin/seo/settings/{path}", put(update_seo_setting))
```

### B3 — Modificar `seo_audit()` para leer de DB

En vez de iterar `static_pages()`, hacer:
```rust
let db_settings = SeoSettingsRepository::list_all(pool).await.unwrap_or_default();
for setting in &db_settings {
    // evaluar con evaluate_page() usando los valores de DB
}
```

### B4 — Modificar `prerender.rs` para leer de DB

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};

struct CachedSeoMeta {
    meta: SeoMeta,
    fetched_at: Instant,
}

// En PrerenderState añadir:
pub seo_cache: Arc<RwLock<HashMap<String, CachedSeoMeta>>>,
pub seo_cache_ttl: Duration, // 5 minutos

// En resolve_seo_meta, para rutas estáticas:
// 1. Check cache
// 2. Si miss o expirado → SELECT from seo_settings WHERE path = $1
// 3. Construir SeoMeta desde el resultado de DB
// 4. Guardar en cache
```

### F2 — Modal de edición SEO

```
┌─────────────────────────────────────────────┐
│  ✏️ Editar SEO: Inicio                 [X]  │
├─────────────────────────────────────────────┤
│                                             │
│  Título                              48/60  │
│  ┌─────────────────────────────────────┐    │
│  │ Nakomi Studio — Agencia Creativa... │    │
│  └─────────────────────────────────────┘    │
│  ⚠ Recomendado: 30-60 caracteres           │
│                                             │
│  Descripción                        120/160 │
│  ┌─────────────────────────────────────┐    │
│  │ Estudio creativo basado en Copen... │    │
│  └─────────────────────────────────────┘    │
│  ⚠ Recomendado: 70-160 caracteres          │
│                                             │
│  Imagen OG (URL)                            │
│  ┌─────────────────────────────────────┐    │
│  │ https://nakomi.studio/assets/og...  │    │
│  └─────────────────────────────────────┘    │
│  ℹ Vacío = imagen por defecto               │
│                                             │
│         [ Cancelar ]  [ Guardar ]           │
└─────────────────────────────────────────────┘
```

### F4 — Tabla con columna de acciones

```
| Página      | Title                | Descripción          | OG    | JSON-LD  | Estado | Acciones |
|-------------|----------------------|----------------------|-------|----------|--------|----------|
| 🏷 Estática | Inicio               | Nakomi Studio...     | Sí    | Org+Web  | OK     | [✏️]     |
| 🏷 Estática | Servicios            | Nuestros Servicios.. | Sí    | Org      | OK     | [✏️]     |
| 🏷 Servicio | Diseño Web           | Creamos sitios...    | Sí    | Service  | OK     | [🔗]     |
| 🏷 Proyecto | Kamples              | App de productividad | Sí    | —        | ⚠️     | [🔗]     |
```

---

## 5. Orden de ejecución

```
Paso 1: B1 + B2 — Crear repositorio seo_settings
Paso 2: B3 — Endpoints + modificar audit para leer de DB
Paso 3: B4 — Modificar prerender para leer de DB con cache
Paso 4: F1 — API calls frontend
Paso 5: F2 + F3 — Modal de edición
Paso 6: F4 + F5 — Añadir columna acciones a tabla
Paso 7: Validación (cargo check + clippy + tsc)
Paso 8: Code review + commit
```

**Commits separados:**
- Commit 1: Backend (B1-B4) — repositorio + endpoints + prerender dinámico
- Commit 2: Frontend (F1-F5) — modal + acciones + integración

---

## 6. Despliegue a producción

### Pre-requisito
La migración `20260727100000_seo_settings.up.sql` debe aplicarse. Incluye:
- Creación de tabla `seo_settings`
- INSERT de las 10 páginas estáticas con valores correctos

### Después del deploy
1. Verificar que la tabla existe: `SELECT * FROM seo_settings;`
2. Verificar que el audit muestra los valores de DB
3. Verificar que el prerender sirve los valores de DB a crawlers
4. Editar un título desde el panel → verificar que el prerender lo refleja

### Para cambios futuros
El usuario puede editar cualquier página estática desde el panel SEO → Páginas → ✏️. Los cambios se reflejan inmediatamente en el audit y en ~5 minutos en el prerender (cache TTL).

---

## 7. Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| Cache stale en prerender (usuario edita pero crawlers ven lo viejo 5min) | Alta | Bajo | 5 min TTL es aceptable. Se puede forzar flush con restart |
| Migración no aplicada en producción | Media | Alto | Verificar antes de deploy. Si falta, el prerender fallback a hardcoded |
| Path encoding en URL (`/` → `%2F`) | Media | Medio | Usar `{path}` wildcard de Axum, no query param |
| Race condition en cache RwLock | Muy baja | Bajo | `RwLock` permite múltiples lectores concurrentes |

---

## 8. Estimación

| Tarea | Esfuerzo |
|-------|----------|
| B1: Repositorio seo_settings | 30min |
| B2: Registrar módulo | 5min |
| B3: Endpoints + audit dinámico | 1-1.5h |
| B4: Prerender dinámico con cache | 1.5-2h |
| F1: API calls | 15min |
| F2+F3: Modal de edición | 1.5h |
| F4+F5: Columna acciones | 45min |
| Validación + review | 30min |
| **Total** | **6-7h** |

---

## 9. Criterio de cierre

- [ ] Panel SEO muestra tabla con columna "Acciones"
- [ ] Botón ✏️ abre modal de edición para páginas estáticas
- [ ] Botón 🔗 navega al CMS para páginas dinámicas
- [ ] Guardar cambios → se refleja en audit inmediatamente
- [ ] Guardar cambios → se refleja en prerender en ≤5 min
- [ ] Prerender fallback a hardcoded si la DB no tiene datos
- [ ] `cargo check` + `cargo clippy` + `npx tsc --noEmit` pasan
- [ ] Sin CSS custom innecesario (reutiliza patrones existentes)
