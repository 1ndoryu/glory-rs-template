import assert from 'node:assert/strict';
import test from 'node:test';
import { matches } from '../scope.mjs';

test('scope usa globs deterministas y normaliza separadores', () => {
  assert.equal(matches('frontend/src/router.ts', 'frontend/**/*.ts'), true);
  assert.equal(matches('frontend/router.ts', 'frontend/**/*.ts'), true);
  assert.equal(matches('frontend/src/router.test.ts', 'frontend/**/*.css'), false);
  assert.equal(matches('src/styles/app.css', '.css'), true);
  assert.equal(matches('scripts/quality/cache.mjs', 'scripts/quality/'), true);
  assert.equal(matches('frontend/src/router.ts', 'backend/**/*.ts'), false);
  assert.equal(matches('frontend\\src\\router.ts', 'frontend/**/*.ts'), true);
});
