import { execFile } from 'node:child_process';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { promisify } from 'node:util';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { summarize } from './quality-profile.mjs';

const execFileAsync = promisify(execFile);
const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const DEFAULT_OUT = path.join(projectRoot, '.quality-bench', 'baseline.json');

/* [028A-8 Fase 0] Baseline reproducible: ejecuta el gate N veces limpias
 * (--fresh, sin caché) y N incrementales (caché caliente), lee el metrics.json
 * de cada ejecución y agrega p50/p95 por etapa y total. El baseline se guarda
 * FUERA de `.quality-reports/cache` (`.quality-bench/baseline.json`) para no
 * contaminar los fingerprints del gate. No es parte del gate: es diagnóstico. */

function parseArgs(argv) {
  const parsed = { taskId: '028A-16', clean: 5, incremental: 5, json: DEFAULT_OUT, dryRun: false };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--task') parsed.taskId = argv[++index] ?? parsed.taskId;
    else if (arg === '--clean') parsed.clean = Number(argv[++index]) || 5;
    else if (arg === '--incremental') parsed.incremental = Number(argv[++index]) || 5;
    else if (arg === '--json') parsed.json = argv[++index] ?? parsed.json;
    else if (arg === '--dry-run') parsed.dryRun = true;
  }
  return parsed;
}

/* [028A-8] Localiza el metrics.json más reciente de la tarea bajo el árbol de
 * ramas: el branch key depende de la identidad git actual. */
async function latestMetrics(taskId) {
  const branchesRoot = path.join(projectRoot, '.quality-reports', 'branches');
  let best = null;
  let bestTime = 0;
  const branches = await readdir(branchesRoot, { withFileTypes: true }).catch(() => []);
  for (const branch of branches) {
    if (!branch.isDirectory()) continue;
    const metricsPath = path.join(branchesRoot, branch.name, taskId, 'metrics.json');
    try {
      const metrics = JSON.parse(await readFile(metricsPath, 'utf8'));
      const generatedAt = Date.parse(metrics.generatedAt);
      if (Number.isFinite(generatedAt) && generatedAt >= bestTime) {
        best = metrics;
        bestTime = generatedAt;
      }
    } catch { /* tarea sin métricas en esta rama */ }
  }
  return best;
}

async function runGateOnce(taskId, fresh) {
  const args = ['scripts/quality/task-check.mjs', taskId];
  if (fresh) args.push('--fresh');
  const result = await execFileAsync(process.execPath, args, { cwd: projectRoot, windowsHide: true });
  const metrics = await latestMetrics(taskId);
  if (!metrics) throw new Error(`Sin metrics.json tras ejecutar task:check ${taskId}${fresh ? ' --fresh' : ''}`);
  return { ...metrics, exitCode: result.code ?? 0 };
}

/* [028A-8] Agrega ejecuciones por etapa y total: p50/p95 con summarize. */
export function aggregateRuns(runs) {
  const stageNames = [...new Set(runs.flatMap(run => run.stages?.map(stage => stage.stage) ?? []))];
  const stages = stageNames.map(stage => {
    const samples = runs
      .flatMap(run => run.stages ?? [])
      .filter(item => item.stage === stage)
      .map(item => item.durationMs)
      .filter(Number.isFinite);
    return { stage, ...summarize(samples) };
  });
  return {
    runs: runs.length,
    total: summarize(runs.map(run => run.durationMs).filter(Number.isFinite)),
    stages,
  };
}

export function formatBaseline(baseline) {
  const lines = [`[bench] ${baseline.taskId} · limpias ${baseline.clean.runs} · incrementales ${baseline.incremental.runs}`];
  for (const mode of ['clean', 'incremental']) {
    const section = baseline[mode];
    lines.push(`[bench] ${mode}: total p50 ${section.total.p50}ms · p95 ${section.total.p95}ms`);
    for (const stage of section.stages) {
      lines.push(`[bench]   ${stage.stage.padEnd(9)} p50 ${stage.p50 ?? '—'}ms · p95 ${stage.p95 ?? '—'}ms (${stage.samples} muestras)`);
    }
  }
  return lines;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.dryRun) {
    process.stdout.write(`[bench] Dry run: ${args.clean} limpias + ${args.incremental} incrementales de ${args.taskId} → ${path.relative(projectRoot, args.json)}\n`);
    return;
  }
  const cleanRuns = [];
  for (let index = 0; index < args.clean; index += 1) cleanRuns.push(await runGateOnce(args.taskId, true));
  const incrementalRuns = [];
  for (let index = 0; index < args.incremental; index += 1) incrementalRuns.push(await runGateOnce(args.taskId, false));
  const baseline = {
    schemaVersion: 1,
    taskId: args.taskId,
    generatedAt: new Date().toISOString(),
    machine: { platform: process.platform, arch: process.arch, node: process.version },
    clean: aggregateRuns(cleanRuns),
    incremental: aggregateRuns(incrementalRuns),
  };
  const { mkdir } = await import('node:fs/promises');
  await mkdir(path.dirname(args.json), { recursive: true });
  await writeFile(args.json, `${JSON.stringify(baseline, null, 2)}\n`, 'utf8');
  for (const line of formatBaseline(baseline)) process.stdout.write(`${line}\n`);
  process.stdout.write(`[bench] Detalle: ${path.relative(projectRoot, args.json)}\n`);
}

await main();
