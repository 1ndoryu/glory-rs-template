import { parseArgs } from './args.mjs';
import crypto from 'node:crypto';
import { fingerprint, readCachedPass, writeCachedPass } from './cache.mjs';
import { acquireHeavyRun, formatHeavyGuardMessage, inspectHeavyRun } from './heavy-run-guard.mjs';
import { preflight, projectRoot } from './preflight.mjs';
import { createReport, printCompact } from './reporter.mjs';
import { selectReminders } from './reminders.mjs';
import { cancelAll } from './runner.mjs';
import { acquireTaskLock } from './lock.mjs';
import { detectScope } from './scope.mjs';
import { stageDefinitions } from './stage-definitions.mjs';
import { isFullExecution } from './profile-contract.mjs';
import { runBoundedStages } from './stage-runner.mjs';
import { runReportRetentionBestEffort } from './report-retention-stage.mjs';

let interrupted = false;
function handleInterruption(signal) {
  interrupted = true;
  cancelAll();
  process.stderr.write(`[quality] CANCELLED (${signal}) — finalizando etapas y liberando el lock.\n`);
}
process.once('SIGINT', () => handleInterruption('SIGINT'));
process.once('SIGTERM', () => handleInterruption('SIGTERM'));

async function executeStage(context, scope, definition, options) {
  const stageFingerprint = await fingerprint(context, scope, definition.name);
  if (!options.fresh && !options.ci) {
    const cached = await readCachedPass(context, definition.name, stageFingerprint);
    if (cached) return { ...cached, cache: 'hit' };
  }
  const result = await definition.run();
  await writeCachedPass(context, definition.name, stageFingerprint, result);
  return { ...result, cache: 'miss' };
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
  /* [297A-58] Marca el árbol de procesos del gate como validación sancionada:
   * el guard de comandos directos la propaga a las etapas (fmt/type-check)
   * y no las bloquea. Fuera del gate el token no existe. */
  process.env.GLORY_QUALITY_GATE_TOKEN ||= crypto.randomUUID();

  try {
    /* [028A-6] Un perfil explícito se resuelve contra quality.config.json
     * después de preflight; no se puede decidir el heavy path con seguridad
     * antes de conocer la allowlist del proyecto. */
    const explicitProfileRequested = Array.isArray(args.profiles) && args.profiles.length > 0
      || typeof process.env.GLORY_QUALITY_PROFILE === 'string'
      && process.env.GLORY_QUALITY_PROFILE.split(',').some(profile => profile.trim().length > 0);
    if (args.full && !args.ci && !explicitProfileRequested) {
      const heavyDecision = await inspectHeavyRun({
        projectRoot,
        mode: 'full',
        allowHeavy: args.allowHeavy,
      });
      if (!heavyDecision.allowed) {
        args.full = false;
        args.heavyDeferred = heavyDecision;
        process.stderr.write(`[quality] FULL diferido: ${formatHeavyGuardMessage(heavyDecision)}\n`);
        process.stderr.write('[quality] Se ejecutará el modo local-light para no bloquear el equipo.\n');
      }
    }
    const context = await preflight(args);
    /* [018A-4] Un agente no debe acumular procesos esperando el mismo gate.
     * La espera larga queda disponible para consumidores de la librería, pero
     * el comando público falla rápido y deja una acción clara al agente. */
    const releaseTaskLock = await acquireTaskLock(context, args.taskId, context.qualityConfig.lockWaitMs ?? 0, {
      isCancelled: () => interrupted,
    });
    try {
      let scope = await detectScope(context, args);
      let heavyLease = null;
      const previousHeavyToken = process.env.GLORY_HEAVY_RUN_TOKEN;
      /* [028A-6] Un perfil explícito puede conservar `scope.full` para el
       * fingerprint sin solicitar el heavy lease: solo se adquiere cuando
       * realmente se ejecutarán todas las etapas. */
      const runsAllStages = isFullExecution(scope);
      if ((args.full || args.ci) && runsAllStages) {
        heavyLease = await acquireHeavyRun({
          projectRoot: context.projectRoot,
          mode: args.ci ? 'ci' : 'full',
          taskId: args.taskId,
          command: `task:check ${args.taskId} ${args.ci ? '--ci' : '--full'}`,
          allowHeavy: args.allowHeavy,
        });
        if (!heavyLease.allowed) {
          args.full = false;
          args.heavyDeferred = heavyLease;
          context.full = false;
          context.heavyDeferred = heavyLease;
          scope = await detectScope(context, args);
          process.stderr.write(`[quality] FULL diferido: ${formatHeavyGuardMessage(heavyLease)}\n`);
          process.stderr.write('[quality] Se ejecutará el modo local-light para no bloquear el equipo.\n');
        }
      }
      if (heavyLease?.allowed) process.env.GLORY_HEAVY_RUN_TOKEN = heavyLease.token;
      const definitions = stageDefinitions(context, scope, args.taskId);
      let finalStatus = 'error';
      try {
        const stages = await runBoundedStages(
          definitions,
          definition => executeStage(context, scope, definition, args),
          { maxConcurrency: context.qualityConfig.maxConcurrentStages ?? 1, isCancelled: () => interrupted },
        );
        const reminders = selectReminders(scope, stages, context.qualityConfig.maxReminders, context);
        /* La poda nunca cambia el resultado del gate: registra su estado para
         * el reporte y continúa aunque el filesystem esté ocupado. */
        context.reportRetention = await runReportRetentionBestEffort({
          projectRoot: context.projectRoot,
          currentBranchKey: context.branch.branchKey,
          currentTaskId: args.taskId,
          config: context.qualityConfig.reportRetention,
        });
        const report = await createReport(context, args, scope, stages, reminders, startedAt);
        printCompact(report, context);
        finalStatus = interrupted ? 'cancelled' : report.report.decision.label;
        process.exitCode = interrupted ? 130 : report.report.decision.exitCode;
      } finally {
        if (previousHeavyToken === undefined) delete process.env.GLORY_HEAVY_RUN_TOKEN;
        else process.env.GLORY_HEAVY_RUN_TOKEN = previousHeavyToken;
        if (heavyLease?.allowed) await heavyLease.release({ status: finalStatus });
      }
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
