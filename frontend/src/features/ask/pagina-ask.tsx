import { useState } from 'react';
import { useAsk } from '../../hooks/ask/use-ask';
import { useLogin } from '../../hooks/sesion/use-login';
import { useSesion } from '../../hooks/sesion/use-sesion';
import { CHECKLIST_ASK, calcularCompletitud, type PreguntaAsk } from '../../domain/ficha-ask';
import { ETIQUETAS_TIPO } from '../../domain/inmueble';
import {
  ANCHO_PAGINA,
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
      <div className={`mx-auto w-full ${ANCHO_PAGINA} ${RELLENO_LATERAL_SITIO}`}>
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
            titulo={ask.seleccionado.titulo || 'Sin título'}
            tipoEtiqueta={ETIQUETAS_TIPO[ask.seleccionado.tipo]}
            preguntas={CHECKLIST_ASK[ask.seleccionado.tipo] ?? []}
            indice={ask.indice}
            extras={ask.ficha.extras}
            precioMinimo={ask.ficha.precioMinimo}
            tipo={ask.seleccionado.tipo}
            guardando={ask.guardando}
            alResponder={(v) => void ask.responder(v)}
            alSaltar={ask.saltar}
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
                const { porcentaje } = calcularCompletitud(i.tipo, i.extras ?? {}, i.precioMinimo ?? null);
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
  titulo,
  tipoEtiqueta,
  preguntas,
  indice,
  extras,
  precioMinimo,
  tipo,
  guardando,
  alResponder,
  alSaltar,
  alVolver,
}: {
  titulo: string;
  tipoEtiqueta: string;
  preguntas: PreguntaAsk[];
  indice: number;
  extras: ReturnType<typeof useAsk>['ficha']['extras'];
  precioMinimo: number | null;
  tipo: Parameters<typeof calcularCompletitud>[0];
  guardando: boolean;
  alResponder: (v: string | number | boolean | null) => void;
  alSaltar: () => void;
  alVolver: () => void;
}) {
  const { porcentaje, faltan } = calcularCompletitud(tipo, extras, precioMinimo);
  const pregunta = preguntas[indice];
  return (
    <div className="mt-4">
      <p className={`text-sm ${CLASE_TINTA} opacity-70`}>
        {titulo} · {tipoEtiqueta}
      </p>
      {/* Progreso: aviso visual del % completado (nunca bloquea). */}
      <div className={`mt-2 h-2 w-full border ${CLASE_BORDE}`}>
        <div className={`${CLASE_ACTIVO} h-full`} style={{ width: `${porcentaje}%` }} />
      </div>
      <p className={`mt-1 text-xs ${CLASE_TINTA} opacity-70`}>Ficha al {porcentaje}%{faltan.length > 0 ? ` · faltan ${faltan.length}` : ' · completa'}</p>
      {pregunta ? (
        <EntradaPregunta key={pregunta.clave} pregunta={pregunta} guardando={guardando} alResponder={alResponder} alSaltar={alSaltar} />
      ) : (
        <div className={`mt-4 border ${CLASE_BORDE} px-4 py-6 text-center`}>
          <p className={`${CLASE_TINTA}`}>¡Listo! Respondiste todas las preguntas de esta propiedad.</p>
          <p className={`mt-1 text-sm ${CLASE_TINTA} opacity-70`}>Ficha al {porcentaje}%.</p>
        </div>
      )}
      <button type="button" onClick={alVolver} className={`mt-4 cursor-pointer text-sm ${CLASE_TINTA} underline`}>
        ← Volver a mis propiedades
      </button>
    </div>
  );
}

function EntradaPregunta({
  pregunta,
  guardando,
  alResponder,
  alSaltar,
}: {
  pregunta: PreguntaAsk;
  guardando: boolean;
  alResponder: (v: string | number | boolean | null) => void;
  alSaltar: () => void;
}) {
  const [texto, setTexto] = useState('');
  const contestarNumero = () => {
    const n = Number(texto.replace(',', '.'));
    if (texto.trim() && Number.isFinite(n) && n >= 0) alResponder(n);
  };
  return (
    <div className={`mt-4 border ${CLASE_BORDE} px-4 py-6`}>
      <p className={`text-lg ${CLASE_TINTA}`}>{pregunta.etiqueta}</p>
      {pregunta.privada && <p className={`mt-1 text-xs ${CLASE_TINTA} opacity-60`}>🔒 Privado: nadie lo ve en la página.</p>}
      {pregunta.ayuda && <p className={`mt-1 text-xs ${CLASE_TINTA} opacity-60`}>{pregunta.ayuda}</p>}
      {pregunta.tipo === 'si_no' ? (
        <div className="mt-4 flex gap-3">
          <button
            type="button"
            disabled={guardando}
            onClick={() => alResponder(true)}
            className={`flex-1 cursor-pointer rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-3 ${CLASE_TEXTO} disabled:cursor-wait disabled:opacity-60`}
          >
            Sí
          </button>
          <button
            type="button"
            disabled={guardando}
            onClick={() => alResponder(false)}
            className={`flex-1 cursor-pointer rounded-none border ${CLASE_BORDE} bg-transparent px-4 py-3 ${CLASE_TEXTO} ${CLASE_TINTA} disabled:cursor-wait disabled:opacity-60`}
          >
            No
          </button>
        </div>
      ) : (
        <div className="mt-4 flex flex-col gap-3">
          <input
            type={pregunta.tipo === 'texto_corto' ? 'text' : 'number'}
            value={texto}
            min={pregunta.tipo === 'texto_corto' ? undefined : 0}
            onChange={(e) => setTexto(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                if (pregunta.tipo === 'texto_corto') texto.trim() && alResponder(texto);
                else contestarNumero();
              }
            }}
            placeholder={pregunta.tipo === 'texto_corto' ? 'Escribe tu respuesta…' : '0'}
            className={`w-full rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-3 text-sm outline-none placeholder:text-black/40 ${CLASE_TINTA}`}
          />
          <button
            type="button"
            disabled={guardando}
            onClick={() => {
              if (pregunta.tipo === 'texto_corto') texto.trim() && alResponder(texto);
              else contestarNumero();
            }}
            className={`cursor-pointer rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-2 ${CLASE_TEXTO} disabled:cursor-wait disabled:opacity-60`}
          >
            {guardando ? 'Guardando…' : 'Guardar y seguir'}
          </button>
        </div>
      )}
      <div className="mt-3 flex gap-4">
        <button type="button" onClick={alSaltar} className={`cursor-pointer text-sm ${CLASE_TINTA} underline`}>
          Saltar por ahora
        </button>
        <button type="button" onClick={() => alResponder(null)} className={`cursor-pointer text-sm ${CLASE_TINTA} opacity-60 underline`}>
          No aplica
        </button>
      </div>
    </div>
  );
}
