// Cliente HTTP de la API Rust (MN-Inmobiliaria): sesión + CRUD admin.
// Sin dependencias: fetch + localStorage (la sesión sobrevive al cierre
// del navegador; solo se sale con "Salir" o 401 del servidor). Los fallos
// se lanzan como `ErrorApi` con el mensaje del servidor (nunca fallos silenciosos).
// Las fotos del servidor son URLs absolutas (/uploads/...); las del
// formulario son dataURL y se suben con `sincronizarFotos`.

import type { CopyInmueble, Inmueble, MejoraServidor } from '../../domain/inmueble';
import { PRESETS_EXPORTACION, type RecetaPublicidad } from '../../domain/plantilla-publicidad';

export const API_URL =
  ((import.meta.env.VITE_API_URL as string | undefined)?.trim().replace(/\/$/, '')) || 'http://127.0.0.1:3000';

const CLAVE_TOKEN = 'inmobiliaria:token';
const CLAVE_EMAIL = 'inmobiliaria:email';

export const EVENTO_SESION_EXPIRADA = 'inmobiliaria:sesion-expirada';

export class ErrorApi extends Error {
  estado: number;
  constructor(mensaje: string, estado: number) {
    super(mensaje);
    this.name = 'ErrorApi';
    this.estado = estado;
  }
}

export function leerToken(): string | null {
  try {
    return localStorage.getItem(CLAVE_TOKEN);
  } catch {
    return null;
  }
}

export function leerEmailSesion(): string | null {
  try {
    return localStorage.getItem(CLAVE_EMAIL);
  } catch {
    return null;
  }
}

function guardarSesion(token: string, email: string): void {
  localStorage.setItem(CLAVE_TOKEN, token);
  localStorage.setItem(CLAVE_EMAIL, email);
}

export function borrarSesion(): void {
  try {
    localStorage.removeItem(CLAVE_TOKEN);
    localStorage.removeItem(CLAVE_EMAIL);
  } catch {
    // Sin almacenamiento no hay sesión que borrar.
  }
}

/* 401 = token ausente/caducado: se limpia la sesión y se avisa a la app
 * para que vuelva al login (el hook `useSesion` escucha el evento). */
function avisarSesionExpirada(): void {
  borrarSesion();
  try {
    window.dispatchEvent(new Event(EVENTO_SESION_EXPIRADA));
  } catch {
    // Sin window no hay a quién avisar.
  }
}

interface CuerpoError {
  error?: unknown;
  message?: unknown;
}

/* Exportado para reutilizar sesión+JWT+401 en otros clientes staff
 * (p. ej. `data/chat/cliente-admin.ts`): no duplicar gestión de sesión. */
export async function apiFetch<T>(ruta: string, opciones: RequestInit = {}): Promise<T> {
  const token = leerToken();
  let respuesta: Response;
  try {
    respuesta = await fetch(`${API_URL}${ruta}`, {
      ...opciones,
      headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}), ...opciones.headers },
    });
  } catch {
    throw new ErrorApi('No se pudo contactar con la API (¿arrancada en ' + API_URL + '?).', 0);
  }
  if (respuesta.status === 401) {
    avisarSesionExpirada();
    throw new ErrorApi('Sesión caducada o inválida: vuelve a entrar.', 401);
  }
  if (respuesta.status === 204) return undefined as T;
  let cuerpo: CuerpoError | null = null;
  try {
    cuerpo = (await respuesta.json()) as CuerpoError;
  } catch {
    cuerpo = null;
  }
  if (!respuesta.ok) {
    const mensaje =
      (cuerpo && typeof cuerpo.message === 'string' && cuerpo.message) ||
      (cuerpo && typeof cuerpo.error === 'string' && cuerpo.error) ||
      `La API devolvió ${respuesta.status}.`;
    throw new ErrorApi(mensaje, respuesta.status);
  }
  return cuerpo as T;
}

/* ---------- Auth ---------- */

interface RespuestaAuth {
  token: string;
  user_id: string;
}

export async function entrar(email: string, password: string): Promise<string> {
  const r = await apiFetch<RespuestaAuth>('/api/auth/login', {
    method: 'POST',
    body: JSON.stringify({ email: email.trim(), password }),
  });
  guardarSesion(r.token, email.trim());
  return email.trim();
}

export async function registrarPropietario(email: string, password: string): Promise<string> {
  try {
    const r = await apiFetch<RespuestaAuth>('/api/auth/register', {
      method: 'POST',
      body: JSON.stringify({ email: email.trim(), password }),
    });
    guardarSesion(r.token, email.trim());
    return email.trim();
  } catch (e) {
    if (e instanceof ErrorApi && e.estado === 403) {
      throw new ErrorApi('Ya existe un propietario: entra con su correo y contraseña.', 403);
    }
    throw e;
  }
}

/* ---------- Mapeos snake_case <-> dominio ---------- */

interface FotoRemota {
  id: string;
  url: string;
  orden: number;
  origen: string;
}

interface CopyRemoto {
  corta: string;
  larga: string;
  modelo: string;
  actualizada_en: string;
}

/* Identidad remota (snake_case de la API). */
export interface RemotoIdentidad {
  id: string;
  titulo: string;
  descripcion: string;
  ubicacion: string;
  residencia: string;
  slug: string;
}

/* Clasificación remota. */
export interface RemotoClase {
  tipo: string;
  operacion: string;
  estado: string;
  publicado: boolean;
}

/* Medidas remotas. */
export interface RemotoMedidas {
  precio: number;
  habitaciones: number;
  banos: number;
  metros: number;
  metros_terreno: number;
  puestos: number;
}

/* Receta publicitaria tal como la entrega el backend (`inmuebles.receta` JSONB). */
export interface RecetaRemota {
  fondo_idx: number;
  circular_grande_idx: number;
  circular_mediano_idx: number;
  formato: string;
  con_precio: boolean;
  titulo1: string;
  titulo2: string;
}

interface RemotoMedia {
  copy: CopyRemoto | null;
  fotos: FotoRemota[];
  receta?: RecetaRemota | null;
}

/* Auditoría remota. */
export interface RemotoAuditoria {
  created_at: string;
  updated_at: string;
}

export interface InmuebleRemoto
  extends RemotoIdentidad,
    RemotoClase,
    RemotoMedidas,
    RemotoMedia,
    RemotoAuditoria {
  /* [279A-3] Ficha /ask: `extras` JSONB y `precio_minimo` privado. El
   * backend los incluye en los endpoints admin (nunca en los públicos). */
  extras?: Record<string, string | number | boolean> | null;
  precio_minimo?: number | null;
}

export function urlAbsoluta(url: string): string {
  return url.startsWith('/') ? `${API_URL}${url}` : url;
}

function relativa(url: string): string {
  return url.startsWith(API_URL) ? url.slice(API_URL.length) : url;
}

function copyARemoto(copy: CopyInmueble | null): CopyRemoto | null {
  if (!copy) return null;
  return { corta: copy.corta, larga: copy.larga, modelo: copy.modelo, actualizada_en: copy.actualizadaEn };
}

/* Receta remota (snake_case) -> dominio (camelCase), con la misma guarda
 * que `recetaVigenteDe`: índices sanos y formato del allowlist; si el
 * servidor trae algo raro se ignora (= automática) en vez de romper. */
function adaptarRecetaRemota(r: RecetaRemota | null | undefined): RecetaPublicidad | null {
  if (!r || typeof r !== 'object') return null;
  if (!PRESETS_EXPORTACION.some((p) => p.formato === r.formato)) return null;
  return {
    fondoIdx: r.fondo_idx,
    circularGrandeIdx: r.circular_grande_idx,
    circularMedianoIdx: r.circular_mediano_idx,
    formato: r.formato as RecetaPublicidad['formato'],
    conPrecio: r.con_precio,
    titulo1: r.titulo1,
    titulo2: r.titulo2,
  };
}

function recetaADominio(r: RecetaPublicidad | null): RecetaRemota | null {
  if (!r) return null;
  return {
    fondo_idx: r.fondoIdx,
    circular_grande_idx: r.circularGrandeIdx,
    circular_mediano_idx: r.circularMedianoIdx,
    formato: r.formato,
    con_precio: r.conPrecio,
    titulo1: r.titulo1 ?? '',
    titulo2: r.titulo2 ?? '',
  };
}

export function remotoADominio(r: InmuebleRemoto): Inmueble {
  const ordenadas = [...r.fotos].sort((a, b) => a.orden - b.orden);
  const originales = ordenadas.filter((f) => f.origen !== 'mejorada');
  return {
    id: r.id,
    titulo: r.titulo,
    descripcion: r.descripcion,
    ubicacion: r.ubicacion,
    residencia: r.residencia ?? '',
    precio: r.precio,
    tipo: r.tipo as Inmueble['tipo'],
    operacion: r.operacion as Inmueble['operacion'],
    habitaciones: r.habitaciones,
    banos: r.banos,
    metros: r.metros,
    metrosTerreno: r.metros_terreno,
    puestos: r.puestos ?? 0,
    fotos: originales.map((f) => urlAbsoluta(f.url)),
    idsFotos: originales.map((f) => f.id),
    mejoradasServidor: ordenadas
      .filter((f) => f.origen === 'mejorada')
      .map((f) => ({ id: f.id, orden: f.orden, url: urlAbsoluta(f.url) })),
    estado: r.estado as Inmueble['estado'],
    publicado: r.publicado,
    copy: r.copy ? { corta: r.copy.corta, larga: r.copy.larga, modelo: r.copy.modelo, actualizadaEn: r.copy.actualizada_en } : null,
    receta: adaptarRecetaRemota(r.receta),
    /* Ficha /ask (ausente = aún sin responder; nunca sale a lo público). */
    extras: r.extras ?? undefined,
    precioMinimo: r.precio_minimo ?? undefined,
    createdAt: r.created_at,
    updatedAt: r.updated_at,
  };
}

function dominioACuerpo(i: Inmueble): Record<string, unknown> {
  return {
    titulo: i.titulo,
    descripcion: i.descripcion,
    ubicacion: i.ubicacion,
    residencia: i.residencia,
    precio: i.precio,
    tipo: i.tipo,
    operacion: i.operacion,
    habitaciones: i.habitaciones,
    banos: i.banos,
    metros: i.metros,
    metros_terreno: i.metrosTerreno,
    puestos: i.puestos,
    estado: i.estado,
    copy: copyARemoto(i.copy),
    receta: recetaADominio(i.receta),
  };
}

/* ---------- CRUD admin ---------- */

export async function listarRemoto(): Promise<Inmueble[]> {
  const r = await apiFetch<{ items: InmuebleRemoto[] }>('/api/admin/inmuebles?page=1&per_page=100');
  return r.items.map(remotoADominio);
}

async function obtenerRemoto(id: string): Promise<InmuebleRemoto> {
  return apiFetch<InmuebleRemoto>(`/api/admin/inmuebles/${encodeURIComponent(id)}`);
}

export async function crearRemoto(inmueble: Inmueble): Promise<Inmueble> {
  const r = await apiFetch<InmuebleRemoto>('/api/admin/inmuebles', {
    method: 'POST',
    body: JSON.stringify(dominioACuerpo(inmueble)),
  });
  return remotoADominio(r);
}

export async function actualizarRemoto(inmueble: Inmueble): Promise<Inmueble> {
  /* PUT (no PATCH): el contrato OpenAPI del backend (`inmuebles.rs`,
   * `put(update_inmueble)`) es reemplazo completo y `dominioACuerpo` manda
   * el objeto entero. Con PATCH el router devolvía 405 [199A-4]. */
  const r = await apiFetch<InmuebleRemoto>(`/api/admin/inmuebles/${encodeURIComponent(inmueble.id)}`, {
    method: 'PUT',
    body: JSON.stringify(dominioACuerpo(inmueble)),
  });
  return remotoADominio(r);
}

/* Guarda solo la receta publicitaria en el servidor: lee el vigente, le
 * fija la receta y lo reenvía por PUT (el contrato es reemplazo completo).
 * Devuelve el inmueble ya mapeado. Lanza `ErrorApi` si falla. */
export async function guardarReceta(id: string, receta: RecetaPublicidad): Promise<Inmueble> {
  const actual = remotoADominio(await obtenerRemoto(id));
  return actualizarRemoto({ ...actual, receta });
}

export async function fijarPublicado(id: string, publicado: boolean): Promise<Inmueble> {
  const r = await apiFetch<InmuebleRemoto>(`/api/admin/inmuebles/${encodeURIComponent(id)}/publicacion`, {
    method: 'PATCH',
    body: JSON.stringify({ publicado }),
  });
  return remotoADominio(r);
}

export async function eliminarRemoto(id: string): Promise<void> {
  await apiFetch<unknown>(`/api/admin/inmuebles/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

/* ---------- Fotos ---------- */

const MIME_A_EXTENSION: Record<string, string> = { 'image/jpeg': 'jpg', 'image/png': 'png', 'image/webp': 'webp' };

function extensionDeDataUrl(dataUrl: string): string {
  const m = /^data:(image\/[a-z]+);base64,/.exec(dataUrl);
  return (m && MIME_A_EXTENSION[m[1]]) || 'jpg';
}

/* El backend valida magia contra extensión (400 si no coinciden) y la
 * etiqueta del dataURL miente a veces (mejoradas `image/png` con magia
 * JPEG): la extensión se decide por magia, la etiqueta solo es reserva. */
function extensionPorMagia(bytes: Uint8Array): string | null {
  if (bytes.length >= 2 && bytes[0] === 0xff && bytes[1] === 0xd8) return 'jpg';
  if (bytes.length >= 4 && bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4e && bytes[3] === 0x47)
    return 'png';
  if (
    bytes.length >= 12 &&
    bytes[0] === 0x52 &&
    bytes[1] === 0x49 &&
    bytes[2] === 0x46 &&
    bytes[3] === 0x46 &&
    bytes[8] === 0x57 &&
    bytes[9] === 0x45 &&
    bytes[10] === 0x42 &&
    bytes[11] === 0x50
  )
    return 'webp';
  return null;
}

function dataUrlABytes(dataUrl: string): { bytes: ArrayBuffer; extension: string } {
  const coma = dataUrl.indexOf(',');
  const bin = atob(dataUrl.slice(coma + 1));
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return { bytes: bytes.buffer as ArrayBuffer, extension: extensionPorMagia(bytes) ?? extensionDeDataUrl(dataUrl) };
}

async function fotoABytes(foto: string): Promise<{ bytes: ArrayBuffer; extension: string }> {
  if (foto.startsWith('data:image/')) return dataUrlABytes(foto);
  // URL ya subida (o externa): se descarga para re-subirla en su posición.
  const r = await fetch(foto);
  if (!r.ok) throw new ErrorApi(`No se pudo leer la foto para subirla (HTTP ${r.status}).`, r.status);
  const tipo = (r.headers.get('content-type') || '').split(';')[0].trim();
  const extension = MIME_A_EXTENSION[tipo] || 'jpg';
  return { bytes: await r.arrayBuffer(), extension };
}

interface RespuestaSubida {
  url: string;
}

async function subirFotoBytes(
  inmuebleId: string,
  bytes: ArrayBuffer,
  extension: string,
  origen: string,
  orden: number,
): Promise<string> {
  const token = leerToken();
  const nombre = `foto-${orden + 1}.${extension}`;
  const ruta = `/api/admin/fotos/upload?inmueble_id=${encodeURIComponent(inmuebleId)}&filename=${encodeURIComponent(nombre)}&origen=${origen}&orden=${orden}`;
  let respuesta: Response;
  try {
    respuesta = await fetch(`${API_URL}${ruta}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/octet-stream', ...(token ? { Authorization: `Bearer ${token}` } : {}) },
      body: bytes,
    });
  } catch {
    throw new ErrorApi('No se pudo contactar con la API al subir fotos.', 0);
  }
  if (respuesta.status === 401) {
    avisarSesionExpirada();
    throw new ErrorApi('Sesión caducada o inválida: vuelve a entrar.', 401);
  }
  if (!respuesta.ok) {
    let detalle = '';
    try {
      const c = (await respuesta.json()) as CuerpoError;
      detalle = (typeof c.message === 'string' && c.message) || (typeof c.error === 'string' && c.error) || '';
    } catch {
      detalle = '';
    }
    throw new ErrorApi(detalle || `La subida devolvió ${respuesta.status}.`, respuesta.status);
  }
  const cuerpo = (await respuesta.json()) as RespuestaSubida;
  return urlAbsoluta(cuerpo.url);
}

/* Sube una mejorada recién producida al servidor (origen=mejorada): la
 * cola la llama al completar para que la mejora no viva solo en el
 * navegador. Idempotente: si ya hay mejorada en ese `orden` no duplica.
 * Lanza `ErrorApi` si falla: el llamador conserva la copia local. */
export async function subirMejorada(inmuebleId: string, mejorada: string, orden: number): Promise<string> {
  if (!mejorada.startsWith('data:image/')) return mejorada; // ya es URL del servidor
  const actual = await obtenerRemoto(inmuebleId);
  const existente = actual.fotos.find((f) => f.origen === 'mejorada' && f.orden === orden);
  if (existente) return urlAbsoluta(existente.url);
  const { bytes, extension } = dataUrlABytes(mejorada);
  return subirFotoBytes(inmuebleId, bytes, extension, 'mejorada', orden);
}

/* Iguala los originales del servidor con `deseadas` (dataURL nuevas +
 * URLs conservadas): si ya coinciden en orden no toca nada; si cambian,
 * borra originales y mejoradas y los re-sube en orden. Las mejoradas
 * siguen a sus originales (ver [199A-6] abajo): sin ese remapeo la portada
 * pública (mejorada `orden=0`) mostraba la mejora de otra foto tras elegir
 * una principal distinta. `cambiaron` avisa al llamador para invalidar las
 * copias locales de mejora (describen fotos que ya no existen). Devuelve
 * también las mejoradas vigentes con sus URLs nuevas: tras re-subir, las
 * URLs viejas ya no existen y el llamador debe adoptar estas (si conserva
 * las anteriores, la publicidad y la tira dibujan imágenes borradas o, por
 * el caché del canvas, las fotos viejas). */
export async function sincronizarFotos(
  inmuebleId: string,
  deseadas: string[],
): Promise<{ fotos: string[]; mejoradas: MejoraServidor[]; cambiaron: boolean }> {
  const actual = await obtenerRemoto(inmuebleId);
  const ordenadas = [...actual.fotos]
    .filter((f) => f.origen !== 'mejorada')
    .sort((a, b) => a.orden - b.orden);
  const mejoradas = [...actual.fotos]
    .filter((f) => f.origen === 'mejorada')
    .sort((a, b) => a.orden - b.orden);
  const deseadasRel = deseadas.map(relativa);
  const iguales =
    ordenadas.length === deseadasRel.length &&
    ordenadas.every((f, i) => relativa(urlAbsoluta(f.url)) === deseadasRel[i]);
  if (iguales) {
    return {
      fotos: ordenadas.map((f) => urlAbsoluta(f.url)),
      mejoradas: mejoradas.map((m) => ({ id: m.id, orden: m.orden, url: urlAbsoluta(m.url) })),
      cambiaron: false,
    };
  }
  /* [199A-5] Primero se leen los bytes de TODAS las deseadas, cuando las
   * URLs conservadas aún existen en el servidor. Si alguna falla no se ha
   * borrado nada. Antes se borraba primero y la re-descarga devolvía 404
   * ("No se pudo leer la foto para subirla") dejando el inmueble sin fotos. */
  const nuevas: { bytes: ArrayBuffer; extension: string }[] = [];
  for (let i = 0; i < deseadas.length; i++) {
    nuevas.push(await fotoABytes(deseadas[i]));
  }
  /* [199A-6] Las mejoradas también se descargan antes de borrar y siguen a
   * sus originales: si el original emparejado por `orden` queda en la
   * posición N, su mejorada se re-sube con `orden=N`. La mejorada de un
   * original eliminado se descarta (su foto ya no existe). */
  const viejasRel = ordenadas.map((f) => relativa(urlAbsoluta(f.url)));
  const seguidoras: { bytes: ArrayBuffer; extension: string; orden: number }[] = [];
  for (const m of mejoradas) {
    const viejaPos = ordenadas.findIndex((f) => f.orden === m.orden);
    if (viejaPos < 0) continue;
    const nuevaPos = deseadasRel.indexOf(viejasRel[viejaPos]);
    if (nuevaPos < 0) continue;
    const { bytes, extension } = await fotoABytes(urlAbsoluta(m.url));
    seguidoras.push({ bytes, extension, orden: nuevaPos });
  }
  for (const f of [...ordenadas, ...mejoradas]) {
    await apiFetch<unknown>(`/api/admin/fotos/${encodeURIComponent(f.id)}`, { method: 'DELETE' });
  }
  for (let i = 0; i < nuevas.length; i++) {
    const { bytes, extension } = nuevas[i];
    await subirFotoBytes(inmuebleId, bytes, extension, 'original', i);
  }
  for (const s of seguidoras.sort((a, b) => a.orden - b.orden)) {
    await subirFotoBytes(inmuebleId, s.bytes, s.extension, 'mejorada', s.orden);
  }
  /* [259A-1] La subida solo devuelve la URL, no el id de fila: se re-lee
   * el inmueble para devolver fotos y mejoradas con sus ids reales (los
   * necesita el botón Restaurar y el panel muestra los IDs). */
  const fresco = remotoADominio(await obtenerRemoto(inmuebleId));
  return { fotos: fresco.fotos, mejoradas: fresco.mejoradasServidor, cambiaron: true };
}

/* Restaura el original de una foto: borra su mejorada del servidor y
 * devuelve el inmueble fresco (con `idsFotos` y `mejoradasServidor`
 * vigentes). Solo borra, nunca re-sube: el original sigue intacto.
 * Lanza `ErrorApi` si ya no hay mejorada en ese `orden`. */
export async function eliminarMejorada(inmuebleId: string, orden: number): Promise<Inmueble> {
  const actual = await obtenerRemoto(inmuebleId);
  const objetivo = actual.fotos.find((f) => f.origen === 'mejorada' && f.orden === orden);
  if (!objetivo) throw new ErrorApi('Esa foto ya no tiene mejorada en el servidor.', 404);
  await apiFetch<unknown>(`/api/admin/fotos/${encodeURIComponent(objetivo.id)}`, { method: 'DELETE' });
  return remotoADominio(await obtenerRemoto(inmuebleId));
}
