import { normalizeEntries, resultFromFindings } from './common.mjs';
import { runStructuredTool } from './structured-tool.mjs';
import { buildVarsenseInvocation } from './varsense-contract.mjs';

async function runCommand(context, scope) {
  const invocation = buildVarsenseInvocation(context, scope);
  const result = await runStructuredTool(context, {
    name: 'varsense', executable: process.execPath, args: invocation.args, reportPath: invocation.reportPath,
    timeoutMs: context.qualityConfig.timeoutsMs.varsense,
    expectedSchemaVersion: context.tools.varsense.outputSchemaVersion,
  });
  return { ...result, scope: invocation.scope };
}

export async function runVarsense(context, scope) {
  const startedAt = Date.now();
  const current = await runCommand(context, scope);
  if (current.failure) return { ...current.failure, metadata: { varsenseScope: current.scope } };
  return {
    ...resultFromFindings('varsense', normalizeEntries(current.report.entries), Date.now() - startedAt, current.logPath),
    metadata: { varsenseScope: current.scope },
  };
}
