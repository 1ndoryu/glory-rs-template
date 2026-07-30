import assert from 'node:assert/strict';
import test from 'node:test';
import { runProcess } from '../runner.mjs';

test('runner distingue éxito y timeout', async () => {
  const success = await runProcess(process.execPath, ['-e', 'process.stdout.write("ok")'], { timeoutMs: 2_000 });
  assert.equal(success.code, 0);
  assert.equal(success.stdout, 'ok');

  const timeout = await runProcess(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], { timeoutMs: 100 });
  assert.equal(timeout.code, 2);
  assert.equal(timeout.timedOut, true);
});
