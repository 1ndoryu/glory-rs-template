import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile, mkdir } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { percentile, summarize } from '../quality-profile.mjs';

test('percentile calcula p50 y p95 nearest-rank sin mutar la entrada', () => {
  const values = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
  assert.equal(percentile([...values].sort((a, b) => a - b), 0.5), 5);
  assert.equal(percentile([...values].sort((a, b) => a - b), 0.95), 10);
  assert.equal(percentile([], 0.5), null);
  assert.equal(percentile([7], 0.5), 7);
  assert.equal(percentile([100, 400], 0.95), 400);
});

test('summarize agrupa p50/p95/min/max/mean y descarta no finitos', () => {
  const summary = summarize([100, 200, 300, 400, 500, Number.NaN, Number.POSITIVE_INFINITY]);
  assert.equal(summary.samples, 5);
  assert.equal(summary.p50, 300);
  assert.equal(summary.p95, 500);
  assert.equal(summary.min, 100);
  assert.equal(summary.max, 500);
  assert.equal(summary.mean, 300);
  assert.deepEqual(summarize([]), { samples: 0, p50: null, p95: null, min: null, max: null, mean: null });
});

test('quality-profile lee reportes reales y calcula p50/p95 por etapa (028A-8 Fase 4)', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'quality-profile-'));
  try {
    const branchRoot = path.join(root, 'wandorius--test');
    await mkdir(path.join(branchRoot, 'T-1'), { recursive: true });
    await mkdir(path.join(branchRoot, 'T-2'), { recursive: true });
    const report = (taskId, sentinelMs, varsenseMs, totalMs, cache) => JSON.stringify({
      taskId,
      generatedAt: '2026-08-05T00:00:00.000Z',
      durationMs: totalMs,
      stages: [
        { stage: 'sentinel', status: 'pass', durationMs: sentinelMs, findings: [], summary: 'ok', cache },
        { stage: 'varsense', status: 'pass', durationMs: varsenseMs, findings: [], summary: 'ok', cache },
      ],
    });
    await writeFile(path.join(branchRoot, 'T-1', 'latest.json'), report('T-1', 100, 200, 300, 'hit'), 'utf8');
    await writeFile(path.join(branchRoot, 'T-2', 'latest.json'), report('T-2', 400, 500, 900, 'miss'), 'utf8');
    const { collectReports, buildProfile } = await import('../quality-profile.mjs');
    const entries = await collectReports(branchRoot, null, 20);
    assert.equal(entries.length, 2);
    const profile = buildProfile(entries);
    assert.equal(profile.reports, 2);
    assert.equal(profile.total.p50, 300);
    assert.equal(profile.total.p95, 900);
    const sentinel = profile.stages.find(stage => stage.stage === 'sentinel');
    assert.equal(sentinel.p50, 100);
    assert.equal(sentinel.p95, 400);
    assert.equal(sentinel.cacheHits, 1, 'un miss no puede contar como hit (precedencia ?? vs ?:)');
    const varsense = profile.stages.find(stage => stage.stage === 'varsense');
    assert.equal(varsense.p50, 200);
    assert.equal(varsense.p95, 500);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
