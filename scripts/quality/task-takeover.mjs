#!/usr/bin/env node
/* [028A-17] Coordinación de tomas de tarea entre agentes en el checkout
 * compartido. Cada vez que un agente EMPIEZA una tarea la marca con
 * `npm run task:take -- --task <ID> --by <agente>`; al terminar la libera con
 * `npm run task:release -- --task <ID>`. El identificador de cada toma
 * codifica el instante exacto en que se tomó (formato `T-<epochMs>-<hex8>`),
 * de modo que basta leerlo para saber cuándo se tomó sin abrir el registro.
 *
 * Reglas:
 * - Una tarea tomada por OTRO agente activo no se puede tomar de nuevo.
 * - Un marcado que supera TAKEOVER_TTL_MS (6 h) sin liberarse se considera
 *   olvidado: cualquier agente puede re-tomarlo (con aviso) o liberarlo.
 * - El registro vive en `.quality-reports/task-takeover/<taskId>.json`
 *   (ignorado por git): es coordinación local del checkout, no un contrato.
 * - `task:check` solo informa/recuerda; nunca toma ni libera por sorpresa.
 *
 * Seguridad: el taskId se valida contra un patrón seguro (nada de traversal),
 * la creación de un marcado usa `wx` (falla si ya existe) para que dos tomas
 * concurrentes no se pisen, y los campos del registro son solo cadenas/números. */
import os from 'node:os';
import path from 'node:path';
import { randomBytes } from 'node:crypto';
import { mkdir, readFile, readdir, unlink, writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { writeAtomic } from './atomic-file.mjs';
import { projectRoot } from './preflight.mjs';

export const TAKEOVER_TTL_MS = 6 * 60 * 60 * 1000;
export const TAKEOVER_SCHEMA_VERSION = 1;

const SAFE_TASK_ID = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/u;

export function sanitizeTaskId(taskId) {
  if (typeof taskId !== 'string' || !SAFE_TASK_ID.test(taskId)) {
    throw new Error(`taskId inválido para toma de tarea: ${String(taskId)}`);
  }
  return taskId;
}

export function takeoverRegistryRoot(root = projectRoot) {
  return path.join(root, '.quality-reports', 'task-takeover');
}

export function takeoverEntryPath(root, taskId) {
  return path.join(takeoverRegistryRoot(root), `${sanitizeTaskId(taskId)}.json`);
}

/* Identificador de toma: `T-<epochMs>-<hex8>`. El epoch codifica el instante
 * exacto de la toma; basta decodificarlo para saber cuándo se tomó. */
export function createTakeoverId(takenAtMs) {
  return `T-${Math.trunc(takenAtMs)}-${randomBytes(4).toString('hex')}`;
}

export function decodeTakeoverId(id) {
  if (typeof id !== 'string') return null;
  const match = /^T-(\d{13})-([0-9a-f]{8})$/u.exec(id);
  if (!match) return null;
  const takenAtMs = Number(match[1]);
  if (!Number.isSafeInteger(takenAtMs)) return null;
  return { takenAtMs, randomHex: match[2] };
}

export function isStale(entry, nowMs = Date.now()) {
  return typeof entry?.takenAtMs === 'number'
    && nowMs - entry.takenAtMs > TAKEOVER_TTL_MS;
}

export function defaultAgent(env = process.env) {
  return env.GLORY_AGENT_ID?.trim() || os.hostname();
}

/* El nombre de agente entra por CLI (--by) o env y se vuelca en el reporte
 * Markdown: se recorta y se eliminan caracteres de control para que un valor
 * con `\n` no inyecte contenido en el reporte. */
export function sanitizeAgentName(value) {
  const cleaned = String(value ?? '').replace(/[\u0000-\u001f\u007f]/gu, '').trim();
  return cleaned.slice(0, 64) || 'unknown';
}

export async function readTakeover(root, taskId) {
  try {
    const raw = await readFile(takeoverEntryPath(root, taskId), 'utf8');
    const entry = JSON.parse(raw);
    if (entry?.schemaVersion !== TAKEOVER_SCHEMA_VERSION || typeof entry.takenAtMs !== 'number') return null;
    return entry;
  } catch (error) {
    /* Archivo inexistente o JSON corrupto (otro proceso escribiendo a medias
     * o registro dañado): ambos se tratan como “sin tomar”, nunca como error
     * que bloquee el gate. Un archivo corrupto persiste hasta ser re-tomado. */
    if (error?.code === 'ENOENT' || error instanceof SyntaxError) return null;
    throw error;
  }
}

export async function listTakeovers(root, nowMs = Date.now()) {
  const registry = takeoverRegistryRoot(root);
  let names = [];
  try {
    names = await readdir(registry);
  } catch (error) {
    if (error?.code === 'ENOENT') return [];
    throw error;
  }
  const entries = [];
  for (const name of names) {
    if (!name.endsWith('.json')) continue;
    const taskId = name.slice(0, -'.json'.length);
    try {
      const entry = await readTakeover(root, taskId);
      if (entry) entries.push({ taskId, entry, stale: isStale(entry, nowMs) });
    } catch {
      /* Marcado ilegible/corrupto: se informa pero no aborta el listado. */
      entries.push({ taskId, entry: null, stale: false, corrupt: true });
    }
  }
  return entries.sort((left, right) => (left.entry?.takenAtMs ?? 0) - (right.entry?.takenAtMs ?? 0));
}

/* Toma la tarea. Devuelve:
 * - { status: 'taken', entry }            marcado nuevo
 * - { status: 'refreshed', entry }        mismo agente re-tomando (renueva)
 * - { status: 'taken-over-stale', entry } marcado viejo olvidado, re-tomado
 * - { status: 'conflict', entry }         tomada por otro agente activo */
export async function takeTask(root, taskId, { by = defaultAgent(), force = false, nowMs = Date.now() } = {}) {
  sanitizeTaskId(taskId);
  const agent = sanitizeAgentName(by);
  const target = takeoverEntryPath(root, taskId);
  await mkdir(path.dirname(target), { recursive: true });
  const existing = await readTakeover(root, taskId);
  if (existing && existing.takenBy === agent) {
    /* Re-toma del mismo agente (con o sin --force): renueva el marcado.
     * El archivo ya existe, así que se reemplaza atómicamente, no con `wx`. */
    const entry = buildEntry(taskId, agent, nowMs);
    await writeAtomic(target, `${JSON.stringify(entry, null, 2)}\n`);
    return { status: 'refreshed', entry };
  }
  if (existing && !isStale(existing, nowMs)) {
    return { status: 'conflict', entry: existing };
  }
  if (existing && force) {
    /* Re-toma forzada de un marcado expirado: se reemplaza atómicamente
     * (el archivo ya existe, `wx` fallaría con EEXIST). */
    const entry = buildEntry(taskId, agent, nowMs);
    await writeAtomic(target, `${JSON.stringify(entry, null, 2)}\n`);
    return { status: 'taken-over-stale', entry };
  }
  if (existing) {
    /* Marcado olvidado (>6h): re-toma con aviso. Compare-and-delete: solo se
     * retira si sigue siendo el mismo marcado que observamos; si otro agente
     * lo re-tomó entre la lectura y este punto, se devuelve conflicto y no se
     * pisa la toma fresca ajena. */
    const current = await readTakeover(root, taskId);
    if (current && current.id !== existing.id) {
      return { status: 'conflict', entry: current };
    }
    await unlink(target).catch(() => {});
  }
  try {
    return { status: existing ? 'taken-over-stale' : 'taken', entry: await writeEntry(target, taskId, agent, nowMs) };
  } catch (error) {
    if (error?.code === 'EEXIST') {
      const other = await readTakeover(root, taskId);
      if (other && !isStale(other, nowMs)) return { status: 'conflict', entry: other };
      return { status: 'conflict', entry: other ?? null };
    }
    throw error;
  }
}

function buildEntry(taskId, by, nowMs) {
  const takenAtMs = Math.trunc(nowMs);
  return {
    schemaVersion: TAKEOVER_SCHEMA_VERSION,
    taskId,
    id: createTakeoverId(takenAtMs),
    takenAt: new Date(takenAtMs).toISOString(),
    takenAtMs,
    takenBy: sanitizeAgentName(by),
    expiresAt: new Date(takenAtMs + TAKEOVER_TTL_MS).toISOString(),
    expiresAtMs: takenAtMs + TAKEOVER_TTL_MS,
  };
}

async function writeEntry(target, taskId, by, nowMs) {
  const entry = buildEntry(taskId, by, nowMs);
  /* `wx` garantiza que dos agentes tomando a la vez no se pisen: si el
   * archivo ya existe, la creación falla y el caller decide (conflicto). */
  await writeFile(target, `${JSON.stringify(entry, null, 2)}\n`, { encoding: 'utf8', flag: 'wx' });
  return entry;
}

/* Libera la tarea. Devuelve:
 * - { status: 'released', entry }      liberada (autor o con --force)
 * - { status: 'released-stale', entry } marcado olvidado liberado por otro
 * - { status: 'not-taken', entry: null } no estaba tomada
 * - { status: 'conflict', entry }      activa y de otro agente (sin --force) */
export async function releaseTask(root, taskId, { by = defaultAgent(), force = false, nowMs = Date.now() } = {}) {
  sanitizeTaskId(taskId);
  const agent = sanitizeAgentName(by);
  const target = takeoverEntryPath(root, taskId);
  const existing = await readTakeover(root, taskId);
  if (!existing) return { status: 'not-taken', entry: null };
  if (existing.takenBy !== agent && !isStale(existing, nowMs) && !force) {
    return { status: 'conflict', entry: existing };
  }
  await unlink(target).catch(error => {
    if (error?.code !== 'ENOENT') throw error;
  });
  return { status: existing.takenBy === agent ? 'released' : 'released-stale', entry: existing };
}

/* Recordatorios para el gate: el cierre debe recordar LIBERAR la tarea si el
 * agente la tomó, avisar si la tomó otro, y sugerir marcarla si no está. */
export function takeoverReminders({ taskId, entry, agent = defaultAgent(), nowMs = Date.now() } = {}) {
  if (!entry) {
    return [`Marca la tarea antes de trabajarla: npm run task:take -- --task ${taskId} --by <agente>`];
  }
  const stale = isStale(entry, nowMs);
  if (entry.takenBy === agent) {
    return [`Libera la tarea al terminar: npm run task:release -- --task ${taskId}`];
  }
  if (stale) {
    return [
      `El marcado de ${entry.takenBy} en ${taskId} expiró (olvidó liberarla): puedes re-tomarla con npm run task:take -- --task ${taskId} --by <agente> --force o liberarla con --force`,
    ];
  }
  return [
    `TAREA TOMADA por ${entry.takenBy} (${entry.id}) desde ${entry.takenAt} — expira ${entry.expiresAt}; no la trabajes en paralelo sin coordinar (npm run task:status)`,
  ];
}

function formatEntryLine({ taskId, entry, stale, corrupt }) {
  if (corrupt || !entry) return `- ${taskId} · registro ilegible`;
  const state = stale ? 'EXPIRADA (olvidada)' : 'activa';
  return `- ${taskId} · ${entry.id} · por ${entry.takenBy} · desde ${entry.takenAt} · expira ${entry.expiresAt} · ${state}`;
}

async function main() {
  const argv = process.argv.slice(2);
  const command = argv[0];
  const args = {};
  for (let index = 1; index < argv.length; index++) {
    const arg = argv[index];
    if (arg === '--task') args.task = argv[++index];
    else if (arg === '--by') args.by = argv[++index];
    else if (arg === '--force') args.force = true;
    else if (arg === '--root') args.root = argv[++index];
  }
  const root = args.root ?? projectRoot;

  if (command === 'take' || command === 'release') {
    if (!args.task) {
      process.stderr.write(`[task-takeover] ${command} requiere --task <ID> [--by <agente>] [--force]\n`);
      process.exitCode = 2;
      return;
    }
    const result = command === 'take'
      ? await takeTask(root, args.task, { by: args.by, force: args.force })
      : await releaseTask(root, args.task, { by: args.by, force: args.force });
    const { entry } = result;
    if (result.status === 'conflict' && entry) {
      const stale = isStale(entry);
      process.stderr.write(`[task-takeover] ${command.toUpperCase()} RECHAZADO — ${args.task} tomada por ${entry.takenBy} (${entry.id}) desde ${entry.takenAt}${stale ? ' (expirada)' : ''}\n`);
      process.stderr.write('[task-takeover] Next: npm run task:status\n');
      process.exitCode = 1;
      return;
    }
    if (result.status === 'conflict' && !entry) {
      process.stderr.write(`[task-takeover] ${command.toUpperCase()} RECHAZADO — carrera de escritura, reintenta\n`);
      process.exitCode = 1;
      return;
    }
    const verb = {
      taken: 'TOMADA', refreshed: 'RENOVADA', 'taken-over-stale': 'RE-TOMADA (marcado olvidado liberado)',
      released: 'LIBERADA', 'released-stale': 'LIBERADA (marcado olvidado)',
    }[result.status] ?? result.status.toUpperCase();
    process.stdout.write(`[task-takeover] ${verb} ${args.task} · ${entry?.id} · por ${entry?.takenBy}\n`);
    return;
  }

  if (command === 'status') {
    const entries = await listTakeovers(root);
    if (entries.length === 0) {
      process.stdout.write('[task-takeover] Ninguna tarea tomada.\n');
      return;
    }
    process.stdout.write('[task-takeover] Tomás de tarea (TTL 6 h; expiradas = olvidadas):\n');
    for (const item of entries) process.stdout.write(`${formatEntryLine(item)}\n`);
    if (args.task) {
      const match = entries.find(item => item.taskId === args.task);
      if (!match) process.stdout.write(`- ${args.task} · NO tomada\n`);
    }
    return;
  }

  process.stderr.write(`[task-takeover] uso: take|release|status --task <ID> [--by <agente>] [--force] [--root <ruta>]\n`);
  process.exitCode = 2;
}

const isEntryPoint = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isEntryPoint) await main();
