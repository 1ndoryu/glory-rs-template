import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { expandLocalDependencies, matches } from '../scope.mjs';

test('scope usa globs deterministas y normaliza separadores', () => {
  assert.equal(matches('frontend/src/router.ts', 'frontend/**/*.ts'), true);
  assert.equal(matches('frontend/router.ts', 'frontend/**/*.ts'), true);
  assert.equal(matches('frontend/src/router.test.ts', 'frontend/**/*.css'), false);
  assert.equal(matches('src/styles/app.css', '.css'), true);
  assert.equal(matches('scripts/quality/cache.mjs', 'scripts/quality/'), true);
  assert.equal(matches('frontend/src/router.ts', 'backend/**/*.ts'), false);
  assert.equal(matches('frontend\\src\\router.ts', 'frontend/**/*.ts'), true);
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
