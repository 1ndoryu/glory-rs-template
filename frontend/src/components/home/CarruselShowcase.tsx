/// <reference types="vite/client" />
/*
 * Componente: CarruselShowcase
 * Carrusel infinito de proyectos con desplazamiento automatico y soporte drag.
 * [084A-11] Ahora consume API pública.
 * [094A-20] Sin fallback estático: home debe reflejar el CMS o quedar vacío. */
import React, {useMemo, useRef, useState, useLayoutEffect} from 'react';
import {useQuery} from '@tanstack/react-query';
import {Badge} from '../ui/Badge';
import OptimizedImage from '../ui/OptimizedImage';
import {spaClick} from '../../navegacionSPA';
import './CarruselShowcase.css';
import {useCarruselInfinito} from '../../hooks/useCarruselInfinito';
import {mapAdminProjectsToProyectos} from '../../data/showcase';
import {apiListPublicProjects} from '../../api/admin-projects';
import {IMAGENES_SHOWCASE} from '../../hooks/useImagenes';

export const CarruselShowcase: React.FC = () => {
    /* [084A-11] Fetch proyectos publicados del API.
     * [084A-30] Mientras carga no renderizamos un dataset fantasma. */
    const {data: apiProjects} = useQuery({
        queryKey: ['public-projects-showcase'],
        queryFn: apiListPublicProjects,
        staleTime: 5 * 60 * 1000,
        retry: 1,
    });

    const baseProyectos = useMemo(
        /* [124A-PROJ] Solo proyectos con in_carousel=true aparecen en el carrusel */
        () => mapAdminProjectsToProyectos((apiProjects || []).filter(p => p.in_carousel)),
        [apiProjects]
    );

    const proyectosConImagen = baseProyectos.map((proyecto, idx) => ({
        ...proyecto,
        imagen: proyecto.imagen || (IMAGENES_SHOWCASE.length > 0 ? IMAGENES_SHOWCASE[idx % IMAGENES_SHOWCASE.length] : ''),
    }));

    const itemsTotales = [...proyectosConImagen, ...proyectosConImagen];

    /* [084A-25] Hook SIEMPRE antes de early return — Rules of Hooks.
     * Se llama con totalItems=0 cuando hay 0 proyectos, pero nunca se omite. */
    const {indiceActual, conTransicion, dragOffset, handlers} = useCarruselInfinito({
        totalItems: proyectosConImagen.length,
        tiempoEspera: 8000,
        tiempoTransicion: 800
    });

    /* [124A-CMS5] Medir offsets reales del DOM para items de ancho variable.
     * ResizeObserver re-mide cuando imágenes terminan de cargar y cambian el ancho. */
    const pistaRef = useRef<HTMLDivElement>(null);
    const [offsetX, setOffsetX] = useState(0);

    useLayoutEffect(() => {
        const pista = pistaRef.current;
        if (!pista) return;

        const medir = () => {
            const items = pista.children;
            if (indiceActual >= 0 && indiceActual < items.length) {
                setOffsetX((items[indiceActual] as HTMLElement).offsetLeft);
            }
        };

        medir();

        const observer = new ResizeObserver(medir);
        observer.observe(pista);
        return () => observer.disconnect();
    }, [indiceActual, itemsTotales.length]);

    /* [279A-1] Desplazamiento via setProperty (patron runtime whitelisteado e
     * indexado por VarSense): style={} con --var dispara cssInlineReact aunque
     * solo inyecte datos dinamicos, y el objeto nunca entra al indice de vars.
     * useLayoutEffect = pre-paint, sin flash respecto al style prop. */
    useLayoutEffect(() => {
        const pista = pistaRef.current;
        if (!pista) return;
        pista.style.setProperty('--carrusel-desplazamiento', `translateX(${-offsetX + dragOffset}px)`);
        pista.style.setProperty('--carrusel-transicion', conTransicion ? 'transform 800ms cubic-bezier(0.25, 1, 0.5, 1)' : 'none');
    }, [offsetX, dragOffset, conTransicion]);

    if (proyectosConImagen.length === 0) return null;

    return (
        <div className="carruselContenedorPrincipal">
            <div
                className="carruselPista"
                ref={pistaRef}
                {...handlers}>
                {itemsTotales.map((proyecto, index) => {
                    const categoriasArr = Array.isArray(proyecto.categorias)
                        ? proyecto.categorias
                        : (proyecto.categorias ? [proyecto.categorias] : []);

                    /* [124A-IMG] Primer item visible = LCP candidato = eager + alta prioridad.
                     * El resto lazy para no bloquear la carga inicial. */
                    const isFirst = index === 0;

                    return (
                        <a key={`proj-${index}`} href={proyecto.link || '#'} className="carruselItem" draggable={false} onClick={e => { if (proyecto.link) spaClick(e, proyecto.link); }}>
                            <div className="carruselImagenWrapper">
                                {proyecto.imagen && (
                                    <OptimizedImage
                                        src={proyecto.imagen}
                                        alt={proyecto.titulo}
                                        className="carruselImagen"
                                        /* [164A-20] El inicio no debe escalar por DPR en este carrusel.
                                         * Forzamos proxy fijo a w=1600&q=80 para evitar sobredescarga. */
                                        fixedWidth={1600}
                                        quality={80}
                                        loading={isFirst ? 'eager' : 'lazy'}
                                        fetchPriority={isFirst ? 'high' : 'auto'}
                                        draggable={false}
                                    />
                                )}
                            </div>
                            <div className="carruselContenido">
                                <h3 className="carruselTitulo">{proyecto.titulo}</h3>
                                <div className="carruselTags">
                                    {categoriasArr.slice(0, 3).map(cat => (
                                        <Badge key={cat} label={cat} />
                                    ))}
                                </div>
                            </div>
                        </a>
                    );
                })}
            </div>
            <div className="carruselOverlay" />
        </div>
    );
};
