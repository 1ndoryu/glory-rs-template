import { Suspense, lazy } from 'react';
import { PaginaPublica } from './features/publica/pagina-publica';
import { rutaActual } from './platform/ventana';

/* [249A-1] El panel va en chunk aparte (`React.lazy`): el visitante público
 * solo descarga la página pública + sus datos, sin el código de gestión. */
const AppAdmin = lazy(() =>
  import('./app/app-admin').then((m) => ({ default: m.AppAdmin })),
);
/* [279A-3] /ask: cuestionario privado para completar fichas en chunk aparte (igual
 * que el panel: el visitante público no descarga este código). */
const PaginaAsk = lazy(() =>
  import('./features/ask/pagina-ask').then((m) => ({ default: m.PaginaAsk })),
);

export default function App() {
  /* `/ask*` = cuestionario privado con sesión; `/admin*` = gestión con
   * sesión; el resto = web pública. */
  if (rutaActual().startsWith('/ask')) {
    return (
      <Suspense fallback={<p className="px-6 py-16 text-center text-sm">Cargando cuestionario…</p>}>
        <PaginaAsk />
      </Suspense>
    );
  }
  /* `/admin*` = gestión con sesión; el resto = web pública. */
  if (rutaActual().startsWith('/admin')) {
    return (
      <Suspense fallback={<p className="px-6 py-16 text-center text-sm">Cargando panel…</p>}>
        <AppAdmin />
      </Suspense>
    );
  }
  return <PaginaPublica />;
}
