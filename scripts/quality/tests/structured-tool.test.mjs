import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { runStructuredTool } from '../adapters/structured-tool.mjs';

test('structured adapter describe un reporte versionado', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'structured-adapter-'));
  const context = {
    projectRoot: root,
    reportRoot: root,
    logsRoot: root,
    qualityConfig: { maxFindings: 3 },
  };
  const result = await runStructuredTool(context, {
    name: 'structured-test',
    executable: process.execPath,
    args: ['-e', 'process.stdout.write(\"ok\")'],
    reportPath: path.join(root, 'missing-structured-report.json'),
    expectedSchemaVersion: '1',
    timeoutMs: 2000,
  });
  try {
    assert.equal(result.failure.stage, 'structured-test');
    assert.equal(result.failure.status, 'error');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
