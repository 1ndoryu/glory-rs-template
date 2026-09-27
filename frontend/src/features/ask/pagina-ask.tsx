import { useAsk } from '../../hooks/ask/use-ask';
import { useLogin } from '../../hooks/sesion/use-login';
import { useSesion } from '../../hooks/sesion/use-sesion';
import {
  etiquetaUbicacion,
  progresoPasos,
  type ColumnasAsk,
  type PasoAsk,
  type ValorUbicacion,
} from '../../domain/pasos-ask';
import { ETIQUETAS_TIPO, formatearPrecio, portadaDe, type Inmueble } from '../../domain/inmueble';
import { EntradaPregunta } from './entrada-pregunta';
import { EntradaUbicacion } from './entrada-ubicacion';
import {
  CLASE_ACTIVO,
  CLASE_BORDE,
  CLASE_FONDO,
  CLASE_TEXTO,
  CLASE_TINTA,
  RELLENO_LATERAL_SITIO,
} from '../publica/disenno';

/* /ask (279A-3 + 279A-4): cuestionario privado para completar fichas,
 * misma línea visual que la página pública (fondo #e8e7e3, tinta, sin
 * redondeados ni sombras, Söhne; tokens de `publica/disenno`, nunca
 * literales). Exige sesión del panel (reutiliza `useSesion`+`useLogin`);
 * sin lista de propiedades: al entrar elige sola una con algo que
 * preguntar, en orden aleatorio. El % es solo aviso visual. */

export function PaginaAsk() {
  const { email, alEntrar } = useSesion();
  if (!email) return <EntradaAsk alEntrar={alEntrar} />;
  return <CuestionarioAsk />;
}

function EntradaAsk({ alEntrar }: { alEntrar: (email: string) => void }) {
  const login = useLogin(alEntrar);
  return (
    <main className={`flex min-h-dvh flex-col items-center justify-center ${CLASE_FONDO} py-16 font-soehne`}>
      <div className={`mx-auto w-full max-w-md ${RELLENO_LATERAL_SITIO}`}>
        <h1 className={`text-center text-xl ${CLASE_TINTA}`}>Completar ficha de propiedad</h1>
        <p className={`mt-1 text-center text-sm ${CLASE_TINTA} opacity-70`}>
          Entra con tu cuenta del panel para completar la ficha de las propiedades.
        </p>
        <form
          className="mt-4 flex w-full flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            void login.ejecutar('entrar');
          }}
        >
          <label className={`flex flex-col gap-1 text-sm ${CLASE_TINTA}`}>
            Correo
            <input
              type="email"
              value={login.email}
              onChange={(e) => login.setEmail(e.target.value)}
              placeholder="admin@ejemplo.com"
              className={`w-full rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-2 text-sm outline-none placeholder:text-black/40`}
            />
          </label>
          <label className={`flex flex-col gap-1 text-sm ${CLASE_TINTA}`}>
            Contraseña
            <input
              type="password"
              value={login.clave}
              onChange={(e) => login.setClave(e.target.value)}
              placeholder="Tu contraseña"
              className={`w-full rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-2 text-sm outline-none placeholder:text-black/40`}
            />
          </label>
          {login.error && (
            <p role="alert" className="text-sm text-red-800">
              {login.error}
            </p>
          )}
          <button
            type="submit"
            disabled={login.ocupado}
            className={`cursor-pointer rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-2 ${CLASE_TEXTO} disabled:cursor-wait disabled:opacity-60`}
          >
            {login.ocupado ? 'Entrando…' : 'Entrar'}
          </button>
        </form>
      </div>
    </main>
  );
}

function CuestionarioAsk() {
  const ask = useAsk();
  return (
    <main className={`flex min-h-dvh flex-col items-center justify-start ${CLASE_FONDO} pt-4 pb-16 font-soehne`}>
      <div className={`mx-auto w-full max-w-md text-center ${RELLENO_LATERAL_SITIO}`}>
        <h1 className={`text-xl ${CLASE_TINTA}`}>Completar ficha de propiedad</h1>
        {ask.error && (
          <p role="alert" className={`mt-2 border ${CLASE_BORDE} px-3 py-2 text-sm text-red-800`}>
            {ask.error}
          </p>
        )}
        {ask.cargando && !ask.seleccionado && !ask.terminado ? (
          <p className={`mt-4 text-sm ${CLASE_TINTA}`}>Buscando preguntas…</p>
        ) : ask.terminado ? (
          <PanelTerminado
            vacio={ask.inmuebles.length === 0}
            alRevisar={ask.siguiente}
          />
        ) : !ask.seleccionado ? (
          <p className={`mt-4 text-sm ${CLASE_TINTA}`}>Buscando preguntas…</p>
        ) : ask.cargando ? (
          <p className={`mt-4 text-sm ${CLASE_TINTA}`}>Cargando ficha…</p>
        ) : (
          <PreguntaActual
            inmueble={ask.seleccionado}
            pasos={ask.pasos}
            indice={ask.indice}
            extras={ask.ficha.extras}
            precioMinimo={ask.ficha.precioMinimo}
            guardando={ask.guardando}
            alResponder={(v) => void ask.responder(v)}
            alSaltar={ask.saltar}
            alAnterior={ask.anterior}
            alOtra={ask.siguiente}
          />
        )}
      </div>
    </main>
  );
}

/* Sin pendientes: todo al día (o aún sin propiedades). Sin lista: solo
 * el botón para revisar de nuevo. */
function PanelTerminado({ vacio, alRevisar }: { vacio: boolean; alRevisar: () => void }) {
  return (
    <div className={`mt-4 border ${CLASE_BORDE} px-4 py-6 text-center`}>
      <p className={`${CLASE_TINTA}`}>
        {vacio ? 'Aún no hay propiedades cargadas.' : '¡Todo al día! No quedan preguntas pendientes.'}
      </p>
      {!vacio && (
        <button
          type="button"
          onClick={alRevisar}
          className={`mt-3 cursor-pointer rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-2 text-sm ${CLASE_TEXTO}`}
        >
          Revisar de nuevo
        </button>
      )}
    </div>
  );
}

function PreguntaActual({
  inmueble,
  pasos,
  indice,
  extras,
  precioMinimo,
  guardando,
  alResponder,
  alSaltar,
  alAnterior,
  alOtra,
}: {
  inmueble: Inmueble;
  pasos: PasoAsk[];
  indice: number;
  extras: ReturnType<typeof useAsk>['ficha']['extras'];
  precioMinimo: number | null;
  guardando: boolean;
  alResponder: (v: string | number | boolean | null | ValorUbicacion) => void;
  alSaltar: () => void;
  alAnterior: () => void;
  alOtra: () => void;
}) {
  const columnas: ColumnasAsk = inmueble;
  const { porcentaje, faltan } = progresoPasos(pasos, columnas, extras, precioMinimo);
  const paso = pasos[indice];
  /* Valor ya guardado (para resaltar al deshacer y corregir). */
  const valorFicha = paso?.kind === 'ficha'
    ? paso.pregunta.clave === 'precio_minimo'
      ? precioMinimo
      : (extras[paso.pregunta.clave] ?? null)
    : null;
  const portada = portadaDe(inmueble);
  return (
    <div className="mt-4">
      {/* Cabecera: foto de portada + precio para situar la propiedad. */}
      {portada && <img src={portada} alt="" className={`mx-auto aspect-[4/3] w-full border ${CLASE_BORDE} object-cover`} />}
      <p className={`mt-2 text-sm ${CLASE_TINTA}`}>
        {inmueble.titulo || 'Sin título'} <span className="opacity-60">· {ETIQUETAS_TIPO[inmueble.tipo]}</span>
      </p>
      <p className={`mt-1 text-base ${CLASE_TINTA}`}>
        {inmueble.precio > 0 ? `${formatearPrecio(inmueble.precio)} · ${inmueble.operacion}` : 'Precio a consultar'}
      </p>
      {inmueble.descripcion.trim() && (
        <p className={`mt-1 text-xs ${CLASE_TINTA} opacity-70`}>{inmueble.descripcion}</p>
      )}
      {/* Progreso: aviso visual del % completado (nunca bloquea). */}
      <div className={`mt-2 h-2 w-full border ${CLASE_BORDE}`}>
        <div className={`${CLASE_ACTIVO} h-full`} style={{ width: `${porcentaje}%` }} />
      </div>
      <p className={`mt-1 text-xs ${CLASE_TINTA} opacity-70`}>Ficha al {porcentaje}%{faltan > 0 ? ` · faltan ${faltan}` : ' · completa'}</p>
      {paso?.kind === 'ficha' ? (
        <EntradaPregunta
          key={`ficha-${paso.pregunta.clave}-${String(valorFicha)}`}
          pregunta={paso.pregunta}
          guardando={guardando}
          conNoAplica
          valorActual={valorFicha ?? undefined}
          alResponder={alResponder}
          alSaltar={alSaltar}
        />
      ) : paso?.kind === 'ubicacion' ? (
        <EntradaUbicacion
          key={`ubicacion-${columnas.ubicacion}-${columnas.residencia}`}
          etiqueta={etiquetaUbicacion(columnas)}
          ubicacionActual={columnas.ubicacion}
          residenciaActual={columnas.residencia}
          guardando={guardando}
          alResponder={alResponder}
          alSaltar={alSaltar}
        />
      ) : paso?.kind === 'columna' ? (
        <EntradaPregunta
          key={`columna-${paso.columna}-${columnas[paso.columna]}`}
          pregunta={{
            clave: paso.columna,
            etiqueta: paso.etiqueta,
            tipo: paso.entero ? 'entero' : 'decimal',
            ayuda: paso.ayuda,
            unidad: paso.unidad,
            privada: false,
          }}
          guardando={guardando}
          conNoAplica={false}
          valorActual={columnas[paso.columna] > 0 ? columnas[paso.columna] : undefined}
          alResponder={alResponder}
          alSaltar={alSaltar}
        />
      ) : (
        <div className={`mt-4 border ${CLASE_BORDE} px-4 py-6 text-center`}>
          <p className={`${CLASE_TINTA}`}>¡Listo! Respondiste todas las preguntas de esta propiedad.</p>
          <p className={`mt-1 text-sm ${CLASE_TINTA} opacity-70`}>Ficha al {porcentaje}%.</p>
          <button
            type="button"
            onClick={alOtra}
            disabled={guardando}
            className={`mt-3 cursor-pointer rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-2 text-sm ${CLASE_TEXTO} disabled:cursor-wait disabled:opacity-60`}
          >
            Otra pregunta aleatoria →
          </button>
        </div>
      )}
      <div className="mt-4 flex justify-center gap-6">
        {indice > 0 && (
          <button type="button" onClick={alAnterior} className={`cursor-pointer text-sm ${CLASE_TINTA} underline`}>
            ← Anterior
          </button>
        )}
        <button
          type="button"
          onClick={alOtra}
          disabled={guardando}
          className={`cursor-pointer text-sm ${CLASE_TINTA} opacity-70 underline disabled:cursor-wait`}
        >
          Otra propiedad
        </button>
      </div>
    </div>
  );
}

/* [279A-3 F2] EntradaPregunta vive en `./entrada-pregunta.tsx` y
 * EntradaUbicacion en `./entrada-ubicacion.tsx` (límite de líneas). */
