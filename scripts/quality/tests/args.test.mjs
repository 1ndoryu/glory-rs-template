import assert from 'node:assert/strict';
import test from 'node:test';
import { parseArgs } from '../args.mjs';

test('parseArgs exige un task ID válido', () => {
  assert.equal(parseArgs(['297A-6']).taskId, '297A-6');
  assert.throws(() => parseArgs([]), /Uso:/);
  assert.throws(() => parseArgs(['tarea']), /Uso:/);
});

test('parseArgs acepta flags internos conocidos', () => {
  const args = parseArgs(['297A-6', '--fresh', '--allow-heavy', '--base', 'HEAD~1']);
  assert.equal(args.fresh, true);
  assert.equal(args.allowHeavy, true);
  assert.equal(args.base, 'HEAD~1');
});
