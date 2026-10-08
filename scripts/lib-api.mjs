// [08AA-27] Ayudas HTTP compartidas para los scripts de MN-Inmobiliaria.
// JWT solo en memoria; nada se loguea salvo longitudes/códigos.
import { existsSync, readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

export const AQUI = dirname(fileURLToPath(import.meta.url));
export const ENV_DEFECTO = join(AQUI, '.env.prod.local');

export function leerEnv(ruta, requeridas) {
  if (!existsSync(ruta)) {
    console.error(`FALLO preflight: no existe ${ruta}.`);
    process.exit(2);
  }
  const cfg = {};
  for (const linea of readFileSync(ruta, 'utf8').split('\n')) {
    const l = linea.trim();
    if (!l || l.startsWith('#')) continue;
    const i = l.indexOf('=');
    if (i > 0) cfg[l.slice(0, i).trim()] = l.slice(i + 1).trim();
  }
  for (const k of requeridas) {
    if (!cfg[k]) {
      console.error(`FALLO preflight: falta ${k} en ${ruta}.`);
      process.exit(2);
    }
  }
  return cfg;
}

export async function api(base, ruta, { metodo = 'GET', token, json, bytes } = {}) {
  const cabeceras = {};
  if (token) cabeceras.Authorization = `Bearer ${token}`;
  let cuerpo;
  if (json !== undefined) {
    cabeceras['Content-Type'] = 'application/json';
    cuerpo = JSON.stringify(json);
  } else if (bytes && bytes !== true) {
    cabeceras['Content-Type'] = 'application/octet-stream';
    cuerpo = bytes;
  }
  const res = await fetch(`${base}${ruta}`, {
    method: metodo,
    headers: cabeceras,
    body: cuerpo,
    signal: AbortSignal.timeout(60000),
  });
  if (bytes === true) {
    if (!res.ok) throw new Error(`${metodo} ${ruta} → ${res.status}`);
    return Buffer.from(await res.arrayBuffer());
  }
  const texto = await res.text();
  let datos = null;
  try {
    datos = texto ? JSON.parse(texto) : null;
  } catch {
    throw new Error(`${metodo} ${ruta} → respuesta no JSON: ${texto.slice(0, 120)}`);
  }
  if (!res.ok) throw new Error(`${metodo} ${ruta} → ${res.status}: ${texto.slice(0, 200)}`);
  return datos;
}

export async function login(base, email, password, quien, envRuta) {
  const r = await api(base, '/api/auth/login', {
    metodo: 'POST',
    json: { email, password },
  }).catch(() => {
    console.error(`FALLO auth ${quien}: revisa email/password en ${envRuta}.`);
    process.exit(2);
  });
  return r.token;
}

/* Núcleo de alta/actualización: los campos que manda el servidor (el slug lo
 * genera desde el título; el emparejamiento siempre es por slug, nunca por id). */
export function nucleo(p) {
  return {
    titulo: p.titulo, descripcion: p.descripcion, ubicacion: p.ubicacion,
    puestos: p.puestos, residencia: p.residencia, precio: p.precio,
    tipo: p.tipo, operacion: p.operacion, habitaciones: p.habitaciones,
    banos: p.banos, metros: p.metros, metros_terreno: p.metros_terreno,
    estado: p.estado, copy: p.copy ?? null, receta: p.receta ?? null,
  };
}

/* Magia por BYTES (nunca por extensión). Devuelve 'jpg'|'png'|'webp' o null. */
export function magia(buf) {
  if (buf.length >= 3 && buf[0] === 0xff && buf[1] === 0xd8 && buf[2] === 0xff) return 'jpg';
  if (
    buf.length >= 8 && buf[0] === 0x89 && buf[1] === 0x50 && buf[2] === 0x4e &&
    buf[3] === 0x47 && buf[4] === 0x0d && buf[5] === 0x0a && buf[6] === 0x1a && buf[7] === 0x0a
  ) return 'png';
  if (
    buf.length >= 12 && buf.toString('ascii', 0, 4) === 'RIFF' &&
    buf.toString('ascii', 8, 12) === 'WEBP'
  ) return 'webp';
  return null;
}
