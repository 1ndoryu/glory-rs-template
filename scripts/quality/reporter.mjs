import path from 'node:path';
import { writeAtomic } from './atomic-file.mjs';
import { sanitize } from './redaction.mjs';

function finalDecision(stages) {
  if (stages.some(stage => stage.state === 'cancelled')) return { exitCode: 130, label: 'CANCELLED' };
  if (stages.some(stage => stage.status === 'error')) return { exitCode: 2, label: 'SETUP ERROR' };
  if (stages.some(stage => stage.status === 'fail')) return { exitCode: 1, label: 'FAIL' };
  return { exitCode: 0, label: 'PASS' };
}

/* [038A-1] Duración por etapa legible: ms por debajo de 1s, segundos con 1 decimal en adelante. */
function formatDuration(ms) {
  if (!Number.isFinite(ms) || ms < 0) return '—';
  if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.round(ms)}ms`;
}

/* [028A-8] Alcance honesto: requested/automatic/effective quedan separados en
 * el JSON; el texto compacto muestra el motivo cuando el fingerprint no
 * coincide con la ejecución efectiva (p. ej. full diferido por el guard). */
function formatScope(scope) {
  const fingerprint = scope.full ? 'full' : 'incremental';
  const execution = (scope.effectiveFull ?? scope.executionFull ?? scope.full) ? 'full' : 'incremental';
  const base = fingerprint === execution ? fingerprint : `${fingerprint} · ejecución ${execution}`;
  const reason = scope.fullReason && scope.fullReason !== 'incremental' ? ` (${scope.fullReason})` : '';
  return `${base}${reason}`;
}

const SEVERITY_ORDER = new Map([['error', 0], ['warning', 1], ['info', 2]]);

function compareText(left, right) {
  return left < right ? -1 : left > right ? 1 : 0;
}

/* Orden estable para comparar artifacts entre ejecuciones. No usa
 * localeCompare: el locale del agente/CI no debe cambiar el JSON publicado. */
function compareFindings(left, right) {
  const severity = (SEVERITY_ORDER.get(left.severity) ?? 99) - (SEVERITY_ORDER.get(right.severity) ?? 99);
  if (severity !== 0) return severity;
  for (const [leftValue, rightValue] of [
    [left.ruleId ?? '', right.ruleId ?? ''],
    [left.file ?? '', right.file ?? ''],
  ]) {
    const result = compareText(String(leftValue), String(rightValue));
    if (result !== 0) return result;
  }
  const line = Number(left.line ?? 0) - Number(right.line ?? 0);
  if (line !== 0) return line;
  return compareText(String(left.message ?? ''), String(right.message ?? ''));
}

function markdown(report) {
  const lines = [
    `# Quality report ${report.taskId}`,
    '',
    `- Estado: **${report.decision.label}**`,
    `- Alcance: ${formatScope(report.scope)} (${report.scope.files.length} archivos)`,
    `- Duración: ${report.durationMs}ms (${formatDuration(report.durationMs)})`,
    `- Política: ${report.policy.policyHash} · ${report.policy.decision?.action ?? 'unknown'} · ${report.policy.reason}`,
    ...(report.reportRetention?.status === 'error' ? [`- Retención: **error no bloqueante** — ${report.reportRetention.message}`] : []),
    ...(report.reportRetention?.overQuota ? [`- Retención: **overQuota** — ${report.reportRetention.currentBranchBytes} bytes en la rama activa`] : []),
    ...(report.heavyGuard ? [`- Full diferido: **${report.heavyGuard.reason}** — ${report.heavyGuard.nextAllowedAt ?? report.heavyGuard.message ?? 'reintento bloqueado'}`] : []),
    '',
    '## Etapas',
    '',
    ...report.stages.map(stage => `- **${stage.stage}:** ${stage.status}${stage.cached ? ' (cache)' : ''} — ${formatDuration(stage.durationMs)} — ${stage.summary}`),
  ];
  if (report.findings.length > 0) {
    lines.push('', '## Hallazgos', '');
    for (const item of report.findings) lines.push(`- [${item.severity}] ${item.ruleId}: ${item.message}`);
  }
  lines.push('', '## Recordatorios', '', ...report.reminders.map(item => `- ${item}`), '');
  return lines.join('\n');
}

export async function createReport(context, args, scope, stages, reminders, startedAt) {
  const decision = finalDecision(stages);
  const findings = stages.flatMap(stage => stage.findings).sort(compareFindings);
  const deferred = context.heavyDeferred ?? null;
  const report = sanitize({
    schemaVersion: 1,
    taskId: args.taskId,
    generatedAt: new Date().toISOString(),
    durationMs: Date.now() - startedAt,
    mode: args.ci ? 'ci' : (scope.effectiveFull ?? scope.executionFull ?? scope.full) ? 'full' : 'local-light',
    heavyGuard: deferred,
    branch: context.branch ?? null,
    reportRetention: context.reportRetention ?? null,
    policy: context.policyIdentity ?? {
      projectRoot: context.projectRoot,
      policyPath: null,
      policyHash: 'unavailable',
      runtimeVersion: null,
      decision: { status: 'invalid-policy', mode: 'observe', action: 'error', blocked: false, reason: 'identidad de política no disponible' },
      reason: 'identidad de política no disponible',
      recommendedCommand: `npm run task:check -- ${args.taskId}`,
    },
    scope: {
      base: scope.base,
      full: scope.full,
      requestedFull: scope.requestedFull ?? scope.full,
      automaticFull: scope.automaticFull ?? false,
      effectiveFull: scope.effectiveFull ?? scope.executionFull ?? scope.full,
      fullReason: scope.fullReason ?? null,
      heavyDeferred: scope.heavyDeferred ?? false,
      executionFull: scope.executionFull ?? scope.full,
      files: scope.files,
      deletedFiles: scope.deletedFiles ?? [],
      profiles: [...scope.profiles],
    },
    tools: Object.fromEntries(Object.entries(context.tools).map(([name, tool]) => [name, {
      version: tool.version, commit: tool.commit, outputSchemaVersion: tool.outputSchemaVersion,
    }])),
    stages,
    findings,
    reminders,
    decision,
    nextCommand: deferred
      ? `npm run task:check -- ${args.taskId} --full --allow-heavy`
      : decision.exitCode === 0 ? 'git status --short' : `npm run task:check -- ${args.taskId}`,
  });
  const jsonPath = path.join(context.reportRoot, 'latest.json');
  const markdownPath = path.join(context.reportRoot, 'latest.md');
  await writeAtomic(jsonPath, `${JSON.stringify(report, null, 2)}\n`);
  await writeAtomic(markdownPath, markdown(report));
  return { report, jsonPath, markdownPath };
}

export function compactLines(reportResult, context) {
  const { report } = reportResult;
  const lines = [
    `[quality] ${report.taskId} — ${report.decision.label}`,
    `[quality] Scope: ${formatScope(report.scope)} · ${report.scope.files.length} archivos`,
  ];
  for (const stage of report.stages) {
    lines.push(`[quality] ${stage.stage.padEnd(9)} ${stage.status.toUpperCase()}${stage.cached ? ' (cached)' : ''} · ${formatDuration(stage.durationMs)} · ${stage.summary}`);
  }
  for (const finding of report.findings.slice(0, context.qualityConfig.maxFindings)) {
    const location = finding.file ? `${finding.file}${finding.line ? `:${finding.line}` : ''} · ` : '';
    lines.push(`[quality] ${finding.severity.toUpperCase()} ${location}${finding.ruleId}: ${finding.message}`);
  }
  /* [028A-6] Límite defensivo también aquí: el contrato compacto publica como
   * máximo maxFindings hallazgos y maxReminders recordatorios (3/4 por
   * defecto), aunque el origen pase listas más largas. El reporte JSON/Markdown
   * completo conserva el detalle total. */
  for (const reminder of report.reminders.slice(0, context.qualityConfig.maxReminders)) {
    lines.push(`[quality] REMEMBER ${reminder}`);
  }
  lines.push(`[quality] Report: ${path.relative(context.projectRoot, reportResult.markdownPath)}`);
  lines.push(`[quality] Next: ${report.nextCommand}`);
  return lines;
}

export function printCompact(reportResult, context) {
  for (const line of compactLines(reportResult, context)) console.log(line);
}
