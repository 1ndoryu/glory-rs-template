# Plan de Implementación SEO — Nakomi Studio

> **Fecha:** 2026-07-27
> **Estado:** En progreso — bloque 277A completado, pendiente aprobación para continuar
> **Rama:** `glory-rust-nakomi`
> **Auditoría base:** `Agente/documentacion/seo/auditoria-seo-nakomi-2026-07-25.md`
> **Plan original:** `Agente/planes/plan-seo-dashboard-blog-descuento-2026-07-25.md`

---

## 1. Estado actual — Qué ya está hecho ✅

### Fixes SEO (commits 277A-8, 277A-9, 277A-10)

| Fix                                                                 | Archivo                                                              | Commit     |
| ------------------------------------------------------------------- | -------------------------------------------------------------------- | ---------- |
| Ruta 404 audit: `/api/api/admin/seo/audit` → `/admin/seo/audit`     | `src/handlers/admin_seo.rs`                                          | `a837fdcf` |
| Eliminar hreflang incorrecto (misma URL para todos los idiomas)     | `SEOHead.tsx`                                                        | `2ca61834` |
| `dateModified` en blogPostSchema via `updated_at`                   | `useBlogSingle.ts`, `BlogSingleIsland.tsx`                           | `2ca61834` |
| FAQ schema hosting (4 preguntas WP + 4 normal)                      | `SolucionHostingIsland.tsx`                                          | `2ca61834` |
| FAQ schema VPS (4 preguntas)                                        | `SolucionVpsIsland.tsx`                                              | `2ca61834` |
| JSON-LD types correctos en audit endpoint                           | `admin_seo.rs`                                                       | `d90ab450` |
| OG image status correcto en audit (ya no flaggea "default")         | `admin_seo.rs`                                                       | `d90ab450` |
| Título Proyectos: 24→36 chars                                       | `ProyectosIsland.tsx`                                                | `d90ab450` |
| Título Servicios: 8→48 chars                                        | `ServiciosIsland.tsx`                                                | `d90ab450` |
| Descripción Nosotros: 58→115 chars                                  | `NosotrosIsland.tsx`                                                 | `d90ab450` |
| `organizationSchema` en Servicios y Proyectos                       | `ServiciosIsland.tsx`, `ProyectosIsland.tsx`, `CatalogPageShell.tsx` | `d90ab450` |
| `breadcrumbSchema` en ServicioIndividual (@graph con serviceSchema) | `ServicioIndividualIsland.tsx`                                       | `d90ab450` |
| `breadcrumbSchema` en ProyectoIndividual                            | `ProyectoIndividualIsland.tsx`                                       | `d90ab450` |
| Endpoint `GET /api/orders/first-order-discount`                     | `src/handlers/orders.rs`                                             | `d90ab450` |
| Banner 50% OFF en ModalCompra                                       | `ModalCompra.tsx`, `ModalCompra.css`, `orders.ts`                    | `d90ab450` |

### Ya existía antes del 277A

| Componente                         | Estado | Detalle                                                  |
| ---------------------------------- | ------ | -------------------------------------------------------- |
| SEOHead react-helmet               | ✅     | title, description, canonical, OG, twitter card, JSON-LD |
| Middleware prerender para crawlers | ✅     | 16 bots detectados, inyecta meta tags en HTML            |
| Sitemap dinámico                   | ✅     | Servicios, proyectos, blog, páginas estáticas            |
| robots.txt                         | ✅     | Correcto                                                 |
| blogPostSchema                     | ✅     | Con dateModified (añadido 277A)                          |
| organizationSchema + websiteSchema | ✅     | En Home via BienvenidaIsland                             |
| serviceSchema                      | ✅     | En ServicioIndividualIsland                              |
| personSchema + breadcrumbSchema    | ✅     | En NosotrosIsland                                        |
| faqSchema                          | ✅     | En hosting, VPS, y definido en schemas.ts                |
| Image proxy + OptimizedImage       | ✅     | WebP, srcset, lazy loading                               |
| SEO tab en editor blog             | ✅     | meta_title, meta_description, slug, preview Google       |
| SEO audit endpoint admin           | ✅     | `GET /api/admin/seo/audit` funcional                     |

---

## 2. Pendiente — Qué falta por hacer 🔴

### Prioridad ALTA (impacto directo en indexación y rich snippets)

#### 2.1 JSON-LD en middleware prerender (servicios y proyectos)

**Problema:** El middleware inyecta meta tags (title, desc, OG) pero NO structured data JSON-LD para las rutas de servicios y proyectos. Los crawlers que no ejecutan JS (Bing, Slack, Discord, previsualizadores) no ven el schema.

**Qué hacer:**

- En `prerender.rs`, para `/servicios/:slug`: inyectar `<script type="application/ld+json">` con `serviceSchema()` generado server-side
- Para `/proyectos/:slug`: inyectar `projectSchema()` (ver 2.2)
- Para `/`: inyectar `organizationSchema` + `websiteSchema`
- Para `/nosotros`: inyectar `personSchema` + `breadcrumbSchema`

**Esfuerzo:** 2-3h  
**Impacto:** Alto — rich snippets en Google, Bing, y previews de redes sociales  
**Archivos:** `src/middleware/prerender.rs`

---

#### 2.2 Rutas faltantes en middleware prerender

**Problema:** Estas rutas NO tienen meta tags en el middleware (crawlers sin JS ven HTML vacío):

| Ruta                            | Página              | Estado actual     |
| ------------------------------- | ------------------- | ----------------- |
| `/soluciones/hosting-wordpress` | Hosting WordPress   | ❌ Sin middleware |
| `/soluciones/vps`               | Servidores VPS      | ❌ Sin middleware |
| `/blog/:slug`                   | Posts individuales  | ❌ Sin middleware |
| `/contacto`                     | Contacto            | ❌ Sin middleware |
| `/privacidad`                   | Política privacidad | ❌ Sin middleware |
| `/vps` (alias)                  | Redirect a VPS      | ❌ Sin middleware |

**Qué hacer:**

- Añadir `/soluciones/hosting-wordpress` y `/soluciones/vps` como rutas estáticas con meta fijos
- Añadir `/blog/:slug` a `resolve_dynamic_meta()` consultando `blog_posts` (title, excerpt, published_at, og_image)
- Añadir `/contacto` y `/privacidad` como rutas estáticas

**Esfuerzo:** 3-4h  
**Impacto:** Alto — páginas actualmente invisibles para crawlers importantes  
**Archivos:** `src/middleware/prerender.rs`

---

#### 2.3 Hosting WP y VPS en sitemap

**Problema:** El sitemap no incluye `/soluciones/hosting-wordpress` ni `/soluciones/vps`.

**Qué hacer:** Añadir ambas rutas al sitemap estático en `seo.rs`:

- `/soluciones/hosting-wordpress` — priority 0.8, monthly
- `/soluciones/vps` — priority 0.8, monthly

**Esfuerzo:** 15min  
**Impacto:** Medio  
**Archivos:** `src/handlers/seo.rs`

---

#### 2.4 OG image dedicado para Nakomi

**Problema:** La imagen OG por defecto es la portada del proyecto Kamples (`/assets/Proyectos%20portadas/Kamples%20portada.jpg`). Cuando se comparten páginas sin imagen específica, aparece Kamples.

**Qué hacer:**

- Crear `og-image-nakomi.jpg` (1200×630) con logo + tagline de Nakomi
- Subir a `/assets/og-image-nakomi.jpg`
- Actualizar `DEFAULT_IMAGE` en `SEOHead.tsx`
- Idealmente también servirlo desde el middleware prerender

**Esfuerzo:** 1h (diseño) + 15min (código)  
**Impacto:** Medio — imagen de marca correcta al compartir enlaces  
**Archivos:** `SEOHead.tsx`, nuevo asset

---

### Prioridad MEDIA (mejora estructural)

#### 2.5 projectSchema en schemas.ts

**Problema:** No existe un schema `CreativeWork` o `WebPage` para proyectos del portfolio. `ProyectoIndividualIsland` solo tiene `breadcrumbSchema`.

**Qué hacer:**

- Crear función `projectSchema(titulo, descripcion, imagen, url)` en `schemas.ts`
- Integrar en `ProyectoIndividualIsland.tsx` con `@graph` (breadcrumb + project)
- Añadir al middleware prerender para `/proyectos/:slug`

**Esfuerzo:** 1-2h  
**Impacto:** Medio  
**Archivos:** `schemas.ts`, `ProyectoIndividualIsland.tsx`, `prerender.rs`

---

#### 2.6 `sameAs` en organizationSchema

**Problema:** `sameAs: []` vacío. Debe contener enlaces a perfiles sociales para reforzar la entidad.

**Qué hacer:** Añadir URLs reales de Nakomi:

- LinkedIn (si existe)
- GitHub (si existe)
- Behance / Dribbble (si existen)
- Cualquier otra presencia social

**Esfuerzo:** 15min  
**Impacto:** Bajo-Medio  
**Archivos:** `schemas.ts`

---

#### 2.7 `SoftwareApplication` schema para Kamples

**Problema:** Kamples es open source pero no tiene schema dedicado.

**Qué hacer:**

- Crear `kamplesSchema()` en `schemas.ts` con `@type: SoftwareApplication`
- Integrar en la página/section de Kamples (si existe una dedicada)

**Esfuerzo:** 30min  
**Impacto:** Bajo-Medio — rich snippets de software en Google  
**Archivos:** `schemas.ts`, componente Kamples

---

#### 2.8 Imágenes sin `alt` descriptivo

**Problema:** Browser-use detectó 2 imágenes sin atributo `alt` en la auditoría.

**Qué hacer:** Auditar todas las imágenes del sitio:

- Las decorativas deben usar `alt=""`
- Las de contenido deben tener alt descriptivo en español

**Esfuerzo:** 1-2h  
**Impacto:** Medio — accesibilidad + SEO de imágenes  
**Archivos:** Varios componentes TSX

---

### Prioridad BAJA (monitoreo y optimización continua)

#### 2.9 Google Search Console

**Problema:** No hay evidencia de Search Console configurado.

**Qué hacer:**

1. Crear cuenta en search.google.com/search-console
2. Verificar propiedad (meta tag o DNS)
3. Enviar sitemap: `https://nakomi.studio/sitemap.xml`
4. Monitorear indexación, errores de cobertura, Core Web Vitals

**Esfuerzo:** 30min  
**Impacto:** Alto (para diagnóstico, no para ranking directo)  
**Externo:** Requiere acceso del usuario

---

#### 2.10 PageSpeed / Core Web Vitals

**Qué hacer:**

1. Ejecutar PageSpeed Insights para desktop y mobile
2. Identificar bottlenecks (bundle size, LCP, CLS)
3. Optimizar según resultados

**Esfuerzo:** 2-4h (dependiendo de resultados)  
**Impacto:** Alto  
**Herramienta:** web.dev/measure

---

#### 2.11 Enlaces internos estratégicos

**Problema:** Blog no enlaza a servicios/proyectos y viceversa.

**Qué hacer:**

- En posts del blog: enlaces contextuales a servicios relevantes
- En páginas de servicio: "Artículos relacionados" del blog
- En proyectos: enlaces al servicio que utilizan

**Esfuerzo:** 2-3h  
**Impacto:** Medio  
**Archivos:** Contenido del blog + componentes de detalle

---

#### 2.12 RSS/Atom feed para el blog

**Problema:** No existe feed RSS.

**Qué hacer:**

- Crear endpoint `GET /blog/feed.xml` en `seo.rs`
- Generar XML con posts publicados (title, link, description, pubDate)
- Añadir `<link rel="alternate" type="application/rss+xml">` en SEOHead para rutas de blog

**Esfuerzo:** 1-2h  
**Impacto:** Bajo — distribución automática a agregadores  
**Archivos:** `src/handlers/seo.rs`, `SEOHead.tsx`

---

## 3. Bloque SEO Dashboard Admin (plan completo)

> Detalle completo en `plan-seo-dashboard-blog-descuento-2026-07-25.md` sección 2.

**Estado:** Planificado, pendiente aprobación.

### Resumen

Sección nueva en el panel admin con 4 sub-tabs:

| Sub-tab     | Contenido                                                      | Patrón reutilizado               |
| ----------- | -------------------------------------------------------------- | -------------------------------- |
| **Resumen** | 4 tarjetas (páginas, problemas, mejorables, OK) + top 5 issues | `PagoResumenCard`                |
| **Páginas** | Tabla: Página, Title, Desc, OG, JSON-LD, Status                | `SeccionCorreo` (tabla + filtro) |
| **Blog**    | Lista: Título, slug, meta_title, meta_desc, SEO status         | `ListaBlog`                      |
| **GEO**     | Checklist: llms.txt, FAQ schema, sameAs, sitemap               | Custom (simple)                  |

### Backend necesario

- Endpoint `GET /api/admin/seo/audit` — ✅ **ya implementado**
- Retorna: summary, pages[], blog_posts[], geo_checks[]

### Frontend necesario

| Archivo                                              | Acción    | Detalle                                 |
| ---------------------------------------------------- | --------- | --------------------------------------- |
| `frontend/src/data/panel.ts`                         | Modificar | Añadir `'seo'` a `SeccionPanel`         |
| `frontend/src/islands/PanelIsland.tsx`               | Modificar | Importar + renderizar `SeccionSeo`      |
| `frontend/src/api/admin-seo.ts`                      | **Nuevo** | Tipos + `apiGetSeoAudit()`              |
| `frontend/src/components/panel/SeccionSeo.tsx`       | **Nuevo** | Componente principal con sub-tabs       |
| `frontend/src/components/panel/SeccionSeo.css`       | **Nuevo** | Estilos (reutiliza patrones existentes) |
| `frontend/src/components/panel/SubTabSeoResumen.tsx` | **Nuevo** | Tarjetas + issues                       |
| `frontend/src/components/panel/SubTabSeoPaginas.tsx` | **Nuevo** | Tabla de páginas                        |
| `frontend/src/components/panel/SubTabSeoBlog.tsx`    | **Nuevo** | Lista de posts                          |
| `frontend/src/components/panel/SubTabSeoGeo.tsx`     | **Nuevo** | Checklist GEO                           |

**Esfuerzo total:** 8-10h  
**Impacto:** Medio (herramienta de monitoreo, no afecta ranking directamente)

---

## 4. Bloque Blog + Contenido GEO

> Detalle completo en `plan-seo-dashboard-blog-descuento-2026-07-25.md` sección 3.

### Fixes técnicos (desbloquean contenido)

| #   | Tarea                                      | Estado           | Archivo                          |
| --- | ------------------------------------------ | ---------------- | -------------------------------- |
| B1  | `/blog/:slug` en middleware prerender      | ❌ Pendiente     | `prerender.rs`                   |
| B2  | `/blog` (listado) en middleware            | ❌ Pendiente     | `prerender.rs`                   |
| B3  | `dateModified` en blogPostSchema           | ✅ Hecho (277A)  | `schemas.ts`, `useBlogSingle.ts` |
| B4  | Verificar SeccionBlog muestra posts reales | ⚠️ Sin verificar | `SeccionBlog.tsx`                |

### Estrategia de contenido GEO

Cada blog post debe seguir:

1. **Título claro** con keyword principal
2. **TL;DR** de 40-60 palabras al inicio
3. **BLUF** — respuesta directa en el primer párrafo
4. **Datos concretos** — métricas reales, no opiniones
5. **FAQ** de 3-5 preguntas reales
6. **CTA** con enlace a servicio relevante

### Calendario editorial sugerido (primer mes)

| #   | Título                                                                  | Keyword GEO target             | Tipo        |
| --- | ----------------------------------------------------------------------- | ------------------------------ | ----------- |
| 1   | "Cuánto cuesta crear una web profesional en 2026"                       | coste web, precio página web   | Captura     |
| 2   | "WordPress vs desarrollo a medida: cuándo elegir cada uno"              | wordpress vs custom            | Comparativa |
| 3   | "5 señales de que necesitas cambiar tu hosting"                         | cambiar hosting, hosting lento | Captura     |
| 4   | "Cómo un agente IA puede atender a tus clientes 24/7"                   | chatbot ia, agente ia          | Servicio IA |
| 5   | "Guía para lanzar tu tienda online paso a paso"                         | crear tienda online            | Captura     |
| 6   | "Por qué tu negocio necesita una web que cargue en menos de 2 segundos" | velocidad web, lcp             | Autoridad   |

**Esfuerzo técnico:** 1.5h (fixes B1-B2)  
**Esfuerzo contenido:** 2-4h por post (a cargo del usuario)

---

## 5. Orden de ejecución sugerido

```
FASE 1 — Fixes técnicos críticos (hacer primero)
├── 2.1  JSON-LD en middleware prerender        [2-3h]  🔴 Alto impacto
├── 2.2  Rutas faltantes en middleware           [3-4h]  🔴 Alto impacto
├── 2.3  Hosting WP + VPS en sitemap            [15min] 🟡 Medio impacto
└── 2.4  OG image dedicado                      [1.25h] 🟡 Medio impacto

FASE 2 — Mejoras estructurales (después)
├── 2.5  projectSchema                           [1-2h]  🟡 Medio impacto
├── 2.6  sameAs en organizationSchema            [15min] 🟡 Bajo-Medio
├── 2.7  SoftwareApplication para Kamples        [30min] 🟡 Bajo-Medio
├── 2.8  Imágenes sin alt                        [1-2h]  🟡 Medio impacto
└── 2.12 RSS feed                                [1-2h]  🟢 Bajo impacto

FASE 3 — Herramientas y monitoreo (paralelo)
├── 2.9  Google Search Console                   [30min] 🔴 Alto (diagnóstico)
├── 2.10 PageSpeed / Core Web Vitals             [2-4h]  🔴 Alto impacto
└── 2.11 Enlaces internos                        [2-3h]  🟡 Medio impacto

FASE 4 — Contenido (requiere aprobación usuario)
├── B1-B2 Fixes técnicos blog middleware         [1.5h]  🔴 Desbloqueante
├── Blog posts GEO (6 posts primer mes)          [12-24h] 🔴 Alto impacto
└── SEO Dashboard Admin (panel completo)         [8-10h]  🟡 Medio impacto
```

---

## 6. Estimación total

| Fase                                | Esfuerzo   | Dependencias                                       |
| ----------------------------------- | ---------- | -------------------------------------------------- |
| Fase 1 — Fixes críticos             | 7-9h       | Ninguna                                            |
| Fase 2 — Mejoras estructurales      | 4-6h       | Ninguna                                            |
| Fase 3 — Herramientas               | 5-8h       | Search Console requiere acceso usuario             |
| Fase 4 — Contenido + Dashboard      | 22-36h     | Blog: contenido del usuario; Dashboard: aprobación |
| **Total técnico (sin contenido)**   | **16-23h** |                                                    |
| **Total con contenido + dashboard** | **38-59h** |                                                    |

---

## 7. Criterio de cierre por fase

### Fase 1 ✅

- [ ] JSON-LD inyectado en middleware para todas las rutas estáticas
- [ ] JSON-LD dinámico en middleware para servicios y proyectos
- [ ] `/blog/:slug` con meta tags en middleware
- [ ] `/soluciones/hosting-wordpress` y `/vps` en middleware + sitemap
- [ ] OG image de marca (no Kamples)
- [ ] Verificar con Google Rich Results Test

### Fase 2 ✅

- [ ] projectSchema en ProyectoIndividual
- [ ] sameAs rellenado con redes sociales reales
- [ ] Todas las imágenes con alt correcto
- [ ] RSS feed accesible en `/blog/feed.xml`

### Fase 3 ✅

- [ ] Google Search Console configurado y sitemap enviado
- [ ] PageSpeed > 90 desktop, > 70 mobile
- [ ] Enlaces internos entre blog ↔ servicios ↔ proyectos

### Fase 4 ✅

- [ ] 6+ posts publicados con estructura GEO
- [ ] SEO Dashboard visible en panel admin
- [ ] Monitoreo activo via dashboard + Search Console

---

## 8. Archivos relevantes

| Archivo                                               | Responsabilidad                                 |
| ----------------------------------------------------- | ----------------------------------------------- |
| `src/middleware/prerender.rs`                         | Middleware SEO para crawlers (Fase 1 principal) |
| `src/handlers/seo.rs`                                 | robots.txt, sitemap.xml                         |
| `src/handlers/admin_seo.rs`                           | Endpoint audit admin                            |
| `frontend/src/components/seo/SEOHead.tsx`             | Meta tags React                                 |
| `frontend/src/components/seo/schemas.ts`              | Schemas JSON-LD centralizados                   |
| `frontend/src/islands/*.tsx`                          | Islands con SEOHead + jsonLd                    |
| `frontend/src/components/layout/CatalogPageShell.tsx` | Shell compartida catálogos                      |
| `frontend/src/api/admin-seo.ts`                       | API admin SEO                                   |
| `frontend/src/components/panel/SeccionSeo.tsx`        | Dashboard SEO (pendiente crear)                 |
