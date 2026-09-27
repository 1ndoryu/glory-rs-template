/* [054A-19] Hook: useHeader
 * Encapsula lógica de estado del Header: dropdown, modal, menú móvil.
 * Extraído del componente Header para cumplir SRP (<3 useState).
 * [064A-61] Submenú móvil para navegación anidada en overlay modal. */
import {useState, useRef, useCallback, useEffect} from 'react';
import {useAuthStore} from '../stores/authStore';
/* [259A-5] Acceso window via platform/navigation (boundary sentinel). */
import {obtenerRuta} from '../platform/navigation';

/* Comprueba si la ruta actual coincide con un path dado */
function esRutaActual(path: string): boolean {
    return obtenerRuta().replace(/\/+$/, '') === path.replace(/\/+$/, '');
}

export const useHeader = () => {
    const [dropdownAbierto, setDropdownAbierto] = useState<string | null>(null);
    const [modalAbierto, setModalAbierto] = useState(false);
    const [menuMovilAbierto, setMenuMovilAbierto] = useState(false);
    /* [064A-61] Submenú activo en el menú móvil (null = menú principal) */
    const [subMenuMovil, setSubMenuMovil] = useState<string | null>(null);
    const logueado = useAuthStore(s => s.logueado);
    const enPanel = esRutaActual('/panel');
    const dropdownRef = useRef<HTMLDivElement>(null);

    /* Cierra dropdown al hacer Escape */
    const handleKeyDownDropdown = useCallback(
        (e: React.KeyboardEvent, label: string) => {
            if (e.key === 'Escape') {
                setDropdownAbierto(null);
            } else if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                setDropdownAbierto(prev => (prev === label ? null : label));
            } else if (e.key === 'ArrowDown' && dropdownAbierto === label) {
                e.preventDefault();
                const submenu = dropdownRef.current?.querySelector('.subMenuEnlace') as HTMLElement | null;
                submenu?.focus();
            }
        },
        [dropdownAbierto]
    );

    /* Navegación por teclado dentro del submenú */
    const handleKeyDownSubmenu = useCallback((e: React.KeyboardEvent) => {
        const target = e.currentTarget as HTMLElement;
        if (e.key === 'Escape') {
            setDropdownAbierto(null);
            (target.closest('.enlaceNavegacionWrapper')?.querySelector('.enlaceNavegacion') as HTMLElement)?.focus();
        } else if (e.key === 'ArrowDown') {
            e.preventDefault();
            (target.nextElementSibling as HTMLElement)?.focus();
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            (target.previousElementSibling as HTMLElement)?.focus();
        }
    }, []);

    /* Cerrar menú móvil con Escape. Si hay submenú abierto, cierra el submenú primero */
    useEffect(() => {
        if (!menuMovilAbierto) return;
        const handleEsc = (e: KeyboardEvent) => {
            if (e.key === 'Escape') {
                if (subMenuMovil) {
                    setSubMenuMovil(null);
                } else {
                    setMenuMovilAbierto(false);
                }
            }
        };
        document.addEventListener('keydown', handleEsc);
        return () => document.removeEventListener('keydown', handleEsc);
    }, [menuMovilAbierto, subMenuMovil]);

    /* [064A-61] Al cerrar el menú móvil, resetear submenú */
    const cerrarMenuMovil = useCallback(() => {
        setMenuMovilAbierto(false);
        setSubMenuMovil(null);
    }, []);

    return {
        dropdownAbierto, setDropdownAbierto,
        modalAbierto, setModalAbierto,
        menuMovilAbierto, setMenuMovilAbierto,
        subMenuMovil, setSubMenuMovil,
        cerrarMenuMovil,
        logueado, enPanel,
        dropdownRef,
        handleKeyDownDropdown,
        handleKeyDownSubmenu,
    };
};
