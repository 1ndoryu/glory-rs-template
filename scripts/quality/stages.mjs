#!/usr/bin/env node
/* [028A-6 Fase 3] Genera el JSON declarativo que `sentinel check <task>
 * --stages <json>` consume: convierte las etapas del orquestador (misma
 * selección por alcance que stageDefinitions) en procesos `stage-process.mjs`
 * con el contrato estructurado. Es la vía observe: el gate agnóstico ejecuta
 * exactamente la misma lógica que hoy corre dentro de task-check, pero con su
 * propio reporte/caché/exit code para comparar decisiones. */
import path from 'node:path';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { preflight, projectRoot } from './preflight.mjs';
import { detectScope, manifestToScope } from './scope.mjs';
import { isFullExecution } from './profile-contract.mjs';
import { PROFILE_STAGE_RULES } from './profile-contract.mjs';
import { readFile } from 'node:fs/promises';

const STAGE_ORDER = ['sentinel', 'varsense', 'rust', 'frontend', 'docs', 'custom'];
const STAGE_TIMEOUT_MS = {
  sentinel: 180_000,
  varsense: 300_000,
  rust: 30 * 60_000,
  frontend: 10 * 60_000,
  docs: 60_000,
  custom: 60_000,
};

function parseArgs(argv) {
  const parsed = { taskId: null, output: null, full: false, ci: false, profile: null, reportRoot: null, scopeManifest: null };
  for (let index = 0; index < argv.length; index++) {
    const arg = argv[index];
    if (arg === '--task-id') parsed.taskId = argv[++index] ?? null;
    else if (arg === '--output') parsed.output = argv[++index] ?? null;
    else if (arg === '--report-root') parsed.reportRoot = argv[++index] ?? null;
    else if (arg === '--scope-manifest') parsed.scopeManifest = argv[++index] ?? null;
    else if (arg === '--full') parsed.full = true;
    else if (arg === '--ci') parsed.ci = true;
    else if (arg === '--profile') parsed.profile = argv[++index] ?? null;
  }
  return parsed;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.taskId) {
    process.stderr.write('[stages] requiere --task-id <id>\n');
    process.exitCode = 2;
    return;
  }
  const scopeArgs = { full: args.full, ci: args.ci, profiles: args.profile ? args.profile.split(',').map(item => item.trim()).filter(Boolean) : [] };
  const context = await preflight({ taskId: args.taskId, cwd: projectRoot, ...scopeArgs });
  let scope;
  /* [028A-6 Fase 3] Vía observe: reutiliza el alcance que task:check ya
   * decidió (incluido el diferimiento del guard de ejecuciones pesadas) para
   * que ambos gates comparen el mismo conjunto de archivos y etapas. */
  if (args.scopeManifest) {
    const manifest = JSON.parse(await readFile(args.scopeManifest, 'utf8'));
    scope = manifestToScope(manifest);
  } else {
    scope = await detectScope(context, scopeArgs);
  }

  const stageNames = isFullExecution(scope)
    ? STAGE_ORDER
    : ['sentinel', ...new Set([...scope.profiles].flatMap(profile => PROFILE_STAGE_RULES[profile] ?? []))];
  const reportRoot = path.resolve(args.reportRoot ?? path.join(context.reportRoot, '..', 'check', 'stages'));
  const wrapper = path.join(projectRoot, 'scripts', 'quality', 'stage-process.mjs');
  const scopeArgsForStage = args.scopeManifest ? ['--scope-manifest', args.scopeManifest] : [];
  const declarations = stageNames.map(name => ({
    name,
    executable: process.execPath,
    args: [wrapper, '--stage', name, '--report', '{reportPath}', '--task-id', args.taskId, ...scopeArgsForStage],
    expectedSchemaVersion: '1',
    timeoutMs: STAGE_TIMEOUT_MS[name] ?? 120_000,
    reportPath: path.join(reportRoot, `${name}.json`),
  }));

  const outputPath = path.resolve(args.output ?? path.join(reportRoot, 'stages.json'));
  await mkdir(path.dirname(outputPath), { recursive: true });
  await writeFile(outputPath, `${JSON.stringify(declarations, null, 2)}\n`, 'utf8');
  process.stdout.write(`${outputPath}\n`);
}

await main();
