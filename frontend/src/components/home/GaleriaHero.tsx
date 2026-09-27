/* GaleriaHero: muestra una imagen a la vez de los proyectos marcados con in_carousel.
 * Usa gallery_image si existe, si no usa featured_image.
 * Las imágenes rotan con crossfade. Proporción 1200x600 con bordes redondeados.
 * Overlay inferior-derecho: nombre del proyecto (enlace al detalle) + icono info que
 * expande descripción y enlaces del proyecto. Estilo pill inspirado en chatWidgetBubble. */
/* [259A-5 5b-6] sentinel-disable-file html-nativo-en-vez-de-componente: solo las zonas de click del carousel usan <button> nativo (areas overlay full-height, no acciones; <Button> romperia el layout). */
import React from 'react';
import {Info, X} from 'lucide-react';
import OptimizedImage from '../ui/OptimizedImage';
import {Button} from '../ui/Button';
import {spaClick} from '../../navegacionSPA';
import {useGaleriaHero} from '../../hooks/useGaleriaHero';
import './GaleriaHero.css';

export const GaleriaHero: React.FC = () => {
    const {actual, entradas, expandido, href, indice, irAnterior, irSiguiente, toggleExpandido} = useGaleriaHero();

    if (!actual) return null;

    return (
        <div className="galeriaHeroContenedor">
            <OptimizedImage
                key={`${actual.url}-${indice}`}
                src={actual.url}
                alt={actual.proyecto.title}
                className="galeriaHeroImagen galeriaHeroImagenActiva"
                sizes="(max-width: 768px) calc(100vw - 32px), min(100vw - 48px, 1200px)"
                quality={72}
                loading="eager"
                fetchPriority="high"
            />

            {entradas.length > 1 && (
                <>
                    <button
                        type="button"
                        className="galeriaHeroZonaClick galeriaHeroZonaClickAnterior"
                        onClick={irAnterior}
                        aria-label="Ver proyecto anterior"
                    />
                    <button
                        type="button"
                        className="galeriaHeroZonaClick galeriaHeroZonaClickSiguiente"
                        onClick={irSiguiente}
                        aria-label="Ver proyecto siguiente"
                    />
                </>
            )}

            {/* Overlay inferior-derecho */}
            <div className="galeriaHeroOverlay">
                {expandido && (
                    <div className="galeriaHeroDetalle">
                        {actual.proyecto.description && (
                            <p className="galeriaHeroDescripcion">{actual.proyecto.description}</p>
                        )}
                        {actual.proyecto.links.length > 0 && (
                            <div className="galeriaHeroEnlaces">
                                {actual.proyecto.links.map(enlace => (
                                    <a
                                        key={enlace.url}
                                        href={enlace.url}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        className="galeriaHeroEnlace"
                                    >
                                        {enlace.etiqueta || enlace.tipo}
                                    </a>
                                ))}
                            </div>
                        )}
                    </div>
                )}
                <div className="galeriaHeroPill">
                    <a
                        href={href}
                        className="galeriaHeroPillTitulo"
                        onClick={e => spaClick(e, href)}
                    >
                        {actual.proyecto.title}
                    </a>
                    <Button
                        variante="texto"
                        className="galeriaHeroPillInfo"
                        onClick={toggleExpandido}
                        aria-label={expandido ? 'Cerrar información' : 'Ver información del proyecto'}
                        aria-expanded={expandido}
                    >
                        {expandido ? <X size={14} /> : <Info size={14} />}
                    </Button>
                </div>
            </div>
        </div>
    );
};
