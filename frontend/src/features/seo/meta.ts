/* wandori.us — SEO Meta Tags
 * Actualiza meta tags dinámicamente al navegar entre páginas.
 * Importante para compartir links en redes sociales. */

interface MetaOptions {
  title?: string;
  description?: string;
  image?: string;
  url?: string;
  type?: string;
}

const SITE_NAME = 'wandorius';
const DEFAULT_DESCRIPTION = 'blog, portfolio y proyectos de wandorius';
const DEFAULT_IMAGE = '/uploads/og-default.jpg';

export function updateMeta(options: MetaOptions): void {
  const {
    title,
    description = DEFAULT_DESCRIPTION,
    image = DEFAULT_IMAGE,
    url = window.location.href,
    type = 'website',
  } = options;

  const fullTitle = title ? `${title} — ${SITE_NAME}` : SITE_NAME;

  /* Title */
  document.title = fullTitle;
  setMeta('name', 'title', fullTitle);
  setMeta('property', 'og:title', fullTitle);
  setMeta('name', 'twitter:title', fullTitle);

  /* Description */
  setMeta('name', 'description', description);
  setMeta('property', 'og:description', description);
  setMeta('name', 'twitter:description', description);

  /* Image */
  setMeta('property', 'og:image', image);
  setMeta('name', 'twitter:image', image);

  /* URL */
  setMeta('property', 'og:url', url);
  setMeta('name', 'twitter:url', url);

  /* Type */
  setMeta('property', 'og:type', type);

  /* Twitter card */
  setMeta('name', 'twitter:card', 'summary_large_image');

  /* Canonical link */
  let canonical = document.querySelector('link[rel="canonical"]');
  if (!canonical) {
    canonical = document.createElement('link');
    canonical.setAttribute('rel', 'canonical');
    document.head.appendChild(canonical);
  }
  canonical.setAttribute('href', url);
}

function setMeta(attr: string, key: string, value: string): void {
  let el = document.querySelector(`meta[${attr}="${key}"]`) as HTMLMetaElement | null;
  if (!el) {
    el = document.createElement('meta');
    el.setAttribute(attr, key);
    document.head.appendChild(el);
  }
  el.setAttribute('content', value);
}

/* Configurar meta tags para una página de artículo */
export function updateArticleMeta(article: {
  title: string;
  excerpt?: string;
  cover_image?: string;
  slug: string;
}): void {
  updateMeta({
    title: article.title,
    description: article.excerpt || DEFAULT_DESCRIPTION,
    image: article.cover_image || DEFAULT_IMAGE,
    url: `${window.location.origin}/article/${article.slug}`,
    type: 'article',
  });
}

/* === JSON-LD Structured Data === */

function setJsonLd(data: Record<string, unknown>): void {
  removeJsonLd();

  const script = document.createElement('script');
  script.type = 'application/ld+json';
  script.textContent = JSON.stringify(data);
  document.head.appendChild(script);
}

/** Schema para el sitio completo (Website + Person) */
export function setSiteJsonLd(): void {
  setJsonLd({
    '@context': 'https://schema.org',
    '@type': 'WebSite',
    name: SITE_NAME,
    url: window.location.origin,
    description: DEFAULT_DESCRIPTION,
    author: {
      '@type': 'Person',
      name: 'wandorius',
      url: window.location.origin,
      sameAs: [
        'https://instagram.com/wandorius',
        'https://facebook.com/wandorius',
        'https://threads.net/wandorius',
        'https://youtube.com/@wandorius',
        'https://open.spotify.com/user/wandorius',
        'https://github.com/1ndoryu',
      ],
    },
  });
}

/** Schema para un artículo individual (Article/BlogPosting) */
export function setArticleJsonLd(article: {
  title: string;
  excerpt?: string;
  cover_image?: string;
  slug: string;
  published_at?: string | null;
  created_at: string;
}): void {
  const data: Record<string, unknown> = {
    '@context': 'https://schema.org',
    '@type': 'BlogPosting',
    headline: article.title,
    description: article.excerpt || DEFAULT_DESCRIPTION,
    url: `${window.location.origin}/article/${article.slug}`,
    datePublished: article.published_at || article.created_at,
    author: {
      '@type': 'Person',
      name: 'wandorius',
    },
    publisher: {
      '@type': 'Person',
      name: 'wandorius',
    },
    mainEntityOfPage: {
      '@type': 'WebPage',
      '@id': `${window.location.origin}/article/${article.slug}`,
    },
  };

  if (article.cover_image) {
    data.image = article.cover_image;
  }

  setJsonLd(data);
}

/** Schema para una página estática */
export function setPageJsonLd(title: string, description: string): void {
  setJsonLd({
    '@context': 'https://schema.org',
    '@type': 'WebPage',
    name: title,
    description,
    url: window.location.href,
    isPartOf: {
      '@type': 'WebSite',
      name: SITE_NAME,
      url: window.location.origin,
    },
  });
}

/** Elimina JSON-LD de la página */
export function removeJsonLd(): void {
  document.querySelectorAll('script[type="application/ld+json"]').forEach(el => el.remove());
}

/* Reset a meta tags por defecto */
export function resetMeta(): void {
  updateMeta({});
  removeJsonLd();
}
