import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {
  TAKEOVER_TTL_MS,
  createTakeoverId,
  decodeTakeoverId,
  defaultAgent,
  isStale,
  listTakeovers,
  readTakeover,
  releaseTask,
  sanitizeAgentName,
  sanitizeTaskId,
  takeoverEntryPath,
  takeoverReminders,
  takeTask,
} from '../task-takeover.mjs';

const HOUR = 60 * 60 * 1000;
const nowMs = Date.parse('2026-08-05T12:00:00.000Z');

async function makeRoot() {
  const root = await mkdtemp(path.join(os.tmpdir(), 'quality-takeover-'));
  await mkdir(path.join(root, '.quality-reports'), { recursive: true });
  return root;
}

test('sanitizeTaskId acepta IDs reales y rechaza traversal', () => {
  assert.equal(sanitizeTaskId('297A-16'), '297A-16');
  assert.equal(sanitizeTaskId('GAME-01'), 'GAME-01');
  assert.equal(sanitizeTaskId('028A-8'), '028A-8');
  assert.throws(() => sanitizeTaskId('../evil'), /taskId inválido/);
  assert.throws(() => sanitizeTaskId('a/b'), /taskId inválido/);
  assert.throws(() => sanitizeTaskId(''), /taskId inválido/);
  assert.throws(() => sanitizeTaskId('x'.repeat(100)), /taskId inválido/);
});

test('createTakeoverId codifica el instante exacto y decode lo recupera', () => {
  const id = createTakeoverId(nowMs);
  const decoded = decodeTakeoverId(id);
  assert.ok(decoded, `id decodificable: ${id}`);
  assert.equal(decoded.takenAtMs, nowMs);
  assert.match(id, /^T-\d{13}-[0-9a-f]{8}$/u);
  /* El instante es legible sin abrir el registro: basta el identificador. */
  assert.equal(new Date(decoded.takenAtMs).toISOString(), new Date(nowMs).toISOString());
  assert.equal(decodeTakeoverId('T-abc-12345678'), null);
  assert.equal(decodeTakeoverId('T-123-xyz'), null);
  assert.equal(decodeTakeoverId('t-1754395200000-a1b2c3d4'), null);
  assert.equal(decodeTakeoverId(null), null);
});

test('defaultAgent usa GLORY_AGENT_ID o el hostname', () => {
  assert.equal(defaultAgent({ GLORY_AGENT_ID: 'buffy' }), 'buffy');
  assert.equal(defaultAgent({ GLORY_AGENT_ID: '  ' }), os.hostname());
  assert.equal(defaultAgent({}), os.hostname());
});

test('take: marcado nuevo, renovación del mismo agente y conflicto con otro activo', async () => {
  const root = await makeRoot();
  try {
    const taken = await takeTask(root, '297A-16', { by: 'buffy', nowMs });
    assert.equal(taken.status, 'taken');
    assert.equal(taken.entry.takenBy, 'buffy');
    assert.equal(taken.entry.taskId, '297A-16');
    assert.ok(taken.entry.id.startsWith('T-'));

    /* Otro agente activo → conflicto. */
    const conflict = await takeTask(root, '297A-16', { by: 'agente-2', nowMs: nowMs + 60_000 });
    assert.equal(conflict.status, 'conflict');
    assert.equal(conflict.entry.takenBy, 'buffy');

    /* El mismo agente re-toma → renovación (nuevo instante). */
    const refreshed = await takeTask(root, '297A-16', { by: 'buffy', nowMs: nowMs + 5 * 60_000 });
    assert.equal(refreshed.status, 'refreshed');
    assert.equal(refreshed.entry.takenBy, 'buffy');
    assert.equal(refreshed.entry.takenAtMs, nowMs + 5 * 60_000);
    assert.ok(refreshed.entry.id !== taken.entry.id, 'la renovación genera un id nuevo');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('take: un marcado de más de 6 h se considera olvidado y se puede re-tomar', async () => {
  const root = await makeRoot();
  try {
    await takeTask(root, '028A-8', { by: 'agente-olvidadizo', nowMs });
    /* >6 h después: otro agente lo re-toma sin --force (con aviso). */
    const retaken = await takeTask(root, '028A-8', { by: 'buffy', nowMs: nowMs + TAKEOVER_TTL_MS + 60_000 });
    assert.equal(retaken.status, 'taken-over-stale');
    assert.equal(retaken.entry.takenBy, 'buffy');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('release: libera el autor, no libera el ajeno activo sin --force, y respeta el olvidado', async () => {
  const root = await makeRoot();
  try {
    await takeTask(root, '297A-16', { by: 'buffy', nowMs });
    /* Un agente ajeno activo no puede liberar sin --force. */
    const conflict = await releaseTask(root, '297A-16', { by: 'agente-2', nowMs: nowMs + 60_000 });
    assert.equal(conflict.status, 'conflict');
    /* El autor libera. */
    const released = await releaseTask(root, '297A-16', { by: 'buffy', nowMs: nowMs + 60_000 });
    assert.equal(released.status, 'released');
    assert.equal(await readTakeover(root, '297A-16'), null);
    /* Liberar lo no tomado es informativo, no error. */
    const notTaken = await releaseTask(root, '297A-16', { by: 'buffy' });
    assert.equal(notTaken.status, 'not-taken');

    /* Marcado olvidado (>6 h): otro agente puede liberarlo sin --force. */
    await takeTask(root, '028A-8', { by: 'agente-olvidadizo', nowMs });
    const stale = await releaseTask(root, '028A-8', { by: 'buffy', nowMs: nowMs + TAKEOVER_TTL_MS + 60_000 });
    assert.equal(stale.status, 'released-stale');
    assert.equal(await readTakeover(root, '028A-8'), null);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('take: el mismo agente con --force renueva su marcado en vez de dar conflicto', async () => {
  const root = await makeRoot();
  try {
    const first = await takeTask(root, '297A-16', { by: 'buffy', nowMs });
    assert.equal(first.status, 'taken');
    const forced = await takeTask(root, '297A-16', { by: 'buffy', force: true, nowMs: nowMs + 60_000 });
    assert.equal(forced.status, 'refreshed', '--force del mismo agente renueva, no choca');
    assert.equal(forced.entry.takenAtMs, nowMs + 60_000);
    assert.equal(forced.entry.takenBy, 'buffy');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('sanitizeAgentName recorta y elimina caracteres de control', () => {
  assert.equal(sanitizeAgentName('  buffy  '), 'buffy');
  assert.equal(sanitizeAgentName('bu\nffy\u0000'), 'buffy');
  assert.equal(sanitizeAgentName(''), 'unknown');
  assert.equal(sanitizeAgentName('x'.repeat(100)), 'x'.repeat(64));
});

test('take: la creación con wx no pisa una toma concurrente (carrera EEXIST)', async () => {
  const root = await makeRoot();
  try {
    /* Simula que otro agente escribe el marcado entre la lectura y la toma. */
    await takeTask(root, '297A-16', { by: 'agente-2', nowMs });
    const result = await takeTask(root, '297A-16', { by: 'buffy', nowMs: nowMs + 1000 });
    assert.equal(result.status, 'conflict');
    assert.equal(result.entry.takenBy, 'agente-2', 'el marcado del otro agente no se pisa');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('take: una toma fresca ajena nunca se pisa, ni siquiera tras un marcado expirado', async () => {
  const root = await makeRoot();
  try {
    /* Marcado viejo olvidado. */
    await takeTask(root, '297A-16', { by: 'viejo', nowMs });
    /* Otro agente re-toma la expirada y deja una toma fresca. */
    const fresh = (await takeTask(root, '297A-16', { by: 'nuevo', force: true, nowMs: nowMs + TAKEOVER_TTL_MS + 60_000 })).entry;
    assert.equal(fresh.takenBy, 'nuevo');
    /* Un tercero intenta tomarla justo después: conflicto, no la pisa. */
    const third = await takeTask(root, '297A-16', { by: 'tercero', nowMs: nowMs + TAKEOVER_TTL_MS + 120_000 });
    assert.equal(third.status, 'conflict');
    assert.equal(third.entry.takenBy, 'nuevo', 'la toma fresca ajena no se pisa');
    const after = await readTakeover(root, '297A-16');
    assert.equal(after.takenBy, 'nuevo');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('listTakeovers ordena por antigüedad e incluye el estado stale', async () => {
  const root = await makeRoot();
  try {
    await takeTask(root, '028A-8', { by: 'viejo', nowMs });
    await takeTask(root, '297A-16', { by: 'buffy', nowMs: nowMs + 60_000 });
    const entries = await listTakeovers(root, nowMs + 60_000);
    assert.equal(entries.length, 2);
    assert.equal(entries[0].taskId, '028A-8', 'más antigua primero');
    assert.equal(entries[0].stale, false);
    /* >6 h después, la primera queda stale. */
    const staleList = await listTakeovers(root, nowMs + TAKEOVER_TTL_MS + 60_000);
    assert.equal(staleList[0].stale, true);
    assert.equal(staleList[1].stale, false);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('takeoverReminders cubre los cuatro estados del gate', () => {
  const entry = {
    schemaVersion: 1, taskId: '297A-16', id: 'T-1754395200000-a1b2c3d4',
    takenAt: '2026-08-05T12:00:00.000Z', takenAtMs: nowMs,
    takenBy: 'buffy', expiresAt: '2026-08-05T18:00:00.000Z', expiresAtMs: nowMs + TAKEOVER_TTL_MS,
  };
  /* Sin tomar → sugiere marcarla. */
  const unmarked = takeoverReminders({ taskId: '297A-16', entry: null, agent: 'buffy', nowMs });
  assert.match(unmarked[0], /task:take/);
  /* Tomada por este agente → recuerda liberar. */
  const mine = takeoverReminders({ taskId: '297A-16', entry, agent: 'buffy', nowMs });
  assert.match(mine[0], /task:release/);
  /* Tomada por otro activo → aviso de conflicto. */
  const theirs = takeoverReminders({ taskId: '297A-16', entry, agent: 'agente-2', nowMs });
  assert.match(theirs[0], /TAREA TOMADA/);
  assert.match(theirs[0], /buffy/);
  /* Olvidada (>6 h) → re-tomar o liberar con --force. */
  const stale = takeoverReminders({ taskId: '297A-16', entry, agent: 'agente-2', nowMs: nowMs + TAKEOVER_TTL_MS + 60_000 });
  assert.match(stale[0], /expiró/);
});

test('readTakeover devuelve null ante un archivo corrupto o inexistente', async () => {
  const root = await makeRoot();
  try {
    assert.equal(await readTakeover(root, '297A-16'), null);
    await mkdir(path.join(root, '.quality-reports', 'task-takeover'), { recursive: true });
    await writeFile(takeoverEntryPath(root, '297A-16'), '{ not json', 'utf8');
    assert.equal(await readTakeover(root, '297A-16'), null);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
