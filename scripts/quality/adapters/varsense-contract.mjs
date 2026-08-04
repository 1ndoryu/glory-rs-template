import path from 'node:path';

export function buildVarsenseInvocation(context, scope = {}) {
  const reportPath = path.join(context.reportRoot, 'varsense-all.json');
  const args = [
    context.tools.varsense.cliPath,
    'all',
    '--workspace', context.projectRoot,
    '--config', path.join(context.projectRoot, 'varsense.config.json'),
    '--format', 'json',
    '--output', reportPath,
  ];
  const executionFull = scope.executionFull ?? scope.full ?? true;
  const requestedScopedAnalysis = !executionFull;
  const version = context.tools.varsense.version ?? 'unknown';
  return {
    args,
    reportPath,
    scope: {
      requestedScopedAnalysis,
      applied: !requestedScopedAnalysis,
      manifestPath: scope.changedFilesPath ?? null,
      limitation: requestedScopedAnalysis ? `varsense-cli-${version}-no-files-from` : null,
    },
  };
}

export const VARSENSE_SCOPE_LIMITATION_CODE = version => `varsense-cli-${version}-no-files-from`;
