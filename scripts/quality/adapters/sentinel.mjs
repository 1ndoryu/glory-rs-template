import path from 'node:path';
import { normalizeEntries, resultFromFindings } from './common.mjs';
import { runStructuredTool } from './structured-tool.mjs';

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
  /* [028A-6] `full` describe el fingerprint; `executionFull` describe si
   * Sentinel puede analizar todo el workspace. Un perfil explícito mantiene
   * fingerprint full sin ampliar accidentalmente el análisis. */
  if (!(scope.executionFull ?? scope.full) || scope.profileOverride) args.push('--files-from', scope.changedFilesPath);

  const result = await runStructuredTool(context, {
    name: 'sentinel', executable: process.execPath, args, reportPath,
    timeoutMs: context.qualityConfig.timeoutsMs.sentinel,
    expectedSchemaVersion: context.tools.sentinel.outputSchemaVersion,
  });
  if (result.failure) return result.failure;
  return resultFromFindings('sentinel', normalizeEntries(result.report.entries), result.execution.durationMs, result.logPath);
}
