import path from 'node:path';
import { runProcess } from '../runner.mjs';
import { writeStageLog } from './common.mjs';

/**
 * Custom quality checks — standalone scripts from the audit plan.
 * These run independently of Sentinel CLI (which doesn't support them yet).
 * [Auditoría v4] [Plan mejora quality tool]
 */
export async function runCustom(context, scope) {
  const scriptsDir = path.join(context.projectRoot, 'scripts', 'quality');
  const frontendSrc = path.join(context.projectRoot, 'frontend', 'src');

  const checks = [
    { name: 'dom-abstraction', script: 'check-dom-abstraction.sh', args: [frontendSrc] },
    { name: 'singleton-state', script: 'check-singleton-state.sh', args: [frontendSrc] },
    { name: 'window-refs', script: 'check-window-refs.sh', args: [frontendSrc] },
    { name: 'sentinel-extended', script: 'check-sentinel-extended.sh', args: [frontendSrc] },
  ];

  const findings = [];
  let hasErrors = false;
  const startedAt = Date.now();

  for (const check of checks) {
    const scriptPath = path.join(scriptsDir, check.script);
    const execution = await runProcess('bash', [scriptPath, ...check.args], {
      cwd: context.projectRoot,
      timeoutMs: 30_000,
    });

    if (execution.code !== 0) {
      /* Parse output for warnings/info lines */
      const lines = (execution.stdout || '').split('\n').filter(l => l.trim());
      for (const line of lines) {
        if (line.includes('⚠️')) {
          findings.push({
            severity: 'warning',
            message: `[${check.name}] ${line.trim()}`,
            rule: check.name,
          });
        } else if (line.includes('✅')) {
          /* Pass — no finding */
        }
      }
      /* Script exited non-zero = has violations, but we treat them as warnings */
    }
  }

  const logPath = await writeStageLog(context, 'custom',
    findings.map(f => `[${f.severity}] ${f.message}`).join('\n') || 'All custom checks passed.');

  const errors = findings.filter(f => f.severity === 'error').length;
  const warnings = findings.filter(f => f.severity === 'warning').length;
  return {
    stage: 'custom',
    status: hasErrors ? 'fail' : 'pass',
    durationMs: Date.now() - startedAt,
    findings,
    summary: findings.length > 0
      ? `${errors} errores, ${warnings} warnings`
      : 'todas las verificaciones pasaron',
    logPath,
  };
}
