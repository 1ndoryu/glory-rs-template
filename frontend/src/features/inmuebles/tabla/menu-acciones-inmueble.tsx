/* [08AA-13] Menú contextual de 3 puntos de la tabla de inmuebles.
 * Vive en `tabla/` (no en `tabla-inmuebles.tsx`) por el límite de 300 líneas
 * y para no abarrotar `features/inmuebles/` (máx 10 archivos). Sin cambio
 * de conducta: mismas acciones y orden. */
import { EllipsisVertical, Eye, EyeOff, Globe, ImageDown, ImagePlus, Images, Megaphone, Pencil, Trash2 } from 'lucide-react';
import { fotosVisiblesDe, type Inmueble } from '@/domain/inmueble';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

interface PropsMenuAcciones {
  inmueble: Inmueble;
  onVer: () => void;
  onEditar: () => void;
  onCopy: () => void;
  onAnadirFotos: () => void;
  onEliminar: () => void;
  onPublicar: () => void;
  onDescargarPublicidad: () => void;
  onDescargarMejoradas: () => void;
  onEditarPublicidad: () => void;
}

/* Menú contextual de 3 puntos: Ver, Editar, Copy, Añadir fotos,
 * Publicar/Retirar, imagen publicitaria (editar/descargar), descargar
 * mejoradas y Eliminar (con confirmación). */
export function MenuAcciones({
  inmueble,
  onVer,
  onEditar,
  onCopy,
  onAnadirFotos,
  onEliminar,
  onPublicar,
  onDescargarPublicidad,
  onDescargarMejoradas,
  onEditarPublicidad,
}: PropsMenuAcciones) {
  const nombre = inmueble.titulo || 'Sin título';
  const conFotos = fotosVisiblesDe(inmueble).length > 0;
  const confirmarEliminar = () => {
    if (window.confirm(`Eliminar "${nombre}". Esta acción no se puede deshacer.`)) {
      onEliminar();
    }
  };
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button variant="ghost" size="icon" title="Acciones" aria-label={`Acciones de ${nombre}`}>
            <EllipsisVertical />
          </Button>
        }
      />
      <DropdownMenuContent align="end" className="w-64">
        <DropdownMenuItem onClick={onVer}>
          <Eye /> Ver
        </DropdownMenuItem>
        <DropdownMenuItem onClick={onEditar}>
          <Pencil /> Editar
        </DropdownMenuItem>
        <DropdownMenuItem onClick={onCopy}>
          <Megaphone /> Copy redes
        </DropdownMenuItem>
        <DropdownMenuItem onClick={onAnadirFotos}>
          <ImagePlus /> Añadir fotos
        </DropdownMenuItem>
        <DropdownMenuItem onClick={onPublicar}>
          {inmueble.publicado ? <EyeOff /> : <Globe />}
          {inmueble.publicado ? 'Retirar' : 'Publicar'}
        </DropdownMenuItem>
        {conFotos && (
          <>
            <DropdownMenuItem onClick={onEditarPublicidad}>
              <Pencil /> Editar imagen publicitaria
            </DropdownMenuItem>
            <DropdownMenuItem onClick={onDescargarPublicidad}>
              <ImageDown /> Descargar imagen publicitaria
            </DropdownMenuItem>
            <DropdownMenuItem onClick={onDescargarMejoradas}>
              <Images /> Descargar fotos mejoradas
            </DropdownMenuItem>
          </>
        )}
        <DropdownMenuSeparator />
        <DropdownMenuItem variant="destructive" onClick={confirmarEliminar}>
          <Trash2 /> Eliminar
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
