# Plan: SEO Dashboard + Blog + GEO + Descuento Primer Pedido

> **Fecha:** 2026-07-25
> **Estado:** Planificado — pendiente de aprobación
> **Prioridad:** Alta — impacto directo en adquisición de clientes
> **Rama:** `glory-rust-nakomi`
> **Objetivo:** Posicionar Nakomi Studio para que agentes IA (ChatGPT, Perplexity, Claude, Gemini) lo recomienden cuando usuarios busquen servicios de desarrollo web, y capturar clientes pequeños con un descuento agresivo del primer pedido.

---

## 1. Contexto estratégico

### 1.1 El cambio de paradigma: SEO → GEO (Generative Engine Optimization)

El SEO tradicional (keywords, backlinks, blue links) sigue importando, pero hay un
cambio fundamental: **los usuarios ahora preguntan a IA antes que a Google**.

Cuando alguien dice "necesito una web para mi negocio" en ChatGPT, Perplexity o
Gemini, esas IA buscan en internet y eligen fuentes para recomendar. El objetivo
es que Nakomi sea una de esas fuentes.

**Principios GEO (2025-2026):**

| Principio | Qué significa | Cómo aplicar a Nakomi |
|---|---|---|
| **Fact Density** | Las IA priorizan contenido con datos concretos, no opiniones | Cada servicio/proyecto con métricas reales: "LCP < 2s", "99.9% uptime", "40% más rápido" |
| **BLUF** (Bottom Line Up Front) | Primera oración de cada sección = respuesta directa (40-60 palabras) | Descripciones de servicios que empiezan con qué hace, no con filosofía |
| **Neutralidad enciclopédica** | Las IA evitan contenido "salesy" | Tono informativo, no publicitario. "Nakomi ofrece X" vs "¡El mejor servicio del mundo!" |
| **Entity Clustering** | Google/IA reconocen entidades consistentes | Nombre, dirección, equipo, servicios = misma información en todo internet |
| **FAQ Schema** | Las IA extraen Q&A directamente | Cada página de servicio/solución con FAQ estructurado |
| **Content-Answer Fit** | El contenido debe responder preguntas reales | Blog posts que respondan "cuánto cuesta una web", "qué hosting necesito", etc. |

### 1.2 Objetivo de captura

**Cliente ideal:** PYME o emprendedor con proyecto pequeño (landing, web corporativa,
tienda básica). Presupuesto limitado, busca agencia confiable.

**Estrategia de captura:**
1. Que las IA recomienden Nakomi → contenido GEO optimizado
2. Que el usuario llegue al sitio → landing page clara con oferta
3. Que convierta → 50% descuento primer pedido como incentivo
4. Que confíe → caso de éxito + chat IA inmediato

---

## 2. Bloque A — Dashboard SEO Admin (profundizado)

### 2.1 Objetivo

Sección nueva en el panel admin existente. Muestra el estado SEO de todas las
páginas públicas del sitio con sub-tabs internas. Minimalista, compacto, sin
diseño custom — reutiliza los patrones visuales ya establecidos en el panel.

### 2.2 Patrones de diseño del panel (referencia)

Estudiados de los componentes existentes:

**Sub-tabs (patrón SeccionContenido):**
```tsx
<div className="contenidoSubTabs">
    <Button variante="texto" className={`contenidoSubTab ${activo ? 'contenidoSubTab--activo' : ''}`}>
        <Icono size={16} /> Label
    </Button>
</div>
```
CSS: borde inferior 2px, `--text-sm`, `--text-muted` → `--brand-black` en activo.

**Filas de lista (patrón ListaServicios):**
```tsx
<div className="listaServiciosFila">
    <div className="listaServiciosFilaInfo">
        <span className="listaServiciosNombre">Nombre</span>
        <BadgeStatus status="published" />
        <span className="listaServiciosPrecio">Dato</span>
    </div>
</div>
```
CSS: `--border-default`, `--radius-md`, hover con `--brand-primary`, compacto.

**Badges de status (patrón ListaServicios):**
```css
.listaServiciosBadge { font-size: var(--text-2xs); padding: 2px 6px; border-radius: var(--radius-sm); }
.listaServiciosBadge--published { background: var(--bg-item-active); color: var(--text-primary); }
.listaServiciosBadge--draft { background: var(--bg-item-hover); color: var(--text-tertiary); }
```

**Tabla (patrón SeccionCorreo):**
```html
<table className="correosTabla">
    <thead><tr><th>...</th></tr></thead>
    <tbody><tr className="correosFila">...</tr></tbody>
</table>
```
CSS: `--color-borde`, `--color-fondo-secundario` en thead, hover `--color-fondo-hover`.

**Pestañas (patrón SeccionCorreo):**
```tsx
<button className={`correosPestaniaBtn ${activa ? 'correosPestaniaActiva' : ''}`}>
    <Icono size={16} /> Label
</button>
```
CSS: borde inferior 2px, `--color-primario` en activa.

### 2.3 Estructura de la sección

```
SeccionSeo
├── Sub-tabs: Resumen | Páginas | Blog | GEO
│
├── SubTabResumen (default)
│   ├── Tarjetas de resumen (mismo patrón que PagoResumenCard)
│   │   ├── Páginas auditadas: N
│   │   ├── Problemas: N (rojo)
│   │   ├── Mejorables: N (gris)
│   │   └── OK: N (gris oscuro)
│   └── Lista de issues más críticos (top 5)
│
├── SubTabPaginas
│   ├── Filtro (mismo patrón correosFiltro)
│   └── Tabla SEO (mismo patrón correosTabla)
│       ├── Columnas: Página | Title | Desc | OG | JSON-LD | Status
│       └── Click en fila → abre editor correspondiente
│
├── SubTabBlog
│   └── Tabla de posts con estado SEO (mismo patrón ListaBlog)
│       ├── Columnas: Título | Slug | meta_title | meta_desc | Status
│       └── BadgeStatus: published / draft / archived
│
└── SubTabGeo
    ├── Checklist GEO (lista de tareas con checkbox)
    │   ├── ✅ llms.txt creado
    │   ├── ❌ FAQ schema en hosting
    │   ├── ⚠️ sameAs vacío en organizationSchema
    │   └── ...
    └── Próximos posts sugeridos (lista simple)
```

### 2.4 Detalle de cada sub-tab

#### SubTabResumen

4 tarjetas compactas en fila (mismo patrón `PagoResumenCard` simplificado):

```
┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│  18           │ │  2           │ │  8           │ │  8           │
│  Páginas      │ │  Problemas   │ │  Mejorables  │ │  OK          │
└──────────────┘ └──────────────┘ └──────────────┘ └──────────────┐
```

Debajo: lista de los 5 issues más críticos como filas simples con badge de severidad.

**CSS:** Reutilizar `.pagoResumenCard` / `.pagoResumenCardBody` o crear variante
mínima con los mismos tokens. Sin colores custom — solo escala de grises + el
mismo patrón de badges del panel.

#### SubTabPaginas

Tabla con el patrón exacto de `SeccionCorreo`:

| Columna | Ancho | Contenido |
|---|---|---|
| Página | `max-width: 200px` | Ruta legible (ej: "Servicios > Diseño Web") |
| Title | `max-width: 250px` | Truncado con ellipsis + tooltip con longitud |
| Description | `max-width: 300px` | Truncado con ellipsis + tooltip con longitud |
| OG | `40px` | Badge "Sí" / "No" (patrón `correosTag`) |
| JSON-LD | `60px` | Badge con tipo (Service, BlogPosting, etc.) |
| Status | `80px` | BadgeStatus: ok / warning / error |

**Filtro:** Select para filtrar por tipo (Todas / Estáticas / Servicios / Proyectos / Blog).

**Click en fila:** Dispara `window.dispatchEvent(new CustomEvent('panel-cambiar-tab', {detail: 'contenido'}))`
y abre el editor correspondiente. O simplemente muestra un popover con la vista
previa de Google (mismo patrón que los editores SEO ya tienen).

#### SubTabBlog

Mismo patrón que `ListaBlog` existente:

```
┌─ listaBlogFila ─────────────────────────────────────────┐
│  [imagen]  Título del post    published   /blog/slug     │
│            meta_title o "Sin meta title"                  │
│            meta_desc truncada o "Sin meta description"    │
└─────────────────────────────────────────────────────────┘
```

Badges de status: reutilizar `listaBlogBadge--published/draft/archived`.

Columna adicional: indicador SEO simple:
- ✅ meta_title + meta_description completos
- ⚠️ Uno de los dos vacío
- ❌ Ambos vacíos

#### SubTabGeo

Lista simple de checks (mismo patrón que la lista de features en planes):

```
┌─ Fila ──────────────────────────────────────────────────┐
│  [✓] llms.txt creado y accesible                        │
│  [✗] FAQ schema en /soluciones/hosting                  │
│  [⚠] sameAs vacío en organizationSchema                 │
│  [✓] Sitemap incluye todas las páginas                  │
│  [✗] Blog posts con dateModified                        │
└─────────────────────────────────────────────────────────┘
```

Cada fila es clickable y navega al recurso correspondiente o muestra instrucción.

Debajo: "Próximos pasos sugeridos" como lista simple de texto.

### 2.5 Backend necesario

**Nuevo endpoint:** `GET /api/admin/seo/audit`

Retorna:

```rust
#[derive(Serialize)]
struct SeoAuditResponse {
    summary: SeoAuditSummary,
    pages: Vec<SeoPageEntry>,
    blog_posts: Vec<SeoBlogEntry>,
    geo_checks: Vec<GeoCheck>,
}

#[derive(Serialize)]
struct SeoAuditSummary {
    total_pages: usize,
    ok: usize,
    warnings: usize,
    errors: usize,
    blog_published: usize,
    services_active: usize,
    projects_published: usize,
}

#[derive(Serialize)]
struct SeoPageEntry {
    path: String,                    // "/servicios/diseno-web"
    label: String,                   // "Diseño de Sitios Web"
    page_type: String,               // "static" | "service" | "project"
    entity_id: Option<Uuid>,
    title: Option<String>,
    title_len: usize,
    description: Option<String>,
    description_len: usize,
    og_image_is_default: bool,
    json_ld_type: Option<String>,
    status: String,                  // "ok" | "warning" | "error"
    issues: Vec<String>,             // ["TITLE_TOO_LONG", "MISSING_DESCRIPTION"]
}

#[derive(Serialize)]
struct SeoBlogEntry {
    id: Uuid,
    title: String,
    slug: String,
    meta_title: Option<String>,
    meta_description: Option<String>,
    status: String,                  // "published" | "draft" | "archived"
    seo_status: String,              // "ok" | "warning" | "error"
    issues: Vec<String>,
}

#[derive(Serialize)]
struct GeoCheck {
    id: String,                      // "llms-txt", "faq-hosting", "sameas-org"
    label: String,                   // "llms.txt creado y accesible"
    passed: bool,
    detail: Option<String>,          // "Falta sección de servicios"
    action_path: Option<String>,     // "/soluciones/hosting" o null
}
```

**Lógica del endpoint:**

1. Definir rutas estáticas hardcodeadas (igual que `prerender.rs`): `/`, `/servicios`, `/proyectos`, `/nosotros`, `/soluciones/hosting`, `/blog`, `/contacto`, `/privacidad`
2. Consultar `services` (is_active=true), `projects` (status='published'), `blog_posts` (status='published')
3. Para cada entrada, evaluar reglas:
   - `title.is_none()` → error
   - `title.len() > 60` → warning
   - `title.len() < 30` → warning
   - `description.is_none()` → error
   - `description.len() > 160` → warning
   - `description.len() < 70` → warning
   - `og_image_is_default` → warning
   - `json_ld_type.is_none()` → warning
4. Calcular summary (contadores)
5. Generar geo_checks estáticos (verificar llms.txt existe, verificar FAQ schema en páginas, etc.)
6. Retornar todo en una sola response

**Archivo:** `src/handlers/admin_seo.rs` (nuevo)
**Ruta:** `GET /api/admin/seo/audit` (requiere JWT admin)
**Registrar en:** `src/handlers/mod.rs` y `src/cli/mod.rs` (utoipa)

### 2.6 Frontend — Archivos a crear/modificar

| Archivo | Acción | Detalle |
|---|---|---|
| `frontend/src/data/panel.ts` | Modificar | Añadir `'seo'` a `SeccionPanel` type union. Añadir entry en `TABS_ADMIN`: `{id: 'seo', label: 'SEO', descripcion: 'Auditoría SEO de todas las páginas públicas. Títulos, descripciones, structured data y estado de indexación.'}` |
| `frontend/src/islands/PanelIsland.tsx` | Modificar | Añadir `import {SeccionSeo}` y `case 'seo': return <SeccionSeo />;` |
| `frontend/src/api/admin-seo.ts` | **Nuevo** | Tipos (`SeoAuditResponse`, `SeoPageEntry`, etc.) + `apiGetSeoAudit()` con `useQuery` key `['admin-seo-audit']` |
| `frontend/src/components/panel/SeccionSeo.tsx` | **Nuevo** | Componente principal con sub-tabs |
| `frontend/src/components/panel/SeccionSeo.css` | **Nuevo** | Estilos — 100% reutilización de patrones existentes |
| `frontend/src/components/panel/SubTabSeoResumen.tsx` | **Nuevo** | Tarjetas resumen + lista de issues críticos |
| `frontend/src/components/panel/SubTabSeoPaginas.tsx` | **Nuevo** | Tabla de páginas con filtro |
| `frontend/src/components/panel/SubTabSeoBlog.tsx` | **Nuevo** | Lista de posts con indicador SEO |
| `frontend/src/components/panel/SubTabSeoGeo.tsx` | **Nuevo** | Checklist GEO |

### 2.7 Implementación de SeccionSeo.tsx

Sigue el patrón exacto de `SeccionContenido`:

```tsx
import React, {useState, useEffect} from 'react';
import {BarChart3, FileText, PenTool, Globe} from 'lucide-react';
import {Button} from '../ui/Button';
import {SubTabSeoResumen} from './SubTabSeoResumen';
import {SubTabSeoPaginas} from './SubTabSeoPaginas';
import {SubTabSeoBlog} from './SubTabSeoBlog';
import {SubTabSeoGeo} from './SubTabSeoGeo';
import './SeccionSeo.css'; // reutiliza clases de SeccionContenido

type SubTab = 'resumen' | 'paginas' | 'blog' | 'geo';

const SUB_TABS = [
    {id: 'resumen' as SubTab, label: 'Resumen', icono: BarChart3},
    {id: 'paginas' as SubTab, label: 'Páginas', icono: FileText},
    {id: 'blog' as SubTab, label: 'Blog', icono: PenTool},
    {id: 'geo' as SubTab, label: 'GEO', icono: Globe},
];

const SEO_SUBTAB_KEY = 'panel-seo-subtab';

export const SeccionSeo: React.FC = () => {
    const [subTab, setSubTab] = useState<SubTab>(() => {
        const stored = localStorage.getItem(SEO_SUBTAB_KEY) as SubTab | null;
        if (stored && SUB_TABS.some(t => t.id === stored)) return stored;
        return 'resumen';
    });

    useEffect(() => {
        localStorage.setItem(SEO_SUBTAB_KEY, subTab);
    }, [subTab]);

    return (
        <div className="contenidoContenedor">
            <div className="contenidoSubTabs">
                {SUB_TABS.map(tab => {
                    const Icono = tab.icono;
                    return (
                        <Button
                            key={tab.id}
                            type="button"
                            variante="texto"
                            className={`contenidoSubTab ${subTab === tab.id ? 'contenidoSubTab--activo' : ''}`}
                            onClick={() => setSubTab(tab.id)}
                        >
                            <Icono size={16} />
                            {tab.label}
                        </Button>
                    );
                })}
            </div>
            <div className="contenidoPanel">
                {subTab === 'resumen' && <SubTabSeoResumen />}
                {subTab === 'paginas' && <SubTabSeoPaginas />}
                {subTab === 'blog' && <SubTabSeoBlog />}
                {subTab === 'geo' && <SubTabSeoGeo />}
            </div>
        </div>
    );
};
```

**Nota:** Reutiliza las clases CSS `.contenidoContenedor`, `.contenidoSubTabs`,
`.contenidoSubTab`, `.contenidoSubTab--activo`, `.contenidoPanel` directamente
de `SeccionContenido.css`. No crea clases nuevas para el layout base.

### 2.8 Implementación de SubTabSeoResumen.tsx

4 tarjetas en fila + lista de issues:

```tsx
// Tarjetas: reutiliza el patrón de PagoResumenCard simplificado
<div className="seoResumenTarjetas">
    <div className="seoResumenTarjeta">
        <span className="seoResumenValor">{summary.total_pages}</span>
        <span className="seoResumenLabel">Páginas</span>
    </div>
    <div className="seoResumenTarjeta seoResumenTarjeta--error">
        <span className="seoResumenValor">{summary.errors}</span>
        <span className="seoResumenLabel">Problemas</span>
    </div>
    {/* ... warning y ok ... */}
</div>

// Issues críticos: filas simples con badge
<div className="seoIssuesLista">
    {criticalIssues.map(issue => (
        <div key={issue.path} className="seoIssueFila">
            <span className={`seoIssueBadge seoIssueBadge--${issue.severity}`}>
                {issue.severity}
            </span>
            <span className="seoIssuePath">{issue.path}</span>
            <span className="seoIssueMensaje">{issue.message}</span>
        </div>
    ))}
</div>
```

**CSS:** Las tarjetas usan los mismos tokens que el panel:
- `background: var(--bg-primary)`
- `border: 1px solid var(--border-default)`
- `border-radius: var(--radius-md)`
- `padding: var(--spacing-md)`
- Badges: mismo patrón que `listaServiciosBadge` pero con variantes ok/warning/error

### 2.9 Implementación de SubTabSeoPaginas.tsx

Tabla con el patrón exacto de `SeccionCorreo`:

```tsx
<div className="correosFiltro">
    <Filter size={18} />
    <Select value={filtro} onChange={...}>
        <option value="">Todas las páginas</option>
        <option value="static">Estáticas</option>
        <option value="service">Servicios</option>
        <option value="project">Proyectos</option>
    </Select>
    <span className="correosTotal">{filtered.length} páginas</span>
</div>

<div className="correosTablaWrapper">
    <table className="correosTabla">
        <thead>
            <tr>
                <th>Página</th>
                <th>Title</th>
                <th>Descripción</th>
                <th>OG</th>
                <th>JSON-LD</th>
                <th>Estado</th>
            </tr>
        </thead>
        <tbody>
            {filtered.map(page => (
                <tr key={page.path} className="correosFila"
                    onClick={() => handleEditSeo(page)}>
                    <td>
                        <span className="correosTag">{page.page_type}</span>
                        <span className="seoPaginaRuta">{page.label}</span>
                    </td>
                    <td className="seoPaginaTitle">
                        {page.title || <em>Sin título</em>}
                        <span className="seoPaginaLen">{page.title_len}c</span>
                    </td>
                    <td className="correosAsunto">
                        {page.description || <em>Sin descripción</em>}
                    </td>
                    <td>
                        <span className={`correosBadge ${page.og_image_is_default ? 'badgeNeutral' : 'badgeExito'}`}>
                            {page.og_image_is_default ? 'Default' : 'Sí'}
                        </span>
                    </td>
                    <td>
                        <span className="correosTag">
                            {page.json_ld_type || '—'}
                        </span>
                    </td>
                    <td>
                        <span className={`listaServiciosBadge listaServiciosBadge--${page.status === 'ok' ? 'published' : page.status === 'warning' ? 'draft' : 'archived'}`}>
                            {page.status}
                        </span>
                    </td>
                </tr>
            ))}
        </tbody>
    </table>
</div>
```

**CSS:** Reutiliza `.correosTabla`, `.correosTablaWrapper`, `.correosFila`,
`.correosFiltro`, `.correosBadge`, `.correosTag`, `.correosAsunto`,
`.correosTotal` directamente de `SeccionCorreo.css`. Solo añade:
- `.seoPaginaRuta` — `font-size: var(--text-sm); color: var(--text-primary);`
- `.seoPaginaLen` — `font-size: var(--text-2xs); color: var(--text-tertiary); margin-left: var(--spacing-xs);`
- `.seoPaginaTitle` — `max-width: 250px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;`

### 2.10 Implementación de SubTabSeoBlog.tsx

Mismo patrón que `ListaBlog`:

```tsx
<div className="listaServiciosLista">
    {posts.map(post => (
        <div key={post.id} className="listaServiciosFila"
             onClick={() => navigateToBlogEditor(post.id)}>
            <div className="listaServiciosFilaInfo">
                <span className="listaServiciosNombre">{post.title}</span>
                <span className={`listaBlogBadge listaBlogBadge--${post.status}`}>
                    {post.status}
                </span>
                <span className={`listaServiciosBadge listaServiciosBadge--${post.seo_status === 'ok' ? 'published' : 'draft'}`}>
                    SEO: {post.seo_status}
                </span>
                <span className="listaServiciosPrecio">/blog/{post.slug}</span>
            </div>
            <div className="seoBlogMeta">
                {post.meta_title
                    ? <span className="seoBlogMetaTexto">{post.meta_title}</span>
                    : <em className="seoBlogMetaVacio">Sin meta title</em>}
            </div>
        </div>
    ))}
</div>
```

**CSS:** Reutiliza `.listaServiciosLista`, `.listaServiciosFila`, `.listaServiciosFilaInfo`,
`.listaServiciosNombre`, `.listaServiciosBadge`, `.listaBlogBadge` directamente.
Solo añade:
- `.seoBlogMeta` — `font-size: var(--text-xs); color: var(--text-tertiary);`
- `.seoBlogMetaVacio` — `color: var(--text-tertiary); opacity: 0.7;`

### 2.11 Implementación de SubTabSeoGeo.tsx

Lista de checks simple:

```tsx
<div className="seoGeoLista">
    {checks.map(check => (
        <div key={check.id} className="seoGeoFila"
             onClick={() => check.action_path && navegar(check.action_path)}>
            <span className={`seoGeoIcono ${check.passed ? 'seoGeoIcono--ok' : 'seoGeoIcono--fail'}`}>
                {check.passed ? <Check size={16} /> : <X size={16} />}
            </span>
            <div className="seoGeoInfo">
                <span className="seoGeoLabel">{check.label}</span>
                {check.detail && <span className="seoGeoDetalle">{check.detail}</span>}
            </div>
        </div>
    ))}
</div>
```

**CSS:**
- `.seoGeoLista` — `display: flex; flex-direction: column; gap: var(--spacing-xs);`
- `.seoGeoFila` — `display: flex; align-items: center; gap: var(--spacing-sm); padding: var(--spacing-sm) var(--spacing-md); border: 1px solid var(--border-default); border-radius: var(--radius-md); cursor: pointer;`
- `.seoGeoIcono--ok` — `color: var(--text-tertiary);`
- `.seoGeoIcono--fail` — `color: var(--text-primary);`
- `.seoGeoLabel` — `font-size: var(--text-sm); color: var(--text-primary);`
- `.seoGeoDetalle` — `font-size: var(--text-xs); color: var(--text-tertiary);`

### 2.12 Estimación revisada

| Tarea | Esfuerzo |
|---|---|
| Backend: endpoint `GET /api/admin/seo/audit` | 3-4h |
| Frontend: integración en panel (panel.ts + PanelIsland) | 15min |
| Frontend: `SeccionSeo.tsx` + sub-tabs (patrón existente) | 1h |
| Frontend: `SubTabSeoResumen` | 1h |
| Frontend: `SubTabSeoPaginas` (reutiliza CSS correos) | 1.5h |
| Frontend: `SubTabSeoBlog` (reutiliza CSS lista) | 1h |
| Frontend: `SubTabSeoGeo` | 45min |
| Frontend: `admin-seo.ts` (tipos + API) | 30min |
| CSS nuevo (mínimo — reutiliza patrones) | 30min |
| **Total** | **8-10h** |

---

## 3. Bloque B — Reactivación del Blog + SEO

### 3.1 Estado actual del blog

El blog está **técnicamente completo** pero sin contenido real:

| Componente | Estado | Nota |
|---|---|---|
| Modelo `BlogPost` | ✅ | title, slug, excerpt, content, status, tags, meta_title, meta_description, published_at |
| Repository CRUD | ✅ | create, update, delete, list_published, find_by_slug |
| API endpoints | ✅ | GET /api/blog, GET /api/blog/:slug, CRUD admin |
| Editor admin (`EditorBlog`) | ✅ | Tabs: contenido, media, SEO. Rich text editor (Tiptap) |
| SEO tab en editor | ✅ | meta_title, meta_description, slug, vista previa Google |
| BlogPostSchema JSON-LD | ✅ | BlogPosting con headline, description, author, publisher |
| `SEOHead` en BlogSingleIsland | ✅ | title, description (160 chars del contenido), type=article |
| Sitemap dinámico | ✅ | BlogRepository::public_slugs() incluido |
| Seeder | ✅ | 4 posts de ejemplo |
| Middleware prerender | ❌ | `/blog/:slug` NO resuelve meta tags para crawlers |

### 3.2 Fixes técnicos para reactivar

| # | Tarea | Archivo | Esfuerzo |
|---|---|---|---|
| B1 | Añadir `/blog/:slug` a `resolve_dynamic_meta()` en middleware | `src/middleware/prerender.rs` | 30min |
| B2 | Añadir `/blog` (listado) al middleware con meta fijos | `src/middleware/prerender.rs` | 15min |
| B3 | Añadir `dateModified` a `blogPostSchema` | `frontend/src/components/seo/schemas.ts` | 15min |
| B4 | Verificar que `SeccionBlog` en home muestre posts reales de API | `frontend/src/components/home/SeccionBlog.tsx` | 30min |

### 3.3 Estrategia de contenido GEO

Cada blog post debe seguir la estructura GEO:

```
# Título claro con keyword principal

> TL;DR: Resumen de 40-60 palabras con la respuesta directa.

## La respuesta directa (BLUF)
Primer párrafo: respuesta directa a la pregunta del título.
Datos concretos, sin relleno.

## Desarrollo con datos
- Métricas reales
- Comparativas con datos

## FAQ (3-5 preguntas)
Preguntas reales que los usuarios hacen a las IA.

## Conclusión con CTA
"Si necesitas [servicio], Nakomi Studio puede ayudarte."
```

### 3.4 Calendario editorial sugerido (primer mes)

| # | Título | Keyword GEO target | Tipo |
|---|---|---|---|
| 1 | "Cuánto cuesta crear una web profesional en 2026" | coste web, precio página web | Captura |
| 2 | "WordPress vs desarrollo a medida: cuándo elegir cada uno" | wordpress vs custom | Comparativa |
| 3 | "5 señales de que necesitas cambiar tu hosting" | cambiar hosting, hosting lento | Captura |
| 4 | "Cómo un agente IA puede atender a tus clientes 24/7" | chatbot ia, agente ia | Servicio IA |
| 5 | "Guía para lanzar tu tienda online paso a paso" | crear tienda online | Captura |
| 6 | "Por qué tu negocio necesita una web que cargue en menos de 2 segundos" | velocidad web, lcp | Autoridad |

### 3.5 Estimación

- Fixes técnicos (B1-B4): 1.5h
- Contenido: 2-4h por post (a cargo del usuario)
- **Total técnico: 1.5h**

---

## 4. Bloque C — Estrategia GEO (Generative Engine Optimization)

### 4.1 Acciones técnicas

| # | Tarea | Impacto | Esfuerzo |
|---|---|---|---|
| C1 | Crear `/llms.txt` — resumen Markdown del sitio para IA | Alto | 1h |
| C2 | Añadir FAQ schema a todas las páginas de solución | Alto | 2-3h |
| C3 | Añadir `SoftwareApplication` schema para Kamples | Medio | 30min |
| C4 | Rellenar `sameAs` en organizationSchema con redes sociales | Medio | 15min |
| C5 | Añadir `FAQPage` schema a páginas de hosting y VPS | Alto | 1-2h |
| C6 | Contenido con "fact density" — métricas reales en cada servicio | Alto | Revisión editorial |

### 4.2 llms.txt

Archivo servido en `https://nakomi.studio/llms.txt` con resumen del sitio
optimizado para crawlers de IA (descripción, servicios, contacto, stack, páginas).

### 4.3 FAQ Schema por página

Cada página de solución necesita FAQPage schema con 3-5 preguntas reales.

### 4.4 Estimación

- **Total: 6-8h**

---

## 5. Bloque D — Descuento 50% Primer Pedido

### 5.1 Concepto

50% de descuento automático en el primer servicio contratado. Target: PYMEs
y emprendedores con proyectos pequeños.

### 5.2 Implementación

**Backend — Modificar `OrderService::create_order()`:**
- `user_has_previous_orders(pool, user_id)` — SELECT COUNT
- Si 0 órdenes previas → discount = 50%
- No acumula con descuento por payment_mode (usar el mayor)
- No aplica a hosting/VPS ni a admins

**Frontend:**
- Banner en ModalCompra: "50% OFF en tu primer servicio"
- Badge en cards de planes del sitio público

### 5.3 Reglas de negocio

| Regla | Implementación |
|---|---|
| Solo si no tiene órdenes previas | `SELECT COUNT(*) FROM orders WHERE user_id = $1` |
| No acumula con payment_mode | `discount.max(first_order_discount)` |
| Solo servicios (no hosting/VPS) | Verificar service_slug |
| Registrado en la orden | `discount_percent` en BD |

### 5.4 Integración con GEO

- En llms.txt: "50% de descuento en el primer servicio para nuevos clientes"
- En FAQ schema: "¿Tienen descuento para nuevos clientes?"
- En descripciones de servicios: "Primera consulta con 50% de descuento"

### 5.5 Estimación

- **Total: 5-8h**

---

## 6. Orden de ejecución

```
1. Bloque D — Descuento 50% (impacto inmediato, independiente)
2. Bloque B — Fixes técnicos del blog (1.5h, desbloquea contenido)
3. Bloque A — Dashboard SEO Admin (herramienta para monitorear)
4. Bloque C — GEO (llms.txt + FAQ schema + revisión editorial)
5. Contenido del blog (el usuario escribe, el agente optimiza SEO)
```

---

## 7. Criterio de cierre

### Dashboard SEO Admin
- [ ] Sección `'seo'` visible en panel admin con 4 sub-tabs
- [ ] Sub-tab Resumen con tarjetas + issues críticos
- [ ] Sub-tab Páginas con tabla reutilizando CSS de SeccionCorreo
- [ ] Sub-tab Blog con lista reutilizando CSS de ListaServicios
- [ ] Sub-tab Geo con checklist de tareas
- [ ] Endpoint `GET /api/admin/seo/audit` funcional
- [ ] Persistencia de sub-tab en localStorage
- [ ] Zero CSS custom para layout base (reutiliza contenidoContenido/SubTabs)

### Blog
- [ ] `/blog/:slug` resuelve meta tags en middleware para crawlers
- [ ] `dateModified` en blogPostSchema
- [ ] Al menos 6 posts publicados con estructura GEO

### GEO
- [ ] `/llms.txt` accesible
- [ ] FAQ schema en páginas de hosting, VPS y servicios
- [ ] `sameAs` rellenado en organizationSchema

### Descuento
- [ ] 50% automático para usuarios sin órdenes previas
- [ ] Banner visible en checkout
- [ ] Badge en cards de planes

---

## 8. Riesgos y mitigaciones

| Riesgo | Mitigación |
|---|---|
| Descuento abusivo (cuentas nuevas) | Validar por email + Stripe customer_id |
| Dashboard SEO lento | Cachear resultado 5h, refrescar on-demand |
| Contenido GEO "robotic" | Mantener tono de marca, GEO como guía |
| llms.txt ignorado por Google | No afecta negativamente |

---

## 9. Métricas de éxito

| Métrica | Target (3 meses) | Cómo medir |
|---|---|---|
| Páginas con SEO completo | 100% | Dashboard SEO Admin |
| Blog posts publicados | 6+ | CMS admin |
| Mención en IA | 1+ mención | Buscar en ChatGPT/Perplexity |
| Primeros pedidos con descuento | 3+ | Orders con first_order_discount |
| Páginas con FAQ schema | 10+ | Dashboard SEO Admin |
| Tráfico orgánico | +50% vs actual | Google Search Console |
