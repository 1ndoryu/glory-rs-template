#!/usr/bin/env node
/* [03AA-3 E0.5] Verificador fail-closed de anonimización: exit 1 si encuentra
 * PII residual. Reglas: 7+ dígitos (ignorando separadores) → falla; wa.me,
 * emails, perfiles FB (profile.php?id=, /@usuario) → fallan; cada nombre de
 * --nombres "a,b,c" presente → falla. Fotos en E1 van por HMAC, no por texto.
 * Uso: node scripts/verificar-anonimizacion.mjs [--nombres "a,b"] fichero... (o stdin) */
import { readFileSync } from 'node:fs';

const args = process.argv.slice(2);
const opt = (n) => { const i = args.indexOf(n); return i >= 0 ? args[i + 1] : undefined; };
const nombres = (opt('--nombres') ?? '').split(',').map((s) => s.trim()).filter(Boolean);
const ficheros = args.filter((a) => !a.startsWith('--') && a !== opt('--nombres'));

async function leerEntrada() {
  if (ficheros.length > 0) return ficheros.map((f) => ({ nombre: f, texto: readFileSync(f, 'utf8') }));
  const trozos = [];
  for await (const t of process.stdin) trozos.push(t);
  return [{ nombre: '<stdin>', texto: trozos.join('') }];
}

const fallos = [];
for (const { nombre, texto } of await leerEntrada()) {
  const sinSep = texto.replace(/[\s.\-()]/g, '');
  if (/\d{7,}/.test(sinSep)) fallos.push(`${nombre}: racha de 7+ digitos`);
  if (/wa\.me\//i.test(texto)) fallos.push(`${nombre}: enlace wa.me`);
  if (/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/.test(texto)) fallos.push(`${nombre}: email`);
  if (/facebook\.com\/(profile\.php\?[^ \s]*|@[\w.]+)/i.test(texto)) fallos.push(`${nombre}: perfil FB`);
  for (const n of nombres) {
    if (new RegExp('(^|[^A-Za-zÁÉÍÓÚÜÑáéíóúüñ])' + n.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '(?![A-Za-zÁÉÍÓÚÜÑáéíóúüñ])', 'i').test(texto)) {
      fallos.push(`${nombre}: nombre "${n}"`);
    }
  }
}

if (fallos.length > 0) {
  for (const f of fallos) console.error('PII_RESIDUAL: ' + f);
  process.exit(1);
}
console.log('ANON_OK');
