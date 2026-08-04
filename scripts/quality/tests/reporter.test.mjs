import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { compactLines, createReport } from '../reporter.mjs';

test('la salida compacta conserva estado, siguiente accion y limite de contexto', () => {
  const reportResult = {
    markdownPath: 'C:/repo/.quality-reports/T-1/latest.md',
    report: {
      taskId: 'T-1',
      decision: { label: 'FAIL' },
      scope: { full: true, executionFull: false, files: ['a.ts'] },
      stages: Array.from({ length: 5 }, (_, index) => ({
        stage: `stage-${index}`,
        status: index === 0 ? 'fail' : 'pass',
        cached: false,
        summary: 'resumen',
      })),
      findings: Array.from({ length: 20 }, (_, index) => ({
        severity: 'error', ruleId: `R${index}`, message: 'hallazgo',
      })),
      reminders: ['uno', 'dos', 'tres', 'cuatro'],
      policy: { policyHash: 'abc123', reason: 'política v2 válida', decision: { action: 'enforce' } },
      nextCommand: 'npm run task:check -- T-1',
    },
  };
  const context = { projectRoot: 'C:/repo', qualityConfig: { maxFindings: 3 } };
  const lines = compactLines(reportResult, context);

  assert.ok(lines.length <= 16);
  assert.match(lines[0], /T-1 — FAIL/);
  assert.match(lines[1], /full · ejecución incremental/);
  assert.equal(lines.filter(line => line.includes('hallazgo')).length, 3);
  assert.match(lines.at(-1), /Next: npm run task:check/);
});

test('createReport serializa la identidad de política en JSON y Markdown', async () => {
  const projectRoot = await mkdtemp(path.join(os.tmpdir(), 'quality-reporter-policy-'));
  try {
    await mkdir(path.join(projectRoot, '.quality-reports', 'T-2'), { recursive: true });
    const result = await createReport(
      {
        projectRoot,
        reportRoot: path.join(projectRoot, '.quality-reports', 'T-2'),
        qualityConfig: { maxFindings: 3 },
        tools: {},
        policyIdentity: {
          projectRoot,
          policyPath: path.join(projectRoot, 'sentinel.config.json'),
          policyHash: 'policy-hash-test',
          runtimeVersion: '0.4.0',
          decision: { status: 'policy', mode: 'enforce', action: 'enforce', blocked: false, reason: 'política v2 válida' },
          reason: 'política v2 válida',
          recommendedCommand: 'npm run task:check -- T-2',
        },
      },
      { taskId: 'T-2', ci: false, full: false },
      { base: 'HEAD', full: false, executionFull: false, files: [], profiles: [] },
      [{ stage: 'sentinel', status: 'pass', durationMs: 1, findings: [], summary: '0 errores' }],
      [],
      Date.now(),
    );
    const json = JSON.parse(await readFile(result.jsonPath, 'utf8'));
    const markdown = await readFile(result.markdownPath, 'utf8');
    assert.equal(json.policy.policyHash, 'policy-hash-test');
    assert.match(markdown, /policy-hash-test/);
    assert.match(markdown, /política v2 válida/);
  } finally {
    await rm(projectRoot, { recursive: true, force: true });
  }
});
