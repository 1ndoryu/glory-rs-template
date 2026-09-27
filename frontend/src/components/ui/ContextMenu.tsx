import React, {useLayoutEffect, useRef, useState} from 'react';
import {MoreHorizontal} from 'lucide-react';
import {Button} from './Button';
import {alturaVentana, desuscribirVentana, suscribirVentana} from '../../platform/viewport';
import './ContextMenu.css';

export interface MenuContextualItem {
    id: string;
    label: string;
    onSelect: () => void;
    disabled?: boolean;
    danger?: boolean;
    icon?: React.ReactNode;
}

interface MenuContextualProps {
    abierto: boolean;
    onToggle: () => void;
    onCerrar: () => void;
    items?: MenuContextualItem[];
    ariaLabel: string;
    className?: string;
    triggerClassName?: string;
    panelClassName?: string;
    itemClassName?: string;
    triggerContent?: React.ReactNode;
    triggerVariante?: 'primario' | 'secundario' | 'outline' | 'texto';
    triggerTamano?: 'pequeno' | 'mediano' | 'grande';
    children?: React.ReactNode;
    /* [095A-1] Prop semántica de contexto: 'oscuro' da color claro al trigger
     * para fondos oscuros (footer, hero dark). No inyecta diseño local. */
    contexto?: 'oscuro';
    tipo?: 'menu' | 'apps';
    /* [259A-5 5b-7] Variante canonica del trigger. 'filtro' = selector con
     * etiqueta (era proyectosFiltroEmpleado/seoPaginasFiltroTipo/usuariosFiltroBtn);
     * 'campana' = campana con insignia (era chatBell__trigger/notificationBell__trigger);
     * 'avatar' = foto de perfil (era perfilAvatarBtn); 'accion' = icono de fila
     * (era ordenDetalleOpcionesBoton/usuariosMenuBtn). La receta vive en
     * ContextMenu.css; las instancias solo declaran la variante. */
    variante?: 'filtro' | 'campana' | 'avatar' | 'accion';
}

type MenuContextualPosicion = 'abajoDerecha' | 'abajoIzquierda' | 'arribaDerecha' | 'arribaIzquierda';

export const MenuContextual: React.FC<MenuContextualProps> = ({
    abierto,
    onToggle,
    onCerrar,
    items = [],
    ariaLabel,
    className = '',
    triggerClassName = '',
    panelClassName = '',
    itemClassName = '',
    triggerContent,
    triggerVariante = 'texto',
    triggerTamano = 'pequeno',
    children,
    contexto,
    tipo = 'menu',
    variante,
}) => {
    const contenedorRef = useRef<HTMLDivElement>(null);
    const panelRef = useRef<HTMLDivElement>(null);
    const [posicion, setPosicion] = useState<MenuContextualPosicion>('abajoDerecha');

    useLayoutEffect(() => {
        if (!abierto) { return; }

        const actualizarPosicion = () => {
            const contenedor = contenedorRef.current;
            const panel = panelRef.current;
            if (!contenedor || !panel) { return; }

            const margenViewport = 8;
            const contenedorRect = contenedor.getBoundingClientRect();
            const panelRect = panel.getBoundingClientRect();
            const espacioAbajo = alturaVentana() - contenedorRect.bottom;
            const espacioArriba = contenedorRect.top;
            const abreArriba = panelRect.height + margenViewport > espacioAbajo && espacioArriba > espacioAbajo;
            const alinearIzquierda = contenedorRect.right - panelRect.width < margenViewport;

            setPosicion(`${abreArriba ? 'arriba' : 'abajo'}${alinearIzquierda ? 'Izquierda' : 'Derecha'}` as MenuContextualPosicion);
        };

        actualizarPosicion();
        /* [259A-5] Listeners via platform/viewport (boundary sentinel). */
        suscribirVentana('resize', actualizarPosicion);
        suscribirVentana('scroll', actualizarPosicion, true);
        return () => {
            desuscribirVentana('resize', actualizarPosicion);
            desuscribirVentana('scroll', actualizarPosicion, true);
        };
    }, [abierto]);

    const handleBlur = (event: React.FocusEvent<HTMLDivElement>) => {
        const nextTarget = event.relatedTarget as Node | null;
        if (nextTarget && event.currentTarget.contains(nextTarget)) {
            return;
        }
        onCerrar();
    };

    const clasesPanel = [
        'menuContextualPanel',
        tipo === 'apps' ? 'menuContextualPanelApps' : '',
        posicion.startsWith('arriba') ? 'menuContextualPanelArriba' : '',
        posicion.endsWith('Izquierda') ? 'menuContextualPanelIzquierda' : '',
        panelClassName,
    ].filter(Boolean).join(' ');

    /* [259A-5 5b-7] Receta del trigger por variante canonica. */
    const RECETAS_TRIGGER: Record<NonNullable<typeof variante>, string> = {
        filtro: 'menuContextualFiltro',
        campana: 'menuContextualCampana',
        avatar: 'menuContextualAvatar',
        accion: 'menuContextualAccion',
    };

    return (
        <div ref={contenedorRef} className={`menuContextual${contexto === 'oscuro' ? ' menuContextualOscuro' : ''}${tipo === 'apps' ? ' menuContextualApps' : ''} ${className}`.trim()} onBlur={handleBlur}>
            <Button
                className={`menuContextualBoton${variante ? ` ${RECETAS_TRIGGER[variante]}` : ''} ${triggerClassName}`.trim()}
                onClick={onToggle}
                type="button"
                aria-haspopup="menu"
                aria-expanded={abierto}
                aria-label={ariaLabel}
                variante={triggerVariante}
                tamano={triggerTamano}
            >
                {triggerContent ?? <MoreHorizontal size={18} />}
            </Button>

            {abierto && (
                <div ref={panelRef} className={clasesPanel} role="menu">
                    {children ? (
                        <div className="menuContextualContenido">{children}</div>
                    ) : (
                        <div className="menuContextualContenido">
                            {items.map(item => (
                                <Button
                                    key={item.id}
                                    className={[
                                        'menuContextualItem',
                                        item.danger ? 'menuContextualItemDanger' : '',
                                        itemClassName,
                                    ].filter(Boolean).join(' ')}
                                    onClick={() => {
                                        item.onSelect();
                                        onCerrar();
                                    }}
                                    disabled={item.disabled}
                                    type="button"
                                    variante="texto"
                                    tamano="pequeno"
                                >
                                    {item.icon}
                                    <span>{item.label}</span>
                                </Button>
                            ))}
                        </div>
                    )}
                </div>
            )}
        </div>
    );
};