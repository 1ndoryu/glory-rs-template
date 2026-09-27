import type { ReactNode } from 'react';
import { History } from 'lucide-react';
import { Button } from '@/components/ui/button';

/* Piezas compartidas de los formularios de inmueble: etiqueta y estilo de
 * selects. Extraído de `modal-inmueble` (Sentinel limite-lineas). */

export const CLASE_SELECT =
  'flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm capitalize focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30';

export function Etiqueta(props: { children: ReactNode; error?: string }) {
  return <span className="block text-sm font-medium">{props.children}</span>;
}

export function BannerBorrador(props: { alRestaurar: () => void; alEmpezarDeCero: () => void }) {
  return (
    <div className="flex flex-col gap-2 rounded-md border border-amber-300 bg-amber-50 p-3 text-sm sm:flex-row sm:items-center">
      <p className="flex flex-1 items-center gap-2">
        <History className="h-4 w-4 shrink-0" />
        Hay un borrador sin guardar de la última vez.
      </p>
      <div className="flex gap-2">
        <Button size="sm" onClick={props.alRestaurar}>
          Continuar
        </Button>
        <Button size="sm" variant="outline" onClick={props.alEmpezarDeCero}>
          Empezar de cero
        </Button>
      </div>
    </div>
  );
}
