import path from 'node:path';
import { runProcess } from '../runner.mjs';
import { normalizeEntries, readToolReport, resultFromFindings, toolFailure, writeStageLog } from './common.mjs';

export async function runSentinel(context, scope) {
  const reportPath = path.join(context.reportRoot, 'sentinel.json');
  const args = [
    context.tools.sentinel.cliPath,
    'analyze',
    '--workspace', context.projectRoot,
    '--config', path.join(context.projectRoot, 'sentinel.config.json'),
    '--format', 'json',
    '--output', reportPath,
  ];
  if (!scope.full) args.push('--files-from', scope.changedFilesPath);

  const execution = await runProcess(process.execPath, args, {
    cwd: context.projectRoot,
    timeoutMs: context.qualityConfig.timeoutsMs.sentinel,
  });
  const logPath = await writeStageLog(context, 'sentinel', `${execution.stdout}\n${execution.stderr}`);
  if (execution.code === 2 || execution.timedOut) return toolFailure('sentinel', execution, logPath);

  try {
    const report = await readToolReport(reportPath);
    if (String(report.schemaVersion) !== context.tools.sentinel.outputSchemaVersion) {
      throw new Error(`schema ${report.schemaVersion} incompatible`);
    }
    return resultFromFindings('sentinel', normalizeEntries(report.entries), execution.durationMs, logPath);
  } catch (error) {
    return toolFailure('sentinel', { ...execution, code: 2, stderr: error.message }, logPath);
  }
}
