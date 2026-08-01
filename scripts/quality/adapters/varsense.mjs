import path from 'node:path';
import { normalizeEntries, resultFromFindings } from './common.mjs';
import { runStructuredTool } from './structured-tool.mjs';

async function runCommand(context, command) {
  const reportPath = path.join(context.reportRoot, `varsense-${command}.json`);
  const args = [
    context.tools.varsense.cliPath,
    command,
    '--workspace', context.projectRoot,
    '--config', path.join(context.projectRoot, 'varsense.config.json'),
    '--format', 'json',
    '--output', reportPath,
  ];
  return runStructuredTool(context, {
    name: 'varsense', executable: process.execPath, args, reportPath,
    timeoutMs: context.qualityConfig.timeoutsMs.varsense,
    expectedSchemaVersion: context.tools.varsense.outputSchemaVersion,
  });
}

export async function runVarsense(context) {
  const startedAt = Date.now();
  const current = await runCommand(context, 'all');
  if (current.failure) return current.failure;
  return resultFromFindings('varsense', normalizeEntries(current.report.entries), Date.now() - startedAt, current.logPath);
}
