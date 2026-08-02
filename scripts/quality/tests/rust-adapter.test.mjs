import assert from 'node:assert/strict';
import test from 'node:test';
import { shouldRunExtendedChecks } from '../adapters/rust.mjs';

test('Rust local-light no ejecuta clippy/tests', () => {
  assert.equal(shouldRunExtendedChecks({ ci: false, full: false }), false);
});

test('Rust full y CI sí ejecutan clippy/tests', () => {
  assert.equal(shouldRunExtendedChecks({ ci: false, full: true }), true);
  assert.equal(shouldRunExtendedChecks({ ci: true, full: false }), true);
});
