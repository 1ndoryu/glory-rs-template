/* [08AA-13] Celdas presentacionales de la tabla de inmuebles.
 * Viven en `tabla/` (no en `tabla-inmuebles.tsx`) por el límite de 300 líneas
 * y para no abarrotar `features/inmuebles/` (máx 10 archivos). Sin cambio de
 * conducta ni de estilos. `claseEstadoDe` es función (no objeto exportado)
 * para no disparar `objeto-mutable-exportado`: el mapa vive dentro. */
import { Building2, Globe } from 'lucide-react';
import { formatearPrecio, type EstadoInmueble, type Inmueble } from '@/domain/inmueble';
import { calcularCompletitud } from '@/domain/ficha-ask';
import { Badge } from '@/components/ui/badge';

/* Color del badge de estado. Función con mapa interno: el llamador no puede
 * mutar el mapa compartido. */
export function claseEstadoDe(estado: EstadoInmueble): string {
  const mapa: Record<EstadoInmueble, string> = {
    disponible: 'border-transparent bg-emerald-100 text-emerald-900',
    reservado: 'border-transparent bg-amber-100 text-amber-900',
    vendido: 'border-transparent bg-sky-100 text-sky-900',
    alquilado: '',
  };
  return mapa[estado];
}

/* Sin obligatorios: lo no indicado se muestra neutro ("—" / "Sin título"). */
export function precioVisible(precio: number): string {
  return precio > 0 ? formatearPrecio(precio) : '—';
}

/* Semáforo de ficha /ask (279A-3): % de preguntas respondidas por tipo.
 * Solo aviso visual: nunca bloquea publicar ni editar. */
export function SemaforoFicha({ inmueble }: { inmueble: Inmueble }) {
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
export function Publico({ publicado }: { publicado: boolean }) {
  if (!publicado) return <span className="text-xs text-muted-foreground">Oculto</span>;
  return (
    <Badge variant="secondary" className="border-transparent bg-emerald-100 text-emerald-900">
      <Globe className="h-3 w-3" /> Público
    </Badge>
  );
}

export function Foto({ src, titulo }: { src?: string; titulo: string }) {
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
