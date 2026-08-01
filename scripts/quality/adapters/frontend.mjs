import { runProcess } from '../runner.mjs';
import { conciseFailure, npmInvocation, writeStageLog } from './common.mjs';

async function runStep(context, name, script) {
  const npm = npmInvocation(['--prefix', 'frontend', 'run', script]);
  const execution = await runProcess(npm.executable, npm.args, {
    cwd: context.projectRoot,
    timeoutMs: name === 'test-full'
      ? context.qualityConfig.timeoutsMs.frontendTest
      : context.qualityConfig.timeoutsMs.frontend,
  });
  return { name, execution };
}

export async function runFrontend(context) {
  const steps = [await runStep(context, 'type-check', 'type-check')];
  /* [018A-51] La suite completa queda reservada a CI; local sigue usando
   * type-check + selección incremental para no levantar procesos innecesarios. */
  if (context.ci && steps[0].execution.code === 0) {
    steps.push(await runStep(context, 'test-full', 'test:full'));
  }
  const log = steps.map(step => `## ${step.name}\n${step.execution.stdout}\n${step.execution.stderr}`).join('\n');
  const logPath = await writeStageLog(context, 'frontend', log);
  const failed = steps.filter(step => step.execution.code !== 0 || step.execution.timedOut);
  return {
    stage: 'frontend',
    status: failed.some(step => step.execution.timedOut || step.execution.code === 2 || step.execution.signal)
      ? 'error'
      : failed.length > 0 ? 'fail' : 'pass',
    durationMs: steps.reduce((total, step) => total + step.execution.durationMs, 0),
    findings: failed.map(step => ({
      ruleId: step.execution.timedOut ? 'quality-timeout' : `frontend-${step.name}`,
      severity: 'error',
      message: conciseFailure(`${step.execution.stdout}\n${step.execution.stderr}`, `${step.name} falló`),
    })),
    summary: failed.length > 0 ? `${failed.length} validaciones frontend fallaron` : context.ci ? 'type-check + suite completa pasaron' : 'type-check pasó',
    logPath,
  };
}
