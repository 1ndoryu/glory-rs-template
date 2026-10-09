// [08AA-27] CLI único para publicar/gestionar inmuebles (local + prod).
// Uso:
//   node scripts/inmueble.mjs estado --slug <slug>
//   node scripts/inmueble.mjs publicar --fotos <carpeta> --datos '{"titulo":...}' [--borrador] [--solo-local|--solo-prod] [--sobrescribir]
//   node scripts/inmueble.mjs verificar [--slug <slug>] [--sin-bytes] [--par N]
// --datos acepta JSON inline o @ruta.json. --dry-run muestra sin escribir.
// Exit: 0 ok | 1 verificar con diferencias | 2 preflight/auth | 3 error de sync | 4 verificación fallida.
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { api, leerEnv, login, magia, nucleo, ENV_DEFECTO, AQUI } from './lib-api.mjs';

const REQUERIDAS = ['PROD_BASE', 'PROD_EMAIL', 'PROD_PASSWORD', 'LOCAL_EMAIL', 'LOCAL_PASSWORD'];
const MAX_BYTES = 10 * 1024 * 1024;

function args() {
  const cmd = process.argv[2];
  const o = {};
  const resto = process.argv.slice(3);
  for (let i = 0; i < resto.length; i++) {
    const a = resto[i];
    if (!a.startsWith('--')) continue;
    const k = a.slice(2);
    const v = resto[i + 1] !== undefined && !resto[i + 1].startsWith('--') ? resto[++i] : true;
    o[k] = v;
  }
  return { cmd, o };
}

function leerDatos(v) {
  if (v === undefined) {
    console.error('FALLO preflight: falta --datos (JSON o @ruta.json).');
    process.exit(2);
  }
  const texto = String(v).startsWith('@') ? readFileSync(String(v).slice(1), 'utf8') : String(v);
  try {
    return JSON.parse(texto);
  } catch {
    console.error('FALLO preflight: --datos no es JSON válido.');
    process.exit(2);
  }
}

/* Preflight de fotos: devuelve [{ruta, ext, bytes}] con EXIF ya pelado en C:\tmp.
 * Nunca publica con GPS: si PIL falla, se aborta (nada silencioso). */
function prepararFotos(carpeta) {
  if (!carpeta || !existsSync(carpeta) || !statSync(carpeta).isDirectory()) {
    console.error(`FALLO preflight: carpeta de fotos inexistente: ${carpeta}`);
    process.exit(2);
  }
  const ficheros = readdirSync(carpeta)
    .filter((f) => statSync(join(carpeta, f)).isFile())
    .sort((a, b) => a.localeCompare(b, 'es'));
  if (!ficheros.length) {
    console.error(`FALLO preflight: sin archivos en ${carpeta}`);
    process.exit(2);
  }
  const tmp = join('C:\\tmp', `inmueble-${Date.now()}`);
  mkdirSync(tmp, { recursive: true });
  const listas = [];
  for (const f of ficheros) {
    const ruta = join(carpeta, f);
    const buf = readFileSync(ruta);
    if (buf.length > MAX_BYTES) {
      console.error(`FALLO preflight: ${f} supera 10 MiB (${(buf.length / 1048576).toFixed(1)} MiB).`);
      process.exit(2);
    }
    const ext = magia(buf);
    if (!ext) {
      console.error(`FALLO preflight: ${f} no es JPEG/PNG/WebP (¿HEIC? pide reemplazo).`);
      process.exit(2);
    }
    const limpia = join(tmp, `${listas.length}.${ext}`);
    try {
      const salida = execFileSync('python', [join(AQUI, 'pelar-exif.py'), ruta, limpia], { encoding: 'utf8' });
      const info = JSON.parse(salida.trim().split('\n').pop());
      if (info.gps) console.log(`aviso ${f}: traía GPS, pelado antes de subir`);
    } catch (e) {
      console.error(`FALLO EXIF ${f}: sin pelado no se publica (privacidad). ${e.message.split('\n')[0]}`);
      process.exit(2);
    }
    listas.push({ ruta: limpia, ext, nombre: f });
  }
  return { listas, tmp };
}

async function listar(base, token) {
  return (await api(base, '/api/admin/inmuebles?page=1&per_page=200', { token })).items;
}

function porTitulo(items, titulo) {
  const t = String(titulo ?? '').trim().toLowerCase();
  return items.find((i) => String(i.titulo ?? '').trim().toLowerCase() === t) ?? null;
}

function diffNucleo(a, b) {
  const na = nucleo(a);
  const nb = nucleo(b);
  return Object.keys(na).filter((k) => JSON.stringify(na[k]) !== JSON.stringify(nb[k]));
}

/* Sincroniza UN destino: crea o (con --sobrescribir) reemplaza + fotos + publica. */
async function sincronizarDestino(base, quien, token, datos, fotos, { borrador, sobrescribir, dry }) {
  const items = await listar(base, token);
  const existente = porTitulo(items, datos.titulo);
  if (existente && !sobrescribir) {
    const dif = diffNucleo(existente, { ...existente, ...datos });
    console.error(
      `FALLO ${quien}: ya existe "${datos.titulo}" (slug ${existente.slug})` +
      (dif.length ? `, difiere en: ${dif.join(', ')}` : ', idéntico') +
      '. Reejecuta con --sobrescribir para reemplazar.',
    );
    process.exit(3);
  }
  if (dry) {
    console.log(`[dry-run] ${quien}: ${existente ? 'reemplazar' : 'crear'} "${datos.titulo}" + ${fotos.length} fotos`);
    return { id: existente?.id ?? null, slug: existente?.slug ?? null, seco: true };
  }
  let id;
  if (!existente) {
    const creado = await api(base, '/api/admin/inmuebles', { metodo: 'POST', token, json: datos });
    id = creado.id;
    console.log(`OK ${quien}: creado ${creado.slug} (${id})`);
  } else {
    await api(base, `/api/admin/inmuebles/${existente.id}`, { metodo: 'PUT', token, json: { ...nucleo(existente), ...datos } });
    id = existente.id;
    console.log(`OK ${quien}: reemplazado ${existente.slug}`);
    const actuales = (await listar(base, token)).find((i) => i.id === id);
    for (const f of actuales.fotos) {
      await api(base, `/api/admin/fotos/${f.id}`, { metodo: 'DELETE', token });
    }
  }
  // Fotos: originales con orden 0..N (bytes ya limpios de EXIF).
  let orden = 0;
  for (const f of fotos) {
    const bytes = readFileSync(f.ruta);
    await api(base, `/api/admin/fotos/upload?inmueble_id=${id}&filename=${encodeURIComponent(`foto-${orden + 1}.${f.ext}`)}&origen=original&orden=${orden}`, {
      metodo: 'POST', token, bytes,
    });
    orden += 1;
  }
  if (!borrador) {
    await api(base, `/api/admin/inmuebles/${id}/publicacion`, { metodo: 'PATCH', token, json: { publicado: true } });
  }
  // Verificación del destino.
  const fin = (await listar(base, token)).find((i) => i.id === id);
  const pub = await api(base, `/api/public/inmuebles/${fin.slug}`).catch(() => null);
  const okFotos = fin.fotos.length === fotos.length;
  const okPub = borrador ? true : pub !== null && pub.publicado !== false;
  console.log(`${quien}: ${fin.slug} fotos=${fin.fotos.length}/${fotos.length} publico=${pub !== null}`);
  if (!okFotos || !okPub) {
    console.error(`FALLO verificación en ${quien}.`);
    process.exit(4);
  }
  return { id, slug: fin.slug };
}

async function cmdEstado(o) {
  if (!o.slug) {
    console.error('FALLO preflight: falta --slug.');
    process.exit(2);
  }
  const cfg = leerEnv(o.env ?? ENV_DEFECTO, REQUERIDAS);
  const LOCAL = (cfg.LOCAL_API ?? 'http://127.0.0.1:3110').replace(/\/$/, '');
  const PROD = cfg.PROD_BASE.replace(/\/$/, '');
  const tL = await login(LOCAL, cfg.LOCAL_EMAIL, cfg.LOCAL_PASSWORD, 'local', o.env ?? ENV_DEFECTO);
  const tP = await login(PROD, cfg.PROD_EMAIL, cfg.PROD_PASSWORD, 'prod', o.env ?? ENV_DEFECTO);
  for (const [quien, base, token] of [['local', LOCAL, tL], ['prod', PROD, tP]]) {
    const items = await listar(base, token);
    const it = items.find((i) => i.slug === o.slug);
    if (!it) {
      console.log(`${quien}: slug ${o.slug} NO existe`);
      continue;
    }
    const pub = await api(base, `/api/public/inmuebles/${o.slug}`).catch(() => null);
    const porOrigen = {};
    for (const f of it.fotos) porOrigen[f.origen] = (porOrigen[f.origen] ?? 0) + 1;
    console.log(`${quien}: "${it.titulo}" publicado=${it.publicado} fotos=${it.fotos.length} (${JSON.stringify(porOrigen)}) publico=${pub !== null}`);
  }
}

async function cmdPublicar(o) {
  const datos = leerDatos(o.datos);
  if (!datos.titulo || !String(datos.titulo).trim()) {
    console.error('FALLO preflight: --datos.titulo es obligatorio.');
    process.exit(2);
  }
  const cfg = leerEnv(o.env ?? ENV_DEFECTO, REQUERIDAS);
  const LOCAL = (cfg.LOCAL_API ?? 'http://127.0.0.1:3110').replace(/\/$/, '');
  const PROD = cfg.PROD_BASE.replace(/\/$/, '');
  const soloLocal = o['solo-local'] === true || o['solo-local'] === '';
  const soloProd = o['solo-prod'] === true || o['solo-prod'] === '';
  const dry = o['dry-run'] === true || o['dry-run'] === '';
  const { listas, tmp } = prepararFotos(o.fotos);
  console.log(`Fotos listas: ${listas.length} (EXIF pelado en ${tmp})`);
  try {
    if (!soloProd) {
      const tL = await login(LOCAL, cfg.LOCAL_EMAIL, cfg.LOCAL_PASSWORD, 'local', o.env ?? ENV_DEFECTO);
      await sincronizarDestino(LOCAL, 'local', tL, datos, listas, { borrador: !!o.borrador, sobrescribir: !!o.sobrescribir, dry });
    }
    if (!soloLocal) {
      const tP = await login(PROD, cfg.PROD_EMAIL, cfg.PROD_PASSWORD, 'prod', o.env ?? ENV_DEFECTO);
      await sincronizarDestino(PROD, 'prod', tP, datos, listas, { borrador: !!o.borrador, sobrescribir: !!o.sobrescribir, dry });
    }
    console.log(dry ? 'dry-run OK (nada escrito)' : 'Publicación OK en ' + (soloLocal ? 'local' : soloProd ? 'prod' : 'local+prod'));
  } finally {
    if (!dry) rmSync(tmp, { recursive: true, force: true });
  }
}

const PROMPT_DEFECTO = 'Mejora esta foto inmobiliaria a fotografia profesional: sube nitidez y resolucion, corrige luz y color de forma natural, limpia suciedad leve en paredes, telas u objetos y retira personas y distracciones menores. Haz la imagen ultra HD, mas grande, con mas definicion: maxima resolucion y detalle que puedas devolver. No cambies la estructura, geometria, muebles ni elementos fijos; no inventes nada que no exista en la foto original.';

const dormir = (ms) => new Promise((r) => setTimeout(r, ms));

function aDataUrl(bytes, ext) {
  const mime = ext === 'png' ? 'image/png' : ext === 'webp' ? 'image/webp' : 'image/jpeg';
  return `data:${mime};base64,${Buffer.from(bytes).toString('base64')}`;
}

function deDataUrl(dataUrl) {
  const m = /^data:(image\/[a-zA-Z0-9.+-]+);base64,(.+)$/.exec(dataUrl || '');
  if (!m) return null;
  const ext = m[1] === 'image/png' ? 'png' : m[1] === 'image/webp' ? 'webp' : 'jpg';
  return { ext, bytes: Buffer.from(m[2], 'base64') };
}

/* El worker guarda lo que Gemini devuelve con extensión .png sin validar:
 * a veces no es PNG real (el frontend lo tapa al re-codificar en canvas).
 * Aquí se re-codifica a JPG con PIL. */
function normalizarJpg(bytes) {
  const base = join(tmpdir(), `mejorada-${Date.now()}-${Math.floor(Math.random() * 1e6)}`);
  try {
    writeFileSync(`${base}.bin`, bytes);
    execFileSync('python', ['-c',
      'import sys; from PIL import Image; ' +
      `img = Image.open(${JSON.stringify(`${base}.bin`)}).convert("RGB"); ` +
      `img.save(${JSON.stringify(`${base}.jpg`)}, "JPEG", quality=92)`]);
    return readFileSync(`${base}.jpg`);
  } finally {
    rmSync(`${base}.bin`, { force: true });
    rmSync(`${base}.jpg`, { force: true });
  }
}

async function saludMejora(mejoraBase) {
  try {
    const r = await fetch(`${mejoraBase}/api/salud`, { signal: AbortSignal.timeout(10000) });
    if (!r.ok) return null;
    return await r.json();
  } catch {
    return null;
  }
}

/* Token autónomo: salud → (sin puerto debug? abro el Chrome yo) → renovar →
 * rechequeo. Solo molesta a ella si falta login/cuenta (códigos 3/4). */
async function asegurarToken(o, mejoraBase) {
  let s = await saludMejora(mejoraBase);
  if (s?.listo) {
    console.log('Mejora lista (token válido).');
    return;
  }
  console.log(`Token no listo (${s?.detalle ?? 'sin servidor de mejora'}); renuevo solo…`);
  const renovar = join(AQUI, '..', 'frontend', 'scripts', 'renovar-cookies.mjs');
  const correr = () => spawnSync('node', [renovar], { encoding: 'utf8' });
  let r = correr();
  if (r.status === 2) {
    // Sin puerto debug: abro yo el Chrome dedicado y reintento una vez.
    console.log('Abro el Chrome de mejora yo…');
    try {
      const exe = `${process.env.ProgramFiles}\\Google\\Chrome\\Application\\chrome.exe`;
      const perfil = `${process.env.LOCALAPPDATA}\\ChromeHoracioDebug`;
      execFileSync(exe, [`--remote-debugging-port=9223`, `--user-data-dir=${perfil}`, 'https://gemini.google.com/app'], { stdio: 'ignore' });
      await dormir(10000);
      r = correr();
    } catch (e) {
      console.error(`FALLO: no pude abrir chrome.exe solo (${e.message.split('\n')[0]}). Abre el acceso directo "Chrome-Horacio-debug" del escritorio y reejecuta.`);
      process.exit(2);
    }
  }
  if (r.status === 0) {
    s = await saludMejora(mejoraBase);
    if (s?.listo) {
      console.log('Token renovado solo, sigo sin molestar.');
      return;
    }
  }
  console.error(
    'FALLO token: necesito tu ayuda una vez — abre el acceso directo "Chrome-Horacio-debug" del escritorio' +
    (r.status === 3 ? ' e inicia sesión en Google (cuenta Horacio Cruz)' : '') +
    ', y reejecuta. Las originales ya están publicadas; la mejora espera.',
  );
  process.exit(2);
}

async function cmdMejorar(o) {
  if (!o.slug) {
    console.error('FALLO preflight: falta --slug.');
    process.exit(2);
  }
  const mejoraBase = (o.mejora ?? 'http://127.0.0.1:3122').replace(/\/$/, '');
  await asegurarToken(o, mejoraBase);
  const limite = Math.max(1, Number(o.limite ?? 40) || 40);
  const repetir = o.repetir === true || o.repetir === '';
  const cfg = leerEnv(o.env ?? ENV_DEFECTO, REQUERIDAS);
  const LOCAL = (cfg.LOCAL_API ?? 'http://127.0.0.1:3110').replace(/\/$/, '');
  const tL = await login(LOCAL, cfg.LOCAL_EMAIL, cfg.LOCAL_PASSWORD, 'local', o.env ?? ENV_DEFECTO);
  const items = await listar(LOCAL, tL);
  const it = items.find((i) => i.slug === o.slug);
  if (!it) {
    console.error(`FALLO: slug ${o.slug} no existe en local.`);
    process.exit(2);
  }
  const originales = it.fotos.filter((f) => f.origen === 'original').sort((a, b) => a.orden - b.orden);
  const mejoradas = new Set(it.fotos.filter((f) => f.origen === 'mejorada').map((f) => f.orden));
  const pendientes = originales.filter((f) => repetir || !mejoradas.has(f.orden)).slice(0, limite);
  console.log(`${o.slug}: ${originales.length} originales, ${mejoradas.size} mejoradas, pendientes ${pendientes.length}`);
  let hechas = 0;
  for (const f of pendientes) {
    if (repetir) {
      // Re-mejora: borra la mejorada vieja del mismo orden para no duplicar el pareo.
      const vieja = it.fotos.find((x) => x.origen === 'mejorada' && x.orden === f.orden);
      if (vieja) {
        await api(LOCAL, `/api/admin/fotos/${vieja.id}`, { metodo: 'DELETE', token: tL });
        console.log(`orden ${f.orden}: mejorada anterior borrada`);
      }
    }
    const bytes = await api(LOCAL, f.url, { token: tL, bytes: true });
    const ext = magia(bytes) ?? 'jpg';
    const fotoId = `${it.id}:${f.orden}@${Date.now()}`; // Único por intento: evita reutilizar un `lista` viejo (F21).
    const post = await api(mejoraBase, '/api/mejora', {
      metodo: 'POST',
      json: { fotoId, original: aDataUrl(bytes, ext), prompt: PROMPT_DEFECTO },
    });
    console.log(`orden ${f.orden}: job ${post.jobId.slice(0, 8)}… encolado, espero (puede tardar minutos)`);
    const t0 = Date.now();
    for (;;) {
      await dormir(20000);
      const job = await api(mejoraBase, `/api/mejora/${post.jobId}`, {});
      if (job.estado === 'lista' && job.imagen) {
        const dec = deDataUrl(job.imagen);
        if (!dec) {
          console.error(`orden ${f.orden}: la mejora vino en formato raro, se reintentará luego`);
          break;
        }
        let up = dec.bytes;
        let upExt = dec.ext;
        // Siempre se normaliza a JPG: lo que devuelve Gemini no siempre es
        // un PNG decodificable aunque traiga la magia (el frontend lo tapa
        // al re-codificar en canvas). Si PIL no lo abre, se guarda el testigo.
        console.log(`orden ${f.orden}: mejora recibida (${(up.length / 1024).toFixed(0)} KB, magia=${magia(up) ?? 'ninguna'}), normalizo a JPG…`);
        try {
          up = normalizarJpg(up);
          upExt = 'jpg';
        } catch (e) {
          const testigo = join(tmpdir(), `mejorada-fallo-${f.orden}.bin`);
          try { writeFileSync(testigo, dec.bytes); } catch {}
          console.error(`orden ${f.orden}: bytes no abribles (${e.message.split('\n')[0]}); testigo en ${testigo}, sigo con la siguiente`);
          break;
        }
        await api(LOCAL, `/api/admin/fotos/upload?inmueble_id=${it.id}&filename=${encodeURIComponent(`mejorada-${f.orden + 1}.${upExt}`)}&origen=mejorada&orden=${f.orden}`, {
          metodo: 'POST', token: tL, bytes: up,
        });
        console.log(`orden ${f.orden}: mejorada subida (${(up.length / 1024).toFixed(0)} KB)`);
        hechas += 1;
        break;
      }
      if (job.estado === 'error') {
        console.error(`orden ${f.orden}: error final (${(job.error ?? '').slice(0, 120)}), sigo con la siguiente`);
        break;
      }
      if (Date.now() - t0 > 15 * 60 * 1000) {
        console.error(`orden ${f.orden}: 15 min sin terminar, sigo con la siguiente (el job sigue en cola)`);
        break;
      }
    }
  }
  const fin = (await listar(LOCAL, tL)).find((i) => i.id === it.id);
  console.log(`Mejora OK: ${hechas}/${pendientes.length} nuevas; total mejoradas=${fin.fotos.filter((x) => x.origen === 'mejorada').length}`);
}

/* F4: empuja un slug de local a prod (núcleo + estado + fotos faltantes).
 * Sin --dry-run no toca nada. Pareo de fotos por (origen, orden). */
async function cmdPush(o) {
  if (!o.slug) {
    console.error('FALLO preflight: falta --slug.');
    process.exit(2);
  }
  const seco = o['dry-run'] === true || o['dry-run'] === '';
  const sobrescribir = o.sobrescribir === true || o.sobrescribir === '';
  const cfg = leerEnv(o.env ?? ENV_DEFECTO, REQUERIDAS);
  const LOCAL = (cfg.LOCAL_API ?? 'http://127.0.0.1:3110').replace(/\/$/, '');
  const PROD = (cfg.PROD_BASE ?? 'https://mn-inmobiliaria.com').replace(/\/$/, '');
  const tL = await login(LOCAL, cfg.LOCAL_EMAIL, cfg.LOCAL_PASSWORD, 'local', o.env ?? ENV_DEFECTO);
  const tP = await login(PROD, cfg.PROD_EMAIL, cfg.PROD_PASSWORD, 'prod', o.env ?? ENV_DEFECTO);
  const loc = (await listar(LOCAL, tL)).find((i) => i.slug === o.slug);
  if (!loc) {
    console.error(`FALLO: slug ${o.slug} no existe en local.`);
    process.exit(2);
  }
  let pr = (await listar(PROD, tP)).find((i) => i.slug === o.slug) ?? null;
  const plan = [];
  if (!pr) {
    plan.push('crear inmueble en prod');
    if (!seco) {
      const creado = await api(PROD, '/api/admin/inmuebles', { metodo: 'POST', token: tP, json: {} });
      await api(PROD, `/api/admin/inmuebles/${creado.id}`, { metodo: 'PUT', token: tP, json: nucleo(loc) });
      pr = (await listar(PROD, tP)).find((i) => i.slug === o.slug);
    }
  } else {
    const dif = diffNucleo(pr, loc);
    if (dif.length || sobrescribir) {
      plan.push(`actualizar núcleo (${dif.length ? dif.join(',') : 'forzado'})`);
      if (!seco) await api(PROD, `/api/admin/inmuebles/${pr.id}`, { metodo: 'PUT', token: tP, json: nucleo(loc) });
    }
    if (pr.publicado !== loc.publicado) {
      plan.push(`publicado=${loc.publicado}`);
      if (!seco) await api(PROD, `/api/admin/inmuebles/${pr.id}/publicacion`, { metodo: 'PATCH', token: tP, json: { publicado: loc.publicado } });
    }
    if (sobrescribir && pr.fotos.length) {
      plan.push(`borrar ${pr.fotos.length} fotos de prod`);
      if (!seco) for (const f of pr.fotos) await api(PROD, `/api/admin/fotos/${f.id}`, { metodo: 'DELETE', token: tP });
      pr = { ...pr, fotos: [] };
    }
  }
  const enProd = new Set((pr?.fotos ?? []).map((f) => `${f.origen}:${f.orden}`));
  const faltan = loc.fotos.filter((f) => !enProd.has(`${f.origen}:${f.orden}`));
  plan.push(`subir ${faltan.length}/${loc.fotos.length} fotos`);
  if (!seco) {
    pr ??= (await listar(PROD, tP)).find((i) => i.slug === o.slug);
    for (const f of faltan) {
      const bytes = await api(LOCAL, f.url, { token: tL, bytes: true });
      const ext = magia(bytes) ?? 'jpg';
      await api(PROD, `/api/admin/fotos/upload?inmueble_id=${pr.id}&filename=${encodeURIComponent(`${f.origen}-${f.orden + 1}.${ext}`)}&origen=${f.origen}&orden=${f.orden}`, {
        metodo: 'POST', token: tP, bytes,
      });
    }
  }
  console.log(`${seco ? '[dry-run] ' : ''}push ${o.slug}: ${plan.join(' | ') || 'sin cambios'}`);
  if (!seco) {
    const fin = (await listar(PROD, tP)).find((i) => i.slug === o.slug);
    const pub = await fetch(`${PROD}/api/public/inmuebles/${o.slug}`, { signal: AbortSignal.timeout(15000) });
    console.log(`Push OK: prod=${fin.fotos.length} fotos publicado=${fin.publicado} publico=${pub.ok}`);
    if (!pub.ok || fin.fotos.length !== loc.fotos.length) {
      console.error('FALLO verificación post-push: revisa el slug en prod.');
      process.exit(1);
    }
  }
}

/* [08AA-35] F1: detector permanente prod↔local (solo lectura: login + GETs).
 * P1 metadatos: presencia por slug, núcleo canónico campo a campo, ficha útil
 * ({extras, precio_minimo}; inmueble_id se excluye: difiere por diseño),
 * publicado, set (origen,orden) + aserción de duplicados (sin UNIQUE en BD).
 * P2 bytes: sha256 de cada par (origen,orden) con pool acotado; las mejoradas
 * van aparte (el servidor re-codifica a JPG: mismo visual, distinto sha).
 * Exit 0 limpio | 1 con diferencias | 2 preflight/auth. Cero escrituras. */
const CAMPOS_VERIFICAR = [...Object.keys(nucleo({})), 'publicado'];

function canon(v) {
  if (v === null || v === undefined) return '';
  if (Array.isArray(v)) return v.map(canon);
  if (typeof v === 'object') return Object.fromEntries(Object.keys(v).sort().map((k) => [k, canon(v[k])]));
  if (typeof v === 'number') return String(v);
  if (typeof v === 'string') return v.trim();
  return v;
}

const shaFoto = (buf) => createHash('sha256').update(buf).digest('hex');
const relUrl = (url, base) => (url.startsWith('http') ? url.slice(base.length) : url);

async function pool(tareas, n, fn) {
  const res = new Array(tareas.length);
  let i = 0;
  await Promise.all(Array.from({ length: n }, async () => {
    while (i < tareas.length) {
      const j = i++;
      try { res[j] = await fn(tareas[j]); } catch (e) { res[j] = { __error: e.message }; }
    }
  }));
  return res;
}

async function cmdVerificar(o) {
  const soloSlug = o.slug ?? null;
  const sinBytes = o['sin-bytes'] === true || o['sin-bytes'] === '';
  const par = Math.max(1, Number(o.par ?? 6) || 6);
  const cfg = leerEnv(o.env ?? ENV_DEFECTO, REQUERIDAS);
  const LOCAL = (cfg.LOCAL_API ?? 'http://127.0.0.1:3110').replace(/\/$/, '');
  const PROD = (cfg.PROD_BASE ?? 'https://mn-inmobiliaria.com').replace(/\/$/, '');
  const tL = await login(LOCAL, cfg.LOCAL_EMAIL, cfg.LOCAL_PASSWORD, 'local', o.env ?? ENV_DEFECTO);
  const tP = await login(PROD, cfg.PROD_EMAIL, cfg.PROD_PASSWORD, 'prod', o.env ?? ENV_DEFECTO);
  const jL = (await listar(LOCAL, tL)).filter((x) => !soloSlug || x.slug === soloSlug);
  const jP = (await listar(PROD, tP)).filter((x) => !soloSlug || x.slug === soloSlug);
  const mapL = new Map(jL.map((x) => [x.slug, x]));
  const mapP = new Map(jP.map((x) => [x.slug, x]));
  const nf = (a) => a.reduce((n, x) => n + (x.fotos?.length ?? 0), 0);
  console.log(`verificar: prod=${jP.length}/${nf(jP)} local=${jL.length}/${nf(jL)}${soloSlug ? ` slug=${soloSlug}` : ''} bytes=${sinBytes ? 'no' : `sí(par=${par})`}`);
  let difs = 0;
  const marca = (m) => { console.log(m); difs++; };
  for (const s of new Set([...mapP.keys(), ...mapL.keys()])) {
    const p = mapP.get(s);
    const l = mapL.get(s);
    if (!p) { marca(`FALTA-PROD slug=${s}`); continue; }
    if (!l) { marca(`FALTA-LOCAL slug=${s}`); continue; }
    for (const c of CAMPOS_VERIFICAR) {
      if (JSON.stringify(canon(p[c])) !== JSON.stringify(canon(l[c]))) marca(`DIFIERE slug=${s} campo=${c}`);
    }
    const fichaP = await api(PROD, `/api/admin/inmuebles/${p.id}/ficha`, { token: tP }).catch(() => null);
    const fichaL = await api(LOCAL, `/api/admin/inmuebles/${l.id}/ficha`, { token: tL }).catch(() => null);
    const util = (f) => (f ? { extras: f.extras ?? {}, precio_minimo: f.precio_minimo ?? null } : null);
    if (JSON.stringify(canon(util(fichaP))) !== JSON.stringify(canon(util(fichaL)))) marca(`DIFIERE slug=${s} campo=ficha`);
    const setP = new Set((p.fotos ?? []).map((f) => `${f.origen}:${f.orden}`));
    const setL = new Set((l.fotos ?? []).map((f) => `${f.origen}:${f.orden}`));
    for (const k of setP) if (!setL.has(k)) marca(`FOTO-FALTA-LOCAL slug=${s} ${k}`);
    for (const k of setL) if (!setP.has(k)) marca(`FOTO-FALTA-PROD slug=${s} ${k}`);
    for (const [lado, arr] of [['prod', p.fotos ?? []], ['local', l.fotos ?? []]]) {
      const vistos = new Set();
      for (const f of arr) {
        const k = `${f.origen}:${f.orden}`;
        if (vistos.has(k)) marca(`DUPLICADO-${lado.toUpperCase()} slug=${s} ${k} id=${f.id}`);
        vistos.add(k);
      }
    }
  }
  if (!sinBytes) {
    const pares = [];
    for (const s of [...mapP.keys()].filter((s) => mapL.has(s))) {
      const idxL = new Map((mapL.get(s).fotos ?? []).map((f) => [`${f.origen}:${f.orden}`, f]));
      for (const f of mapP.get(s).fotos ?? []) {
        const g = idxL.get(`${f.origen}:${f.orden}`);
        if (g) pares.push({ slug: s, k: `${f.origen}:${f.orden}`, up: relUrl(f.url, PROD), ul: relUrl(g.url, LOCAL) });
      }
    }
    let okO = 0, malO = 0, okM = 0, malM = 0, errD = 0;
    await pool(pares, par, async (q) => {
      const bp = await api(PROD, q.up, { token: tP, bytes: true }).catch(() => null);
      const bl = await api(LOCAL, q.ul, { token: tL, bytes: true }).catch(() => null);
      if (!bp || !bl) { errD++; marca(`ERROR-DESCARGA slug=${q.slug} ${q.k}`); return; }
      const esOrig = q.k.startsWith('original:');
      if (shaFoto(bp) === shaFoto(bl)) { if (esOrig) okO++; else okM++; return; }
      if (esOrig) malO++; else malM++;
      marca(`BYTES-DISTINTOS slug=${q.slug} ${q.k} prod=${bp.length}B local=${bl.length}B`);
    });
    console.log(`bytes: ${pares.length} pares original=${okO}ok/${malO}mal mejorada=${okM}ok/${malM}mal errores=${errD}`);
  }
  console.log(difs ? `verificar: ${difs} diferencias (exit 1), cero escrituras` : 'verificar: LIMPIO (exit 0), cero escrituras');
  if (difs) process.exit(1);
}

async function main() {
  const { cmd, o } = args();
  if (cmd === 'estado') return cmdEstado(o);
  if (cmd === 'publicar') return cmdPublicar(o);
  if (cmd === 'mejorar') return cmdMejorar(o);
  if (cmd === 'push') return cmdPush(o);
  if (cmd === 'verificar') return cmdVerificar(o);
  console.log('Uso: node scripts/inmueble.mjs <estado|publicar|mejorar|push|verificar> [opciones]');
  console.log('  estado --slug <slug> | publicar --fotos <carpeta> --datos \'{...}\'|@f.json [--borrador] [--solo-local|--solo-prod] [--sobrescribir] [--dry-run]');
  console.log('  mejorar --slug <slug> [--limite N] [--repetir]  (solo local)');
  console.log('  push --slug <slug> [--dry-run] [--sobrescribir]  (local → prod)');
  console.log('  verificar [--slug <slug>] [--sin-bytes] [--par N]  (prod↔local, solo lectura)');
  process.exit(2);
}

main().catch((e) => {
  console.error(`FALLO: ${e.message}`);
  process.exit(3);
});
