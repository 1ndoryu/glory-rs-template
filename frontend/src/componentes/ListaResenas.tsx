/* [134A-32] Página de gestión de reseñas — review gating.
 * Lista admin de reseñas recibidas + botón para solicitar nueva reseña.
 * Las reseñas con 4-5 estrellas muestran botón "Enviar a Google" que abre
 * la URL de Google Reviews configurada en Configuración.
 * Al solicitar una reseña se muestra el enlace generado para copiar/compartir. */

import { useState } from 'react';
import { useResenas } from '../hooks/useResenas';
import { useConfiguracion } from '../hooks/useConfiguracion';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Star, Send, ChevronLeft, ChevronRight, ExternalLink, Copy } from 'lucide-react';
import { toast } from 'sonner';

function Estrellas({ puntuacion }: { puntuacion: number | null | undefined }) {
  if (!puntuacion) return <span className="text-muted-foreground text-sm">Sin respuesta</span>;
  return (
    <div className="flex gap-0.5">
      {[1, 2, 3, 4, 5].map((n) => (
        <Star
          key={n}
          className={`size-4 ${n <= puntuacion ? 'fill-yellow-400 text-yellow-400' : 'text-muted-foreground'}`}
        />
      ))}
    </div>
  );
}

export default function ListaResenas() {
  const {
    resenas,
    total,
    page,
    totalPages,
    isLoading,
    setPage,
    solicitarResena,
    solicitando,
    ultimaUrlGenerada,
  } = useResenas();

  const { config } = useConfiguracion();
  const googleUrl = config.google_review_url;

  const [modalSolicitar, setModalSolicitar] = useState(false);

  const handleSolicitar = () => {
    solicitarResena();
  };

  const copiarEnlace = () => {
    if (ultimaUrlGenerada) {
      navigator.clipboard.writeText(ultimaUrlGenerada);
      toast.success('Enlace copiado al portapapeles');
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">{total} reseña{total !== 1 ? 's' : ''}</p>
        <Button onClick={() => setModalSolicitar(true)}>
          <Send className="size-4 mr-1" /> Solicitar Reseña
        </Button>
      </div>

      <Dialog open={modalSolicitar} onOpenChange={setModalSolicitar}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Solicitar Reseña</DialogTitle>
          </DialogHeader>
          <div className="flex flex-col gap-3">
            <p className="text-sm text-muted-foreground">
              Genera un enlace único para enviar a un cliente. Al abrirlo, podrá dejar su
              valoración. Si puntúa 4 o 5 estrellas, será redirigido automáticamente a Google Reviews.
            </p>
            {!ultimaUrlGenerada && (
              <Button onClick={handleSolicitar} disabled={solicitando}>
                {solicitando ? 'Generando...' : 'Generar enlace'}
              </Button>
            )}
            {ultimaUrlGenerada && (
              <div className="flex flex-col gap-2">
                <p className="text-sm font-medium">Enlace generado:</p>
                <div className="flex gap-2">
                  <Input value={ultimaUrlGenerada} readOnly className="text-xs" />
                  <TooltipButton variant="outline" size="icon" onClick={copiarEnlace} tooltip="Copiar enlace">
                    <Copy className="size-4" />
                  </TooltipButton>
                </div>
                <p className="text-xs text-muted-foreground">
                  Comparte este enlace con el cliente por WhatsApp, email o SMS.
                </p>
              </div>
            )}
          </div>
        </DialogContent>
      </Dialog>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Cargando...</p>
      ) : resenas.length > 0 ? (
        <>
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Cliente</TableHead>
                  <TableHead>Puntuación</TableHead>
                  <TableHead>Comentario</TableHead>
                  <TableHead>Google</TableHead>
                  <TableHead>Fecha</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {resenas.map((r) => (
                  <TableRow key={r.id}>
                    <TableCell className="font-medium">{r.cliente_nombre || 'Anónimo'}</TableCell>
                    <TableCell>
                      <Estrellas puntuacion={r.puntuacion} />
                    </TableCell>
                    <TableCell className="max-w-xs truncate">
                      {r.comentario || '—'}
                    </TableCell>
                    <TableCell>
                      {r.redirigido_google ? (
                        <Badge variant="default">Redirigido</Badge>
                      ) : r.puntuacion && r.puntuacion >= 4 && googleUrl ? (
                        <Button
                          variant="outline"
                          onClick={() => window.open(googleUrl, '_blank')}
                          className="gap-1"
                        >
                          <ExternalLink className="size-4" />
                          Enviar a Google
                        </Button>
                      ) : r.respondida_at ? (
                        <Badge variant="secondary">Local</Badge>
                      ) : (
                        <Badge variant="outline">Pendiente</Badge>
                      )}
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {new Date(r.created_at).toLocaleDateString('es-ES')}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
          {totalPages > 1 && (
            <div className="flex items-center justify-center gap-2">
              <TooltipButton variant="outline" size="icon" disabled={page <= 1} onClick={() => setPage(page - 1)} tooltip="Página anterior">
                <ChevronLeft className="size-4" />
              </TooltipButton>
              <span className="text-sm">{page} / {totalPages}</span>
              <TooltipButton variant="outline" size="icon" disabled={page >= totalPages} onClick={() => setPage(page + 1)} tooltip="Página siguiente">
                <ChevronRight className="size-4" />
              </TooltipButton>
            </div>
          )}
        </>
      ) : (
        <div className="text-center py-12 text-muted-foreground">
          <Star className="size-12 mx-auto mb-3 opacity-30" />
          <p>No hay reseñas todavía</p>
          <p className="text-sm mt-1">Solicita reseñas a tus clientes para mejorar tu reputación online</p>
        </div>
      )}
    </div>
  );
}
