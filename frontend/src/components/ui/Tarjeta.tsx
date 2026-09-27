/* [084A-6] Componente base Tarjeta — wrapper visual reutilizable.
 * Centraliza estilo de tarjeta (borde, fondo, radius, padding).
 * Se usa como contenedor en features, servicios, etc. */

import React from 'react';
import './Tarjeta.css';

interface TarjetaProps extends React.HTMLAttributes<HTMLDivElement | HTMLButtonElement> {
    children: React.ReactNode;
    className?: string;
    fondo?: string;
    onClick?: () => void;
}

export const Tarjeta: React.FC<TarjetaProps> = ({children, className, fondo, onClick, ...props}) => {
    /* [259A-6] Fondo por instancia via --var (style prop solo inyecta --var). */
    const estiloInline = fondo ? ({'--tarjeta-fondo': fondo} as React.CSSProperties) : undefined;
    const Tag = onClick ? 'button' : 'div';

    return (
        <Tag
            className={`tarjetaBase ${className ?? ''}`}
            /* [279A-1] Tag polimorfico (button|div): ref+setProperty exigiria
             * plumbing de tipos por un solo dato dinamico; style prop solo
             * inyecta --tarjeta-fondo (definida en Tarjeta.css).
             * varsense-disable-next-line cssInlineReact */
            style={estiloInline}
            onClick={onClick}
            {...props}
            {...(onClick ? {type: 'button' as const} : {})}
        >
            {children}
        </Tag>
    );
};
