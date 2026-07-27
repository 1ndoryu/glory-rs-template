/* [044A-11] Diálogo de ficha del cliente — extraído de ListaReservas para
 * cumplir límite de 300 líneas. Incluye botón para editar el cliente
 * directamente desde la vista de reservas. */

import { useState } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Pencil } from 'lucide-react';
import { Cliente } from '../api/generated';
import FormularioCliente from './FormularioCliente';

interface Props {
  clienteId: string | null;
  onClose: () => void;
  clienteDetalle: Cliente | undefined;
  clienteCargando: boolean;
}

function DialogoFichaCliente({ clienteId, onClose, clienteDetalle, clienteCargando }: Props) {
  const [editando, setEditando] = useState(false);

  const cerrar = () => {
    onClose();
    setEditando(false);
  };

  return (
    <Dialog open={!!clienteId} onOpenChange={(open) => { if (!open) cerrar(); }}>
      <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{editando ? 'Editar Cliente' : 'Ficha del Cliente'}</DialogTitle>
        </DialogHeader>
        {editando && clienteDetalle ? (
          <FormularioCliente
            cliente={clienteDetalle}
            onExito={cerrar}
          />
        ) : clienteCargando ? (
          <p className="text-sm text-muted-foreground">Cargando cliente...</p>
        ) : clienteDetalle ? (
          <div className="flex flex-col gap-3 text-sm">
            <div className="grid grid-cols-2 gap-2">
              <div>
                <span className="text-muted-foreground">Nombre</span>
                <p className="font-medium">{clienteDetalle.nombre} {clienteDetalle.apellidos}</p>
              </div>
              <div>
                <span className="text-muted-foreground">Teléfono</span>
                <p className="font-medium">{clienteDetalle.prefijo_telefono ? `${clienteDetalle.prefijo_telefono} ` : ''}{clienteDetalle.telefono || '—'}</p>
              </div>
            </div>
            <div className="grid grid-cols-2 gap-2">
              <div>
                <span className="text-muted-foreground">Email</span>
                <p className="font-medium">{clienteDetalle.email || '—'}</p>
              </div>
              <div>
                <span className="text-muted-foreground">Empresa</span>
                <p className="font-medium">{clienteDetalle.empresa || '—'}</p>
              </div>
            </div>
            {clienteDetalle.alergias && (
              <div>
                <span className="text-muted-foreground">Alergias</span>
                <p className="font-medium">{clienteDetalle.alergias}</p>
              </div>
            )}
            {(clienteDetalle.preferencias_ubicacion || clienteDetalle.preferencias_bebida) && (
              <div className="grid grid-cols-2 gap-2">
                {clienteDetalle.preferencias_ubicacion && (
                  <div>
                    <span className="text-muted-foreground">Pref. ubicación</span>
                    <p className="font-medium">{clienteDetalle.preferencias_ubicacion}</p>
                  </div>
                )}
                {clienteDetalle.preferencias_bebida && (
                  <div>
                    <span className="text-muted-foreground">Pref. bebida</span>
                    <p className="font-medium">{clienteDetalle.preferencias_bebida}</p>
                  </div>
                )}
              </div>
            )}
            {clienteDetalle.notas && (
              <div>
                <span className="text-muted-foreground">Notas</span>
                <p className="font-medium">{clienteDetalle.notas}</p>
              </div>
            )}
            <Button variant="outline" className="self-end mt-2" onClick={() => setEditando(true)}>
              <Pencil className="size-4 mr-1.5" /> Editar cliente
            </Button>
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">No se encontró el cliente</p>
        )}
      </DialogContent>
    </Dialog>
  );
}

export default DialogoFichaCliente;
