// [159A-2] Importa el rescate del admin local a la API Rust.
// Uso: node scripts/importar-rescate.mjs [--api URL] [--email E] [--password P]
//      [--rescate RUTA] [--dry-run]
// Importados con publicado=false (el owner los revisa y publica).
// Idempotente a medias: si el email ya existe hace login; los inmuebles se
// crean siempre (re-ejecutar duplica: borrar antes en BD si hace falta).

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const RAIZ = dirname(fileURLToPath(import.meta.url));
const args = Object.fromEntries(
  process.argv.slice(2).map((a, i, arr) => (a.startsWith('--') ? [a.slice(2), arr[i + 1] ?? 'true'] : [])).filter((p) => p.length),
);

const API = args.api ?? 'http://127.0.0.1:3110'; // [03AA-1] local en 3110.
const EMAIL = args.email ?? 'import@example.com';
const PASSWORD = args.password ?? 'import-secreto-123';
const RESCATE = args.rescate ?? join(RAIZ, '..', '..', 'INMOBILIARIA', 'rescates', 'inmobiliaria-rescate-20260915-1205.json');
const DRY = args['dry-run'] === 'true' || args['dry-run'] === '';

const resumen = { inmuebles: 0, originales: 0, mejoradas: 0, omitidas: 0 };

async function api(ruta, { metodo = 'GET', token, json, bytes } = {}) {
  const cabeceras = {};
  if (token) cabeceras.Authorization = `Bearer ${token}`;
  let cuerpo;
  if (json !== undefined) {
    cabeceras['Content-Type'] = 'application/json';
    cuerpo = JSON.stringify(json);
  } else if (bytes) {
    cabeceras['Content-Type'] = 'application/octet-stream';
    cuerpo = bytes;
  }
  const res = await fetch(`${API}${ruta}`, { method: metodo, headers: cabeceras, body: cuerpo });
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

function dataUrlABytes(dataUrl) {
  const m = /^data:image\/[a-z+]+;base64,(.+)$/s.exec(dataUrl);
  if (!m) throw new Error(`dataURL no soportada: ${dataUrl.slice(0, 40)}`);
  const bytes = Buffer.from(m[1], 'base64');
  // La extensión se deduce de los bytes reales (igual que tests/importar_rescate.rs):
  // alguna mejorada declara en la cabecera un formato distinto al contenido
  // y el servidor exige coherencia (magic-bytes).
  const png = [137, 80, 78, 71, 13, 10, 26, 10];
  let ext;
  if (png.every((b, i) => bytes[i] === b)) ext = 'png';
  else if (bytes[0] === 82 && bytes[1] === 73 && bytes[2] === 70 && bytes[3] === 70
    && bytes[8] === 87 && bytes[9] === 69 && bytes[10] === 66 && bytes[11] === 80) ext = 'webp';
  else if (bytes[0] === 255 && bytes[1] === 216 && bytes[2] === 255) ext = 'jpg';
  else throw new Error(`magia de imagen desconocida: ${bytes.subarray(0, 4).join(',')}`);
  return { ext, bytes };
}

async function main() {
  const rescate = JSON.parse(readFileSync(RESCATE, 'utf8'));
  const valor = (v) => (typeof v === 'string' ? JSON.parse(v) : v);
  const inmuebles = valor(rescate.localStorage['inmobiliaria:inmuebles:v1']);
  const fotosPorInmueble = Object.fromEntries(
    rescate.indexeddb['fotos-inmueble'].map((r) => [r.inmuebleId, r.fotos]),
  );
  const mejoras = rescate.indexeddb['fotos-mejora'];
  console.log(`Rescate: ${inmuebles.length} inmuebles, ${mejoras.length} registros de mejora`);

  // Auth: registro (bootstrap del primer owner) o login si ya existe
  let token;
  try {
    const r = await api('/api/auth/register', { metodo: 'POST', json: { email: EMAIL, password: PASSWORD } });
    token = r.token;
    console.log(`Registrado ${EMAIL}`);
  } catch (e) {
    if (!String(e.message).includes('→ 403')) throw e;
    const r = await api('/api/auth/login', { metodo: 'POST', json: { email: EMAIL, password: PASSWORD } });
    token = r.token;
    console.log(`Login ${EMAIL}`);
  }
  if (DRY) {
    console.log('[dry-run] auth OK, nada más que hacer');
    return;
  }

  for (const [indice, viejo] of inmuebles.entries()) {
    const payload = {
      titulo: viejo.titulo ?? '',
      descripcion: viejo.descripcion ?? '',
      ubicacion: viejo.ubicacion ?? '',
      precio: viejo.precio ?? 0,
      tipo: viejo.tipo ?? 'otro',
      operacion: viejo.operacion ?? 'venta',
      habitaciones: viejo.habitaciones ?? 0,
      banos: viejo.banos ?? 0,
      metros: viejo.metros ?? 0,
      metros_terreno: viejo.metrosTerreno ?? 0,
      estado: viejo.estado ?? 'disponible',
    };
    if (viejo.copy?.corta && viejo.copy?.larga) {
      payload.copy = {
        corta: viejo.copy.corta,
        larga: viejo.copy.larga,
        modelo: viejo.copy.modelo ?? 'desconocido',
        actualizada_en: viejo.copy.actualizadaEn ?? new Date().toISOString(),
      };
    }
    const creado = await api('/api/admin/inmuebles', { metodo: 'POST', token, json: payload });
    const mapa = { viejo: viejo.id, nuevo: creado.id };
    resumen.inmuebles += 1;

    const originales = fotosPorInmueble[viejo.id] ?? [];
    for (const [i, dataUrl] of originales.entries()) {
      const { ext, bytes } = dataUrlABytes(dataUrl);
      await api(
        `/api/admin/fotos/upload?inmueble_id=${creado.id}&filename=foto-${i}.${ext}&origen=original&orden=${i}`,
        { metodo: 'POST', token, bytes },
      );
      resumen.originales += 1;
    }
    for (const m of mejoras.filter((r) => r.inmuebleId === viejo.id)) {
      if (m.estado !== 'lista' || !m.mejorada) {
        resumen.omitidas += 1;
        continue;
      }
      const orden = originales.indexOf(m.original);
      if (orden === -1) throw new Error(`Mejorada sin original coincidente (${m.id})`);
      const { ext, bytes } = dataUrlABytes(m.mejorada);
      await api(
        `/api/admin/fotos/upload?inmueble_id=${creado.id}&filename=mejorada-${orden}.${ext}&origen=mejorada&orden=${orden}`,
        { metodo: 'POST', token, bytes },
      );
      resumen.mejoradas += 1;
    }
    console.log(`[${indice + 1}/${inmuebles.length}] ${mapa.nuevo} ← ${mapa.viejo.slice(0, 8)} (${originales.length} originales)`);
  }

  // Verificación: 5 en admin, 0 en pública (publicado=false)
  const admin = await api('/api/admin/inmuebles?page=1&per_page=100', { token });
  const publica = await api('/api/public/inmuebles?page=1&per_page=100');
  const fotosAdmin = admin.items.reduce((n, i) => n + i.fotos.length, 0);
  console.log(`Admin: ${admin.total} inmuebles, ${fotosAdmin} fotos. Pública: ${publica.total}.`);
  console.log(JSON.stringify(resumen));
  if (admin.total !== inmuebles.length || publica.total !== 0) {
    throw new Error('Verificación fallida: totales inesperados');
  }
}

main().catch((e) => {
  console.error(`FALLO: ${e.message}`);
  process.exit(1);
});
