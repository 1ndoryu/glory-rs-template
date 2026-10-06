/* [03AA-3 E3] CLI del asistente Marketplace: Node puro, sin Electron.
 * Flujo: login admin (sesión vigente, sin API key suelta) -> token CLI 8h
 * atado a máquina -> borrador -> audit emision. Fuga 0 por construcción:
 * solo imprime el borrador y contadores; jamás excerpt, credenciales,
 * tokens ni el id de máquina (viaja solo su hash hex64).
 * Uso: MN_EMAIL=a@a.com MN_PASS=x node scripts/mp-cli.mjs --texto "..." */

import { createHash } from 'node:crypto';
import { hostname } from 'node:os';

const BASE = (process.env['MN_BASE'] ?? 'http://127.0.0.1:3110').replace(/\/$/, '');
const EMAIL = process.env['MN_EMAIL'] ?? '';
const PASS = process.env['MN_PASS'] ?? '';

function arg(nombre) {
  const i = process.argv.indexOf(`--${nombre}`);
  return i >= 0 ? (process.argv[i + 1] ?? '') : '';
}

/* Binding E3: el id real jamás sale de la máquina; solo su sha256. */
export function maquinaHash() {
  return createHash('sha256').update(hostname(), 'utf8').digest('hex');
}

async function postJson(ruta, body, headers = {}) {
  const r = await fetch(`${BASE}${ruta}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...headers },
    body: JSON.stringify(body),
  });
  const texto = await r.text();
  let json = null;
  try { json = JSON.parse(texto); } catch { /* error no-JSON */ }
  return { status: r.status, json, texto };
}

const texto = arg('texto');
if (texto === '' || EMAIL === '' || PASS === '') {
  console.error('Uso: MN_EMAIL=e MN_PASS=p node scripts/mp-cli.mjs --texto "..." [--tono amable]');
  process.exit(2);
}
const tono = arg('tono') === '' ? 'amable' : arg('tono');

const login = await postJson('/api/auth/login', { email: EMAIL, password: PASS });
if (login.status !== 200 || !login.json?.token) {
  console.error(`LOGIN-FAIL status=${login.status}`);
  process.exit(1);
}
const mid = maquinaHash();
const tok = await postJson(
  '/api/admin/marketplace/token/cli',
  { maquina_hash: mid },
  { Authorization: `Bearer ${login.json.token}` },
);
if (tok.status !== 201 || !tok.json?.token) {
  console.error(`TOKEN-CLI-FAIL status=${tok.status} ${tok.texto.slice(0, 120)}`);
  process.exit(1);
}
const H = { Authorization: `Bearer ${tok.json.token}`, 'X-MP-Maquina': mid };
const borrador = await postJson('/api/admin/marketplace/borrador', {
  threadId: `cli-${mid.slice(0, 8)}-${Date.now()}`,
  firma: '0'.repeat(64),
  firma_version: 'firma-v1',
  lang: 'es',
  excerpt: {
    remitente_hash: '0'.repeat(64),
    texto,
    hora: new Date(Date.now() - 4 * 3_600_000).toISOString().replace('Z', '-04:00'),
    leido: true,
  },
  avisoId: null,
  extras: { tono, largo: 's' },
}, H);
if (borrador.status !== 200 || typeof borrador.json?.borrador !== 'string') {
  console.error(`BORRADOR-FAIL status=${borrador.status} ${borrador.texto.slice(0, 120)}`);
  process.exit(1);
}
/* Emisión auditada (alimenta `uso` M2); el servidor hashea el thread_id. */
await postJson('/api/admin/marketplace/audit', {
  thread_id: `cli-${mid.slice(0, 8)}-${Date.now()}`,
  evento: 'emision',
}, H);
console.log(`EXPIRA-MIN=${tok.json.expira_en_minutos}`);
console.log(`FUENTE=${borrador.json.fuente}`);
console.log(borrador.json.borrador);
