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

test('runner limita la captura de salida ruidosa', async () => {
  const noisy = await runProcess(
    process.execPath,
    ['-e', 'process.stdout.write("x".repeat(100000))'],
    { timeoutMs: 2_000 },
  );
  assert.equal(noisy.code, 0);
  assert.match(noisy.stdout, /quality output truncated at 65536 bytes/);
  assert.ok(noisy.stdout.length < 70_000);
});
