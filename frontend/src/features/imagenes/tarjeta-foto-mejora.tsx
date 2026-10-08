import { Download, EllipsisVertical, Loader2, RotateCcw, Undo2, X } from 'lucide-react';
import type { FotoMejora } from '@/domain/foto-mejora';
import type { InfoReintento } from '@/hooks/mejora/use-cola-mejora';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
/* [08AA-32] Ampliación, restauración y descarga viven en useTarjetaFotoMejora. */
import { useTarjetaFotoMejora } from '@/hooks/imagenes/use-tarjeta-foto-mejora';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

const ETIQUETA: Record<FotoMejora['estado'], string> = {
  pendiente: 'Pendiente',
  procesando: 'Procesando',
  lista: 'Lista',
  error: 'Error',
};

function variante(estado: FotoMejora['estado']): 'default' | 'secondary' | 'destructive' | 'outline' {
  if (estado === 'lista') return 'default';
  if (estado === 'error') return 'destructive';
  if (estado === 'procesando') return 'secondary';
  return 'outline';
}

/* Tarjeta compacta: original a la izquierda y mejorada a la derecha para
 * comparar; el clic en cada una la abre completa. La mejorada se guarda en
 * la mejor resolución que devuelva el backend. `reintento` muestra el
 * reintento programado por el backend (F19).
 * F21: Cancelar saca la foto de la cola (o aborta el proceso) y la deja
 * pendiente limpia; Reintentar la mueve (los errores finales reutilizan el
 * original retenido sin re-subir).
 * 259A-1: IDs de fila del original y la mejorada visibles + Restaurar
 * (borra la mejorada del servidor con confirmación; la copia local queda
 * pendiente para mejorarla de nuevo).
 * 259A-3: acciones (Reintentar/Cancelar/Restaurar) en menú contextual de
 * 3 puntos + Descargar original/mejorada (vía blob, ver descarga.ts). */
export function TarjetaFotoMejora(props: {
  foto: FotoMejora;
  /** ID de fila del original en la API (null en borradores sin subir). */
  idOriginal: string | null;
  /** ID de fila de la mejorada en la API (null si aún no hay). */
  idServidor: string | null;
  alReintentar: (foto: FotoMejora) => void;
  alCancelar: (foto: FotoMejora) => void;
  /** Restaura el original (borra la mejorada del servidor); null = sin mejorada que restaurar. */
  alRestaurar: (() => Promise<void>) | null;
  ocupado: boolean;
  reintento?: InfoReintento | null;
}) {
  const { foto, idOriginal, idServidor, alReintentar, alCancelar, alRestaurar, ocupado, reintento } = props;
  /* [08AA-32] Estado interactivo en el hook; aquí JSX + presentación. */
  const {
    ampliada,
    setAmpliada,
    restaura,
    descarga,
    conReintento,
    urlAmpliada,
    puedeRestaurar,
    restaurar,
    descargar,
  } = useTarjetaFotoMejora({ foto, alRestaurar, idServidor, reintento });

  return (
    <div className="overflow-hidden rounded-lg border bg-card">
      <div className="grid grid-cols-2 gap-px bg-border">
        <div className="bg-card">
          <button
            type="button"
            onClick={() => setAmpliada('original')}
            title="Ver original completa"
            className="block w-full cursor-zoom-in"
          >
            <img src={foto.original} alt="Original" className="aspect-square w-full object-cover" loading="lazy" />
          </button>
          <p className="px-2 pt-1 text-[11px] text-muted-foreground">Original</p>
          {idOriginal && (
            <p
              className="px-2 pb-1 font-mono text-[10px] break-all text-muted-foreground"
              title={`ID del original en el servidor: ${idOriginal}`}
            >
              ID {idOriginal}
            </p>
          )}
        </div>
        <div className="bg-card">
          {foto.mejorada ? (
            <button
              type="button"
              onClick={() => setAmpliada('mejorada')}
              title="Ver mejorada completa"
              className="block w-full cursor-zoom-in"
            >
              <img src={foto.mejorada} alt="Mejorada" className="aspect-square w-full object-cover" loading="lazy" />
            </button>
          ) : (
            <div className="flex aspect-square w-full items-center justify-center bg-muted text-xs text-muted-foreground">
              {foto.estado === 'procesando' ? <Loader2 className="h-5 w-5 animate-spin" /> : 'Sin mejorar'}
            </div>
          )}
          <p className="px-2 pt-1 text-[11px] text-muted-foreground">Mejorada</p>
          {idServidor && (
            <p
              className="px-2 pb-1 font-mono text-[10px] break-all text-muted-foreground"
              title={`ID de la mejorada en el servidor: ${idServidor}`}
            >
              ID {idServidor}
            </p>
          )}
        </div>
      </div>
      <div className="flex items-center gap-2 px-2 py-2">
        <Badge variant={variante(foto.estado)}>{ETIQUETA[foto.estado]}</Badge>
        {foto.intentos > 0 && !conReintento && (
          <span className="text-[11px] text-muted-foreground">intento {foto.intentos}</span>
        )}
        {conReintento && (
          <span className="text-[11px] text-muted-foreground" title={conReintento.motivo ?? undefined}>
            {(conReintento.enSeg ?? 0) > 60
              ? `reintento en ~${Math.ceil((conReintento.enSeg ?? 0) / 60)} min`
              : 'reintento en cola'}{' '}
            · intento {conReintento.intentos}
          </span>
        )}
        {/* 259A-3: acciones en menú contextual de 3 puntos (mismo patrón
          que tabla-inmuebles): Reintentar/Cancelar/Restaurar + descargas. */}
        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <Button
                variant="ghost"
                size="icon"
                className="ml-auto h-7 w-7"
                title="Acciones de la foto"
                aria-label="Acciones de la foto"
              >
                <EllipsisVertical />
              </Button>
            }
          />
          <DropdownMenuContent align="end" className="w-56">
            <DropdownMenuItem
              disabled={ocupado || foto.estado === 'procesando'}
              onClick={() => alReintentar(foto)}
            >
              <RotateCcw /> Reintentar
            </DropdownMenuItem>
            {foto.estado === 'procesando' && (
              <DropdownMenuItem onClick={() => alCancelar(foto)}>
                <X /> Cancelar
              </DropdownMenuItem>
            )}
            <DropdownMenuSeparator />
            <DropdownMenuItem disabled={descarga.enCurso} onClick={() => void descargar('original')}>
              <Download /> Descargar original
            </DropdownMenuItem>
            {foto.mejorada && (
              <DropdownMenuItem disabled={descarga.enCurso} onClick={() => void descargar('mejorada')}>
                <Download /> Descargar mejorada
              </DropdownMenuItem>
            )}
            {puedeRestaurar && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuItem disabled={ocupado || restaura.enCurso} onClick={() => void restaurar()}>
                  {restaura.enCurso ? <Loader2 className="animate-spin" /> : <Undo2 />} Restaurar
                </DropdownMenuItem>
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      {foto.error && <p className="px-2 pb-2 text-[11px] text-destructive">{foto.error}</p>}
      {restaura.error && <p className="px-2 pb-2 text-[11px] text-destructive">{restaura.error}</p>}
      {descarga.error && <p className="px-2 pb-2 text-[11px] text-destructive">{descarga.error}</p>}
      <Dialog open={ampliada !== null} onOpenChange={(abierto) => !abierto && setAmpliada(null)}>
        <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-3xl">
          <DialogHeader>
            <DialogTitle>Foto {ampliada === 'mejorada' ? 'mejorada' : 'original'}</DialogTitle>
          </DialogHeader>
          <div className="overflow-hidden rounded-lg border">
            <img
              src={urlAmpliada}
              alt={ampliada === 'mejorada' ? 'Mejorada' : 'Original'}
              className="h-auto w-full"
            />
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
