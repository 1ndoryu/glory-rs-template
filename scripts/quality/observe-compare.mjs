#!/usr/bin/env node
/* [028A-6 Fase 3] Doble vía observe: ejecuta el gate actual (`task:check`)
 * y el gate agnóstico (`sentinel check --stages` con los adaptadores del
 * orquestador) sobre la misma tarea, y compara las decisiones y hallazgos
 * normalizados. El modo observe NO cambia el enforcement del workspace:
 * solo informa diferencias para decidir cuándo activar enforce en el core.
 * Exit 0 = decisiones coinciden; 1 = diferencias (fail en uno u otro);
 * 2 = error de configuración. */
import path from 'node:path';
import { execFile } from 'node:child_process';
import { mkdir, readFile } from 'node:fs/promises';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { projectRoot } from './preflight.mjs';

const execFileAsync = promisify(execFile);

function parseArgs(argv) {
  const parsed = { taskId: null, full: false, ci: false, profile: null, base: null, keepStages: false };
  for (let index = 0; index < argv.length; index++) {
    const arg = argv[index];
    if (arg === '--task-id') parsed.taskId = argv[++index] ?? null;
    else if (arg === '--full') parsed.full = true;
    else if (arg === '--ci') parsed.ci = true;
    else if (arg === '--profile') parsed.profile = argv[++index] ?? null;
    else if (arg === '--base') parsed.base = argv[++index] ?? null;
    else if (arg === '--keep-stages') parsed.keepStages = true;
  }
  return parsed;
}

/* [028A-6 Fase 3] Los reportes guardan findings en el nivel de etapa
 * (core y orquestador) y además agregados en la raíz (solo core). Se
 * agregan ambos niveles y se deduplican por ruleId:file:line para
 * comparar decisiones equivalentes entre gates. */
function normalizeFindings(report) {
  const seen = new Set();
  const findings = [];
  const push = (finding) => {
    if (!finding || typeof finding !== 'object') return;
    const ruleId = String(finding.ruleId ?? 'unknown');
    const severity = String(finding.severity ?? 'warning');
    const file = finding.file ? String(finding.file).replace(/\\/g, '/') : null;
    const line = Number.isInteger(finding.line) ? finding.line : null;
    const key = `${ruleId}:${file ?? ''}:${line ?? ''}`;
    if (seen.has(key)) return;
    seen.add(key);
    findings.push({ ruleId, severity, file, line, message: String(finding.message ?? '') });
  };
  for (const stage of report.stages ?? []) {
    for (const finding of stage.findings ?? []) push(finding);
  }
  for (const finding of report.findings ?? []) push(finding);
  return findings.sort((left, right) =>
    left.ruleId.localeCompare(right.ruleId) || (left.file ?? '').localeCompare(right.file ?? ''));
}

function normalizeDecision(report) {
  const stageFailures = (report.stages ?? []).filter(stage => stage.status === 'fail' || stage.status === 'error');
  return {
    label: report.decision?.label ?? (stageFailures.length > 0 ? 'FAIL' : 'PASS'),
    failStages: stageFailures.map(stage => stage.stage),
    findings: normalizeFindings(report),
  };
}

function renderDiff(actual, sentinel, taskId) {
  const lines = [`[observe] Comparación ${taskId} — gate actual (task:check) vs gate agnóstico (sentinel check)`];
  const decisionMatch = actual.label === sentinel.label;
  lines.push(`[observe] Decisión: ${actual.label} (actual) vs ${sentinel.label} (agnóstico) — ${decisionMatch ? 'coincide' : 'DIFIERE'}`);
  if (!decisionMatch) {
    lines.push(`[observe]   Etapas fallidas actual: ${actual.failStages.join(', ') || 'ninguna'}`);
    lines.push(`[observe]   Etapas fallidas agnóstico: ${sentinel.failStages.join(', ') || 'ninguna'}`);
  }
  const actualIds = new Set(actual.findings.map(item => `${item.ruleId}:${item.file ?? ''}:${item.line ?? ''}`));
  const sentinelIds = new Set(sentinel.findings.map(item => `${item.ruleId}:${item.file ?? ''}:${item.line ?? ''}`));
  const onlyActual = actual.findings.filter(item => !sentinelIds.has(`${item.ruleId}:${item.file ?? ''}:${item.line ?? ''}`));
  const onlySentinel = sentinel.findings.filter(item => !actualIds.has(`${item.ruleId}:${item.file ?? ''}:${item.line ?? ''}`));
  lines.push(`[observe] Hallazgos: ${actual.findings.length} (actual) vs ${sentinel.findings.length} (agnóstico)`);
  for (const item of onlyActual.slice(0, 5)) {
    lines.push(`[observe]   solo-actual: ${item.ruleId} ${item.file ?? ''}:${item.line ?? ''} ${item.message}`);
  }
  for (const item of onlySentinel.slice(0, 5)) {
    lines.push(`[observe]   solo-agnóstico: ${item.ruleId} ${item.file ?? ''}:${item.line ?? ''} ${item.message}`);
  }
  if (onlyActual.length === 0 && onlySentinel.length === 0) lines.push('[observe]   hallazgos idénticos');
  return lines.join('\n');
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.taskId) {
    process.stderr.write('[observe] requiere --task-id <id>\n');
    process.exitCode = 2;
    return;
  }
  const observeRoot = path.join(projectRoot, '.quality-reports', 'observe', args.taskId);
  await mkdir(observeRoot, { recursive: true });
  const stagesJson = path.join(observeRoot, 'stages.json');
  /* [028A-6] El core escribe su reporte combinado en
   * .quality-reports/check/<task-id>/latest.json (checkCliTarget); los
   * reportes de etapa del wrapper van a `sentinel-check` (solo diagnósticos). */
  const sentinelReport = path.join(observeRoot, 'sentinel-check');
  const sentinelCheckRoot = path.join(projectRoot, '.quality-reports', 'check', args.taskId);

  /* 1. Gate actual: task:check. --base permite comparar el diff de un
   * commit histórico (árbol limpio) en vez de solo el working tree. */
  const actualArgs = ['run', 'task:check', '--', args.taskId, ...(args.full ? ['--full'] : []), ...(args.ci ? ['--ci'] : []), ...(args.profile ? ['--profile', args.profile] : []), ...(args.base ? ['--base', args.base] : [])];
  let actual = null;
  try {
    await execFileAsync(process.platform === 'win32' ? 'npm.cmd' : 'npm', actualArgs, { cwd: projectRoot, timeout: 30 * 60_000 });
  } catch (error) {
    actual = error; /* exit no-cero esperado cuando hay fallos. */
  }

  /* 2. Generar stages y ejecutar el gate agnóstico. */
  const { resolveBranchIdentity, branchReportRoot } = await import('./branch-identity.mjs');
  const identity = await resolveBranchIdentity(projectRoot);
  const actualReportPath = path.join(branchReportRoot(projectRoot, identity), args.taskId, 'latest.json');
  /* [028A-6 Fase 3] La comparación solo es válida si ambos gates ejecutan el
   * mismo alcance. task:check consulta el guard de ejecuciones pesadas y
   * puede diferir a local-light; el gate agnóstico debe reutilizar el
   * scope-manifest que detectScope escribió, no recalcular el suyo (que
   * ignoraría el diferimiento y ejecutaría full). Si el gate actual falló en
   * setup (sin reporte ni manifiesto), no hay alcance válido que comparar. */
  const actualScopeManifest = path.join(path.dirname(actualReportPath), 'scope-manifest.json');
  let actualManifestExists = false;
  try {
    await readFile(actualScopeManifest, 'utf8');
    actualManifestExists = true;
  } catch { /* No reporte del gate actual: abajo se reporta y se aborta. */ }
  const sentinelCli = path.join(projectRoot, 'tools', 'sentinel', 'out', 'cli', 'index.js');
  const generateArgs = ['scripts/quality/stages.mjs', '--task-id', args.taskId, '--output', stagesJson, '--report-root', sentinelReport, '--scope-manifest', actualScopeManifest, ...(args.full ? ['--full'] : []), ...(args.ci ? ['--ci'] : []), ...(args.profile ? ['--profile', args.profile] : []), ...(args.base ? ['--base', args.base] : [])];
  if (!actualManifestExists) {
    process.stderr.write(`[observe] SETUP ERROR — el gate actual no escribió scope-manifest (${actualScopeManifest}); no hay alcance válido para comparar.\n`);
    process.exitCode = 2;
    return;
  }
  await execFileAsync(process.execPath, generateArgs, { cwd: projectRoot, timeout: 60_000 });
  let sentinelRun = null;
  try {
    await execFileAsync(process.execPath, [sentinelCli, 'check', args.taskId, '--stages', stagesJson, '--workspace', projectRoot, ...(args.full ? ['--full'] : []), ...(args.ci ? ['--ci'] : []), ...(args.profile ? ['--profile', args.profile] : [])], { cwd: projectRoot, timeout: 30 * 60_000 });
  } catch (error) {
    sentinelRun = error; /* exit no-cero esperado cuando hay fallos. */
  }

  /* 3. Leer ambos reportes normalizados. */
  const sentinelReportPath = path.join(sentinelCheckRoot, 'latest.json');
  const actualJson = JSON.parse(await readFile(actualReportPath, 'utf8'));
  const sentinelJson = JSON.parse(await readFile(sentinelReportPath, 'utf8'));

  const actualNorm = normalizeDecision(actualJson);
  const sentinelNorm = normalizeDecision(sentinelJson);
  const diff = renderDiff(actualNorm, sentinelNorm, args.taskId);
  process.stdout.write(`${diff}\n`);
  const report = { taskId: args.taskId, actual: actualNorm, sentinel: sentinelNorm, matched: actualNorm.label === sentinelNorm.label };
  await import('./atomic-file.mjs').then(({ writeAtomic }) => writeAtomic(path.join(observeRoot, 'compare.json'), `${JSON.stringify(report, null, 2)}\n`));
  if (!args.keepStages) {
    const { rm } = await import('node:fs/promises');
    await rm(stagesJson, { force: true });
  }
  process.exitCode = actualNorm.label === sentinelNorm.label ? 0 : 1;
}

await main();
