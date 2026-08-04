import assert from 'node:assert/strict';
import test from 'node:test';
import { buildVarsenseInvocation, VARSENSE_SCOPE_LIMITATION_CODE } from '../adapters/varsense-contract.mjs';

test('VarSense usa una sola invocación all y comparte el reportRoot del gate', () => {
  const invocation = buildVarsenseInvocation({
    projectRoot: 'C:/repo',
    reportRoot: 'C:/repo/.quality-reports/branches/main/T-1',
    tools: { varsense: { cliPath: 'C:/repo/.quality-tools/varsense/dist/cli/index.js', version: '2.2.0' } },
  }, { executionFull: true });
  assert.equal(invocation.args[1], 'all');
  assert.equal(invocation.args.includes('--files-from'), false);
  assert.equal(invocation.reportPath.replace(/\\/g, '/'), 'C:/repo/.quality-reports/branches/main/T-1/varsense-all.json');
  assert.deepEqual(invocation.scope, {
    requestedScopedAnalysis: false,
    applied: true,
    manifestPath: null,
    limitation: null,
  });
});

test('VarSense deja constancia de scope solicitado no aplicable en CLI 2.2.0', () => {
  const invocation = buildVarsenseInvocation({
    projectRoot: 'C:/repo',
    reportRoot: 'C:/reports/T-1',
    tools: { varsense: { cliPath: 'varsense.js', version: '2.2.0' } },
  }, { executionFull: false, changedFilesPath: 'C:/reports/T-1/changed-files.txt' });
  assert.equal(invocation.args[1], 'all');
  assert.equal(invocation.scope.requestedScopedAnalysis, true);
  assert.equal(invocation.scope.applied, false);
  assert.equal(invocation.scope.manifestPath, 'C:/reports/T-1/changed-files.txt');
  assert.equal(invocation.scope.limitation, VARSENSE_SCOPE_LIMITATION_CODE('2.2.0'));
});
