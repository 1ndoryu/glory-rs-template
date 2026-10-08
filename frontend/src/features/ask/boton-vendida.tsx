import { CLASE_TINTA } from '../publica/disenno';
import type { EstadoInmueble, Operacion } from '@/domain/inmueble';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { useVendida } from '@/hooks/ask/use-vendida';

interface Props {
  inmuebleId: string;
  titulo: string;
  operacion: Operacion;
  deshabilitado?: boolean;
  alMarcar: (id: string, estado: EstadoInmueble) => Promise<boolean>;
}

/* [08AA-33] "Esta propiedad se vendió" (al lado de "No aplica", solo
 * ficha): el estado final sigue la operación (venta→vendido,
 * alquiler→alquilado) y el backend despublica en la misma llamada. El
 * modal pequeño centrado es la doble confirmación anti-toques; al
 * confirmar, el padre avanza a la siguiente propiedad. */
export function BotonVendida({ inmuebleId, titulo, operacion, deshabilitado, alMarcar }: Props) {
  const { abierto, marcando, error, abrir, cerrar, confirmar } = useVendida(alMarcar);
  const estadoFinal: EstadoInmueble = operacion === 'alquiler' ? 'alquilado' : 'vendido';
  const etiqueta = operacion === 'alquiler' ? 'alquilada' : 'vendida';

  return (
    <>
      <button
        type="button"
        disabled={deshabilitado}
        onClick={abrir}
        className={`cursor-pointer text-sm ${CLASE_TINTA} underline disabled:cursor-wait disabled:opacity-60`}
      >
        Esta propiedad se vendió
      </button>
      <Dialog
        open={abierto}
        onOpenChange={(o) => {
          if (!o && !marcando) cerrar();
        }}
      >
        {/* Sin `className`: el `DialogContent` del sistema ya es pequeño
            (`sm:max-w-sm`) y centrado, lo que pide el diseño. */}
        <DialogContent showCloseButton={!marcando}>
          <DialogHeader>
            <DialogTitle>¿Marcar como {etiqueta}?</DialogTitle>
            <DialogDescription>
              «{titulo}» pasará a {estadoFinal} y dejará de mostrarse en la página. Se puede revertir
              desde el panel.
            </DialogDescription>
          </DialogHeader>
          {error && (
            <p role="alert" className="text-sm text-red-800">
              {error}
            </p>
          )}
          <DialogFooter>
            <Button variant="outline" disabled={marcando} onClick={cerrar}>
              Cancelar
            </Button>
            <Button variant="destructive" disabled={marcando} onClick={() => void confirmar(inmuebleId, estadoFinal)}>
              {marcando ? 'Marcando…' : `Sí, marcar ${etiqueta}`}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
