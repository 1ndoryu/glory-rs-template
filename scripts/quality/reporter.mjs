import path from 'node:path';
import { writeAtomic } from './atomic-file.mjs';
import { sanitize } from './redaction.mjs';

function finalDecision(stages) {
  if (stages.some(stage => stage.status === 'error')) return { exitCode: 2, label: 'SETUP ERROR' };
  if (stages.some(stage => stage.status === 'fail')) return { exitCode: 1, label: 'FAIL' };
  return { exitCode: 0, label: 'PASS' };
}

function markdown(report) {
  const lines = [
    `# Quality report ${report.taskId}`,
    '',
    `- Estado: **${report.decision.label}**`,
    `- Alcance: ${report.scope.full ? 'full' : 'incremental'} (${report.scope.files.length} archivos)`,
    `- Duración: ${report.durationMs}ms`,
    '',
    '## Etapas',
    '',
    ...report.stages.map(stage => `- **${stage.stage}:** ${stage.status}${stage.cached ? ' (cache)' : ''} — ${stage.summary}`),
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
  const findings = stages.flatMap(stage => stage.findings).sort((a, b) =>
    (a.severity === 'error' ? 0 : 1) - (b.severity === 'error' ? 0 : 1)
  );
  const report = sanitize({
    schemaVersion: 1,
    taskId: args.taskId,
    generatedAt: new Date().toISOString(),
    durationMs: Date.now() - startedAt,
    mode: args.ci ? 'ci' : args.full ? 'full' : 'local-light',
    scope: { base: scope.base, full: scope.full, files: scope.files, profiles: [...scope.profiles] },
    tools: Object.fromEntries(Object.entries(context.tools).map(([name, tool]) => [name, {
      version: tool.version, commit: tool.commit, outputSchemaVersion: tool.outputSchemaVersion,
    }])),
    stages,
    findings,
    reminders,
    decision,
    nextCommand: decision.exitCode === 0 ? 'git status --short' : `npm run task:check -- ${args.taskId}`,
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
    `[quality] Scope: ${report.scope.full ? 'full' : 'incremental'} · ${report.scope.files.length} archivos`,
  ];
  for (const stage of report.stages) {
    lines.push(`[quality] ${stage.stage.padEnd(9)} ${stage.status.toUpperCase()}${stage.cached ? ' (cached)' : ''} · ${stage.summary}`);
  }
  for (const finding of report.findings.slice(0, context.qualityConfig.maxFindings)) {
    const location = finding.file ? `${finding.file}${finding.line ? `:${finding.line}` : ''} · ` : '';
    lines.push(`[quality] ${finding.severity.toUpperCase()} ${location}${finding.ruleId}: ${finding.message}`);
  }
  for (const reminder of report.reminders) lines.push(`[quality] REMEMBER ${reminder}`);
  lines.push(`[quality] Report: ${path.relative(context.projectRoot, reportResult.markdownPath)}`);
  lines.push(`[quality] Next: ${report.nextCommand}`);
  return lines;
}

export function printCompact(reportResult, context) {
  for (const line of compactLines(reportResult, context)) console.log(line);
}
