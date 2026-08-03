import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, symlink, writeFile } from 'node:fs/promises';
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
  await writeFile(path.join(root, 'sentinel.config.json'), JSON.stringify({ includePatterns: [] }), 'utf8');
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

test('respeta una política v2 observe y los patrones declarativos', async () => {
  const root = await fixtureRoot();
  const policy = {
    schemaVersion: 2,
    mode: 'observe',
    gate: { command: ['sentinel', 'check', '--'], taskIdRequired: true },
    guard: { directCommands: { npmScripts: ['test:*'], npxTools: ['vitest'], cargoSubcommands: ['test'], tools: ['rustfmt'] } },
    runtime: { minimumVersion: '0.4.0', protocolVersion: 1, lockFile: 'sentinel.lock.json' },
    analyzers: { sentinel: { enabled: true }, varsense: { enabled: false } },
  };
  await writeFile(path.join(root, 'sentinel.config.json'), JSON.stringify(policy), 'utf8');
  const decision = inspectDirectCommand({ executable: 'npm', args: ['run', 'test:full'], cwd: root });
  assert.equal(decision.blocked, false);
  assert.equal(decision.observed, 'npm test:full');

  policy.mode = 'enforce';
  await writeFile(path.join(root, 'sentinel.config.json'), JSON.stringify(policy), 'utf8');
  const enforced = inspectDirectCommand({ executable: 'npm', args: ['run', 'test:full'], cwd: root });
  assert.equal(enforced.blocked, true);
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

test('un symlink roto de política no se clasifica como no-policy', async () => {
  const root = await fixtureRoot();
  const missing = path.join(root, 'missing-sentinel.config.json');
  await rm(path.join(root, 'sentinel.config.json'));
  await symlink(missing, path.join(root, 'sentinel.config.json'), 'file');
  const decision = inspectDirectCommand({ executable: 'npx', args: ['vitest', 'run'], cwd: root });
  assert.equal(decision.policyStatus, 'invalid-policy');
  assert.equal(decision.blocked, true);
  assert.equal(decision.exitCode, QUALITY_GUARD_EXIT_CODE);
});

test('una política symlink se trata como inválida y no se sigue fuera del workspace', async () => {
  const root = await fixtureRoot();
  const outside = await mkdtemp(path.join(os.tmpdir(), 'glory-quality-command-guard-outside-'));
  try {
    await writeFile(path.join(outside, 'sentinel.config.json'), JSON.stringify({ schemaVersion: 2 }), 'utf8');
    await rm(path.join(root, 'sentinel.config.json'));
    await symlink(path.join(outside, 'sentinel.config.json'), path.join(root, 'sentinel.config.json'), 'file');
    const decision = inspectDirectCommand({ executable: 'npx', args: ['vitest', 'run'], cwd: root });
    assert.equal(decision.policyStatus, 'invalid-policy');
    assert.equal(decision.blocked, true);
    assert.equal(decision.exitCode, QUALITY_GUARD_EXIT_CODE);
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(outside, { recursive: true, force: true });
  }
});

test('un proyecto sin política pasa sin bloqueo', async () => {
  const root = await fixtureRoot();
  await rm(path.join(root, 'sentinel.config.json'));
  const decision = inspectDirectCommand({ executable: 'npx', args: ['vitest', 'run'], cwd: root });
  assert.equal(decision.blocked, false);
  assert.equal(decision.policyStatus, 'no-policy');
});

test('no bloquea comandos fuera de un proyecto Glory', async () => {
  const cwd = await mkdtemp(path.join(os.tmpdir(), 'unrelated-quality-command-'));
  assert.equal(inspectDirectCommand({ executable: 'npx', args: ['vitest', 'run'], cwd }).blocked, false);
});
