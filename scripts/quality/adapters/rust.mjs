import path from 'node:path';
import { runProcess } from '../runner.mjs';
import { conciseFailure, npmInvocation, writeStageLog } from './common.mjs';

async function runStep(context, name, executable, args) {
  const execution = await runProcess(executable, args, {
    cwd: context.projectRoot,
    timeoutMs: context.qualityConfig.timeoutsMs.rust,
  });
  return { name, execution };
}

export async function runRust(context) {
  const startedAt = Date.now();
  const steps = [];
  const npm = npmInvocation(['run', 'fmt:check']);
  steps.push(await runStep(context, 'cargo fmt --check', npm.executable, npm.args));

  const runWithDb = path.join(context.projectRoot, 'scripts', 'run-with-db.mjs');
  const check = await runStep(context, 'cargo check', process.execPath, [runWithDb, 'check']);
  steps.push(check);
  if (check.execution.code === 0) {
    steps.push(await runStep(context, 'cargo clippy', process.execPath, [runWithDb, 'clippy', '--', '-D', 'warnings']));
    steps.push(await runStep(context, 'cargo test', process.execPath, [runWithDb, 'test']));
  }

  const log = steps.map(step => `## ${step.name}\n${step.execution.stdout}\n${step.execution.stderr}`).join('\n');
  const logPath = await writeStageLog(context, 'rust', log);
  const failed = steps.filter(step => step.execution.code !== 0 || step.execution.timedOut);
  const infrastructure = failed.some(step => step.execution.code === 2 || step.execution.timedOut || step.execution.signal);
  return {
    stage: 'rust',
    status: infrastructure ? 'error' : failed.length > 0 ? 'fail' : 'pass',
    durationMs: Date.now() - startedAt,
    findings: failed.map(step => ({
      ruleId: step.execution.timedOut ? 'quality-timeout' : `rust-${step.name.split(' ')[1] ?? 'command'}`,
      severity: 'error',
      message: conciseFailure(`${step.execution.stdout}\n${step.execution.stderr}`, `${step.name} falló`),
    })),
    summary: failed.length > 0 ? `${failed.length} comandos fallaron` : `${steps.length} comandos pasaron`,
    logPath,
  };
}
