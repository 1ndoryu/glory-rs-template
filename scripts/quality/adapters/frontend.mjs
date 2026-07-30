import { runProcess } from '../runner.mjs';
import { conciseFailure, npmInvocation, writeStageLog } from './common.mjs';

export async function runFrontend(context) {
  const npm = npmInvocation(['--prefix', 'frontend', 'run', 'type-check']);
  const execution = await runProcess(npm.executable, npm.args, {
    cwd: context.projectRoot,
    timeoutMs: context.qualityConfig.timeoutsMs.frontend,
  });
  const logPath = await writeStageLog(context, 'frontend', `${execution.stdout}\n${execution.stderr}`);
  const failed = execution.code !== 0 || execution.timedOut;
  return {
    stage: 'frontend',
    status: execution.timedOut || execution.code === 2 || execution.signal ? 'error' : failed ? 'fail' : 'pass',
    durationMs: execution.durationMs,
    findings: failed ? [{
      ruleId: execution.timedOut ? 'quality-timeout' : 'frontend-type-check',
      severity: 'error',
      message: conciseFailure(`${execution.stdout}\n${execution.stderr}`, 'TypeScript falló'),
    }] : [],
    summary: failed ? 'type-check falló' : 'type-check pasó',
    logPath,
  };
}
