import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, symlink, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {
  defaultGuardPolicy,
  discoverPolicy,
  loadPolicy,
  migrateLegacyConfig,
  validatePolicy,
} from '../policy.mjs';
import { resolveLegacyRoot } from '../sentinel-doctor.mjs';

function validPolicy() {
  return {
    schemaVersion: 2,
    mode: 'enforce',
    gate: { command: ['sentinel', 'check', '--'], taskIdRequired: true },
    guard: { directCommands: defaultGuardPolicy() },
    runtime: { minimumVersion: '0.4.0', protocolVersion: 1, lockFile: 'sentinel.lock.json' },
    analyzers: {
      sentinel: { enabled: true, profile: 'project-default', config: 'sentinel.config.json' },
      varsense: { enabled: true, profile: 'project-default', config: 'varsense.config.json' },
    },
  };
}

test('valida una política v2 y rechaza claves desconocidas o rutas inseguras', () => {
  assert.doesNotThrow(() => validatePolicy(validPolicy()));
  assert.throws(() => validatePolicy({ ...validPolicy(), unexpected: true }), /claves desconocidas/);
  assert.throws(() => validatePolicy({ ...validPolicy(), runtime: { ...validPolicy().runtime, lockFile: '../outside.json' } }), /no puede salir/);
  assert.throws(() => validatePolicy({ ...validPolicy(), mode: 'invalid' }), /mode inválido/);
});

test('mapea la configuración legacy a una política v2 sin perder el analizador v1', () => {
  const migrated = migrateLegacyConfig({
    sentinelConfig: { includePatterns: ['**/*.ts'], rules: { 'catch-vacio': { severidad: 'error' } } },
    qualityConfig: { schemaVersion: 1, maxConcurrentStages: 1 },
    toolManifest: { tools: { sentinel: { version: '0.4.0', outputSchemaVersion: '1' } } },
  });
  assert.equal(migrated.policy.schemaVersion, 2);
  assert.equal(migrated.policy.gate.command[0], 'npm');
  assert.equal(migrated.policy.analyzers.sentinel.config.rules['catch-vacio'].severidad, 'error');
  assert.equal(migrated.policy.runtime.protocolVersion, 1);
  assert.equal(migrated.legacy.qualityConfig.maxConcurrentStages, 1);
  assert.equal(migrated.legacy.toolManifest.tools.sentinel.version, '0.4.0');
});

test('doctor no inventa una migración para un proyecto sin política', async () => {
  assert.throws(() => resolveLegacyRoot({ projectRoot: null }), /No se encontró una raíz/);
});

test('rechaza una política symlink para no cargar configuración fuera del workspace', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'sentinel-policy-link-'));
  const outside = await mkdtemp(path.join(os.tmpdir(), 'sentinel-policy-outside-'));
  try {
    await writeFile(path.join(outside, 'sentinel.config.json'), JSON.stringify(validPolicy()), 'utf8');
    await symlink(path.join(outside, 'sentinel.config.json'), path.join(root, 'sentinel.config.json'), 'file');
    const loaded = await loadPolicy(root);
    assert.equal(loaded.status, 'invalid-policy');
    assert.match(loaded.error, /no puede ser symlink/);
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(outside, { recursive: true, force: true });
  }
});

test('resuelve físicamente un startPath junction antes de buscar la política', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'sentinel-policy-junction-'));
  const physical = await mkdtemp(path.join(os.tmpdir(), 'sentinel-policy-physical-'));
  try {
    await mkdir(path.join(root, 'nested'), { recursive: true });
    await writeFile(path.join(root, 'sentinel.config.json'), JSON.stringify(validPolicy()), 'utf8');
    const linked = path.join(physical, 'linked');
    await symlink(root, linked, 'junction');
    const discovered = await discoverPolicy(path.join(linked, 'nested'));
    assert.equal(discovered.projectRoot, root);
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(physical, { recursive: true, force: true });
  }
});

test('descubre la política en un ancestro y diferencia no-policy de legacy-v1', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'sentinel-policy-'));
  try {
    const nested = path.join(root, 'frontend', 'src');
    await mkdir(nested, { recursive: true });
    await writeFile(path.join(root, 'sentinel.config.json'), JSON.stringify({ includePatterns: [] }), 'utf8');
    const discovered = await discoverPolicy(nested);
    assert.equal(discovered.projectRoot, root);
    assert.equal((await loadPolicy(nested)).status, 'legacy-v1');
    assert.equal((await loadPolicy(path.join(os.tmpdir(), 'no-policy-here'))).status, 'no-policy');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
