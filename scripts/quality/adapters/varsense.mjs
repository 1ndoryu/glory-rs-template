import path from 'node:path';
import { runProcess } from '../runner.mjs';
import { normalizeEntries, readToolReport, resultFromFindings, toolFailure, writeStageLog } from './common.mjs';

async function runCommand(context, command) {
  const reportPath = path.join(context.reportRoot, `varsense-${command}.json`);
  const execution = await runProcess(process.execPath, [
    context.tools.varsense.cliPath,
    command,
    '--workspace', context.projectRoot,
    '--config', path.join(context.projectRoot, 'varsense.config.json'),
    '--format', 'json',
    '--output', reportPath,
  ], { cwd: context.projectRoot, timeoutMs: context.qualityConfig.timeoutsMs.varsense });
  return { execution, reportPath };
}

export async function runVarsense(context) {
  const startedAt = Date.now();
  const executions = [];
  const findings = [];

  for (const command of ['scan', 'orphan-classes']) {
    const current = await runCommand(context, command);
    executions.push(current.execution);
    if (current.execution.code === 2 || current.execution.timedOut) {
      const logPath = await writeStageLog(context, 'varsense', executions.map(item => `${item.stdout}\n${item.stderr}`).join('\n'));
      return toolFailure('varsense', current.execution, logPath);
    }
    try {
      const report = await readToolReport(current.reportPath);
      if (String(report.schemaVersion) !== context.tools.varsense.outputSchemaVersion) {
        throw new Error(`schema ${report.schemaVersion} incompatible`);
      }
      findings.push(...normalizeEntries(report.entries));
    } catch (error) {
      const logPath = await writeStageLog(context, 'varsense', error.message);
      return toolFailure('varsense', { ...current.execution, code: 2 }, logPath);
    }
  }

  const logPath = await writeStageLog(context, 'varsense', executions.map(item => `${item.stdout}\n${item.stderr}`).join('\n'));
  return resultFromFindings('varsense', findings, Date.now() - startedAt, logPath);
}
