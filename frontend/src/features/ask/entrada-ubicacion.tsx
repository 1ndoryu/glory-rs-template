import { useState } from 'react';
import type { ValorUbicacion } from '../../domain/ficha-ask';
import { CLASE_ACTIVO, CLASE_BORDE, CLASE_TEXTO, CLASE_TINTA } from '../publica/disenno';

/* Paso inteligente ubicación+residencia (279A-3 F2): una pregunta, dos
 * campos prellenados con lo que ya hay. Guardar rellena las columnas del
 * inmueble (no `extras`); lo que ya está bien se confirma sin reescribir. */

export function EntradaUbicacion({
  etiqueta,
  ubicacionActual,
  residenciaActual,
  guardando,
  alResponder,
  alSaltar,
}: {
  etiqueta: string;
  ubicacionActual: string;
  residenciaActual: string;
  guardando: boolean;
  alResponder: (v: ValorUbicacion) => void;
  alSaltar: () => void;
}) {
  const [ubicacion, setUbicacion] = useState(ubicacionActual);
  const [residencia, setResidencia] = useState(residenciaActual);
  const guardar = () => {
    if (ubicacion.trim() && residencia.trim()) alResponder({ ubicacion, residencia });
  };
  return (
    <div className={`mt-4 border ${CLASE_BORDE} px-4 py-6`}>
      <p className={`text-lg ${CLASE_TINTA}`}>{etiqueta}</p>
      <p className={`mt-1 text-xs ${CLASE_TINTA} opacity-60`}>
        Se guarda en la ubicación y residencia del inmueble (lo ve la página).
      </p>
      <div className="mt-4 flex flex-col gap-3">
        <label className={`flex flex-col gap-1 text-sm ${CLASE_TINTA}`}>
          Ubicación (zona)
          <input
            type="text"
            value={ubicacion}
            onChange={(e) => setUbicacion(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                guardar();
              }
            }}
            placeholder="p. ej. Puerto Ordaz, Castillito"
            className={`w-full rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-3 text-sm outline-none placeholder:text-black/40 ${CLASE_TINTA}`}
          />
        </label>
        <label className={`flex flex-col gap-1 text-sm ${CLASE_TINTA}`}>
          Residencia o conjunto
          <input
            type="text"
            value={residencia}
            onChange={(e) => setResidencia(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                guardar();
              }
            }}
            placeholder="p. ej. Res. Orinokia, Urb. Villa Brasil"
            className={`w-full rounded-none border ${CLASE_BORDE} bg-transparent px-3 py-3 text-sm outline-none placeholder:text-black/40 ${CLASE_TINTA}`}
          />
        </label>
        <button
          type="button"
          disabled={guardando}
          onClick={guardar}
          className={`cursor-pointer rounded-none border ${CLASE_BORDE} ${CLASE_ACTIVO} px-4 py-2 ${CLASE_TEXTO} disabled:cursor-wait disabled:opacity-60`}
        >
          {guardando ? 'Guardando…' : 'Guardar y seguir'}
        </button>
      </div>
      <div className="mt-3 flex gap-4">
        <button type="button" onClick={alSaltar} className={`cursor-pointer text-sm ${CLASE_TINTA} underline`}>
          Saltar por ahora
        </button>
      </div>
    </div>
  );
}
