import { CHECKLIST_ASK, COMUNES, type ExtrasAsk } from '@/domain/ficha-ask';
import type { TipoInmueble } from '@/domain/inmueble';
import { CLASE_SELECT, Etiqueta } from './campos-formulario';
import { Input } from '@/components/ui/input';

/* Sección Ficha /ask del modal admin: las mismas preguntas que la dueña
 * responde en /ask, editables aquí (pisan lo guardado; vacío = sin
 * responder). Sin tipo aún muestra solo las comunes. */

export function FichaFormulario(props: {
  tipo: TipoInmueble | '';
  extras: ExtrasAsk;
  minimo: string;
  alCambiarExtra: (clave: string, valor: string | number | boolean | null) => void;
  alCambiarMinimo: (valor: string) => void;
}) {
  const preguntas = props.tipo ? (CHECKLIST_ASK[props.tipo] ?? COMUNES) : COMUNES;
  return (
    <fieldset className="space-y-3 rounded-md border border-input p-3">
      <legend className="px-1 text-sm font-medium">Ficha /ask (respuestas de la dueña)</legend>
      {preguntas.map((p) => {
        if (p.clave === 'precio_minimo') {
          return (
            <div key={p.clave} className="space-y-1">
              <Etiqueta>{p.etiqueta} 🔒</Etiqueta>
              <Input
                value={props.minimo}
                onChange={(e) => props.alCambiarMinimo(e.target.value)}
                placeholder="0"
                inputMode="decimal"
              />
              <p className="text-xs text-muted-foreground">Privado: nadie lo ve en la página.</p>
            </div>
          );
        }
        const actual = props.extras[p.clave];
        if (p.tipo === 'si_no') {
          return (
            <div key={p.clave} className="space-y-1">
              <Etiqueta>{p.etiqueta}</Etiqueta>
              <select
                value={actual === true ? 'si' : actual === false ? 'no' : ''}
                onChange={(e) =>
                  props.alCambiarExtra(p.clave, e.target.value === 'si' ? true : e.target.value === 'no' ? false : null)
                }
                className={CLASE_SELECT}
              >
                <option value="">Sin responder</option>
                <option value="si">Sí</option>
                <option value="no">No</option>
              </select>
            </div>
          );
        }
        const esNumero = p.tipo === 'entero' || p.tipo === 'decimal';
        return (
          <div key={p.clave} className="space-y-1">
            <Etiqueta>
              {p.etiqueta}
              {p.privada ? ' 🔒' : ''}
              {p.unidad ? ` (${p.unidad})` : ''}
            </Etiqueta>
            <Input
              value={actual == null ? '' : String(actual)}
              onChange={(e) => {
                const t = e.target.value;
                if (!esNumero) {
                  props.alCambiarExtra(p.clave, t);
                  return;
                }
                if (!t.trim()) {
                  props.alCambiarExtra(p.clave, null);
                  return;
                }
                const n = Number(t.replace(',', '.'));
                if (Number.isFinite(n) && n >= 0) props.alCambiarExtra(p.clave, p.tipo === 'entero' ? Math.round(n) : n);
              }}
              placeholder={p.ayuda ?? ''}
              inputMode={esNumero ? 'decimal' : 'text'}
            />
          </div>
        );
      })}
    </fieldset>
  );
}
