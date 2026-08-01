import { parseArgs } from './args.mjs';
import { fingerprint, readCachedPass, writeCachedPass } from './cache.mjs';
import { preflight } from './preflight.mjs';
import { createReport, printCompact } from './reporter.mjs';
import { selectReminders } from './reminders.mjs';
import { cancelAll } from './runner.mjs';
import { acquireTaskLock } from './lock.mjs';
import { detectScope } from './scope.mjs';
import { runDocs } from './adapters/docs.mjs';
import { runFrontend } from './adapters/frontend.mjs';
import { runRust } from './adapters/rust.mjs';
import { runSentinel } from './adapters/sentinel.mjs';
import { runVarsense } from './adapters/varsense.mjs';
import { runCustom } from './adapters/custom.mjs';

let interrupted = false;
function handleInterruption(signal) {
  interrupted = true;
  cancelAll();
  process.stderr.write(`[quality] CANCELLED (${signal}) — finalizando etapas y liberando el lock.\n`);
}
process.once('SIGINT', () => handleInterruption('SIGINT'));
process.once('SIGTERM', () => handleInterruption('SIGTERM'));

function stageDefinitions(context, scope, taskId) {
  const definitions = [{ name: 'sentinel', run: () => runSentinel(context, scope) }];
  if (scope.full || scope.profiles.has('css') || scope.profiles.has('frontend')) {
    definitions.push({ name: 'varsense', run: () => runVarsense(context) });
  }
  if (scope.full || scope.profiles.has('rust')) definitions.push({ name: 'rust', run: () => runRust(context) });
  if (scope.full || scope.profiles.has('frontend')) definitions.push({ name: 'frontend', run: () => runFrontend(context) });
  if (scope.full || scope.profiles.has('docs')) definitions.push({ name: 'docs', run: () => runDocs(context, taskId) });
  /* [Auditoría v4] Custom checks: DOM abstraction, singleton state, window refs */
  if (scope.full || scope.profiles.has('frontend')) definitions.push({ name: 'custom', run: () => runCustom(context, scope) });
  return definitions;
}

async function executeStage(context, scope, definition, options) {
  const stageFingerprint = await fingerprint(context, scope, definition.name);
  if (!options.fresh && !options.ci) {
    const cached = await readCachedPass(context, definition.name, stageFingerprint);
    if (cached) return cached;
  }
  const result = await definition.run();
  await writeCachedPass(context, definition.name, stageFingerprint, result);
  return result;
}

async function main() {
  const startedAt = Date.now();
  let args;
  try { args = parseArgs(process.argv.slice(2)); }
  catch (error) {
    process.stderr.write(`[quality] SETUP ERROR — ${error.message}\n`);
    process.stderr.write('[quality] Next: npm run task:check -- 297A-N\n');
    process.exitCode = 2;
    return;
  }

  try {
    const context = await preflight(args);
    /* [018A-4] Un agente no debe acumular procesos esperando el mismo gate.
     * La espera larga queda disponible para consumidores de la librería, pero
     * el comando público falla rápido y deja una acción clara al agente. */
    const releaseTaskLock = await acquireTaskLock(context, args.taskId, context.qualityConfig.lockWaitMs ?? 0, {
      isCancelled: () => interrupted,
    });
    try {
      const scope = await detectScope(context, args);
      const stages = [];
      for (const definition of stageDefinitions(context, scope, args.taskId)) {
        if (interrupted) throw new Error('quality gate cancelado durante la ejecución');
        stages.push(await executeStage(context, scope, definition, args));
      }
      const reminders = selectReminders(scope, stages, context.qualityConfig.maxReminders);
      const report = await createReport(context, args, scope, stages, reminders, startedAt);
      printCompact(report, context);
      process.exitCode = interrupted ? 130 : report.report.decision.exitCode;
    } finally {
      await releaseTaskLock();
    }
  } catch (error) {
    if (interrupted) {
      process.stderr.write('[quality] CANCELLED — repite el comando cuando decidas continuar.\n');
      process.exitCode = 130;
      return;
    }
    process.stderr.write(`[quality] SETUP ERROR — ${error.message}\n`);
    process.stderr.write('[quality] Next: npm run quality:setup\n');
    process.exitCode = 2;
  }
}

await main();
