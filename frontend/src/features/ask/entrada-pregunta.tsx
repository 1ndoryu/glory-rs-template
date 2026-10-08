import { useState } from 'react';
import type { PreguntaAsk } from '../../domain/ficha-ask';
import { NO_SE } from '../../domain/ficha-ask';
import { assertNunca } from '../../domain/pasos-ask';
import { CLASE_ACTIVO, CLASE_BORDE, CLASE_TEXTO, CLASE_TINTA } from '../publica/disenno';

/* Entrada de una pregunta /ask (279A-3 F2 + 279A-7): Sí/No, opciones fijas
 * (amoblado, agua…), o campo de texto/número con unidad, stepper y error
 * visible. «No lo sé» (279A-7) es respuesta válida en TODAS: se guarda
 * `NO_SE`, no vuelve a preguntarse y el dato queda vacío — por eso ya no
 * salta como antes. `conNoAplica` solo en ficha (`extras`); las columnas
 * guardan su «no sé» en la marca `*_nose` (ver `marcaNoSePaso`).
 * `valorActual` resalta lo ya respondido al deshacer (anterior). */

export function EntradaPregunta({
  pregunta,
  guardando,
  conNoAplica,
  conNoSe,
  valorActual,
  alResponder,
  alSaltar,
}: {
  pregunta: PreguntaAsk;
  guardando: boolean;
  conNoAplica: boolean;
  conNoSe: boolean;
  valorActual?: string | number | boolean | null;
  alResponder: (v: string | number | boolean | null) => void;
  alSaltar: () => void;
}) {
  const [texto, setTexto] = useState(
    typeof valorActual === 'string' && valorActual !== NO_SE
      ? valorActual
      : typeof valorActual === 'number'
        ? String(valorActual)
        : '',
  );
  const [error, setError] = useState('');

  const numeroValido = (): number | null => {
    const n = Number(texto.replace(',', '.'));
    if (!texto.trim() || !Number.isFinite(n) || n < 0) return null;
    return pregunta.tipo === 'entero' ? Math.round(n) : n;
  };

  const contestarNumero = () => {
    const n = numeroValido();
    if (n === null) {
      setError('Escribe un número válido (0 o más).');
      return;
    }
    setError('');
    alResponder(n);
  };

  const moverPaso = (delta: number) => {
    const base = numeroValido() ?? (typeof valorActual === 'number' ? valorActual : 0);
    const siguiente = Math.max(0, Math.round(base + delta));
    setTexto(String(siguiente));
    setError('');
  };

  /* Botón de opción (Sí/No, amoblado…): borde sin fondo, como los demás;
   * activo resalta lo ya respondido. «No lo sé» nunca lleva fondo, ni
   * seleccionado: se marca con negrita + subrayado. */
  const botonOpcion = (etiqueta: string, valor: string | boolean) => {
    const activo = valorActual === valor;
    const esNoSe = valor === NO_SE;
    return (
      <button
        type="button"
        disabled={guardando}
        aria-pressed={activo}
        onClick={() => alResponder(valor)}
        className={`flex-1 cursor-pointer rounded-none border px-4 py-3 disabled:cursor-wait disabled:opacity-60 ${
          activo && !esNoSe
            ? `${CLASE_ACTIVO} ${CLASE_TEXTO} border-transparent`
            : `${CLASE_BORDE} bg-transparent ${CLASE_TINTA}${activo ? ' font-semibold underline underline-offset-4' : ''}`
        }`}
      >
        {etiqueta}
      </button>
    );
  };

  return (
    <div className={`mt-4 border ${CLASE_BORDE} px-4 py-6 text-center`}>
      <p className={`text-lg ${CLASE_TINTA}`}>{pregunta.etiqueta}</p>
      {pregunta.privada && <p className={`mt-1 text-xs ${CLASE_TINTA} opacity-60`}>Privado: nadie lo ve en la página.</p>}
      {pregunta.ayuda && <p className={`mt-1 text-xs ${CLASE_TINTA} opacity-60`}>{pregunta.ayuda}</p>}
      {pregunta.tipo === 'si_no' ? (
        <div className="mt-4 flex gap-2">
          {botonOpcion('Sí', true)}
          {botonOpcion('No', false)}
          {botonOpcion('No lo sé', NO_SE)}
        </div>
      ) : pregunta.tipo === 'opciones' ? (
        <div className="mt-4 grid grid-cols-2 gap-2">
          {(pregunta.opciones ?? []).map((o) => (
            <span key={String(o.valor)} className="flex">
              {botonOpcion(o.etiqueta, o.valor)}
            </span>
          ))}
        </div>
      ) : pregunta.tipo === 'texto_corto' || pregunta.tipo === 'entero' || pregunta.tipo === 'decimal' ? (
        <div className="mt-4 flex flex-col gap-3">
          <div className="flex items-stretch gap-2">
            {pregunta.tipo === 'entero' && (
              <button
                type="button"
                onClick={() => moverPaso(-1)}
                aria-label="Quitar uno"
                className={`cursor-pointer rounded-none border ${CLASE_BORDE} bg-transparent px-4 text-lg ${CLASE_TINTA}`}
              >
                −
              </button>
            )}
            <input
              type="text"
              value={texto}
              inputMode={pregunta.tipo === 'texto_corto' ? 'text' : 'decimal'}
              onChange={(e) => {
                setTexto(e.target.value);
                setError('');
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  if (pregunta.tipo === 'texto_corto') texto.trim() && alResponder(texto);
                  else contestarNumero();
                }
              }}
              placeholder={pregunta.tipo === 'texto_corto' ? 'Escribe tu respuesta…' : '0'}
              className={`min-w-0 flex-1 rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-3 text-center text-sm outline-none placeholder:text-black/40 ${CLASE_TINTA}`}
            />
            {pregunta.tipo === 'entero' && (
              <button
                type="button"
                onClick={() => moverPaso(1)}
                aria-label="Añadir uno"
                className={`cursor-pointer rounded-none border ${CLASE_BORDE} bg-transparent px-4 text-lg ${CLASE_TINTA}`}
              >
                +
              </button>
            )}
            {pregunta.unidad && <span className={`self-center text-sm ${CLASE_TINTA} opacity-70`}>{pregunta.unidad}</span>}
          </div>
          {error && <p className="text-sm text-red-700">{error}</p>}
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
          {conNoSe && (
            <button
              type="button"
              disabled={guardando}
              aria-pressed={valorActual === NO_SE}
              onClick={() => alResponder(NO_SE)}
              className={`cursor-pointer rounded-none border ${CLASE_BORDE} bg-transparent px-4 py-2 disabled:cursor-wait disabled:opacity-60 ${CLASE_TINTA}${
                valorActual === NO_SE ? ' font-semibold underline underline-offset-4' : ''
              }`}
            >
              No lo sé
            </button>
          )}
        </div>
      ) : (
        assertNunca(pregunta.tipo)
      )}
      <div className="mt-3 flex justify-center gap-4">
        {pregunta.tipo !== 'si_no' && pregunta.tipo !== 'opciones' && (
          <button type="button" onClick={alSaltar} className={`cursor-pointer text-sm ${CLASE_TINTA} underline`}>
            Saltar por ahora
          </button>
        )}
        {conNoAplica && (
          <button type="button" onClick={() => alResponder(null)} className={`cursor-pointer text-sm ${CLASE_TINTA} opacity-60 underline`}>
            No aplica
          </button>
        )}
      </div>
    </div>
  );
}
