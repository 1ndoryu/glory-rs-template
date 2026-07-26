/* [044A-28] Schemas JSON-LD para structured data.
 * Centraliza la generación de datos estructurados para Google rich snippets.
 * [074A-marketing] Enriquecido con dirección Copenhague, contactPoint, image, y Person schema. */

const SITE_URL = 'https://nakomi.studio';
const LOGO_URL = `${SITE_URL}/favicon.svg`;
const OG_IMAGE = `${SITE_URL}/assets/Proyectos%20portadas/Kamples%20portada.jpg`;

export const organizationSchema = {
    '@context': 'https://schema.org',
    '@type': ['ProfessionalService', 'LocalBusiness'],
    name: 'Nakomi Studio',
    url: SITE_URL,
    logo: LOGO_URL,
    image: OG_IMAGE,
    description: 'Estudio creativo especializado en desarrollo web, aplicaciones, agentes de IA e identidad de marca. Basados en Copenhague, disponibles en todo el mundo.',
    address: {
        '@type': 'PostalAddress',
        addressLocality: 'Copenhagen',
        addressCountry: 'DK',
    },
    contactPoint: {
        '@type': 'ContactPoint',
        contactType: 'customer service',
        availableLanguage: ['Spanish', 'English', 'Japanese'],
    },
    hasOfferCatalog: {
        '@type': 'OfferCatalog',
        name: 'Servicios Digitales',
        itemListElement: [
            {
                '@type': 'Offer',
                itemOffered: {
                    '@type': 'Service',
                    name: 'Diseño de Sitios Web',
                },
            },
            {
                '@type': 'Offer',
                itemOffered: {
                    '@type': 'Service',
                    name: 'Desarrollo de Aplicaciones',
                },
            },
            {
                '@type': 'Offer',
                itemOffered: {
                    '@type': 'Service',
                    name: 'Agentes de IA',
                },
            },
        ],
    },
    sameAs: [
        'https://github.com/1ndoryu',
        'https://www.linkedin.com/company/nakomi-studio',
    ],
};

export const websiteSchema = {
    '@context': 'https://schema.org',
    '@type': 'WebSite',
    name: 'Nakomi Studio',
    url: SITE_URL,
    potentialAction: {
        '@type': 'SearchAction',
        target: `${SITE_URL}/servicios?q={search_term_string}`,
        'query-input': 'required name=search_term_string',
    },
};

export const serviceSchema = (nombre: string, descripcion: string, slug: string) => ({
    '@context': 'https://schema.org',
    '@type': 'Service',
    name: nombre,
    description: descripcion,
    url: `${SITE_URL}/servicios/${slug}`,
    provider: {
        '@type': 'ProfessionalService',
        name: 'Nakomi Studio',
        url: SITE_URL,
    },
    areaServed: 'Worldwide',
    availableLanguage: ['Spanish', 'English', 'Japanese'],
});

export const blogPostSchema = (titulo: string, descripcion: string, slug: string, fecha?: string, fechaModificacion?: string) => ({
    '@context': 'https://schema.org',
    '@type': 'BlogPosting',
    headline: titulo,
    description: descripcion,
    url: `${SITE_URL}/blog/${slug}`,
    image: OG_IMAGE,
    ...(fecha && {datePublished: fecha}),
    ...(fechaModificacion && {dateModified: fechaModificacion}),
    author: {
        '@type': 'Organization',
        name: 'Nakomi Studio',
        url: SITE_URL,
    },
    publisher: {
        '@type': 'Organization',
        name: 'Nakomi Studio',
        url: SITE_URL,
        logo: {
            '@type': 'ImageObject',
            url: LOGO_URL,
        },
    },
});

export const breadcrumbSchema = (items: {name: string; url: string}[]) => ({
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: items.map((item, i) => ({
        '@type': 'ListItem',
        position: i + 1,
        name: item.name,
        item: `${SITE_URL}${item.url}`,
    })),
});

/* [074A-marketing] Person schema para miembros del equipo en /nosotros */
export const faqSchema = (preguntas: {question: string; answer: string}[]) => ({
    '@context': 'https://schema.org',
    '@type': 'FAQPage',
    mainEntity: preguntas.map(p => ({
        '@type': 'Question',
        name: p.question,
        acceptedAnswer: {
            '@type': 'Answer',
            text: p.answer,
        },
    })),
});

export const personSchema = (nombre: string, cargo: string, bio: string, avatar: string, slug: string) => ({
    '@context': 'https://schema.org',
    '@type': 'Person',
    name: nombre,
    jobTitle: cargo,
    description: bio,
    image: avatar.startsWith('http') ? avatar : `${SITE_URL}${avatar}`,
    url: `${SITE_URL}/nosotros#${encodeURIComponent(slug)}`,
    worksFor: {
        '@type': 'Organization',
        name: 'Nakomi Studio',
        url: SITE_URL,
    },
});
