import { readFile, rename, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { redact, truncate } from '../redaction.mjs';

export function normalizeSeverity(value) {
  if (value === 'information' || value === 'hint') return 'info';
  return value === 'error' ? 'error' : 'warning';
}

export function npmInvocation(args) {
  if (!process.env.npm_execpath) throw new Error('npm_execpath ausente; ejecuta mediante npm run task:check');
  return { executable: process.execPath, args: [process.env.npm_execpath, ...args] };
}

export function conciseFailure(output, fallback) {
  const lines = redact(output).split(/\r?\n/).map(line => line.trim()).filter(Boolean);
  return lines.slice(-4).join(' | ') || fallback;
}

export function normalizeEntries(entries = []) {
  return entries.flatMap(entry => (entry.findings ?? []).map(finding => ({
    ruleId: String(finding.ruleId ?? 'unknown'),
    severity: normalizeSeverity(finding.severity),
    file: entry.ruta ? String(entry.ruta).replace(/\\/g, '/') : undefined,
    line: Number.isInteger(finding.range?.start?.line) ? finding.range.start.line + 1 : undefined,
    message: redact(finding.message ?? 'Hallazgo sin mensaje'),
    help: finding.suggestion ? redact(finding.suggestion) : undefined,
  })));
}

export async function readToolReport(reportPath) {
  try { return JSON.parse(await readFile(reportPath, 'utf8')); }
  catch (error) { throw new Error(`JSON inválido en ${reportPath}: ${error.message}`); }
}

export async function writeStageLog(context, stage, content) {
  const target = path.join(context.logsRoot, `${stage}.log`);
  const temporary = `${target}.tmp`;
  await writeFile(temporary, truncate(content), 'utf8');
  await rename(temporary, target);
  return target;
}

export function toolFailure(stage, execution, logPath) {
  const timedOut = execution.timedOut;
  return {
    stage,
    status: 'error',
    durationMs: execution.durationMs,
    findings: [{
      ruleId: timedOut ? 'quality-timeout' : 'quality-tool-error',
      severity: 'error',
      message: timedOut ? `${stage} excedió el timeout` : `${stage} terminó con código ${execution.code}`,
    }],
    summary: timedOut ? 'timeout' : `error ${execution.code}`,
    logPath,
  };
}

export function resultFromFindings(stage, findings, durationMs, logPath) {
  const errors = findings.filter(item => item.severity === 'error').length;
  const warnings = findings.filter(item => item.severity === 'warning').length;
  return {
    stage,
    status: errors > 0 ? 'fail' : 'pass',
    durationMs,
    findings,
    summary: `${errors} errores, ${warnings} warnings`,
    logPath,
  };
}
