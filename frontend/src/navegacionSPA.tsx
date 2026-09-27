/* [044A-1] Wrapper de navegacion sobre React Router.
 * Reemplaza el motor SPA basado en fetch/islands de WordPress.
 * Exporta navegar() con la misma API para compatibilidad con componentes existentes.
 * Los componentes que importan navegar() siguen funcionando sin cambios.
 * [044A-39] Añadido spaClick: handler para <a> que previene reload preservando cmd/ctrl+click. */

import type React from 'react';
/* [259A-5] Fallback de navegacion via platform (boundary sentinel). */
import {redirigir} from './platform/navigation';

let navigateRef: ((to: string) => void) | null = null;

/**
 * Registra la funcion navigate de React Router.
 * Se llama una vez desde App.tsx al montar el router.
 */
export function registrarNavigate(fn: (to: string) => void): void {
    navigateRef = fn;
}

/**
 * Navegacion programatica compatible con la API anterior.
 * Usa React Router internamente en lugar de fetch + parse HTML.
 */
export function navegar(url: string): void {
    if (navigateRef) {
        navigateRef(url);
    } else {
        /* [259A-5] Fallback via platform/navigation (boundary sentinel). */
        redirigir(url);
    }
}

/* [044A-39] Handler para <a href> que intercepta clicks normales para SPA navigation.
 * Preserva cmd/ctrl+click (nueva pestaña) y shift+click (nueva ventana).
 * Uso: <a href={path} onClick={e => spaClick(e, path)}> */
export function spaClick(e: React.MouseEvent<HTMLAnchorElement>, href: string): void {
    if (e.metaKey || e.ctrlKey || e.shiftKey || e.button !== 0) return;
    e.preventDefault();
    navegar(href);
}
