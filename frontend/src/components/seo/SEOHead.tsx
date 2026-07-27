/* [044A-28] Componente SEO reutilizable.
 * Gestiona title, meta description, Open Graph, Twitter Card, canonical
 * y structured data JSON-LD desde cada página/island.
 * [074A-marketing] DEFAULT_IMAGE actualizado a imagen real existente.
 * og:image:width/height añadidos para cumplir best practices de OG. */
import {Helmet} from 'react-helmet-async';
import {useTranslation} from 'react-i18next';

const SITE_NAME = 'Nakomi Studio';
const SITE_URL = 'https://nakomi.studio';
/* [277A-14] OG image por defecto: Kamples portada como placeholder.
 * TODO: Crear og-nakomi.jpg (1200×630) con logo + tagline de Nakomi. */
const DEFAULT_IMAGE = `${SITE_URL}/assets/Proyectos%20portadas/Kamples%20portada.jpg`;

interface SEOHeadProps {
    title?: string;
    description?: string;
    path?: string;
    image?: string;
    imageWidth?: number;
    imageHeight?: number;
    type?: 'website' | 'article';
    jsonLd?: Record<string, unknown>;
    noindex?: boolean;
}

export const SEOHead = ({
    title,
    description,
    path = '',
    image = DEFAULT_IMAGE,
    imageWidth = 1200,
    imageHeight = 630,
    type = 'website',
    jsonLd,
    noindex = false,
}: SEOHeadProps) => {
    const {i18n} = useTranslation();
    const fullTitle = title ? `${title} | ${SITE_NAME}` : SITE_NAME;
    const canonicalUrl = `${SITE_URL}${path}`;

    return (
        <Helmet>
            <html lang={i18n.language} />
            <title>{fullTitle}</title>
            {description && <meta name="description" content={description} />}
            <link rel="canonical" href={canonicalUrl} />
            {noindex && <meta name="robots" content="noindex,nofollow" />}

            {/* Open Graph */}
            <meta property="og:title" content={fullTitle} />
            {description && <meta property="og:description" content={description} />}
            <meta property="og:url" content={canonicalUrl} />
            <meta property="og:type" content={type} />
            <meta property="og:image" content={image} />
            <meta property="og:image:width" content={String(imageWidth)} />
            <meta property="og:image:height" content={String(imageHeight)} />
            {description && <meta property="og:image:alt" content={description} />}
            <meta property="og:site_name" content={SITE_NAME} />
            <meta property="og:locale" content={i18n.language} />

            {/* Twitter Card */}
            <meta name="twitter:card" content="summary_large_image" />
            <meta name="twitter:title" content={fullTitle} />
            {description && <meta name="twitter:description" content={description} />}
            <meta name="twitter:image" content={image} />
            {description && <meta name="twitter:image:alt" content={description} />}

            {/* [277A-9] hreflang eliminado: todos los idiomas apuntan a la misma URL.
             * Google considera hreflang con URLs idénticas como señal contradictoria.
             * Se restaurará cuando existan URLs separadas por idioma (/en/, /ja/). */}

            {/* Structured Data */}
            {jsonLd && (
                <script type="application/ld+json">
                    {JSON.stringify(jsonLd)}
                </script>
            )}
        </Helmet>
    );
};
