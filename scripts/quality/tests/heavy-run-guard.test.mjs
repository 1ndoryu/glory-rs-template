import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
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

test('solo test, clippy y bench son comandos Cargo pesados', () => {
  assert.equal(isHeavyCargoCommand(['test']), true);
  assert.equal(isHeavyCargoCommand(['--locked', 'clippy']), true);
  assert.equal(isHeavyCargoCommand(['check']), false);
  assert.equal(isHeavyCargoCommand(['fmt', '--check']), false);
});
