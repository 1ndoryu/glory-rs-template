#!/usr/bin/env node
/* [028A-6 Fase 3] Adaptador de proceso para la doble vía observe: ejecuta una
 * etapa del orquestador (sentinel/varsense/rust/frontend/docs/custom) por
 * nombre y escribe el reporte en el contrato estructurado que el core de
 * Sentinel consume (`--stages` → readToolReport → findings tipados). Cada
 * invocación reconstruye el contexto real del orquestador (preflight +
 * alcance) para que la etapa ejecute exactamente la misma lógica que hoy
 * corre dentro de task-check. Nunca importa el core: solo produce JSON. */
import path from 'node:path';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { stageDefinitions } from './stage-definitions.mjs';
import { preflight, projectRoot } from './preflight.mjs';
import { detectScope, manifestToScope } from './scope.mjs';
import { readFile } from 'node:fs/promises';


const SENTINEL_STAGE_JSON_SCHEMA_VERSION = '1';

function parseArgs(argv) {
  const parsed = { reportPath: null, taskId: null, stage: null, full: false, ci: false, profile: null, scopeManifest: null };
  for (let index = 0; index < argv.length; index++) {
    const arg = argv[index];
    if (arg === '--stage') parsed.stage = argv[++index] ?? null;
    else if (arg === '--report') parsed.reportPath = argv[++index] ?? null;
    else if (arg === '--task-id') parsed.taskId = argv[++index] ?? null;
    else if (arg === '--scope-manifest') parsed.scopeManifest = argv[++index] ?? null;
    else if (arg === '--full') parsed.full = true;
    else if (arg === '--ci') parsed.ci = true;
    else if (arg === '--profile') parsed.profile = argv[++index] ?? null;
  }
  return parsed;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.stage || !args.reportPath) {
    process.stderr.write('[stage-process] requiere --stage <nombre> --report <ruta>\n');
    process.exitCode = 2;
    return;
  }
  const reportRoot = path.dirname(path.resolve(args.reportPath));
  const scopeArgs = { full: args.full, ci: args.ci, profiles: args.profile ? args.profile.split(',').map(item => item.trim()).filter(Boolean) : [] };

  const context = await preflight({ taskId: args.taskId ?? 'observe', cwd: projectRoot, ...scopeArgs });
  context.reportRoot = reportRoot;
  context.logsRoot = path.join(reportRoot, 'logs');
  await mkdir(context.logsRoot, { recursive: true });
  let scope;
  /* [028A-6 Fase 3] Vía observe: misma decisión de alcance que task:check
   * (incluido el diferimiento heavy). El transporte --files-from necesita un
   * changed-files.txt propio en el reportRoot de la etapa, reconstruido
   * desde el manifiesto para no depender de rutas de otra corrida. */
  if (args.scopeManifest) {
    const manifest = JSON.parse(await readFile(args.scopeManifest, 'utf8'));
    scope = manifestToScope(manifest, reportRoot);
    await writeFile(scope.changedFilesPath, `${scope.files.join('\n')}\n`, 'utf8');
  } else {
    scope = await detectScope(context, scopeArgs);
  }
  const definition = stageDefinitions(context, scope, args.taskId).find(item => item.name === args.stage);
  if (!definition) {
    process.stderr.write(`[stage-process] etapa desconocida: ${args.stage}\n`);
    process.exitCode = 2;
    return;
  }
  const result = await definition.run();
  /* [028A-6] Contrato del core: {schemaVersion, entries: [{ruta, findings}]}.
   * Los hallazgos del orquestador ya están normalizados (ruleId/severity/
   * message); se agrupan en una entrada con ruta null porque la etapa decide
   * su propio alcance. El exit code replica task-check: fail=1, error=2. */
  /* [028A-6] Contrato exacto del core: entries[].findings[] con ruleId/
   * message/severity. Los hallazgos del orquestador ya vienen normalizados
   * (ruleId/severity/message); se envuelven en una entrada única con ruta
   * null porque la etapa decide su propio alcance. Severidades del core:
   * information/hint/info/critical/error/warning. */
  const report = {
    schemaVersion: SENTINEL_STAGE_JSON_SCHEMA_VERSION,
    stage: result.stage ?? args.stage,
    entries: [{ findings: (result.findings ?? []).map(item => ({
      ruleId: String(item.ruleId ?? 'unknown'),
      severity: String(item.severity ?? 'warning'),
      file: item.file ? String(item.file).replace(/\\/g, '/') : undefined,
      line: Number.isInteger(item.line) ? item.line : undefined,
      message: String(item.message ?? 'Hallazgo sin mensaje'),
      help: item.help ? String(item.help) : undefined,
    })) }],
    summary: result.summary ?? '',
    durationMs: result.durationMs ?? 0,
  };
  await mkdir(reportRoot, { recursive: true });
  await writeFile(path.resolve(args.reportPath), `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  process.exitCode = result.status === 'error' ? 2 : result.status === 'fail' ? 1 : 0;
}

await main();
