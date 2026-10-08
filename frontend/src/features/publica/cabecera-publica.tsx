import { useState } from 'react';
import { Menu, MessageCircle, Plus, UserRound, X } from 'lucide-react';
import {
  ANCHO_PAGINA,
  CABECERA_FIJA,
  CLASE_ACTIVO,
  CLASE_BORDE,
  CLASE_FONDO,
  CLASE_REPOSO,
  CLASE_TEXTO,
  OPCION_MENU_MOVIL,
  PANEL_MENU_MOVIL,
  SOLO_ESCRITORIO_BLOQUE,
  SOLO_ESCRITORIO_FLEX,
  SOLO_MOVIL,
} from './disenno';

/* Cabecera en cuadro con bordes, apenas separada de los filtros.
 * Fija al viewport con 16px de aire al borde: se mantiene visible al
 * bajar y la foto la cubre por detrás arriba de la vista. La hoja con la
 * lista se ancla 200px debajo al subir.
 * En móvil (<md) muestra solo el logo sin letras y un botón de
 * hamburguesa que abre el mismo menú (Mensaje, Publicar, Entrar); en
 * escritorio (md+) van el nombre y los tres botones directos.
 * El boton Mensaje abre el chat del visitante (lo manda la pagina);
 * Publicar abre el modal de solicitud (también lo manda la pagina).
 * El icono de entrar abre el login del panel, sin texto. */
export function CabeceraPublica({
  alPedirChat,
  alPedirPublicar,
  alPedirEntrar,
}: {
  alPedirChat: () => void;
  alPedirPublicar: () => void;
  alPedirEntrar: () => void;
}) {
  const [menuAbierto, setMenuAbierto] = useState(false);

  function elegir(accion: () => void) {
    setMenuAbierto(false);
    accion();
  }

  return (
    <header className={`${CABECERA_FIJA} ${ANCHO_PAGINA} flex items-center justify-between rounded-none border ${CLASE_BORDE} ${CLASE_FONDO} px-4 py-3`}>
      <div className="flex items-center gap-2">
        <img
          src="/img/logo-mn.svg"
          alt=""
          aria-hidden="true"
          className="h-[30px] w-[30px]"
        />
        <p className={`m-0 ${SOLO_ESCRITORIO_BLOQUE} ${CLASE_TEXTO}`}>MN Inmobiliaria</p>
      </div>
      <div className={`shrink-0 gap-2 ${SOLO_ESCRITORIO_FLEX}`}>
        <button
          type="button"
          aria-label="Enviar mensaje"
          onClick={alPedirChat}
          className={`flex cursor-pointer items-center gap-2 rounded-none border ${CLASE_BORDE} ${CLASE_REPOSO} px-4 py-2 ${CLASE_TEXTO}`}
        >
          <MessageCircle className="h-4 w-4" />
          Mensaje
        </button>
        <button
          type="button"
          aria-label="Publicar mi inmueble"
          onClick={alPedirPublicar}
          className={`flex cursor-pointer items-center gap-2 rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-2 ${CLASE_TEXTO}`}
        >
          <Plus className="h-4 w-4" />
          Publicar mi inmueble
        </button>
        <button
          type="button"
          aria-label="Entrar al panel"
          title="Entrar al panel"
          onClick={alPedirEntrar}
          className={`flex cursor-pointer items-center rounded-none border ${CLASE_BORDE} ${CLASE_REPOSO} px-3 py-2 ${CLASE_TEXTO}`}
        >
          <UserRound className="h-4 w-4" />
        </button>
      </div>
      <button
        type="button"
        aria-label={menuAbierto ? 'Cerrar menú' : 'Abrir menú'}
        aria-expanded={menuAbierto}
        aria-controls="menu-movil"
        onClick={() => setMenuAbierto((v) => !v)}
        className={`flex cursor-pointer items-center rounded-none border ${CLASE_BORDE} ${CLASE_REPOSO} px-3 py-2 ${CLASE_TEXTO} ${SOLO_MOVIL}`}
      >
        {menuAbierto ? <X className="h-4 w-4" /> : <Menu className="h-4 w-4" />}
      </button>
      {menuAbierto && (
        <nav id="menu-movil" aria-label="Menú" className={PANEL_MENU_MOVIL}>
          <button
            type="button"
            onClick={() => elegir(alPedirChat)}
            className={`${OPCION_MENU_MOVIL} ${CLASE_TEXTO}`}
          >
            <MessageCircle className="h-4 w-4" />
            Mensaje
          </button>
          <button
            type="button"
            onClick={() => elegir(alPedirPublicar)}
            className={`${OPCION_MENU_MOVIL} ${CLASE_TEXTO}`}
          >
            <Plus className="h-4 w-4" />
            Publicar mi inmueble
          </button>
          <button
            type="button"
            onClick={() => elegir(alPedirEntrar)}
            className={`${OPCION_MENU_MOVIL} ${CLASE_TEXTO}`}
          >
            <UserRound className="h-4 w-4" />
            Entrar al panel
          </button>
        </nav>
      )}
    </header>
  );
}
