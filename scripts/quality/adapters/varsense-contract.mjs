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
  const manifestPath = scope.changedFilesPath ?? null;
  const supportsFilesFrom = context.tools.varsense.capabilities?.filesFrom === true;
  const canApplyScopedAnalysis = requestedScopedAnalysis && supportsFilesFrom && typeof manifestPath === 'string' && manifestPath.length > 0;
  if (canApplyScopedAnalysis) args.push('--files-from', manifestPath);
  return {
    args,
    reportPath,
    scope: {
      requestedScopedAnalysis,
      applied: !requestedScopedAnalysis || canApplyScopedAnalysis,
      manifestPath,
      limitation: requestedScopedAnalysis && !canApplyScopedAnalysis
        ? (supportsFilesFrom ? `varsense-cli-${version}-missing-manifest` : `varsense-cli-${version}-no-files-from`)
        : null,
    },
  };
}

export const VARSENSE_SCOPE_LIMITATION_CODE = version => `varsense-cli-${version}-no-files-from`;
export const VARSENSE_SCOPE_MISSING_MANIFEST_CODE = version => `varsense-cli-${version}-missing-manifest`;
