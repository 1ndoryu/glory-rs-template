import assert from 'node:assert/strict';
import test from 'node:test';
import { compactLines } from '../reporter.mjs';

test('la salida compacta conserva estado, siguiente accion y limite de contexto', () => {
  const reportResult = {
    markdownPath: 'C:/repo/.quality-reports/T-1/latest.md',
    report: {
      taskId: 'T-1',
      decision: { label: 'FAIL' },
      scope: { full: true, files: ['a.ts'] },
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
      nextCommand: 'npm run task:check -- T-1',
    },
  };
  const context = { projectRoot: 'C:/repo', qualityConfig: { maxFindings: 3 } };
  const lines = compactLines(reportResult, context);

  assert.ok(lines.length <= 16);
  assert.match(lines[0], /T-1 — FAIL/);
  assert.equal(lines.filter(line => line.includes('hallazgo')).length, 3);
  assert.match(lines.at(-1), /Next: npm run task:check/);
});
