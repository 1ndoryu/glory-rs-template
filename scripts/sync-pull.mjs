// [08AA-2] Trae el snapshot prod→local (inmuebles + fotos) y lo deja espejo.
// Solo LEE prod (login + GETs, como un navegador). Todo lo que escribe es LOCAL.
// Uso: node scripts/sync-pull.mjs [--dry-run] [--env RUTA]
//   --dry-run: compara y muestra qué haría, sin escribir nada en local.
//   --env: ruta del fichero de credenciales (defecto: scripts/.env.prod.local).
// Credenciales en scripts/.env.prod.local (gitignored, nunca en git ni en logs):
//   PROD_BASE=https://mn-inmobiliaria.com
//   PROD_EMAIL=admin@admin.com
//   PROD_PASSWORD=...
//   LOCAL_API=http://127.0.0.1:3110
//   LOCAL_EMAIL=admin@admin.com
//   LOCAL_PASSWORD=...
// Exit: 0 ok | 2 preflight/auth | 3 error de sync | 4 verificación fallida.
// Requisito previo: backup local (pg_dump) antes de correr sin --dry-run.

import { api, leerEnv, login, nucleo, ENV_DEFECTO } from './lib-api.mjs';

const args = Object.fromEntries(
  process.argv.slice(2).map((a, i, arr) => (a.startsWith('--') ? [a.slice(2), arr[i + 1] ?? 'true'] : [])).filter((p) => p.length),
);
const DRY = args['dry-run'] === 'true' || args['dry-run'] === '';
const ENV_RUTA = args.env ?? ENV_DEFECTO;

async function main() {
  const cfg = leerEnv(ENV_RUTA, ['PROD_BASE', 'PROD_EMAIL', 'PROD_PASSWORD', 'LOCAL_EMAIL', 'LOCAL_PASSWORD']);
  cfg.LOCAL_API = cfg.LOCAL_API ?? 'http://127.0.0.1:3110';
  const PROD = cfg.PROD_BASE.replace(/\/$/, '');
  const LOCAL = cfg.LOCAL_API.replace(/\/$/, '');
  console.log(`Modo: ${DRY ? 'dry-run (sin escribir)' : 'ESPEJO prod→local (solo escribe en local)'}`);
  console.log(`Prod (solo lectura): ${PROD} | Local: ${LOCAL}`);

  // F0: preflight de salud en ambos lados.
  for (const [nombre, base] of [['prod', PROD], ['local', LOCAL]]) {
    const h = await api(base, '/api/health').catch((e) => { throw new Error(`preflight ${nombre}: ${e.message}`); });
    if (h.status !== 'ok') throw new Error(`preflight ${nombre}: estado=${h.status}`);
  }
  const login = async (base, email, password, quien) =>
    (await api(base, '/api/auth/login', { metodo: 'POST', json: { email, password } }).catch(() => {
      console.error(`FALLO auth ${quien}: revisa email/password en ${ENV_RUTA}.`);
      process.exit(2);
    })).token;
  const tokenP = await login(PROD, cfg.PROD_EMAIL, cfg.PROD_PASSWORD, 'prod', ENV_RUTA);
  const tokenL = await login(LOCAL, cfg.LOCAL_EMAIL, cfg.LOCAL_PASSWORD, 'local', ENV_RUTA);
  console.log('Auth prod+local OK (tokens solo en memoria).');

  const itemsP = (await api(PROD, '/api/admin/inmuebles?page=1&per_page=200', { token: tokenP })).items;
  const itemsL = (await api(LOCAL, '/api/admin/inmuebles?page=1&per_page=200', { token: tokenL })).items;
  const porSlugL = new Map(itemsL.map((i) => [i.slug, i]));
  console.log(`Prod: ${itemsP.length} inmuebles, ${itemsP.reduce((n, i) => n + i.fotos.length, 0)} fotos.`);
  console.log(`Local (antes): ${itemsL.length} inmuebles, ${itemsL.reduce((n, i) => n + i.fotos.length, 0)} fotos.`);

  const resumen = { creados: 0, actualizados: 0, fotos: 0, sinCambios: 0, fichasOmitidas: 0 };
  for (const p of itemsP) {
    // La ficha /ask puede no existir en prod (backend anterior a 279A-3: la
    // ruta GET ni siquiera matchea → 404 sin cuerpo). En ese caso se deja la
    // ficha local intacta y se sigue con el resto (no es motivo de fallo).
    const fichaP = await api(PROD, `/api/admin/inmuebles/${p.id}/ficha`, { token: tokenP }).catch(() => null);
    let idL = porSlugL.get(p.slug)?.id;
    if (DRY) {
      console.log(`[dry-run] ${idL ? 'actualizar' : 'crear'} ${p.slug} (${p.fotos.length} fotos)`);
      continue;
    }
    if (!idL) {
      const creado = await api(LOCAL, '/api/admin/inmuebles', { metodo: 'POST', token: tokenL, json: nucleo(p) });
      idL = creado.id;
      resumen.creados += 1;
    } else {
      await api(LOCAL, `/api/admin/inmuebles/${idL}`, { metodo: 'PUT', token: tokenL, json: nucleo(p) });
      resumen.actualizados += 1;
    }
    const localAhora = (await api(LOCAL, '/api/admin/inmuebles?page=1&per_page=200', { token: tokenL })).items.find((i) => i.slug === p.slug);
    if (localAhora.publicado !== p.publicado) {
      await api(LOCAL, `/api/admin/inmuebles/${idL}/publicacion`, { metodo: 'PATCH', token: tokenL, json: { publicado: p.publicado } });
    }
    if (fichaP) {
      await api(LOCAL, `/api/admin/inmuebles/${idL}/ficha`, { metodo: 'PUT', token: tokenL, json: { extras: fichaP.extras, precio_minimo: fichaP.precio_minimo ?? null } });
    } else {
      resumen.fichasOmitidas += 1;
      console.log(`aviso ${p.slug}: sin ficha en prod, se conserva la local`);
    }
    // Fotos: reemplazo total por (orden, origen) para quedar espejo exacto.
    for (const f of localAhora.fotos) {
      await api(LOCAL, `/api/admin/fotos/${f.id}`, { metodo: 'DELETE', token: tokenL });
    }
    const ordenadas = [...p.fotos].sort((a, b) => a.orden - b.orden);
    for (const f of ordenadas) {
      const bytes = await api(PROD, f.url, { token: tokenP, bytes: true });
      const filename = f.url.split('/').pop();
      await api(LOCAL, `/api/admin/fotos/upload?inmueble_id=${idL}&filename=${encodeURIComponent(filename)}&origen=${encodeURIComponent(f.origen)}&orden=${f.orden}`, { metodo: 'POST', token: tokenL, bytes });
      resumen.fotos += 1;
    }
    console.log(`OK ${p.slug} (${ordenadas.length} fotos)`);
  }

  // Verificación: el espejo debe cuadrar.
  const finL = (await api(LOCAL, '/api/admin/inmuebles?page=1&per_page=200', { token: tokenL })).items;
  const fotosL = finL.reduce((n, i) => n + i.fotos.length, 0);
  const fotosP = itemsP.reduce((n, i) => n + i.fotos.length, 0);
  console.log(`Local (después): ${finL.length} inmuebles, ${fotosL} fotos.`);
  console.log(JSON.stringify({ ...resumen, prod: { inmuebles: itemsP.length, fotos: fotosP }, local: { inmuebles: finL.length, fotos: fotosL } }));
  if (DRY) return;
  if (finL.length !== itemsP.length || fotosL !== fotosP) {
    console.error('FALLO verificación: el espejo no cuadra. Revisa el log; prod intacta.');
    process.exit(4);
  }
  console.log('Espejo OK: local == prod.');
}

main().catch((e) => {
  console.error(`FALLO: ${e.message}`);
  process.exit(3);
});
