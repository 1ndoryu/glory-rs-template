import { Suspense, lazy, useMemo, useState } from 'react';
import { useDetallePublico } from '../../hooks/publica/use-detalle-publico';
import { useFiltrosAvanzados } from '../../hooks/publica/use-filtros-avanzados';
import { useFiltrosPagina } from '../../hooks/publica/use-filtros-pagina';
import { useModalLogin } from '../../hooks/publica/modales/use-modal-login';
import { useModalPublicar } from '../../hooks/publica/modales/use-modal-publicar';
import { useMontarAlAbrir } from '../../hooks/publica/use-montar-al-abrir';
import { usePublica } from '../../hooks/publica/use-publica';
import { abrirExterna } from '../../platform/ventana';
import { BuscadorPublica } from './lista/buscador-publica';
import { EsqueletoLista } from './lista/esqueleto-lista';
import { construirIndice, extraerTerminos, filtrarIndice } from './busqueda';
import { CabeceraPublica } from './cabecera-publica';
import { CajaInmueble } from './lista/caja-inmueble';
/* [249A-4] Overlays fuera del JS inicial (PSI: 60.3 KiB sin usar de 121):
 * `lazy` los saca del chunk crítico; `Suspense` sin fallback porque cada
 * modal ya devuelve `null` cerrado. El mapeo a `default` es porque los
 * modales usan exports nombrados. */
const ModalDetallePublico = lazy(() => import('./modales/modal-detalle-publico').then((m) => ({ default: m.ModalDetallePublico })));
const ModalFiltros = lazy(() => import('./modales/modal-filtros').then((m) => ({ default: m.ModalFiltros })));
const ModalLogin = lazy(() => import('./modales/modal-login').then((m) => ({ default: m.ModalLogin })));
const ModalPublicar = lazy(() => import('./modales/modal-publicar').then((m) => ({ default: m.ModalPublicar })));
const ChatVisitante = lazy(() => import('./chat/chat-visitante').then((m) => ({ default: m.ChatVisitante })));
import { ANCHO_PAGINA, CLASE_BORDE, CLASE_DIVISOR, CLASE_FONDO, POR_PAGINA, RELLENO_LATERAL_SITIO, RELLENO_VACIO, SOLO_ESCRITORIO_ANCHO } from './disenno';
import { FiltrosTipo } from './lista/filtros-tipo';
import { Paginacion } from './lista/paginacion';
import { PiePublico } from './pie-publico';
import { Presentacion } from './presentacion/presentacion';
import { enlaceWhatsApp, esEntornoLocal, mensajeGeneral } from '../../platform/whatsapp';

/* Página pública: composición de cabecera, buscador, filtros por tipo,
 * lista centrada de publicados y paginación. El estado (filtro, búsqueda
 * y página) vive aquí; el diseño vive en `disenno.ts`. */
export function PaginaPublica() {
  const { inmuebles, cargando, error } = usePublica();
  const detalle = useDetallePublico();
  const { filtro, pagina, setPagina, busqueda, elegirFiltro, alBuscar } = useFiltrosPagina();
  const [chatAbierto, setChatAbierto] = useState(false);
  /* El chat con IA solo funciona en local. En producción el botón
   * Mensaje (escritorio y menú móvil) abre WhatsApp con saludo listo. */
  const pedirContacto = () => {
    if (esEntornoLocal()) {
      setChatAbierto(true);
      return;
    }
    /* [08AA-23] Apertura vía plataforma (sin `window` directo aquí). */
    abrirExterna(enlaceWhatsApp(mensajeGeneral()));
  };
  /* [169A-2] El modal de publicar trae su propio estado + borrador: la
   * página solo lo abre y lo renderiza (sin más `useState` aquí). */
  const publicar = useModalPublicar();
  /* Filtros avanzados: hook propio con borrador + aplicados; la página
   * solo lo abre, lo renderiza y resetea la página al aplicar. */
  const avanzados = useFiltrosAvanzados();
  /* Login del panel: hook propio; la página solo lo abre y lo renderiza. */
  const acceso = useModalLogin();
  /* [249A-5] Pestillos de montaje: los overlays `lazy` montan en su primera
   * apertura (no en la carga: salen de la ruta crítica) y quedan montados
   * para conservar la animación de cierre. El estado vive en cada hook. */
  const detalleMontado = useMontarAlAbrir(detalle.seleccionado !== null);
  const publicarMontado = useMontarAlAbrir(publicar.abierto);
  const filtrosMontado = useMontarAlAbrir(avanzados.abierto);
  const accesoMontado = useMontarAlAbrir(acceso.abierto);
  const chatMontado = useMontarAlAbrir(chatAbierto);

  const indice = useMemo(() => construirIndice(inmuebles), [inmuebles]);
  const terminos = useMemo(() => extraerTerminos(busqueda), [busqueda]);
  const filtrados = filtrarIndice(indice, filtro, terminos, avanzados.aplicados);
  const visibles = filtrados;
  const totalPaginas = Math.max(1, Math.ceil(visibles.length / POR_PAGINA));
  const paginaSegura = Math.min(pagina, totalPaginas);
  const enPagina = visibles.slice((paginaSegura - 1) * POR_PAGINA, paginaSegura * POR_PAGINA);

  return (
    <main className={`flex min-h-dvh flex-col items-center justify-start ${CLASE_FONDO} font-soehne text-black pt-4 pb-16`}>
      {/* Presentación sencilla (concepto 2): titular a la izquierda en
        * flujo normal. La caja animada vive aparte en `presentacion-caja`. */}
      <Presentacion />
      <div className={`mx-auto w-full ${ANCHO_PAGINA} ${RELLENO_LATERAL_SITIO}`}>
        <CabeceraPublica
          alPedirChat={pedirContacto}
          alPedirPublicar={publicar.abrir}
          alPedirEntrar={acceso.abrir}
        />
      </div>
      <div className={`mx-auto w-full ${ANCHO_PAGINA} ${RELLENO_LATERAL_SITIO}`}>
        {/* Hoja con el contenido en flujo normal y sin scroll propio, sin
          * asistencia de snap (concepto 2): el descanso con snap a 200px
          * bajo la cabecera era del concepto 1 y vive en MARGEN_ANCLA_LISTA
          * por si vuelve a hacer falta. */}
        <div className={`relative z-10 ${CLASE_FONDO}`}>
        <BuscadorPublica
          valor={busqueda}
          alCambiar={alBuscar}
          alAbrirFiltros={avanzados.abrir}
          filtrosActivos={avanzados.hayActivos}
        />
        <div className={SOLO_ESCRITORIO_ANCHO}>
          <FiltrosTipo filtro={filtro} elegir={elegirFiltro} inmuebles={inmuebles} cargando={cargando} />
        </div>
        <div className={`w-full divide-y ${CLASE_DIVISOR} border border-t-0 ${CLASE_BORDE} px-0 py-0`}>
          {cargando ? (
            /* [249A-5] Esqueleto con la altura final en vez de `Cargando…`:
             * reserva el alto de la lista antes de que llegue la API. */
            <EsqueletoLista />
          ) : error ? (
            <p className={`border ${CLASE_BORDE} text-center ${RELLENO_VACIO}`}>{error}</p>
          ) : visibles.length === 0 ? (
            <p className={`text-center ${RELLENO_VACIO}`}>
              {inmuebles.length === 0
                ? 'Sin propiedades publicadas.'
                : busqueda.trim()
                  ? 'Sin resultados para esta búsqueda.'
                  : avanzados.hayActivos
                    ? 'Sin resultados para estos filtros.'
                    : 'Sin propiedades de este tipo.'}
            </p>
          ) : (
            enPagina.map((i, indice) => <CajaInmueble key={i.id} inmueble={i} alElegir={detalle.elegir} prioritaria={indice === 0} />)
          )}
        </div>
        {/* [249A-5] La paginación aparece al cargar (1→2 páginas con 11
          * inmuebles) y empujaba el pie: cargando reserva su alto exacto
          * (`mt-5` + botones `h-10`). */}
        {cargando ? (
          <div aria-hidden className="mt-5 flex h-10 w-full" />
        ) : (
          <Paginacion pagina={paginaSegura} totalPaginas={totalPaginas} irA={setPagina} />
        )}
        </div>
        <Suspense fallback={null}>
          {detalleMontado && <ModalDetallePublico inmueble={detalle.seleccionado} alCerrar={detalle.cerrar} />}
          {publicarMontado && <ModalPublicar modal={publicar} />}
          {filtrosMontado && <ModalFiltros modal={avanzados} alAplicar={() => setPagina(1)} tipo={filtro} alElegirTipo={elegirFiltro} />}
          {accesoMontado && <ModalLogin modal={acceso} />}
        </Suspense>
      </div>
      {/* [169A-1] Chat del visitante: ventana solo si lo abre el boton Mensaje. */}
      <Suspense fallback={null}>
        {chatMontado && <ChatVisitante abierto={chatAbierto} alCerrar={() => setChatAbierto(false)} />}
      </Suspense>
      {/* [189A-1] Pie simple: solo la línea de derechos. La suscripción
        * (backend + cliente + hook) queda aparcada hasta el panel. */}
      <PiePublico />
    </main>
  );
}
