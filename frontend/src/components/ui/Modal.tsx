/* [044A-40] Componente base Modal reutilizable.
 * Todos los modales del sistema deben usar este componente como contenedor.
 * Incluye: overlay con click-to-close, Escape listener, focus trap, scroll lock.
 * [114A-10] Usa createPortal para evitar que stacking contexts padre
 * causen desaparición del fondo. Compensa scrollbar width al bloquear scroll. */
import React, {useEffect, useRef} from 'react';
import {createPortal} from 'react-dom';
import {useFocusTrap} from '../../hooks/useFocusTrap';
import {bloquearDesplazamientoPagina, desbloquearDesplazamientoPagina, nodoCuerpoDocumento} from '../../platform/dom';
import './Modal.css';

function combinarClases(...clases: Array<string | undefined>): string {
    return clases.filter(Boolean).join(' ');
}

interface ModalProps {
    abierto: boolean;
    onCerrar: () => void;
    children: React.ReactNode;
    /** Clase CSS adicional para el contenedor interno */
    className?: string;
}

type ModalBodyBaseProps = {
    className?: string;
    children: React.ReactNode;
};

type ModalBodyDivProps = ModalBodyBaseProps
    & { as?: 'div' }
    & Omit<React.HTMLAttributes<HTMLDivElement>, 'children' | 'className'>;

type ModalBodyFormProps = ModalBodyBaseProps
    & { as: 'form' }
    & Omit<React.FormHTMLAttributes<HTMLFormElement>, 'children' | 'className'>;

type ModalBodyProps = ModalBodyDivProps | ModalBodyFormProps;

type ModalFieldProps = React.HTMLAttributes<HTMLDivElement> & {
    children: React.ReactNode;
};

type ModalLabelProps = React.LabelHTMLAttributes<HTMLLabelElement> & {
    children: React.ReactNode;
};

/* [035A-24] El layout comun de formularios/campos del modal vive en el sistema UI.
 * Evita que cada modal redefina .algoFormCrear o .algoCampo para la misma receta. */
export function ModalBody(props: ModalBodyDivProps): React.ReactElement;
export function ModalBody(props: ModalBodyFormProps): React.ReactElement;
export function ModalBody({as = 'div', className, children, ...rest}: ModalBodyProps) {
    return React.createElement(
        as,
        {
            ...(rest as Record<string, unknown>),
            className: combinarClases('modalFormulario', className),
        },
        children,
    );
}

export function ModalField({className, children, ...rest}: ModalFieldProps) {
    return (
        <div {...rest} className={combinarClases('modalCampo', className)}>
            {children}
        </div>
    );
}

export function ModalLabel({className, children, ...rest}: ModalLabelProps) {
    return (
        <label {...rest} className={combinarClases('modalEtiqueta', className)}>
            {children}
        </label>
    );
}

export const Modal: React.FC<ModalProps> = ({abierto, onCerrar, children, className}) => {
    const modalRef = useRef<HTMLDivElement>(null);
    useFocusTrap(modalRef, abierto);

    /* [114A-10] Bloquear scroll + compensar scrollbar para evitar layout shift */
    useEffect(() => {
        if (!abierto) return;

        const handleEscape = (e: KeyboardEvent) => {
            if (e.key === 'Escape') onCerrar();
        };

        /* [259A-5] Scroll lock via platform/dom (boundary sentinel). */
        bloquearDesplazamientoPagina();

        document.addEventListener('keydown', handleEscape);

        return () => {
            document.removeEventListener('keydown', handleEscape);
            desbloquearDesplazamientoPagina();
        };
    }, [abierto, onCerrar]);

    if (!abierto) return null;

    const handleOverlayClick = (e: React.MouseEvent<HTMLDivElement>) => {
        if (e.target === e.currentTarget) onCerrar();
    };

    const claseContenedor = combinarClases('modalBaseContenedor', className);

    /* [114A-10] Portal al cuerpo del documento para aislar de stacking contexts padre */
    return createPortal(
        <div className="modalBaseOverlay" onClick={handleOverlayClick}>
            <div className={claseContenedor} ref={modalRef} role="dialog" aria-modal="true">
                {children}
            </div>
        </div>,
        nodoCuerpoDocumento(),
    );
};
