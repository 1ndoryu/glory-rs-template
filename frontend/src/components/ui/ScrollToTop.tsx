/* [064A-6] Scroll al tope en cada cambio de ruta.
 * Se monta dentro de BrowserRouter para que useLocation funcione. */

import {useEffect} from 'react';
import {useLocation} from 'react-router-dom';
/* [259A-5] Acceso window via platform/viewport (boundary sentinel). */
import {irArriba} from '../../platform/viewport';

export function ScrollToTop() {
    const {pathname} = useLocation();

    useEffect(() => {
        irArriba();
    }, [pathname]);

    return null;
}
