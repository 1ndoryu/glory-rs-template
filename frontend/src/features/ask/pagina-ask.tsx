import { useAsk } from '../../hooks/ask/use-ask';
import { useLogin } from '../../hooks/sesion/use-login';
import { useSesion } from '../../hooks/sesion/use-sesion';
import { calcularCompletitud } from '../../domain/ficha-ask';
import {
  etiquetaUbicacion,
  progresoPasos,
  type ColumnasAsk,
  type PasoAsk,
  type ValorUbicacion,
} from '../../domain/pasos-ask';
import { ETIQUETAS_TIPO, portadaDe, type Inmueble } from '../../domain/inmueble';
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

/* /ask (279A-3): cuestionario privado de la dueña, misma línea visual que
 * la página pública (fondo #e8e7e3, tinta, sin redondeados ni sombras,
 * Söhne; tokens de `publica/disenno`, nunca literales). Exige sesión del
 * panel (reutiliza `useSesion`+`useLogin`); el % es solo aviso visual. */

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
          Entra con tu cuenta del panel para responder las preguntas de tus propiedades.
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
        {ask.cargando && !ask.seleccionado ? (
          <p className={`mt-4 text-sm ${CLASE_TINTA}`}>Cargando propiedades…</p>
        ) : !ask.seleccionado ? (
          <ListaPropiedades
            inmuebles={ask.inmuebles}
            alElegir={(id) => {
              const encontrado = ask.inmuebles.find((i) => i.id === id);
              if (encontrado) void ask.elegir(encontrado);
            }}
          />
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
            alVolver={ask.volver}
          />
        )}
      </div>
    </main>
  );
}

function ListaPropiedades({
  inmuebles,
  alElegir,
}: {
  inmuebles: ReturnType<typeof useAsk>['inmuebles'];
  alElegir: (id: string) => void;
}) {
  if (inmuebles.length === 0) return <p className={`mt-4 text-sm ${CLASE_TINTA}`}>Aún no hay propiedades cargadas.</p>;
  return (
    <ul className={`mt-4 flex flex-col gap-2`}>
      {inmuebles.map((i) => (
        <li key={i.id}>
          <button
            type="button"
            onClick={() => alElegir(i.id)}
            className={`flex w-full cursor-pointer items-center justify-between gap-3 rounded-none border ${CLASE_BORDE} bg-transparent px-4 py-3 text-left ${CLASE_TINTA} hover:bg-[#dddbd5]`}
          >
            <span className="text-sm">
              {i.titulo || 'Sin título'} <span className="opacity-60">· {ETIQUETAS_TIPO[i.tipo]}</span>
            </span>
            <span className={`text-xs ${CLASE_TINTA} opacity-70`}>
              {(() => {
                const { porcentaje } = calcularCompletitud(i.tipo, i.extras ?? {}, i.precioMinimo ?? null, i);
                return porcentaje === 100 ? 'Completa ✓' : porcentaje === 0 ? 'Sin empezar' : `${porcentaje}% · continuar`;
              })()}
            </span>
          </button>
        </li>
      ))}
    </ul>
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
  alVolver,
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
  alVolver: () => void;
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
      {/* Cabecera: foto de portada + descripción para situar la propiedad. */}
      {portada && <img src={portada} alt="" className={`mx-auto aspect-[4/3] w-full border ${CLASE_BORDE} object-cover`} />}
      <p className={`mt-2 text-sm ${CLASE_TINTA}`}>
        {inmueble.titulo || 'Sin título'} <span className="opacity-60">· {ETIQUETAS_TIPO[inmueble.tipo]}</span>
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
        </div>
      )}
      <div className="mt-4 flex justify-center gap-6">
        {indice > 0 && (
          <button type="button" onClick={alAnterior} className={`cursor-pointer text-sm ${CLASE_TINTA} underline`}>
            ← Anterior
          </button>
        )}
        <button type="button" onClick={alVolver} className={`cursor-pointer text-sm ${CLASE_TINTA} opacity-70 underline`}>
          Volver a mis propiedades
        </button>
      </div>
    </div>
  );
}

/* [279A-3 F2] EntradaPregunta vive en `./entrada-pregunta.tsx` y
 * EntradaUbicacion en `./entrada-ubicacion.tsx` (límite de líneas). */
