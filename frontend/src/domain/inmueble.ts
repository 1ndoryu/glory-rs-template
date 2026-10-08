// Dominio puro: sin DOM, sin localStorage, sin React.
// pensado para reutilizarse tal cual en la futura app móvil (Capacitor).
// El `import type` de la receta no crea ciclo en ejecución (se borra al compilar).
import type { RecetaPublicidad } from './plantilla-publicidad';

export type TipoInmueble = 'apartamento' | 'casa' | 'local' | 'terreno' | 'townhouse';
export type Operacion = 'venta' | 'alquiler';
export type EstadoInmueble = 'disponible' | 'reservado' | 'vendido' | 'alquilado';

/* Copy para redes sociales: se genera UNA vez con IA (al crear el inmueble
 * o a mano desde el menú Copy) y se guarda en el inmueble para no gastar IA
 * de nuevo. No confundir con la ficha de `ia.ts` (esa ordena la ficha al
 * subir; esta escribe el texto publicable). `null` = aún sin generar. */
export interface CopyInmueble {
  /** Texto que iría sobre la imagen (1-2 líneas, gancho). */
  corta: string;
  /** Texto que iría debajo, en la descripción del post (con CTA). */
  larga: string;
  modelo: string;
  actualizadaEn: string;
}

/* Identidad: qué es y dónde está. */
export interface IdentidadInmueble {
  id: string;
  titulo: string;
  descripcion: string;
  ubicacion: string;
  /* Nombre de la residencia/conjunto ('' = sin especificar). */
  residencia: string;
}

/* Clasificación comercial y visibilidad. */
export interface ClasificacionInmueble {
  tipo: TipoInmueble;
  operacion: Operacion;
  estado: EstadoInmueble;
  /* Visible en la web pública (`publicado` lo decide el backend; el admin
   * lo cambia con Publicar/Retirar, nunca a mano en el formulario). */
  publicado: boolean;
}

/* Medidas: habitaciones, baños, metros y puestos. */
export interface MedidasInmueble {
  habitaciones: number;
  banos: number;
  metros: number; // m² de propiedad (construidos); lo guardado antes sigue valiendo como propiedad
  metrosTerreno: number; // m² de terreno/parcela (0 = no indicado)
  puestos: number; // puestos de estacionamiento (0 = no indicado)
}

/* Multimedia y copy: originales, mejoradas del servidor y texto social. */
export interface MultimediaInmueble {
  /* Solo originales (`origen=original` en la API). Las mejoradas del
   * servidor viajan aparte en `mejoradasServidor` para que la tira, el
   * visor y la cola de mejora no las traten como originales (antes se
   * mezclaban y aparecían duplicadas junto a estas). */
  fotos: string[];
  /* IDs de fila (`fotos.id` en la API) de los originales, alineados por
   * índice con `fotos`: permiten identificar cada foto en el panel
   * (259A-1) sin adivinar por posición. Vacío en borradores aún sin subir. */
  idsFotos: string[];
  /* Mejora IA ya guardada en el servidor (`origen=mejorada`), emparejada
   * con su original por `orden`. Vacío = aún sin mejorar. */
  mejoradasServidor: MejoraServidor[];
  copy: CopyInmueble | null;
  /* Receta publicitaria elegida en admin, persistida en el servidor
   * (`inmuebles.receta`); `null` = automática. Es la que usa el frente
   * público; la de localStorage queda como reserva/offline. */
  receta: RecetaPublicidad | null;
}

/* Auditoría: creación y última modificación. */
export interface AuditoriaInmueble {
  createdAt: string;
  updatedAt: string;
}

export interface Inmueble
  extends IdentidadInmueble,
    ClasificacionInmueble,
    MedidasInmueble,
    MultimediaInmueble,
    AuditoriaInmueble {
  precio: number;
  /* [279A-3] Ficha /ask: respuestas por tipo (`extras`) y
   * mínimo privado. Opcionales: registros leídos antes de existir la
   * ficha y borradores aún sin ficha los omiten. Nunca viajan a la web
   * pública (el backend los excluye de los endpoints públicos). */
  extras?: Record<string, string | number | boolean>;
  precioMinimo?: number | null;
}

/** Mejora IA guardada en el servidor, emparejada con su original por `orden`.
 * `id` es la fila en la API: permite borrarla (restaurar original, 259A-1)
 * sin re-listar para adivinarla. */
export interface MejoraServidor {
  id: string;
  orden: number;
  url: string;
}

/* Inmueble visible en la web pública: igual que el de gestión más el slug
 * (el detalle público se pide por slug y solo trae publicados). */
export interface InmueblePublico extends Inmueble {
  slug: string;
}

/* [249A-1] Miniatura de tabla (`min160-<uuid>.jpg` junto al original):
 * la tabla pide la versión de 160 px y el modal sigue a máxima resolución.
 * [249A-4] Antes `thumb-` de 320 px: el backend sirve ambos y borra el
 * legado al regenerar. Solo URLs http(s) de `/uploads` con nombre simple;
 * dataURLs y thumbs ya formados se devuelven intactos. */
export function miniaturaDe(url: string): string {
  if (!url.startsWith('http://') && !url.startsWith('https://')) return url;
  const q = url.indexOf('?');
  const base = q < 0 ? url : url.slice(0, q);
  const resto = q < 0 ? '' : url.slice(q);
  const barra = base.lastIndexOf('/');
  if (barra < 0) return url;
  const nombre = base.slice(barra + 1);
  if (nombre.startsWith('min160-') || nombre.startsWith('thumb-') || nombre.includes('/')) return url;
  return `${base.slice(0, barra + 1)}min160-${nombre}${resto}`;
}

/* Portada para tarjetas y modal: la mejorada del primer `orden` si existe,
 * si no el primer original. */
export function portadaDe(i: Inmueble): string {
  return fotosVisiblesDe(i)[0] ?? '';
}

/* Fotos tal como las ve el público: cada original con su versión mejorada
 * cuando existe (emparejada por `orden`), si no el original. La primera es
 * siempre la principal (fotos[0]) en su mejor versión disponible.
 * [249A-1] Versión de caché `?v=<updatedAt>`: el backend toca `updated_at` al
 * subir/borrar fotos, así cambiar fotos invalida la caché del navegador/CDN
 * sin renombrar ficheros. Solo en URLs http(s): los dataURL de borradores
 * (admin, aún sin subir) se dejan intactos. No versionar en `remotoADominio`:
 * `sincronizarFotos` compara URLs con `relativa()` y el `?v=` rompería la
 * igualdad (re-subiría las fotos en cada guardado). */
export function fotosVisiblesDe(i: Inmueble): string[] {
  const version = `v=${encodeURIComponent(i.updatedAt)}`;
  const conVersion = (url: string): string => {
    if (!url.startsWith('http://') && !url.startsWith('https://')) return url;
    return url.includes('?') ? `${url}&${version}` : `${url}?${version}`;
  };
  return i.fotos.map((f, indice) => conVersion(i.mejoradasServidor.find((m) => m.orden === indice)?.url ?? f));
}

/* Texto libre del borrador. */
export interface TextoDraft {
  titulo: string;
  descripcion: string;
  ubicacion: string;
  residencia: string;
}

/* Números del borrador (texto de formulario, se validan al guardar). */
export interface NumerosDraft {
  precio: string;
  habitaciones: string;
  banos: string;
  metros: string;
  metrosTerreno: string;
  puestos: string;
}

/* Clasificación del borrador. */
export interface ClaseDraft {
  tipo: TipoInmueble | '';
  operacion: Operacion | '';
  estado: EstadoInmueble;
}

/** Borrador del modal: opcional salvo fotos (siempre array). */
export interface InmuebleDraft extends TextoDraft, NumerosDraft, ClaseDraft {
  fotos: string[];
}

export const DRAFT_VACIO: InmuebleDraft = {
  titulo: '',
  descripcion: '',
  ubicacion: '',
  residencia: '',
  precio: '',
  tipo: '',
  operacion: '',
  habitaciones: '',
  banos: '',
  metros: '',
  metrosTerreno: '',
  puestos: '',
  fotos: [],
  estado: 'disponible',
};

export const TIPOS: TipoInmueble[] = ['apartamento', 'casa', 'local', 'terreno', 'townhouse'];

/* Etiquetas de visualización (el valor interno va en minúsculas, apto para la IA). */
export const ETIQUETAS_TIPO: Record<TipoInmueble, string> = {
  apartamento: 'Apartamento',
  casa: 'Casa',
  local: 'Local',
  terreno: 'Terreno',
  townhouse: 'Town House',
};
export const ESTADOS: EstadoInmueble[] = ['disponible', 'reservado', 'vendido', 'alquilado'];

export type ErroresDraft = Partial<Record<'titulo' | 'ubicacion' | 'residencia' | 'precio' | 'tipo' | 'operacion' | 'habitaciones' | 'banos' | 'metros' | 'metrosTerreno' | 'puestos', string>>;

/* Ningún campo es obligatorio: se trabaja con la información disponible.
 * La validación nunca bloquea el guardado; la conversión (`draftAInmueble`)
 * sanea los valores (texto recortado, números no válidos a 0, enums a su
 * defecto). Se conserva la firma para futuros chequeos de formato. */
export function validarDraft(_d: InmuebleDraft): ErroresDraft {
  return {};
}

export function draftTieneContenido(d: InmuebleDraft): boolean {
  return Boolean(
    d.titulo.trim() ||
      d.descripcion.trim() ||
      d.ubicacion.trim() ||
      d.residencia.trim() ||
      d.precio.trim() ||
      d.tipo ||
      d.operacion ||
      d.habitaciones.trim() ||
      d.banos.trim() ||
      d.metros.trim() ||
      d.metrosTerreno.trim() ||
      d.puestos.trim() ||
      d.fotos.length > 0,
  );
}

export function draftAInmueble(d: InmuebleDraft, base?: Inmueble): Inmueble {
  const ahora = new Date().toISOString();
  // Sin obligatorios: lo no indicado queda como "" / 0 / defecto del enum.
  const num = (v: string): number => {
    const n = Number(v);
    return Number.isFinite(n) && n >= 0 ? n : 0;
  };
  return {
    id: base?.id ?? (typeof crypto !== 'undefined' && 'randomUUID' in crypto ? crypto.randomUUID() : String(Date.now())),
    titulo: d.titulo.trim(),
    descripcion: d.descripcion.trim(),
    ubicacion: d.ubicacion.trim(),
    residencia: d.residencia.trim(),
    precio: num(d.precio),
    tipo: (d.tipo || 'apartamento') as TipoInmueble,
    operacion: (d.operacion || 'venta') as Operacion,
    habitaciones: num(d.habitaciones),
    banos: num(d.banos),
    metros: num(d.metros),
    metrosTerreno: num(d.metrosTerreno),
    puestos: num(d.puestos),
    fotos: d.fotos,
    idsFotos: base?.idsFotos ?? [],
    // Las mejoradas del servidor no se editan en el formulario: se conservan.
    mejoradasServidor: base?.mejoradasServidor ?? [],
    estado: d.estado,
    // El copy y la receta no se editan en el formulario: se conservan.
    publicado: base?.publicado ?? false,
    copy: base?.copy ?? null,
    receta: base?.receta ?? null,
    /* La ficha /ask no se edita en el formulario: se conserva. */
    extras: base?.extras,
    precioMinimo: base?.precioMinimo,
    createdAt: base?.createdAt ?? ahora,
    updatedAt: ahora,
  };
}

export function inmuebleADraft(i: Inmueble): InmuebleDraft {
  return {
    titulo: i.titulo,
    descripcion: i.descripcion,
    ubicacion: i.ubicacion,
    residencia: i.residencia ?? '',
    precio: String(i.precio),
    tipo: i.tipo,
    operacion: i.operacion,
    habitaciones: String(i.habitaciones),
    banos: String(i.banos),
    metros: String(i.metros),
    // Registros guardados antes de existir el campo no lo traen.
    metrosTerreno: String(i.metrosTerreno ?? 0),
    puestos: String(i.puestos ?? 0),
    fotos: [...i.fotos],
    estado: i.estado,
  };
}

/* Dólar de frente (279A-6): `$ 43.000` con miles venezolanos, igual que
 * pide /ask (el público usa su propio `features/publica/formato`). */
export function formatearPrecio(precio: number): string {
  const miles = new Intl.NumberFormat('es-VE', { maximumFractionDigits: 0 }).format(precio);
  return `$ ${miles}`;
}

/* Fechas de creación/modificación (ISO). La creación no se puede modificar:
 * `draftAInmueble` conserva la original al editar. */
export function formatearFecha(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '—';
  return new Intl.DateTimeFormat('es-ES', { dateStyle: 'medium', timeStyle: 'short' }).format(d);
}
