import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { expandLocalDependencies, matches, resolveExplicitProfiles, resolveFullDecision } from '../scope.mjs';

test('scope usa globs deterministas y normaliza separadores', () => {
  assert.equal(matches('frontend/src/router.ts', 'frontend/**/*.ts'), true);
  assert.equal(matches('frontend/router.ts', 'frontend/**/*.ts'), true);
  assert.equal(matches('frontend/src/router.test.ts', 'frontend/**/*.css'), false);
  assert.equal(matches('src/styles/app.css', '.css'), true);
  assert.equal(matches('scripts/quality/cache.mjs', 'scripts/quality/'), true);
  assert.equal(matches('frontend/src/router.ts', 'backend/**/*.ts'), false);
  assert.equal(matches('frontend\\src\\router.ts', 'frontend/**/*.ts'), true);
});

test('resolveExplicitProfiles aplica CLI sobre entorno y allowlist estricta', () => {
  const available = { docs: ['.md'], rust: ['.rs'] };
  const cli = resolveExplicitProfiles({ profiles: ['docs'] }, available, {
    GLORY_QUALITY_PROFILE: 'rust',
  });
  assert.equal(cli.explicit, true);
  assert.equal(cli.source, 'cli');
  assert.deepEqual([...cli.profiles], ['docs']);

  const env = resolveExplicitProfiles({ profiles: [] }, available, {
    GLORY_QUALITY_PROFILE: 'rust,docs,rust',
  });
  assert.equal(env.source, 'env');
  assert.deepEqual([...env.profiles], ['rust', 'docs']);
  assert.throws(
    () => resolveExplicitProfiles({ profiles: ['unknown'] }, available, {}),
    /Perfil no permitido: unknown/,
  );
  assert.throws(
    () => resolveExplicitProfiles({ profiles: ['auth'] }, { ...available, auth: ['auth'] }, {}),
    /Perfil sin etapa ejecutable: auth/,
  );
});

test('scope incluye dependencias locales en el fingerprint incremental', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'quality-scope-'));
  try {
    await writeFile(path.join(root, 'entry.ts'), "import { value } from './dependency';\nexport { value };\n", 'utf8');
    await writeFile(path.join(root, 'dependency.ts'), 'export const value = 1;\n', 'utf8');
    assert.deepEqual(await expandLocalDependencies(root, ['entry.ts']), ['dependency.ts', 'entry.ts']);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('resolveFullDecision separa requested/automatic/deferred/effective (028A-8)', () => {
  /* Full pedido y permitido: fingerprint full y ejecución full. */
  assert.deepEqual(
    resolveFullDecision({ requested: true, automatic: false, deferred: false, explicit: false }),
    { full: true, effectiveFull: true, executionFull: true },
  );
  /* AutomaticFull sin --full: el fingerprint es full aunque nadie lo pidió. */
  assert.deepEqual(
    resolveFullDecision({ requested: false, automatic: true, deferred: false, explicit: false }),
    { full: true, effectiveFull: true, executionFull: true },
  );
  /* Full diferido por el guard: effectiveFull=false real, no simulado. */
  assert.deepEqual(
    resolveFullDecision({ requested: true, automatic: true, deferred: true, explicit: false }),
    { full: true, effectiveFull: false, executionFull: false },
  );
  /* Perfil explícito con full pedido: fingerprint full, ejecución filtrada. */
  assert.deepEqual(
    resolveFullDecision({ requested: true, automatic: false, deferred: false, explicit: true }),
    { full: true, effectiveFull: true, executionFull: false },
  );
  /* Cambio incremental ordinario. */
  assert.deepEqual(
    resolveFullDecision({ requested: false, automatic: false, deferred: false, explicit: false }),
    { full: false, effectiveFull: false, executionFull: false },
  );
});
