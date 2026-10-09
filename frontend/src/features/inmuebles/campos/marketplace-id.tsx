import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { Etiqueta } from '../campos-formulario';

/* [09AA-24] Campo ID-aviso: extraído de `modal-inmueble` junto con
 * `CampoAliasTitulos` para mantener ese componente bajo el tope de
 * 300 líneas (regla `limite-lineas` de Sentinel). */
export function CampoMarketplaceId({
  valor,
  error,
  alCambiar,
}: {
  valor: string;
  error?: string;
  alCambiar: (valor: string) => void;
}) {
  return (
    <div className="space-y-1">
      <Etiqueta error={error}>ID aviso Marketplace</Etiqueta>
      <Input
        value={valor}
        onChange={(e) => alCambiar(e.target.value)}
        placeholder="1234567890 (/marketplace/item/<id>)"
        inputMode="numeric"
        aria-invalid={Boolean(error)}
        className={cn(error && 'border-destructive')}
      />
      {error ? (
        <p className="text-xs text-destructive">{error}</p>
      ) : (
        <p className="text-xs text-muted-foreground">Vincula este inmueble con su aviso para el borrador exacto (F7).</p>
      )}
    </div>
  );
}
