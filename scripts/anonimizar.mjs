#!/usr/bin/env node
/* [03AA-3 E0.5] Anonimiza texto de hilos de Messenger (stdin o ficheros → stdout o --salida).
 * Reemplaza: rachas de 7+ digitos (con o sin separadores) → [TEL]; wa.me → [WA];
 * emails → [EMAIL]; perfiles de FB (profile.php?id=, /@usuario) → [PERFIL];
 * nombres dados en --nombres "a,b,c" → [NOMBRE]. Precios como $43.000 se conservan.
 * Uso: node scripts/anonimizar.mjs [--nombres "Janeth,..."] [--salida out.txt] [fichero...] */
import { readFileSync, writeFileSync } from 'node:fs';

const args = process.argv.slice(2);
const opt = (n) => { const i = args.indexOf(n); return i >= 0 ? args[i + 1] : undefined; };
const nombres = (opt('--nombres') ?? '').split(',').map((s) => s.trim()).filter(Boolean);
const salida = opt('--salida');
const ficheros = args.filter((a) => !a.startsWith('--') && a !== opt('--nombres') && a !== salida);

export function anonimizar(texto, listaNombres = []) {
  let t = String(texto ?? '');
  t = t.replace(/wa\.me\/[0-9+\s-]*/gi, '[WA]');
  t = t.replace(/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g, '[EMAIL]');
  t = t.replace(/facebook\.com\/(profile\.php\?[^ \s]*|@[\w.]+)/gi, '[PERFIL]');
  t = t.replace(/\+?58\d{10}/g, '[TEL]');
  t = t.replace(/0\d{3}[\s.\-]?\d{3}[\s.\-]?\d{4}/g, '[TEL]');
  t = t.replace(/\d{7,}/g, '[TEL]');
  for (const n of listaNombres) {
    t = t.replace(new RegExp('(^|[^A-Za-zÁÉÍÓÚÜÑáéíóúüñ])' + n.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '(?![A-Za-zÁÉÍÓÚÜÑáéíóúüñ])', 'gi'), '$1[NOMBRE]');
  }
  return t;
}

async function leerEntrada() {
  if (ficheros.length > 0) return ficheros.map((f) => readFileSync(f, 'utf8')).join('\n');
  const trozos = [];
  for await (const t of process.stdin) trozos.push(t);
  return trozos.join('');
}

const resultado = anonimizar(await leerEntrada(), nombres);
if (salida) writeFileSync(salida, resultado, 'utf8');
else process.stdout.write(resultado);
