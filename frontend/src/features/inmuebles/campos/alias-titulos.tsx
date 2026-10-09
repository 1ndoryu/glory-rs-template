import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/lib/utils';
import { Etiqueta } from '../campos-formulario';

/* [09AA-24] Campo de alias del aviso: vive aquí y no en `modal-inmueble`
 * para no empujar ese componente por encima del tope de 300 líneas
 * (regla `limite-lineas` de Sentinel). Un nombre por línea, máx 10. */
export function CampoAliasTitulos({
  valor,
  error,
  alCambiar,
}: {
  valor: string;
  error?: string;
  alCambiar: (valor: string) => void;
}) {
  return (
    <div className="space-y-1 sm:col-span-2">
      <Etiqueta error={error}>Otros nombres del aviso</Etiqueta>
      <Textarea
        value={valor}
        onChange={(e) => alCambiar(e.target.value)}
        placeholder={'Un nombre por línea, p. ej.\nApartamento en Río Aro Plaza'}
        aria-invalid={Boolean(error)}
        className={cn(error && 'border-destructive')}
      />
      {error ? (
        <p className="text-xs text-destructive">{error}</p>
      ) : (
        <p className="text-xs text-muted-foreground">Si el aviso se publica con otro nombre, el hilo lo detecta igual.</p>
      )}
    </div>
  );
}
