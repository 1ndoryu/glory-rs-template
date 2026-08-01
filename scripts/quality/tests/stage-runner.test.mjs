import assert from 'node:assert/strict';
import test from 'node:test';
import { runBoundedStages } from '../stage-runner.mjs';

test('stage runner conserva orden y respeta concurrencia', async () => {
  let active = 0;
  let maximum = 0;
  const results = await runBoundedStages([1, 2, 3, 4], async value => {
    active += 1;
    maximum = Math.max(maximum, active);
    await new Promise(resolve => setTimeout(resolve, 3));
    active -= 1;
    return value * 2;
  }, { maxConcurrency: 2 });
  assert.deepEqual(results, [2, 4, 6, 8]);
  assert.equal(maximum, 2);
});

test('stage runner cancela antes de iniciar trabajo nuevo', async () => {
  await assert.rejects(
    runBoundedStages([1], async () => 1, { isCancelled: () => true }),
    /cancelado/,
  );
});
