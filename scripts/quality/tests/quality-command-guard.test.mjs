import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {
  formatBlockMessage,
  inspectDirectCommand,
  QUALITY_GUARD_EXIT_CODE,
} from '../quality-command-guard.mjs';

async function fixtureRoot() {
  const root = await mkdtemp(path.join(os.tmpdir(), 'glory-quality-command-guard-'));
  await mkdir(path.join(root, 'scripts', 'quality'), { recursive: true });
  await writeFile(path.join(root, 'quality.config.json'), '{}', 'utf8');
  await writeFile(path.join(root, 'scripts', 'quality', 'heavy-run-guard.mjs'), '', 'utf8');
  return root;
}

test('bloquea vitest directo y recomienda task:check', async () => {
  const root = await fixtureRoot();
  const decision = inspectDirectCommand({ executable: 'npx', args: ['vitest', 'run'], cwd: root });
  assert.equal(decision.blocked, true);
  assert.equal(decision.exitCode, QUALITY_GUARD_EXIT_CODE);
  assert.match(formatBlockMessage(decision), /npm run task:check/);
});

test('bloquea scripts frontend de validación, incluso con --prefix', async () => {
  const root = await fixtureRoot();
  const decision = inspectDirectCommand({
    executable: 'npm.cmd',
    args: ['--prefix', 'frontend', 'run', 'test:full'],
    cwd: root,
  });
  assert.equal(decision.blocked, true);
  assert.equal(decision.command, 'npm test:full');
});

test('bloquea el probe inerte para verificar que la shell cargó el guard', async () => {
  const root = await fixtureRoot();
  const decision = inspectDirectCommand({
    executable: 'npm',
    args: ['run', '__sentinel_guard_probe__'],
    cwd: root,
  });
  assert.equal(decision.blocked, true);
  assert.equal(decision.command, 'npm __sentinel_guard_probe__');
});

test('permite task:check, desarrollo y comandos de herramientas no relacionadas', async () => {
  const root = await fixtureRoot();
  assert.equal(inspectDirectCommand({ executable: 'npm', args: ['run', 'task:check', '--', '028A-5'], cwd: root }).blocked, false);
  assert.equal(inspectDirectCommand({ executable: 'npm', args: ['run', 'dev'], cwd: root }).blocked, false);
  assert.equal(inspectDirectCommand({ executable: 'npx', args: ['orval'], cwd: root }).blocked, false);
  assert.equal(inspectDirectCommand({ executable: 'cargo', args: ['run'], cwd: root }).blocked, false);
});

test('bloquea validaciones Cargo directas para forzar el gate único', async () => {
  const root = await fixtureRoot();
  const decision = inspectDirectCommand({ executable: 'cargo.exe', args: ['check'], cwd: root });
  assert.equal(decision.blocked, true);
  assert.equal(decision.category, 'cargo');
});

test('bloquea rustfmt directo para evitar el bypass de cargo fmt', async () => {
  const root = await fixtureRoot();
  const decision = inspectDirectCommand({ executable: 'rustfmt.exe', args: ['src/lib.rs'], cwd: root });
  assert.equal(decision.blocked, true);
  assert.equal(decision.category, 'tool');
});

test('no bloquea comandos fuera de un proyecto Glory', async () => {
  const cwd = await mkdtemp(path.join(os.tmpdir(), 'unrelated-quality-command-'));
  assert.equal(inspectDirectCommand({ executable: 'npx', args: ['vitest', 'run'], cwd }).blocked, false);
});
