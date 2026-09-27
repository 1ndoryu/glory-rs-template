import { useState } from 'react';
import { BedDouble, Bath, CarFront, Globe, Ruler, LandPlot, MapPin, Pencil, Trash2, Building2, Eye, EyeOff, EllipsisVertical, ImagePlus, ImageDown, Images, Megaphone } from 'lucide-react';
import { ETIQUETAS_TIPO, formatearPrecio, fotosVisiblesDe, portadaDe, type EstadoInmueble, type Inmueble } from '@/domain/inmueble';
import { calcularCompletitud } from '@/domain/ficha-ask';
import { recetaVigenteDe, resolverReceta } from '@/domain/plantilla-publicidad';
import { exportarPublicidad } from '@/platform/canvas-publicidad';
import { usePublicidades } from '@/hooks/publicidad/use-publicidades';
import { ModalDescargarFotos } from '@/features/inmuebles/modal-descargar-fotos';
import { ModalEditorPublicidad } from '@/features/publicidad/modal-editor-publicidad';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { cn } from '@/lib/utils';

const claseEstado: Record<EstadoInmueble, string> = {
  disponible: 'border-transparent bg-emerald-100 text-emerald-900',
  reservado: 'border-transparent bg-amber-100 text-amber-900',
  vendido: 'border-transparent bg-sky-100 text-sky-900',
  alquilado: '',
};

interface Props {
  inmuebles: Inmueble[];
  onVer: (inmueble: Inmueble) => void;
  onEditar: (inmueble: Inmueble) => void;
  onEliminar: (id: string) => void;
  onAnadirFotos: (id: string) => void;
  onCopy: (inmueble: Inmueble) => void;
  onPublicar: (id: string, publicado: boolean) => void;
  /* La receta hace su propio PUT: con esto la lista se refresca con el
   * inmueble ya persistido (sin releer todo). */
  onActualizarInmueble?: (inmueble: Inmueble) => void;
}

/* Sin obligatorios: lo no indicado se muestra neutro ("—" / "Sin título"). */
function precioVisible(precio: number): string {
  return precio > 0 ? formatearPrecio(precio) : '—';
}

/* Semáforo de ficha /ask (279A-3): % de preguntas respondidas por tipo.
 * Solo aviso visual: nunca bloquea publicar ni editar. */
function SemaforoFicha({ inmueble }: { inmueble: Inmueble }) {
  const { porcentaje } = calcularCompletitud(inmueble.tipo, inmueble.extras ?? {}, inmueble.precioMinimo ?? null, inmueble);
  const color =
    porcentaje === 100
      ? 'border-transparent bg-emerald-100 text-emerald-900'
      : porcentaje >= 50
        ? 'border-transparent bg-amber-100 text-amber-900'
        : 'border-transparent bg-red-100 text-red-900';
  return (
    <Badge variant="secondary" className={color} title={`Ficha al ${porcentaje}%`}>
      {porcentaje}%
    </Badge>
  );
}

/* Etiqueta de visibilidad en la web pública. */
function Publico({ publicado }: { publicado: boolean }) {
  if (!publicado) return <span className="text-xs text-muted-foreground">Oculto</span>;
  return (
    <Badge variant="secondary" className="border-transparent bg-emerald-100 text-emerald-900">
      <Globe className="h-3 w-3" /> Público
    </Badge>
  );
}

function Foto({ src, titulo }: { src?: string; titulo: string }) {
  if (!src) {
    return (
      <div className="flex h-14 w-20 items-center justify-center rounded-md bg-muted">
        <Building2 className="h-5 w-5 text-muted-foreground" />
      </div>
    );
  }
  return (
    <img
      src={src}
      alt={titulo}
      className="h-14 w-20 rounded-md object-cover"
      onError={(e) => {
        (e.target as HTMLImageElement).style.display = 'none';
      }}
    />
  );
}

/* Menú contextual de 3 puntos: Ver, Editar, Copy, Añadir fotos,
 * Publicar/Retirar, imagen publicitaria (editar/descargar), descargar
 * mejoradas y Eliminar (con confirmación). */
function MenuAcciones({
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
}: {
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
}) {
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

export function TablaInmuebles({ inmuebles, onVer, onEditar, onEliminar, onAnadirFotos, onCopy, onPublicar, onActualizarInmueble }: Props) {
  const { recetaDe, guardar } = usePublicidades();
  const [editandoPubli, setEditandoPubli] = useState<Inmueble | null>(null);
  const [fallo, setFallo] = useState<string | null>(null);
  const [descargandoFotosDe, setDescargandoFotosDe] = useState<Inmueble | null>(null);
  /* El editor de publicidad trabaja con el objeto fresco de la lista: si la
   * propiedad se editó fuera, la vista previa recarga sola. */
  const editandoPubliFresco = editandoPubli ? (inmuebles.find((i) => i.id === editandoPubli.id) ?? editandoPubli) : null;
  /* El modal de descargas también usa el objeto fresco de la lista. */
  const descargandoFresco = descargandoFotosDe
    ? (inmuebles.find((i) => i.id === descargandoFotosDe.id) ?? descargandoFotosDe)
    : null;

  /* Descarga la imagen publicitaria (JPG) con la receta vigente (manda
   * la del servidor, la que ve el frente público; la local es reserva). */
  async function descargarPublicidad(inmueble: Inmueble) {
    const receta = recetaVigenteDe(inmueble, inmueble.receta ?? recetaDe(inmueble.id));
    const comp = receta ? resolverReceta(inmueble, receta) : null;
    if (!comp) {
      setFallo('Sin fotos para la imagen publicitaria.');
      return;
    }
    setFallo(null);
    try {
      await exportarPublicidad(inmueble, comp, 'jpeg');
    } catch {
      setFallo('No se pudo descargar la imagen publicitaria.');
    }
  }

  function acciones(inmueble: Inmueble) {
    return {
      onVer: () => onVer(inmueble),
      onEditar: () => onEditar(inmueble),
      onCopy: () => onCopy(inmueble),
      onAnadirFotos: () => onAnadirFotos(inmueble.id),
      onEliminar: () => onEliminar(inmueble.id),
      onPublicar: () => onPublicar(inmueble.id, !inmueble.publicado),
      onDescargarPublicidad: () => void descargarPublicidad(inmueble),
      onDescargarMejoradas: () => {
        setFallo(null);
        setDescargandoFotosDe(inmueble);
      },
      onEditarPublicidad: () => {
        setFallo(null);
        setEditandoPubli(inmueble);
      },
    };
  }

  if (inmuebles.length === 0) {
    return (
      <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed px-6 py-16 text-center">
        <Building2 className="h-10 w-10 text-muted-foreground" />
        <p className="font-medium">Sin inmuebles todavía</p>
        <p className="max-w-sm text-sm text-muted-foreground">
          Pulsa «Añadir inmueble» para registrar el primero: fotos, descripción, ubicación y precio.
        </p>
      </div>
    );
  }

  return (
    <>
      {fallo && (
        <p className="mb-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {fallo}
        </p>
      )}
      {/* Escritorio: tabla shadcn */}
      <div className="hidden rounded-lg border md:block">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Foto</TableHead>
              <TableHead>Título</TableHead>
              <TableHead>Ubicación</TableHead>
              <TableHead>Tipo / Operación</TableHead>
              <TableHead>Precio</TableHead>
              <TableHead>Estado</TableHead>
              <TableHead>Público</TableHead>
              <TableHead>Ficha</TableHead>
              <TableHead className="text-right">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {inmuebles.map((i) => (
              <TableRow key={i.id}>
                <TableCell>
                  <button
                    type="button"
                    onClick={() => onVer(i)}
                    title={`Ver ${i.titulo || 'Sin título'}`}
                    aria-label={`Ver ${i.titulo || 'Sin título'}`}
                    className="block cursor-pointer rounded-md transition hover:opacity-80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30"
                  >
                    <Foto src={portadaDe(i)} titulo={i.titulo || 'Inmueble'} />
                  </button>
                </TableCell>
                <TableCell className="max-w-[220px]">
                  <p className={cn('truncate font-medium', !i.titulo && 'text-muted-foreground')}>
                    {i.titulo || 'Sin título'}
                  </p>
                  {i.descripcion && <p className="truncate text-xs text-muted-foreground">{i.descripcion}</p>}
                </TableCell>
                <TableCell className="max-w-[180px] truncate">{i.ubicacion || '—'}</TableCell>
                <TableCell className="whitespace-nowrap capitalize">
                  {ETIQUETAS_TIPO[i.tipo]} · {i.operacion}
                </TableCell>
                <TableCell className="whitespace-nowrap font-semibold">{precioVisible(i.precio)}</TableCell>
                <TableCell>
                  <Badge variant="secondary" className={cn('capitalize', claseEstado[i.estado])}>
                    {i.estado}
                  </Badge>
                </TableCell>
                <TableCell>
                  <Publico publicado={i.publicado} />
                </TableCell>
                <TableCell>
                  <SemaforoFicha inmueble={i} />
                </TableCell>
                <TableCell>
                  <div className="flex justify-end">
                    <MenuAcciones inmueble={i} {...acciones(i)} />
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      {/* Móvil: tarjetas */}
      <div className="grid gap-3 md:hidden">
        {inmuebles.map((i) => (
          <article key={i.id} className="overflow-hidden rounded-lg border bg-card">
            <button
              type="button"
              onClick={() => onVer(i)}
              title={`Ver ${i.titulo || 'Sin título'}`}
              aria-label={`Ver ${i.titulo || 'Sin título'}`}
              className="block w-full cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30"
            >
              {portadaDe(i) ? (
                <img src={portadaDe(i)} alt={i.titulo || 'Inmueble'} className="h-44 w-full object-cover" loading="lazy" />
              ) : (
                <div className="flex h-24 items-center justify-center bg-muted">
                  <Building2 className="h-8 w-8 text-muted-foreground" />
                </div>
              )}
            </button>
            <div className="space-y-2 p-4">
              <div className="flex items-start justify-between gap-2">
                <h3 className={cn('font-semibold leading-tight', !i.titulo && 'text-muted-foreground')}>
                  {i.titulo || 'Sin título'}
                </h3>
                <div className="flex shrink-0 items-center gap-1.5">
                  <Publico publicado={i.publicado} />
                  <SemaforoFicha inmueble={i} />
                  <Badge variant="secondary" className={cn('capitalize', claseEstado[i.estado])}>
                    {i.estado}
                  </Badge>
                </div>
              </div>
              <p className="flex items-center gap-1 text-sm text-muted-foreground">
                <MapPin className="h-3.5 w-3.5" /> {i.ubicacion || '—'}{i.residencia ? ` · ${i.residencia}` : ''}
              </p>
              {i.descripcion && <p className="line-clamp-2 text-sm">{i.descripcion}</p>}
              <p className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
                <span className="capitalize">
                  {ETIQUETAS_TIPO[i.tipo]} · {i.operacion}
                </span>
                {i.habitaciones > 0 && (
                  <span className="flex items-center gap-1">
                    <BedDouble className="h-3.5 w-3.5" /> {i.habitaciones}
                  </span>
                )}
                {i.banos > 0 && (
                  <span className="flex items-center gap-1">
                    <Bath className="h-3.5 w-3.5" /> {i.banos}
                  </span>
                )}
                {i.metros > 0 && (
                  <span className="flex items-center gap-1" title="M² propiedad (construidos)">
                    <Ruler className="h-3.5 w-3.5" /> {i.metros} m²
                  </span>
                )}
                {i.metrosTerreno > 0 && (
                  <span className="flex items-center gap-1" title="M² terreno">
                    <LandPlot className="h-3.5 w-3.5" /> {i.metrosTerreno} m² terreno
                  </span>
                )}
                {i.puestos > 0 && (
                  <span className="flex items-center gap-1" title="Puestos de estacionamiento">
                    <CarFront className="h-3.5 w-3.5" /> {i.puestos} puestos
                  </span>
                )}
                {i.habitaciones === 0 && i.banos === 0 && i.metros === 0 && i.metrosTerreno === 0 && i.puestos === 0 && <span>—</span>}
              </p>
              <div className="flex items-center justify-between pt-1">
                <p className="text-lg font-bold">{precioVisible(i.precio)}</p>
                <MenuAcciones inmueble={i} {...acciones(i)} />
              </div>
            </div>
          </article>
        ))}
      </div>
      <ModalEditorPublicidad
        key={editandoPubliFresco ? `publi-${editandoPubliFresco.id}` : 'publi-cerrado'}
        inmueble={editandoPubliFresco}
        receta={editandoPubliFresco ? recetaVigenteDe(editandoPubliFresco, editandoPubliFresco.receta ?? recetaDe(editandoPubliFresco.id)) : null}
        alGuardar={async (receta) => {
          if (!editandoPubliFresco) return;
          setFallo(null);
          try {
            const actualizado = await guardar(editandoPubliFresco, receta);
            onActualizarInmueble?.(actualizado);
            setEditandoPubli(null);
          } catch (e) {
            // La copia local quedó; se avisa para reintentar con red.
            setFallo(e instanceof Error ? e.message : 'No se pudo guardar la publicidad en el servidor.');
          }
        }}
        alCerrar={() => setEditandoPubli(null)}
        alEditarPropiedad={(inmueble) => {
          setEditandoPubli(null);
          onEditar(inmueble);
        }}
      />
      <ModalDescargarFotos
        key={descargandoFresco ? `fotos-${descargandoFresco.id}` : 'fotos-cerrado'}
        inmueble={descargandoFresco}
        alCerrar={() => setDescargandoFotosDe(null)}
      />
    </>
  );
}
