import {useEffect, useRef, useState} from 'react';

import {
    generateSrcSet,
    generateWebPSrcSet,
    optimizedUrl,
    resolveBestWidth,
    resolveResponsiveWidths,
} from '../utils/imageUtils';
/* [259A-5] Listener window via platform/viewport (boundary sentinel). */
import {desuscribirVentana, suscribirVentana} from '../platform/viewport';

interface UseOptimizedImageParams {
    src: string;
    width?: number;
    fixedWidth?: number;
    sizes?: string;
    quality: number;
    noOptimize: boolean;
}

interface UseOptimizedImageResult {
    pictureRef: React.RefObject<HTMLPictureElement>;
    shouldOptimize: boolean;
    resolvedSizes: string;
    fallbackSrc: string;
    fallbackSrcSet: string;
    webpSrcSet: string;
}

export function useOptimizedImage({
    src,
    width,
    fixedWidth,
    sizes,
    quality,
    noOptimize,
}: UseOptimizedImageParams): UseOptimizedImageResult {
    const pictureRef = useRef<HTMLPictureElement>(null);
    const [renderedWidth, setRenderedWidth] = useState<number | undefined>(width);
    const shouldOptimize = Boolean(src)
        && !noOptimize
        && !src.startsWith('data:')
        && !src.startsWith('http')
        && !src.endsWith('.svg')
        && (src.startsWith('/uploads/') || src.startsWith('/assets/'));

    useEffect(() => {
        if (!shouldOptimize) {
            setRenderedWidth(width);
            return undefined;
        }

        if (fixedWidth) {
            setRenderedWidth(fixedWidth);
            return undefined;
        }

        const pictureElement = pictureRef.current;
        if (!pictureElement) {
            return undefined;
        }

        const updateWidth = (nextWidth?: number) => {
            const measuredWidth = Math.round(nextWidth ?? pictureElement.getBoundingClientRect().width);

            if (measuredWidth > 0) {
                setRenderedWidth((currentWidth) => (currentWidth === measuredWidth ? currentWidth : measuredWidth));
            }
        };

        updateWidth(width);

        if (typeof ResizeObserver === 'undefined') {
            const handleResize = () => updateWidth();
            /* [259A-5] Listener via platform/viewport (boundary sentinel). */
            suscribirVentana('resize', handleResize);
            return () => desuscribirVentana('resize', handleResize);
        }

        const resizeObserver = new ResizeObserver((entries) => {
            const firstEntry = entries[0];
            updateWidth(firstEntry?.contentRect.width);
        });

        resizeObserver.observe(pictureElement);

        return () => resizeObserver.disconnect();
    }, [shouldOptimize, src, width, fixedWidth]);

    /* [104A-13] Si el caller no declara sizes, medimos el ancho real para no caer en 100vw.
     * Esto evita que logos, avatars y cards pequeñas descarguen variantes más grandes de lo necesario. */
    const devicePixelRatio = typeof window === 'undefined' ? 1 : window.devicePixelRatio || 1;
    const effectivePixelRatio = fixedWidth ? 1 : devicePixelRatio;
    const targetWidth = fixedWidth ?? renderedWidth ?? width;
    const responsiveWidths = fixedWidth
        ? []
        : resolveResponsiveWidths(targetWidth, devicePixelRatio);
    const fallbackWidth = resolveBestWidth(targetWidth, effectivePixelRatio);
    const resolvedSizes = fixedWidth
        ? `${Math.round(fixedWidth)}px`
        : sizes ?? (targetWidth ? `${Math.round(targetWidth)}px` : '100vw');

    return {
        pictureRef,
        shouldOptimize,
        resolvedSizes,
        fallbackSrc: optimizedUrl(src, fallbackWidth ? {quality, width: fallbackWidth} : {quality}),
        fallbackSrcSet: fixedWidth ? '' : generateSrcSet(src, responsiveWidths, {quality}),
        /* [154A-IMG] WebP reactivado: backend usa webp::Encoder::from_rgba() lossy desde d6b260c.
         * El lossy WebP produce 80-90% menos peso que PNG y 20-30% menos que JPEG.
         * Fixed-width usa URL única; responsive usa srcset completo. */
        webpSrcSet: fixedWidth ? '' : generateWebPSrcSet(src, responsiveWidths, quality),
    };
}