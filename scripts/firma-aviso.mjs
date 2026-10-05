#!/usr/bin/env node
/* [03AA-3 E0.4] firma-v1 de excerpt: HMAC_SHA256(sal, NFC(trim(colapso(texto))) + "|" + (avisoId ?? "")).
 * La sal real vive en el keyring del SO y llega por env MP_SAL (hex); este
 * script nunca la imprime ni la guarda. `--self-test` usa sal fija de prueba
 * y verifica: determinismo, equivalencia NFC y null→enlazado (cambia, no colisiona). */
import { createHmac } from 'node:crypto';

export function colapso(texto) {
  return String(texto ?? '').trim().replace(/\s+/g, ' ');
}

export function firmaAviso(texto, avisoId, salHex) {
  const sal = Buffer.from(String(salHex ?? '').trim(), 'hex');
  if (sal.length < 16) throw new Error('MP_SAL ausente o corta (hex >=16 bytes)');
  const base = colapso(texto).normalize('NFC') + '|' + (avisoId ?? '');
  return createHmac('sha256', sal).update(base, 'utf8').digest('hex');
}

const SAL_PRUEBA = '00'.repeat(32);

if (process.argv.includes('--self-test')) {
  const f = (t, a) => firmaAviso(t, a, SAL_PRUEBA);
  const assert = (cond, msg) => { if (!cond) { console.error('FALLA: ' + msg); process.exit(1); } };
  assert(f('Hola  mundo', null) === f('Hola mundo', null), 'colapso de espacios');
  assert(f('café', null) === f('café', null), 'equivalencia NFC');
  assert(f('Hola, sigue disponible?', null) !== f('Hola, sigue disponible?', 'aviso-123'), 'null→enlazado cambia la firma');
  assert(f('Precio?', 'a1') !== f('Precio?', 'a2'), 'avisoId distinto no colisiona');
  assert(f('Precio?', null) !== f('Otro texto', null), 'texto distinto no colisiona');
  assert(/^[0-9a-f]{64}$/.test(f('x', null)), 'formato hex 64');
  console.log('FIRMA_SELFTEST_OK (5/5)');
} else if (process.argv.includes('--firmar')) {
  const i = process.argv.indexOf('--firmar');
  const texto = process.argv[i + 1] ?? '';
  const avisoId = process.argv[i + 2] === '-' ? null : (process.argv[i + 2] ?? null);
  console.log(firmaAviso(texto, avisoId, process.env.MP_SAL));
}
