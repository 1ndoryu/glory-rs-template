import path from 'node:path';
import { analyzeWorkspace } from '../custom-rules.mjs';
import { writeStageLog, resultFromFindings } from './common.mjs';

/* Reglas que ya viven en el core de Sentinel. El scanner local se conserva
 * una fase para comparar fixtures, pero no duplica hallazgos en el reporte. */
const MIGRATED_TO_SENTINEL = new Set([
  'dom-access-outside-platform', 'window-reference-outside-platform',
  'unsafe-any', 'default-export', 'console-production',
  'api-call-outside-service', 'catch-vacio', 'unsafe-process-shell',
  'hardcoded-secret-context', 'open-redirect', 'innerhtml-variable',
  'singleton-mutable-state', 'large-interface-isp', 'mixed-barrel-logic',
  'file-size-budget',
]);

/* [018A-5] El bridge legacy deja de depender de Bash/grep y devuelve el mismo
 * contrato estructurado que los adapters de Sentinel/VarSense. Las reglas son
 * portables; sus exclusiones y severidades siguen siendo política del proyecto. */
export async function runCustom(context) {
  const startedAt = Date.now();
  try {
    const sourceRoot = path.join(context.projectRoot, 'frontend', 'src');
    const selected = context.scope?.full ? null : context.scope.files
      .filter(file => /^frontend\/src\/.*\.(?:ts|tsx|js|jsx)$/i.test(file))
      .map(file => path.join(context.projectRoot, file));
    const allFindings = await analyzeWorkspace(sourceRoot, selected);
    const findings = allFindings.filter(item => !MIGRATED_TO_SENTINEL.has(item.ruleId));
    const logPath = await writeStageLog(context, 'custom', JSON.stringify({
      schemaVersion: 1,
      findings,
      migratedToSentinel: allFindings.filter(item => MIGRATED_TO_SENTINEL.has(item.ruleId)),
    }, null, 2));
    return resultFromFindings('custom', findings, Date.now() - startedAt, logPath);
  } catch (error) {
    const logPath = await writeStageLog(context, 'custom', error.stack ?? error.message);
    return {
      stage: 'custom', status: 'error', durationMs: Date.now() - startedAt,
      findings: [{ ruleId: 'quality-tool-error', severity: 'error', message: `custom: ${error.message}` }],
      summary: 'error del analyzer custom', logPath,
    };
  }
}
