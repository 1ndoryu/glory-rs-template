#!/usr/bin/env node
/* [028A-8 Fase 4] Diagnóstico `quality:profile`: lee los últimos reportes del
 * gate (latest.json por tarea en la rama actual) y calcula p50/p95 de duración
 * por etapa y del total, sin ejecutar ninguna validación pesada. Es el alias
 * temporal de `sentinel profile <TareaId>` mientras no exista el runtime
 * global; la decisión de gate nunca pasa por aquí. */
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { branchReportRoot, resolveBranchIdentity } from './branch-identity.mjs';

export const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

/* [028A-8] Percentil nearest-rank: p50/p95 de una lista de duraciones ya
 * ordenada ascendentemente. En muestras pequeñas el p95 tiende al máximo,
 * que es exactamente lo que los presupuestos de calidad quieren vigilar. */
export function percentile(sortedAsc, ratio) {
  if (sortedAsc.length === 0) return null;
  const index = Math.min(sortedAsc.length - 1, Math.max(0, Math.ceil(ratio * sortedAsc.length) - 1));
  return sortedAsc[index];
}

export function summarize(values) {
  const sorted = [...values].filter(Number.isFinite).sort((a, b) => a - b);
  if (sorted.length === 0) return { samples: 0, p50: null, p95: null, min: null, max: null, mean: null };
  const mean = sorted.reduce((sum, value) => sum + value, 0) / sorted.length;
  return {
    samples: sorted.length,
    p50: percentile(sorted, 0.5),
    p95: percentile(sorted, 0.95),
    min: sorted[0],
    max: sorted[sorted.length - 1],
    mean: Math.round(mean * 10) / 10,
  };
}

function parseArgs(argv) {
  const parsed = { taskId: null, limit: 20, json: null, budgets: null };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--task-id') parsed.taskId = argv[++index] ?? null;
    else if (arg === '--limit') parsed.limit = Number(argv[++index]) || 20;
    else if (arg === '--json') parsed.json = argv[++index] ?? null;
    else if (arg === '--budgets') parsed.budgets = argv[++index] ?? null;
  }
  return parsed;
}

/* [028A-8 Fase 0] Presupuesto de tiempo por etapa que falla SOLO ante
 * regresión confirmada: exige muestras suficientes (>= minSamples) y que el
 * p95 supere el presupuesto. Una variación aislada de la máquina con pocas
 * ejecuciones nunca declara regresión. No es parte del gate: es diagnóstico. */
export function evaluateStageBudgets(profile, budgets, minSamples = 5) {
  if (!budgets || typeof budgets !== 'object') return [];
  const violations = [];
  for (const [stage, budgetMs] of Object.entries(budgets)) {
    if (!Number.isInteger(budgetMs) || budgetMs < 1) continue;
    const found = profile.stages.find(item => item.stage === stage);
    if (!found) continue;
    if (found.samples < minSamples) continue;
    if (found.p95 !== null && found.p95 > budgetMs) {
      violations.push({ stage, budgetMs, p95: found.p95, samples: found.samples });
    }
  }
  return violations;
}

/* [028A-8] Colecta latest.json de cada tarea bajo el branch; opcionalmente se
 * filtra por task ID y se limita a los reportes más recientes por generatedAt. */
export async function collectReports(branchRoot, taskId, limit) {
  const entries = [];
  const taskDirs = taskId ? [taskId] : (await readdir(branchRoot, { withFileTypes: true }))
    .filter(entry => entry.isDirectory())
    .map(entry => entry.name);
  for (const dir of taskDirs) {
    try {
      const report = JSON.parse(await readFile(path.join(branchRoot, dir, 'latest.json'), 'utf8'));
      if (!Array.isArray(report.stages) || !Number.isFinite(report.durationMs)) continue;
      entries.push({ taskId: dir, generatedAt: report.generatedAt ?? '', report });
    } catch { /* Reporte ausente o inválido: se omite sin bloquear. */ }
  }
  entries.sort((a, b) => (a.generatedAt < b.generatedAt ? 1 : a.generatedAt > b.generatedAt ? -1 : 0));
  return entries.slice(0, limit);
}

export function buildProfile(entries) {
  const totals = entries.map(entry => entry.report.durationMs);
  const byStage = new Map();
  for (const { taskId, report } of entries) {
    for (const stage of report.stages ?? []) {
      if (!Number.isFinite(stage.durationMs)) continue;
      const key = String(stage.stage ?? 'unknown');
      if (!byStage.has(key)) byStage.set(key, []);
      /* [028A-8 Fase 4] Paréntesis explícitos: `??` liga más que `?:`, así que
       * `stage.cache ?? stage.cached ? ...` se evalúa como
       * `(stage.cache ?? stage.cached) ? ...` y un 'miss' string (truthy)
       * contaría como hit. El cache explícito gana; si falta, cae a `cached`. */
      const cache = stage.cache === 'hit' ? 'hit' : stage.cache === 'miss' ? 'miss' : (stage.cached ? 'hit' : 'miss');
      byStage.get(key).push({ taskId, durationMs: stage.durationMs, cache });
    }
  }
  const stages = [...byStage.entries()].map(([name, samples]) => ({
    stage: name,
    samples: samples.length,
    ...summarize(samples.map(item => item.durationMs)),
    cacheHits: samples.filter(item => item.cache === 'hit').length,
  })).sort((a, b) => a.stage.localeCompare(b.stage));
  return { generatedAt: new Date().toISOString(), reports: entries.length, total: summarize(totals), stages };
}

function renderCompact(profile) {
  const lines = [`[profile] Reportes: ${profile.reports} · Total p50 ${profile.total.p50}ms · p95 ${profile.total.p95}ms`];
  for (const stage of profile.stages) {
    lines.push(`[profile] ${stage.stage.padEnd(9)} p50 ${stage.p50}ms · p95 ${stage.p95}ms · hit ${stage.cacheHits}/${stage.samples}`);
  }
  return lines;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const identity = await resolveBranchIdentity(projectRoot);
  const branchRoot = branchReportRoot(projectRoot, identity);
  const entries = await collectReports(branchRoot, args.taskId, args.limit);
  if (entries.length === 0) {
    process.stderr.write(`[profile] Sin reportes en ${branchRoot}${args.taskId ? `/${args.taskId}` : ''}. Ejecuta primero el gate con una tarea.\n`);
    process.exitCode = 2;
    return;
  }
  const profile = buildProfile(entries);
  const outputPath = path.resolve(args.json ?? path.join(branchRoot, 'profile', 'latest.json'));
  await mkdir(path.dirname(outputPath), { recursive: true });
  await writeFile(outputPath, `${JSON.stringify(profile, null, 2)}\n`, 'utf8');
  for (const line of renderCompact(profile)) console.log(line);
  process.stdout.write(`[profile] Detalle: ${path.relative(projectRoot, outputPath)}\n`);
  /* [028A-8 Fase 0] Regresión confirmada: solo con muestras suficientes y p95
   * por encima del presupuesto. Exit 1 informa, no bloquea el gate. */
  if (args.budgets) {
    let budgets;
    try { budgets = JSON.parse(args.budgets); }
    catch { budgets = null; }
    const violations = evaluateStageBudgets(profile, budgets);
    for (const violation of violations) {
      process.stderr.write(`[profile] REGRESIÓN ${violation.stage}: p95 ${violation.p95}ms > presupuesto ${violation.budgetMs}ms (${violation.samples} muestras)\n`);
    }
    if (violations.length > 0) process.exitCode = 1;
  }
}

/* [028A-8] Guarda de entrada: importar las funciones puras desde un test no
 * debe escribir perfiles ni leer reportes (efecto lateral). */
const isEntryPoint = typeof process.argv[1] === 'string'
  && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isEntryPoint) await main();
