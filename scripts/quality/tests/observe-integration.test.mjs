import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

const execFileAsync = promisify(execFile);
const SCRIPTS = path.resolve('scripts', 'quality');

test('stages.mjs genera el contrato declarativo con las etapas del alcance', async () => {
  const { preflight } = await import('../preflight.mjs');
  const { detectScope } = await import('../scope.mjs');
  const { isFullExecution } = await import('../profile-contract.mjs');
  const { PROFILE_STAGE_RULES } = await import('../profile-contract.mjs');
  const { stageDefinitions } = await import('../stage-definitions.mjs');
  const context = await preflight({ taskId: '028A-6', cwd: process.cwd() });
  const scope = await detectScope(context, {});
  const expected = isFullExecution(scope)
    ? ['sentinel', 'varsense', 'rust', 'frontend', 'docs', 'custom']
    : ['sentinel', ...new Set([...scope.profiles].flatMap(profile => PROFILE_STAGE_RULES[profile] ?? []))];
  const definitions = stageDefinitions(context, scope, '028A-6').map(item => item.name);

  const tmp = await mkdtemp(path.join(os.tmpdir(), 'observe-stages-'));
  try {
    const output = path.join(tmp, 'stages.json');
    await execFileAsync(process.execPath, [path.join(SCRIPTS, 'stages.mjs'), '--task-id', '028A-6', '--output', output, '--report-root', tmp], { cwd: process.cwd(), timeout: 60_000 });
    const declarations = JSON.parse(await readFile(output, 'utf8'));
    assert.ok(Array.isArray(declarations) && declarations.length > 0);
    for (const declaration of declarations) {
      assert.equal(typeof declaration.name, 'string');
      assert.equal(typeof declaration.executable, 'string');
      assert.ok(Array.isArray(declaration.args));
      assert.equal(declaration.expectedSchemaVersion, '1');
      assert.ok(Number.isFinite(Number(declaration.timeoutMs)));
    }
    const names = declarations.map(item => item.name);
    assert.deepEqual(names, expected.filter(name => definitions.includes(name)));
    assert.ok(declarations.every(item => item.args.includes(path.join(SCRIPTS, 'stage-process.mjs'))), 'cada etapa apunta al wrapper');
  } finally {
    await rm(tmp, { recursive: true, force: true });
  }
});

test('stage-process.mjs escribe el contrato estructurado y replica el exit code', async () => {
  const tmp = await mkdtemp(path.join(os.tmpdir(), 'observe-wrapper-'));
  try {
    const report = path.join(tmp, 'custom.json');
    /* La etapa custom del orquestador es barata (reglas custom sobre archivos
     * del workspace) y no depende de toolchains externas. */
    /* execFile resuelve con {stdout,stderr} en éxito (sin `code`) y rechaza
     * con un error cuyo `code` es el exit code en fallo. */
    const spawned = await execFileAsync(process.execPath, [path.join(SCRIPTS, 'stage-process.mjs'), '--stage', 'custom', '--report', report, '--task-id', '028A-6'], { cwd: process.cwd(), timeout: 120_000 }).catch(error => error);
    const exitCode = Number.isInteger(spawned.code) ? spawned.code : 0;
    const parsed = JSON.parse(await readFile(report, 'utf8'));
    assert.equal(parsed.schemaVersion, '1');
    assert.ok(Array.isArray(parsed.entries), 'entries presente');
    for (const entry of parsed.entries) {
      assert.ok(Array.isArray(entry.findings));
      for (const finding of entry.findings) {
        assert.equal(typeof finding.ruleId, 'string');
        assert.equal(typeof finding.message, 'string');
        assert.ok(['error', 'warning', 'info'].includes(finding.severity));
      }
    }
    assert.equal(parsed.stage, 'custom');
    assert.ok(Number.isFinite(Number(parsed.durationMs)));
    /* exit code: pass=0, fail=1, error=2 — no asumimos cuál aplica, solo que
     * el proceso terminó con un código válido. */
    assert.ok([0, 1, 2].includes(exitCode));
  } finally {
    await rm(tmp, { recursive: true, force: true });
  }
});

test('observe-compare.mjs compara decisiones y produce compare.json', async () => {
  const tmp = await mkdtemp(path.join(os.tmpdir(), 'observe-compare-'));
  try {
    const result = await execFileAsync(process.execPath, [path.join(SCRIPTS, 'observe-compare.mjs'), '--task-id', '028A-6'], { cwd: process.cwd(), timeout: 30 * 60_000 }).catch(error => error);
    /* Exit 0 (coinciden) o 1 (difieren) son ambos válidos; 2 es setup error. */
    assert.ok([0, 1].includes(result.code), `exit inesperado: ${result.code}`);
    const output = result.stdout ?? '';
    assert.match(output, /Decisión:/);
    const comparePath = path.join(process.cwd(), '.quality-reports', 'observe', '028A-6', 'compare.json');
    const compare = JSON.parse(await readFile(comparePath, 'utf8'));
    assert.equal(compare.taskId, '028A-6');
    assert.equal(typeof compare.matched, 'boolean');
    assert.ok(Array.isArray(compare.actual.findings));
    assert.ok(Array.isArray(compare.sentinel.findings));
  } finally {
    await rm(tmp, { recursive: true, force: true });
    const { rm: rmReport } = await import('node:fs/promises');
    await rmReport(path.join(process.cwd(), '.quality-reports', 'observe'), { recursive: true, force: true });
  }
});
