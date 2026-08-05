import { parseArgs } from './args.mjs';
import crypto from 'node:crypto';
import { fingerprint, probeCachedPass, readCachedPass, writeCachedPass } from './cache.mjs';
import { acquireHeavyRun, formatHeavyGuardMessage, inspectHeavyRun, logHeavyOverride, manualOverrideSource } from './heavy-run-guard.mjs';
import { preflight, projectRoot } from './preflight.mjs';
import { createReport, printCompact } from './reporter.mjs';
import { selectReminders } from './reminders.mjs';
import { cancelAll } from './runner.mjs';
import { acquireTaskLock } from './lock.mjs';
import { detectScope, loadInjectedScope } from './scope.mjs';
import { stageDefinitions } from './stage-definitions.mjs';
import { isFullExecution } from './profile-contract.mjs';
import { runBoundedStages } from './stage-runner.mjs';
import { runReportRetentionBestEffort } from './report-retention-stage.mjs';
import { runTargetMaintenanceBestEffort } from './target-maintenance-stage.mjs';
import { runIndexMaintenanceBestEffort } from './index-maintenance.mjs';
import { defaultAgent, isStale, readTakeover, takeoverReminders } from './task-takeover.mjs';

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
  let missReason = options.fresh ? 'fresh' : options.ci ? 'ci' : null;
  if (!options.fresh && !options.ci) {
    const replayStartedAt = Date.now();
    const cached = await readCachedPass(context, definition.name, stageFingerprint);
    if (cached) {
      /* [028A-8 Fase 0] Un cache hit NO debe reproducir el durationMs original
       * (el de la corrida que creó la caché): el reporte y el benchmark
       * mostrarían una etapa incremental lenta cuando en realidad fue un
       * replay instantáneo. Se mide el tiempo real del replay y se conserva
       * la marca cache:'hit' para distinguir análisis de reutilización. */
      return { ...cached, cache: 'hit', cacheReason: 'match', durationMs: Date.now() - replayStartedAt };
    }
    /* [028A-8 Fase 4] La razón de invalidación (no-entry, fingerprint-mismatch,
     * not-pass) se captura ANTES de ejecutar: writeCachedPass solo escribe en
     * PASS y con el fingerprint exacto, así que un probe posterior devolvería
     * siempre 'match' y la razón real se perdería. */
    missReason = (await probeCachedPass(context, definition.name, stageFingerprint)).reason;
  }
  const result = await definition.run();
  await writeCachedPass(context, definition.name, stageFingerprint, result);
  return { ...result, cache: 'miss', cacheReason: missReason };
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
        heavyReason: args.heavyReason,
      });
      /* [028A-16][SNT-11] Todo intento manual de excepción denegado por el
       * guard queda auditado — motivo ausente o cooldown con el mecanismo
       * desactivado: el flag ya llegó al gate y debe quedar trazado aunque no
       * conceda la excepción. `manualOverrideSource` no nulo implica que el
       * flag/env llegó; sin fallback a 'env'. */
      if (!heavyDecision.allowed) {
        const manualSource = manualOverrideSource({ allowHeavy: args.allowHeavy });
        if (manualSource) {
          await logHeavyOverride({
            projectRoot,
            source: manualSource,
            command: `task:check ${args.taskId}`,
            reason: typeof args.heavyReason === 'string' ? args.heavyReason : null,
            granted: false,
            taskId: args.taskId,
          });
        }
      }
      if (!heavyDecision.allowed) {
        args.full = false;
        args.heavyDeferred = heavyDecision;
        process.stderr.write(`[quality] FULL diferido: ${formatHeavyGuardMessage(heavyDecision)}\n`);
        process.stderr.write('[quality] Se ejecutará el modo local-light para no bloquear el equipo.\n');
      }
    }
    const context = await preflight(args);
    /* [028A-17] Coordinación de tomas de tarea: el gate solo INFORMA. Si la
     * tarea está tomada por otro agente activo, se avisa fuerte al inicio
     * (el agente decide si es una validación legítima o un conflicto real).
     * Al final, los reminders recuerdan liberar si la tomó este agente,
     * re-tomarla si expiró, o marcarla si nadie la tomó. Un error de lectura
     * del registro (permisos, archivo bloqueado en Windows) degrada a “sin
     * información” y NUNCA convierte el gate en SETUP ERROR. */
    try {
      context.taskTakeover = await readTakeover(context.projectRoot, args.taskId);
    } catch {
      context.taskTakeover = null;
    }
    if (context.taskTakeover && context.taskTakeover.takenBy !== defaultAgent() && !isStale(context.taskTakeover)) {
      process.stderr.write(`[quality] TAREA TOMADA por ${context.taskTakeover.takenBy} (${context.taskTakeover.id}) desde ${context.taskTakeover.takenAt} — expira ${context.taskTakeover.expiresAt}. Si no es tuya, coordina antes de cerrarla (npm run task:status); si es tuya, ejecuta el gate con GLORY_AGENT_ID=<tu-agente>.\n`);
    }
    /* [018A-4] Un agente no debe acumular procesos esperando el mismo gate.
     * La espera larga queda disponible para consumidores de la librería, pero
     * el comando público falla rápido y deja una acción clara al agente. */
    const releaseTaskLock = await acquireTaskLock(context, args.taskId, context.qualityConfig.lockWaitMs ?? 0, {
      isCancelled: () => interrupted,
    });
    try {
      /* [028A-8 Fase 0] Un scope-manifest inyectado (fixtures del benchmark)
       * sustituye la detección git: alcance determinista sin mutar el árbol
       * compartido. Los fixtures son local-light (effectiveFull=false), así
       * que el bloqueo del guard pesado nunca se activa en este camino. */
      let scope = args.scopeManifest ? await loadInjectedScope(context, args) : await detectScope(context, args);
      let heavyLease = null;
      const previousHeavyToken = process.env.GLORY_HEAVY_RUN_TOKEN;
      /* [028A-8] El lease se solicita cuando el alcance efectivo ejecutará las
       * etapas completas, incluido un automaticFull (cambio de migraciones,
       * config o scripts/quality), no solo cuando el usuario escribió --full.
       * Un perfil explícito conserva `scope.full` para el fingerprint sin
       * solicitar el lease: `isFullExecution` ya lo filtra. Si el guard
       * bloquea, se re-detecta el alcance con heavyDeferred y effectiveFull
       * queda realmente en false (local-light), nunca simulado. */
      const runsAllStages = isFullExecution(scope);
      if (scope.effectiveFull && runsAllStages && !args.heavyDeferred) {
        heavyLease = await acquireHeavyRun({
          projectRoot: context.projectRoot,
          mode: args.ci ? 'ci' : 'full',
          taskId: args.taskId,
          command: `task:check ${args.taskId} ${args.ci ? '--ci' : '--full'}`,
          allowHeavy: args.allowHeavy,
          heavyReason: args.heavyReason,
        });
        if (!heavyLease.allowed) {
          args.full = false;
          args.heavyDeferred = heavyLease;
          context.full = false;
          context.heavyDeferred = heavyLease;
          /* [028A-8 Fase 0] Un scope inyectado (--scope-manifest) se re-carga
           * en vez de caer a detección git: loadInjectedScope replica el
           * diferimiento del guard (effectiveFull=false) sin descartar la
           * decisión del manifiesto. */
          scope = args.scopeManifest ? await loadInjectedScope(context, args) : await detectScope(context, args);
          process.stderr.write(`[quality] FULL diferido: ${formatHeavyGuardMessage(heavyLease)}\n`);
          process.stderr.write('[quality] Se ejecutará el modo local-light para no bloquear el equipo.\n');
        }
      }
      if (heavyLease?.allowed) process.env.GLORY_HEAVY_RUN_TOKEN = heavyLease.token;
      /* [028A-16] El reporte expone el override usado (concedido o denegado
       * por falta de motivo) para que `latest.md` conserve la trazabilidad de
       * la excepción junto con la fecha y el comando. */
      context.heavyOverride = heavyLease?.allowed
        ? { source: heavyLease.source ?? 'flag', granted: true, reason: args.heavyReason ?? heavyLease.reason ?? null }
        : args.allowHeavy
          ? { source: 'flag', granted: false, reason: null }
          : null;
      /* [028A-6] Propaga la cancelación al contrato de adapters para que un
       * proceso terminado por SIGINT conserve el estado `cancelled` y no se
       * confunda con un error genérico de herramienta. */
      context.isCancelled = () => interrupted;
      const definitions = stageDefinitions(context, scope, args.taskId);
      let finalStatus = 'error';
      try {
        const stages = await runBoundedStages(
          definitions,
          definition => executeStage(context, scope, definition, args),
          { maxConcurrency: context.qualityConfig.maxConcurrentStages ?? 1, isCancelled: () => interrupted },
        );
        /* [028A-17] Recordatorios de toma de tarea ANTEPUESTOS: compactLines
         * recorta a maxReminders (4 por defecto), así que si se añadieran al
         * final, el recordatorio de liberar se perdería en la salida de
         * terminal justo cuando más importa. Al anteponerlos siempre quedan
         * visibles; el JSON/Markdown conservan la lista completa. */
        const reminders = [
          ...takeoverReminders({ taskId: args.taskId, entry: context.taskTakeover, agent: defaultAgent() }),
          ...selectReminders(scope, stages, context.qualityConfig.maxReminders, context),
        ].filter((reminder, index, all) => all.indexOf(reminder) === index);
        /* La poda nunca cambia el resultado del gate: registra su estado para
         * el reporte y continúa aunque el filesystem esté ocupado. */
        context.reportRetention = await runReportRetentionBestEffort({
          projectRoot: context.projectRoot,
          currentBranchKey: context.branch.branchKey,
          currentTaskId: args.taskId,
          config: context.qualityConfig.reportRetention,
        });
        /* [028A-6] Supervisión automática de targets de cargo: con throttle
         * (una vez por ventana) y presupuesto de tiempo, nunca bloquea el
         * gate. Elimina targets viejos/sobre cuota sin tocar procesos vivos
         * (marcadores del guard + ejecutables en uso). */
        /* [028A-6] targetRoot por defecto: C:\tmp\glory-target (o
         * CARGO_TARGET_DIR_BASE). La política de cuota/edad viene de
         * quality.config.json heavyRun; no hay clave targetRoot en la config. */
        context.targetMaintenance = await runTargetMaintenanceBestEffort({
          projectRoot: context.projectRoot,
        });
        /* [028A-8 Fase 4] Supervisión de índices de analizadores (varsense):
         * TTL y cuota separados de los targets de cargo, con throttle por
         * ventana y presupuesto de tiempo; nunca borra una rama con lock
         * activo ni un índice reescrito en la última media hora. */
        context.indexMaintenance = await runIndexMaintenanceBestEffort({
          projectRoot: context.projectRoot,
          currentBranchKey: context.branch.branchKey,
          config: context.qualityConfig.indexRetention,
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
