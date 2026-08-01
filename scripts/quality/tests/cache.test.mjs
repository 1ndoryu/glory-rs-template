import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fingerprint, readCachedPass, writeCachedPass } from '../cache.mjs';

test('cache de calidad distingue pass, cambios de archivo y formato', async () => {
  const projectRoot = await mkdtemp(path.join(os.tmpdir(), 'quality-cache-'));
  try {
    await writeFile(path.join(projectRoot, 'input.ts'), 'export const value = 1;\n', 'utf8');
    const context = {
      projectRoot,
      qualityConfig: { schemaVersion: 1, lockWaitMs: 0 },
      toolManifest: { schemaVersion: 1, tools: {} },
    };
    const scope = { files: ['input.ts'], fingerprintFiles: ['input.ts'] };
    const first = await fingerprint(context, scope, 'frontend');
    await writeCachedPass(context, 'frontend', first, { status: 'pass', durationMs: 3 });
    const cached = await readCachedPass(context, 'frontend', first);
    assert.equal(cached.cached, true);
    assert.equal(cached.status, 'pass');

    await writeFile(path.join(projectRoot, 'input.ts'), 'export const value = 2;\n', 'utf8');
    const second = await fingerprint(context, scope, 'frontend');
    assert.notEqual(second, first);
    assert.equal(await readCachedPass(context, 'frontend', second), null);
    assert.match(await readFile(path.join(projectRoot, '.quality-reports', 'cache', 'frontend.json'), 'utf8'), /fingerprint/);
  } finally {
    await rm(projectRoot, { recursive: true, force: true });
  }
});
