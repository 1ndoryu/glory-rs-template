# Auditoría SEO — Nakomi Studio

> **Fecha:** 2026-07-25
> **Dominio:** https://nakomi.studio
> **Stack:** Rust (Axum) backend + React 18 SPA (Vite) frontend
> **Estado general:** 🟡 Base técnica sólida, con defectos de implementación y oportunidades significativas sin explotar

---

## 1. Resumen ejecutivo

Nakomi Studio tiene una infraestructura SEO inusualmente bien diseñada para una SPA
React: middleware server-side que inyecta meta tags dinámicos para crawlers, sitemap
generado desde la base de datos, structured data JSON-LD, y un componente `SEOHead`
reutilizable. Sin embargo, hay defectos reales de implementación, contenido dinámico
que no llegua a los crawlers, y oportunidades de alto impacto completamente desaprovechadas.

**Puntuación estimada: 6.5/10** — La base está, pero la ejecución tiene gaps importantes.

---

## 2. Infraestructura SEO implementada

### 2.1 Componente SEOHead (frontend)

**Archivo:** `frontend/src/components/seo/SEOHead.tsx`
**Estado:** ✅ Implementado correctamente

Cada página/island renderiza `<SEOHead>` que gestiona vía `react-helmet-async`:

| Elemento                          | Implementado | Notas                                 |
| --------------------------------- | ------------ | ------------------------------------- |
| `<title>`                         | ✅           | Formato: `{titulo} \| Nakomi Studio`  |
| `<meta description>`              | ✅           | Condicional — solo si se pasa prop    |
| `<link canonical>`                | ✅           | `https://nakomi.studio{path}`         |
| `og:title`                        | ✅           |                                       |
| `og:description`                  | ✅           |                                       |
| `og:url`                          | ✅           |                                       |
| `og:type`                         | ✅           | `website` o `article`                 |
| `og:image`                        | ✅           | Default: imagen de Kamples (temporal) |
| `og:image:width/height`           | ✅           | 1200×630                              |
| `og:site_name`                    | ✅           |                                       |
| `og:locale`                       | ✅           | Desde i18n.language                   |
| `twitter:card`                    | ✅           | `summary_large_image`                 |
| `twitter:title/description/image` | ✅           |                                       |
| `hreflang`                        | ✅           | es, en, ja, x-default                 |
| JSON-LD structured data           | ✅           | Via prop `jsonLd`                     |
| `noindex`                         | ✅           | Via prop `noindex`                    |
| `<html lang>`                     | ✅           | Desde i18n.language                   |

**Defecto:** Las hreflang apuntan todas a la misma URL (no hay contenido multilenguaje real,
solo traducción de UI). Esto puede confundir a Google — ver sección 3.2.

### 2.2 Middleware prerender para crawlers

**Archivo:** `src/middleware/prerender.rs`
**Estado:** ✅ Implementado, funcional

El middleware detecta crawlers por User-Agent y les sirve `index.html` con meta tags
inyectados directamente en el HTML (antes de que React monte). Esto resuelve el
problema fundamental de las SPAs: los crawlers no ejecutan JavaScript.

**Crawlers detectados (16):**
googlebot, bingbot, yandexbot, baiduspider, duckduckbot, slurp, facebot,
facebookexternalhit, twitterbot, linkedinbot, applebot, semrushbot, ahrefsbot,
quora link preview, outbrain, pinterestbot

**Rutas estáticas con meta fijos:**

- `/` — "Nakomi Studio — Agencia Creativa Digital"
- `/servicios` — "Nuestros Servicios — Nakomi Studio"
- `/proyectos` — "Portfolio — Nakomi Studio"
- `/nosotros` — "Sobre Nosotros — Nakomi Studio"
- `/soluciones/hosting` — "Hosting Administrado — Nakomi Studio"

**Rutas dinámicas (consulta BD):**

- `/servicios/:slug` — title + description desde tabla `services`
- `/proyectos/:slug` — COALESCE(meta_title, title) + COALESCE(meta_description, description) + featured_image

**Optimización de rendimiento (home):**

- `<link rel="preload">` para imagen hero con srcset responsivo
- `window.__INITIAL_DATA__` con proyectos publicados para pre-poblar React Query
- Elimina ~300-500ms del LCP en conexiones lentas

### 2.3 Sitemap dinámico

**Archivo:** `src/handlers/seo.rs`
**Estado:** ✅ Funcional, accesible en producción

**Rutas estáticas (7):**

- `/` (priority 1.0, weekly)
- `/servicios` (0.9, weekly)
- `/proyectos` (0.9, weekly)
- `/nosotros` (0.7, monthly)
- `/blog` (0.8, daily)
- `/soluciones` (0.8, monthly)
- `/soluciones/hosting` (0.8, monthly)

**Rutas dinámicas:**

- Servicios públicos desde `ServiceRepository::public_slugs` (0.8, monthly)
- Proyectos publicados desde `ProjectRepository::public_slugs` (0.7, monthly)
- Blog posts publicados desde `BlogRepository::public_slugs` (0.9, weekly)

### 2.4 robots.txt

**Archivo:** `src/handlers/seo.rs`
**Estado:** ✅ Correcto

```
User-agent: *
Allow: /
Disallow: /panel
Disallow: /swagger-ui
Disallow: /api-docs

Sitemap: https://nakomi.studio/sitemap.xml
```

### 2.5 Structured Data (JSON-LD)

**Archivo:** `frontend/src/components/seo/schemas.ts`
**Estado:** ✅ Schemas definidos, parcialmente utilizados

| Schema               | Tipo                                | Usado en            | Estado                                                       |
| -------------------- | ----------------------------------- | ------------------- | ------------------------------------------------------------ |
| `organizationSchema` | ProfessionalService + LocalBusiness | Home (implícito)    | ⚠️ Verificar si se renderiza                                 |
| `websiteSchema`      | WebSite + SearchAction              | Home (implícito)    | ⚠️ Verificar si se renderiza                                 |
| `serviceSchema`      | Service                             | Detalle de servicio | ⚠️ No se pasa `jsonLd` a SEOHead en ServicioIndividualIsland |
| `blogPostSchema`     | BlogPosting                         | BlogSingleIsland    | ✅ Correctamente integrado                                   |
| `breadcrumbSchema`   | BreadcrumbList                      | —                   | ❌ Definido pero NO usado en ninguna página                  |
| `personSchema`       | Person                              | NosotrosIsland      | ⚠️ Definido pero verificar uso real                          |

### 2.6 Imágenes optimizadas

**Componente:** `frontend/src/components/ui/OptimizedImage.tsx`
**Proxy:** `src/handlers/image_proxy.rs`

| Feature                     | Estado | Notas                                                              |
| --------------------------- | ------ | ------------------------------------------------------------------ |
| WebP con fallback           | ✅     | `<picture>` con `<source type="image/webp">`                       |
| srcset responsivo           | ✅     | Buckets: 150, 300, 480, 640, 800, 1024, 1200, 1600, 2400           |
| `sizes` automático          | ✅     | ResizeObserver mide ancho real si no se pasa                       |
| `loading="lazy"`            | ✅     | Por defecto, `eager` para hero/above-fold                          |
| `alt` attributes            | ⚠️     | La mayoría tienen alt, pero browser-use detectó 2 imágenes sin alt |
| `width`/`height` explícitos | ✅     | En la mayoría de componentes                                       |

---

## 3. Defectos encontrados

### 3.1 🔴 CRÍTICO — Servicios individuales no inyectan JSON-LD en crawlers

**Problema:** El middleware `prerender.rs` inyecta meta tags básicos (title, description, OG)
para `/servicios/:slug`, pero NO structured data JSON-LD. El frontend sí renderiza
`serviceSchema()` vía `SEOHead`, pero los crawlers reciben el HTML del middleware
sin el script `application/ld+json`.

**Impacto:** Los servicios no aparecen como rich snippets en Google (Service schema
con provider, areaServed, etc.).

**Fix:** Añadir JSON-LD al HTML inyectado por el middleware para rutas de servicios.

### 3.2 🔴 CRÍTICO — Proyectos individuales no inyectan JSON-LD

**Problema:** Igual que servicios. El middleware inyecta meta tags para `/proyectos/:slug`
pero no structured data. No existe un `projectSchema` en `schemas.ts`.

**Impacto:** Los proyectos del portfolio no tienen rich snippets.

**Fix:** Crear `projectSchema` en `schemas.ts`, integrarlo en `ProyectoIndividualIsland`,
y añadirlo al middleware.

### 3.3 🟠 ALTO — Breadcrumbs no implementados

**Problema:** `breadcrumbSchema` existe en `schemas.ts` pero NO se usa en ninguna página.
Los breadcrumbs son un factor de ranking confirmado por Google y mejoran el CTR
en SERPs mostrando la jerarquía del sitio.

**Páginas afectadas:**

- `/servicios/:slug` → Inicio > Servicios > {nombre}
- `/proyectos/:slug` → Inicio > Proyectos > {nombre}
- `/blog/:slug` → Inicio > Blog > {titulo}
- `/soluciones/hosting` → Inicio > Soluciones > Hosting
- `/soluciones/vps` → Inicio > Soluciones > VPS

**Fix:** Integrar `breadcrumbSchema` + componente visual de breadcrumbs en las páginas
de detalle.

### 3.4 🟠 ALTO — hreflang apunta a la misma URL

**Problema:** `SEOHead` genera hreflang para `es`, `en`, `ja` y `x-default`, pero TODOS
apuntan a `https://nakomi.studio{path}` (la misma URL). No hay contenido multilenguaje
real — solo la UI se traduce con i18next.

**Impacto:** Google puede interpretar esto como señal contradictoria o ignorar los
hreflang. Si no hay URLs separadas por idioma (`/en/servicios`, `/ja/servicios`),
los hreflang no deben existir.

**Fix:** Eliminar hreflang de `SEOHead` hasta que existan URLs separadas por idioma,
o mantener solo `x-default` apuntando a la versión en español.

### 3.5 🟠 ALTO — og:image default es una imagen de proyecto (Kamples)

**Problema:** La imagen OG por defecto en `SEOHead` es:

```
/assets/Proyectos%20portadas/Kamples%20portada.jpg
```

Esto es un placeholder temporal del proyecto Kamples, no una imagen de marca de Nakomi.

**Impacto:** Cuando se comparte cualquier página sin og:image específica (home, servicios,
nosotros, etc.), aparece la portada de Kamples en vez de un branded image.

**Fix:** Crear un `og-image.jpg` dedicado (1200×630) con el logo/tagline de Nakomi y
actualizar `DEFAULT_IMAGE` en `SEOHead.tsx`.

### 3.6 🟠 ALTO — Middleware no maneja rutas de hosting/VPS

**Problema:** `resolve_seo_meta()` en `prerender.rs` solo cubre:

- `/` (home)
- `/servicios` (listado)
- `/proyectos` (listado)
- `/nosotros`
- `/soluciones/hosting`
- `/servicios/:slug` (dinámico)
- `/proyectos/:slug` (dinámico)

**Rutas SIN meta tags para crawlers:**

- `/soluciones/hosting-wordpress` — Hosting WordPress
- `/soluciones/vps` — Servidores VPS
- `/soluciones/vps/configurar/:tier` — Configurador VPS
- `/soluciones/hosting/configurar/:plan` — Configurador hosting
- `/blog/:slug` — Posts individuales del blog
- `/contacto` — Página de contacto
- `/privacidad` — Política de privacidad
- `/equipo/:slug` — Perfiles públicos del equipo

**Impacto:** Estas páginas reciben el SPA vacío de los crawlers (sin meta tags),
aunque el frontend sí renderiza `SEOHead`. Googlebot ejecuta JS eventualmente,
pero otros crawlers (Bing, redes sociales, previsualizadores de enlaces) no.

**Fix:** Añadir las rutas faltantes a `resolve_seo_meta()`.

### 3.7 🟡 MEDIO — Blog posts no tienen meta tags en middleware

**Problema:** Aunque el sitemap incluye `/blog/:slug` dinámicamente, el middleware
`prerender.rs` NO resuelve meta tags para rutas de blog. Los posts tienen
`SEOHead` en el frontend con `blogPostSchema`, pero los crawlers que no ejecutan
JS reciben el HTML genérico del SPA.

**Impacto:** Los posts del blog no tienen title, description ni structured data
cuando los crawlean bots que no ejecutan JavaScript (Bing, Slack, Discord, etc.).

**Fix:** Añadir `/blog/:slug` a `resolve_dynamic_meta()` consultando la tabla `blog_posts`.

### 3.8 🟡 MEDIO — Falta `FAQPage` schema

**Problema:** No existe schema `FAQPage` para las secciones de preguntas frecuentes
(si las hay) ni para las páginas de soluciones que podrían beneficiarse de
rich snippets de FAQ.

**Impacto:** Se pierde la oportunidad de aparecer con desplegables de FAQ en SERPs,
que aumentan el CTR significativamente.

### 3.9 🟡 MEDIO — Sin `dateModified` en blog posts

**Problema:** `blogPostSchema` incluye `datePublished` pero no `dateModified`.
Google usa `dateModified` para determinar frescura del contenido.

**Fix:** Añadir `dateModified` al schema usando el campo `updated_at` de la BD.

### 3.10 🟡 MEDIO — Imágenes sin alt en algunos componentes

**Problema:** Browser-use detectó 2 imágenes sin atributo `alt`. Los avatares del
chat widget usan `alt=""` (decorativos, aceptable), pero algunos componentes
pueden tener imágenes de contenido sin alt descriptivo.

**Fix:** Auditar y corregir imágenes sin alt. Las decorativas deben usar `alt=""`,
las de contenido deben tener alt descriptivo.

### 3.11 🟢 BAJO — `sameAs` vacío en organizationSchema

**Problema:** El schema de organización tiene `sameAs: []`. Este campo debe contener
enlaces a perfiles sociales (LinkedIn, GitHub, Twitter, etc.) para reforzar
la identidad de la organización ante Google.

---

## 4. Oportunidades sin explotar

### 4.1 🔵 ALTO IMPACTO — Contenido del blog

El blog está técnicamente listo (API, sitemap, schema, SEOHead) pero necesita
contenido real y consistente. Cada post genera una URL indexable con structured data.

**Oportunidades de contenido:**

- Estudios de caso de proyectos realizados
- Guías técnicas (hosting, WordPress, desarrollo web)
- Comparativas de tecnología
- Tutoriales de herramientas que Nakomi ofrece

### 4.2 🔵 ALTO IMPACTO — Google Search Console

No hay evidencia de que Search Console esté configurado. Es **gratuito** y proporciona:

- Datos reales de indexación y crawling
- Errores de cobertura (páginas no indexadas)
- Consultas de búsqueda que traen tráfico
- Core Web Vitals reales
- Sitemap submission directo
- Alertas de problemas de seguridad/usabilidad

### 4.3 🔵 ALTO IMPACTO — PageSpeed / Core Web Vitals

El sitio tiene optimizaciones de rendimiento (preload hero, initial data script,
WebP, lazy loading) pero no hay datos de Core Web Vitals reales. El SPA nature
del sitio puede penalizar LCP si el bundle JS es grande.

**Acción:** Ejecutar PageSpeed Insights y optimizar según resultados.

### 4.4 🟢 MEDIO IMPACTO — Páginas de solución sin contenido editorial

Las páginas `/soluciones/hosting`, `/soluciones/vps` tienen contenido comercial
pero no editorial. Google valora las páginas que combinan comercial + informativo.

**Oportunidad:** Añadir secciones de FAQ, comparativas, guías de inicio rápido
a las páginas de solución.

### 4.5 🟢 MEDIO IMPACTO — Schema `SoftwareApplication` para Kamples

Kamples es una aplicación open source. Un schema `SoftwareApplication` con
`applicationCategory`, `operatingSystem`, `offers` (gratis) puede generar
rich snippets específicos de software.

### 4.6 🟢 MEDIO IMPACTO — Enlaces internos estratégicos

Las páginas de detalle de proyecto y servicio no enlazan cruzadamente de forma
consistente. Los enlaces internos son un factor de ranking importante.

**Estado actual:**

- `SeccionProyectosRelacionados` ✅ (en proyectos)
- `SeccionServiciosRelacionados` ✅ (en servicios)
- Blog → Servicios/Proyectos ❌ (no hay enlaces contextuales)
- Servicios → Blog ❌

### 4.7 🟡 BAJO IMPACTO — RSS/Atom feed para el blog

No existe feed RSS. Los feeds permiten:

- Suscripción de lectores
- Distribución automática a agregadores
- Señal de frescura para crawlers

---

## 5. Estado por página pública

| Página              | Ruta                            | SEOHead                      | Middleware        | JSON-LD                                | Sitemap | Estado |
| ------------------- | ------------------------------- | ---------------------------- | ----------------- | -------------------------------------- | ------- | ------ |
| Home                | `/`                             | ⚠️ Sin description explícita | ✅                | ⚠️ organization/website no verificados | ✅      | 🟡     |
| Servicios (listado) | `/servicios`                    | ✅ Via CatalogPageShell      | ✅                | ❌                                     | ✅      | 🟡     |
| Servicio (detalle)  | `/servicios/:slug`              | ⚠️ Verificar                 | ✅ (BD)           | ❌ No en middleware                    | ✅      | 🟠     |
| Proyectos (listado) | `/proyectos`                    | ✅ Via CatalogPageShell      | ✅                | ❌                                     | ✅      | 🟡     |
| Proyecto (detalle)  | `/proyectos/:slug`              | ⚠️ Verificar                 | ✅ (BD)           | ❌ No en middleware                    | ✅      | 🟠     |
| Blog (listado)      | `/blog`                         | ✅                           | ❌ Sin middleware | ❌                                     | ✅      | 🟠     |
| Blog (detalle)      | `/blog/:slug`                   | ✅ Con blogPostSchema        | ❌ Sin middleware | ❌ Solo frontend                       | ✅      | 🟠     |
| Nosotros            | `/nosotros`                     | ✅                           | ✅                | ⚠️ personSchema verificar              | ✅      | 🟡     |
| Hosting             | `/soluciones/hosting`           | ✅                           | ✅                | ❌                                     | ✅      | 🟡     |
| Hosting WP          | `/soluciones/hosting-wordpress` | ✅                           | ❌                | ❌                                     | ❌      | 🔴     |
| VPS                 | `/soluciones/vps`               | ✅                           | ❌                | ❌                                     | ❌      | 🔴     |
| Contacto            | `/contacto`                     | ⚠️ Verificar                 | ❌                | ❌                                     | ❌      | 🔴     |
| Privacidad          | `/privacidad`                   | ⚠️ Verificar                 | ❌                | ❌                                     | ❌      | 🟠     |
| Perfil público      | `/equipo/:slug`                 | ⚠️ Verificar                 | ❌                | ⚠️ personSchema                        | ❌      | 🟠     |

---

## 6. Estado de indexación (producción)

Según la verificación en vivo (2026-07-25):

| Item                  | Estado        | Detalle                                                             |
| --------------------- | ------------- | ------------------------------------------------------------------- |
| Sitemap.xml           | ✅ Accesible  | XML válido con rutas estáticas + dinámicas                          |
| robots.txt            | ✅ Accesible  | Correcto, bloquea /panel, /swagger-ui, /api-docs                    |
| Páginas indexadas     | ✅ Varias     | Home, servicios, proyectos, subdominio visible en `site:`           |
| Meta tags en crawlers | ✅ Funcional  | Middleware inyecta correctamente para rutas conocidas               |
| Structured data       | ⚠️ Parcial    | blogPostSchema funciona, organization/website/schema no verificados |
| Imágenes              | ⚠️ 2 sin alt  | Detectado por browser-use                                           |
| Hreflang              | ❌ Incorrecto | Apunta a la misma URL para todos los idiomas                        |

---

## 7. Plan de acción priorizado

### Fase 1 — Fixes críticos (1-2 días)

| #   | Tarea                                                                                                                 | Esfuerzo | Impacto |
| --- | --------------------------------------------------------------------------------------------------------------------- | -------- | ------- |
| 1   | Añadir rutas faltantes al middleware (`/blog/:slug`, `/soluciones/vps`, `/soluciones/hosting-wordpress`, `/contacto`) | 3-4h     | Alto    |
| 2   | Integrar JSON-LD en middleware para servicios y proyectos                                                             | 2-3h     | Alto    |
| 3   | Eliminar hreflang incorrecto de SEOHead                                                                               | 15min    | Medio   |
| 4   | Crear og-image.jpg dedicado y actualizar DEFAULT_IMAGE                                                                | 1h       | Medio   |
| 5   | Añadir Hosting WP y VPS al sitemap                                                                                    | 30min    | Medio   |

### Fase 2 — Mejoras estructurales (2-3 días)

| #   | Tarea                                                           | Esfuerzo | Impacto |
| --- | --------------------------------------------------------------- | -------- | ------- |
| 6   | Implementar breadcrumbs (schema + visual) en páginas de detalle | 3-4h     | Alto    |
| 7   | Crear `projectSchema` y `faqSchema` en schemas.ts               | 1-2h     | Medio   |
| 8   | Añadir `dateModified` a blogPostSchema                          | 30min    | Medio   |
| 9   | Rellenar `sameAs` en organizationSchema con redes sociales      | 15min    | Bajo    |
| 10  | Auditar y corregir todas las imágenes sin `alt`                 | 1-2h     | Medio   |

### Fase 3 — Optimización y monitoreo (continuo)

| #   | Tarea                                                   | Esfuerzo | Impacto |
| --- | ------------------------------------------------------- | -------- | ------- |
| 11  | Configurar Google Search Console                        | 30min    | Alto    |
| 12  | Ejecutar PageSpeed Insights y optimizar Core Web Vitals | 2-4h     | Alto    |
| 13  | Crear contenido de blog consistente (1-2 posts/mes)     | Continuo | Alto    |
| 14  | Añadir enlaces internos estratégicos (blog ↔ servicios) | 2-3h     | Medio   |
| 15  | Considerar RSS feed para el blog                        | 1-2h     | Bajo    |

---

## 8. Archivos SEO relevantes

| Archivo                                               | Responsabilidad                                  |
| ----------------------------------------------------- | ------------------------------------------------ |
| `frontend/src/components/seo/SEOHead.tsx`             | Componente React para meta tags dinámicos        |
| `frontend/src/components/seo/schemas.ts`              | Schemas JSON-LD centralizados                    |
| `src/handlers/seo.rs`                                 | robots.txt y sitemap.xml (backend)               |
| `src/middleware/prerender.rs`                         | Middleware de inyección SEO para crawlers        |
| `src/handlers/image_proxy.rs`                         | Proxy de optimización de imágenes (WebP, srcset) |
| `frontend/src/components/ui/OptimizedImage.tsx`       | Componente de imagen responsive                  |
| `frontend/src/hooks/useOptimizedImage.ts`             | Hook de srcset/sizes/WebP                        |
| `frontend/src/utils/imageUtils.ts`                    | Generación de srcSet y URLs de proxy             |
| `frontend/src/components/layout/CatalogPageShell.tsx` | Shell compartida con SEOHead para catálogos      |
| `frontend/src/islands/BlogSingleIsland.tsx`           | Página de blog con blogPostSchema                |
| `frontend/src/islands/SolucionHostingIsland.tsx`      | Página de hosting con SEOHead                    |
| `frontend/src/islands/SolucionVpsIsland.tsx`          | Página de VPS con SEOHead                        |
| `frontend/src/islands/NosotrosIsland.tsx`             | Nosotros con personSchema                        |
| `src/repositories/service.rs`                         | `public_slugs()` para sitemap                    |
| `src/repositories/project.rs`                         | `public_slugs()` para sitemap                    |
| `src/repositories/blog.rs`                            | `public_slugs()` para sitemap                    |

---

## 9. Convenciones SEO del proyecto

- **Meta tags:** Siempre via componente `SEOHead` en frontend + middleware `prerender` para crawlers
- **Structured data:** Definir en `schemas.ts`, pasar via prop `jsonLd` a `SEOHead`
- **Imágenes:** Siempre via `OptimizedImage` con `alt`, `width`, `height`, `sizes`
- **URLs:** Slugs canónicos en español, sin prefijo de idioma
- **Títulos:** Formato `{titulo de página} — Nakomi Studio` o `{titulo} \| Nakomi Studio`
- **Descripciones:** Máximo 160 caracteres, incluir palabra clave principal
- **Canonical:** Generado automáticamente por SEOHead desde `path`
