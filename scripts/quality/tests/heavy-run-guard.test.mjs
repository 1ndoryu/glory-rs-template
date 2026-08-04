import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { acquireHeavyRun, inspectHeavyRun, isHeavyCargoCommand } from '../heavy-run-guard.mjs';

test('el guard limita full a una ejecución cada tres horas y permite override explícito', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'glory-heavy-guard-'));
  const targetBase = path.join(root, 'target');
  try {
    await writeFile(path.join(root, 'quality.config.json'), JSON.stringify({ heavyRun: { cooldownMinutes: 180 } }), 'utf8');
    const first = await acquireHeavyRun({ projectRoot: root, targetBase, mode: 'full', taskId: '028A-3' });
    assert.equal(first.allowed, true);
    await first.release({ status: 'pass' });

    const blocked = await inspectHeavyRun({ projectRoot: root, targetBase, mode: 'full' });
    assert.equal(blocked.allowed, false);
    assert.equal(blocked.reason, 'cooldown');
    const override = await acquireHeavyRun({ projectRoot: root, targetBase, mode: 'full', allowHeavy: true });
    assert.equal(override.allowed, true);
    await override.release({ status: 'pass' });
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(path.join(root, '..', 'glory-quality-guard'), { recursive: true, force: true });
  }
});

test('el guard bloquea dos ejecuciones pesadas simultáneas', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'glory-heavy-active-'));
  const targetBase = path.join(root, 'target');
  try {
    const first = await acquireHeavyRun({ projectRoot: root, targetBase, mode: 'full', allowHeavy: true });
    const second = await acquireHeavyRun({ projectRoot: root, targetBase, mode: 'full', allowHeavy: true });
    assert.equal(first.allowed, true);
    assert.equal(second.allowed, false);
    assert.equal(second.reason, 'active');
    await first.release({ status: 'pass' });
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(path.join(root, '..', 'glory-quality-guard'), { recursive: true, force: true });
  }
});

test('findQualityRoot resuelve el directorio físico y omite marcadores symlink', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'glory-heavy-root-'));
  const physical = await mkdtemp(path.join(os.tmpdir(), 'glory-heavy-physical-'));
  try {
    await mkdir(path.join(root, 'scripts', 'quality'), { recursive: true });
    await mkdir(path.join(root, 'nested'), { recursive: true });
    await writeFile(path.join(root, 'quality.config.json'), '{}', 'utf8');
    await writeFile(path.join(root, 'scripts', 'quality', 'heavy-run-guard.mjs'), '', 'utf8');
    const linked = path.join(physical, 'linked');
    await symlink(root, linked, 'junction');
    const { findQualityRoot } = await import('../heavy-run-guard.mjs');
    assert.equal(await findQualityRoot(path.join(linked, 'nested')), root);

    const candidate = path.join(root, 'child');
    const outside = await mkdtemp(path.join(os.tmpdir(), 'glory-heavy-marker-outside-'));
    await mkdir(path.join(candidate, 'scripts', 'quality'), { recursive: true });
    await writeFile(path.join(outside, 'quality.config.json'), '{}', 'utf8');
    await writeFile(path.join(outside, 'heavy-run-guard.mjs'), '', 'utf8');
    await symlink(path.join(outside, 'quality.config.json'), path.join(candidate, 'quality.config.json'), 'file');
    await symlink(path.join(outside, 'heavy-run-guard.mjs'), path.join(candidate, 'scripts', 'quality', 'heavy-run-guard.mjs'), 'file');
    assert.equal(await findQualityRoot(candidate), root);
    await rm(outside, { recursive: true, force: true });
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(physical, { recursive: true, force: true });
  }
});

test('findQualityRoot conserva fallback para una ruta inexistente', async () => {
  const missing = path.join(os.tmpdir(), 'glory-heavy-missing-root', 'nested');
  const { findQualityRoot } = await import('../heavy-run-guard.mjs');
  assert.equal(await findQualityRoot(missing), path.resolve(missing));
});

test('solo test, clippy y bench son comandos Cargo pesados', () => {
  assert.equal(isHeavyCargoCommand(['test']), true);
  assert.equal(isHeavyCargoCommand(['--locked', 'clippy']), true);
  assert.equal(isHeavyCargoCommand(['check']), false);
  assert.equal(isHeavyCargoCommand(['fmt', '--check']), false);
});

test('el cooldown de full no se comparte entre proyectos con el mismo targetBase', async () => {
  const rootA = await mkdtemp(path.join(os.tmpdir(), 'glory-heavy-project-a-'));
  const rootB = await mkdtemp(path.join(os.tmpdir(), 'glory-heavy-project-b-'));
  const targetBase = path.join(rootA, 'target');
  try {
    for (const root of [rootA, rootB]) {
      await writeFile(path.join(root, 'quality.config.json'), JSON.stringify({ heavyRun: { cooldownMinutes: 180 } }), 'utf8');
    }
    /* [028A-6] El cooldown vive en state.projects keyed por projectKey(root):
     * un full en A no debe bloquear un full en B aunque compartan targetBase. */
    const first = await acquireHeavyRun({ projectRoot: rootA, targetBase, mode: 'full', taskId: '028A-6' });
    assert.equal(first.allowed, true);
    await first.release({ status: 'pass' });

    const blockedA = await inspectHeavyRun({ projectRoot: rootA, targetBase, mode: 'full' });
    assert.equal(blockedA.allowed, false);
    assert.equal(blockedA.reason, 'cooldown');
    const allowedB = await inspectHeavyRun({ projectRoot: rootB, targetBase, mode: 'full' });
    assert.equal(allowedB.allowed, true);
  } finally {
    for (const root of [rootA, rootB]) {
      await rm(root, { recursive: true, force: true });
      await rm(path.join(root, '..', 'glory-quality-guard'), { recursive: true, force: true });
    }
  }
});

test('un comando ligero no adquiere lease ni escribe cooldown', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'glory-heavy-light-'));
  const targetBase = path.join(root, 'target');
  try {
    await writeFile(path.join(root, 'quality.config.json'), JSON.stringify({ heavyRun: { cooldownMinutes: 180 } }), 'utf8');
    assert.equal(isHeavyCargoCommand(['check']), false);
    assert.equal(isHeavyCargoCommand(['build']), false);
    const light = await inspectHeavyRun({ projectRoot: root, targetBase, mode: 'local-light' });
    assert.equal(light.allowed, true);
    const afterLight = await inspectHeavyRun({ projectRoot: root, targetBase, mode: 'full' });
    assert.equal(afterLight.allowed, true, 'un comando ligero no puede arrancar el cooldown');
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(path.join(root, '..', 'glory-quality-guard'), { recursive: true, force: true });
  }
});
