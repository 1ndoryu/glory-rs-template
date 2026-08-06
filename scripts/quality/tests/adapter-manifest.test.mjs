import assert from 'node:assert/strict';
import test from 'node:test';
import { readAdapterManifest, validateAdapterManifest, adapterStageNames, materializeTransportArguments, resolveWorkspacePath, adapterEnvironmentAllowlist, assertTaskId, assertImplementedStages } from '../adapter-manifest.mjs';

const manifest = {
  schemaVersion: 1,
  adapter: { id: 'fixture', version: '0.1.0', protocolVersion: 1, capabilities: ['structured-stage-process'], environment: { mode: 'runner-default', allowlisted: ['CI'] }, output: { schemaVersion: '1', exitCodes: { pass: 0, findings: 1, toolError: 2, cancelled: 130 } } },
  transport: { executable: 'node', entrypoint: 'scripts/quality/stage-process.mjs', arguments: ['--stage', '{stage}', '--report', '{reportPath}', '--task-id', '{taskId}'] },
  stages: { sentinel: { timeoutMs: 1000 }, varsense: { timeoutMs: 1000 }, rust: { timeoutMs: 1000 }, frontend: { timeoutMs: 1000 }, docs: { timeoutMs: 1000 }, custom: { timeoutMs: 1000 } },
  profiles: { frontend: ['varsense', 'frontend', 'custom'] },
};

test('lee y valida el manifest del adapter del proyecto', async () => { const loaded = await readAdapterManifest(process.cwd()); assert.equal(loaded.adapter.id, 'wandorius-quality'); assert.deepEqual(adapterStageNames(loaded, ['frontend'], false), ['sentinel', 'varsense', 'frontend', 'custom']); });
test('materializa argv sin shell y conserva placeholders conocidos', () => { assert.deepEqual(materializeTransportArguments(manifest, { stage: 'docs', reportPath: 'report.json', taskId: 'T-1' }), ['--stage', 'docs', '--report', 'report.json', '--task-id', 'T-1']); assert.throws(() => materializeTransportArguments(manifest, { stage: 'docs', reportPath: '', taskId: 'T-1' }), /Falta valor/); });
test('rechaza placeholders, estados de salida y perfiles desconocidos', () => { assert.throws(() => validateAdapterManifest({ ...manifest, transport: { ...manifest.transport, arguments: ['--eval', '{shell}'] } }), /placeholder no permitido/); assert.throws(() => validateAdapterManifest({ ...manifest, adapter: { ...manifest.adapter, output: { ...manifest.adapter.output, exitCodes: { ...manifest.adapter.output.exitCodes, pass: 1 } } } }), /exitCodes.pass/); assert.throws(() => adapterStageNames(manifest, ['missing'], false), /Perfil de adapter desconocido/); });
test('rechaza rutas y task IDs fuera del contrato', () => { assert.throws(() => resolveWorkspacePath('C:/workspace', '../outside.json', 'report'), /fuera del workspace/); assert.throws(() => resolveWorkspacePath('C:/workspace', 'tmp/report.json', 'report', { allowReportRoot: true }), /\.quality-reports/); assert.equal(resolveWorkspacePath('C:/workspace', '.quality-reports/task/report.json', 'report', { allowReportRoot: true }), 'C:\\workspace\\.quality-reports\\task\\report.json'); assert.throws(() => assertTaskId('../escape'), /identificador inválido/); assert.equal(assertTaskId('SNT-12'), 'SNT-12'); });
test('el allowlist del manifest es efectivo y las etapas declaradas deben estar implementadas', () => { assert.deepEqual(adapterEnvironmentAllowlist(manifest), ['CI']); assert.deepEqual([...new Set(['sentinel', 'frontend'])].filter(stage => assertImplementedStages(manifest, ['sentinel', 'frontend'], ['sentinel', 'frontend'])), ['sentinel', 'frontend']); assert.throws(() => assertImplementedStages(manifest, ['sentinel', 'rust'], ['sentinel']), /sin implementación/); });
