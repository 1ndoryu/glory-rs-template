import { access, readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { writeStageLog } from './common.mjs';

async function exists(target) {
  try { await access(target); return true; } catch { return false; }
}

export async function runDocs(context, taskId) {
  const startedAt = Date.now();
  const findings = [];
  const roadmapPath = path.join(context.projectRoot, 'roadmap.md');
  const roadmap = await readFile(roadmapPath, 'utf8');

  if (!roadmap.includes(taskId)) {
    findings.push({ ruleId: 'docs-task-missing', severity: 'error', file: 'roadmap.md', message: `${taskId} no aparece en roadmap.md` });
  }
  if (/##\s+(?:tareas\s+)?completad/i.test(roadmap)) {
    findings.push({ ruleId: 'docs-roadmap-completed', severity: 'error', file: 'roadmap.md', message: 'roadmap.md no debe acumular tareas completadas' });
  }

  const plansRoot = path.join(context.projectRoot, 'Agente', 'planes');
  for (const entry of await readdir(plansRoot, { withFileTypes: true })) {
    if (!entry.isFile() || !entry.name.endsWith('.md')) continue;
    const content = await readFile(path.join(plansRoot, entry.name), 'utf8');
    if (!/- \[[ x]\]/i.test(content)) {
      findings.push({ ruleId: 'docs-plan-no-checklist', severity: 'error', file: `Agente/planes/${entry.name}`, message: 'Plan activo sin checklist' });
    }
  }

  const canonical = [...roadmap.matchAll(/`((?:Agente\/)[^`]+\.md)`/g)].map(match => match[1]);
  for (const relativePath of canonical) {
    if (!await exists(path.join(context.projectRoot, relativePath))) {
      findings.push({ ruleId: 'docs-link-missing', severity: 'error', file: 'roadmap.md', message: `Referencia inexistente: ${relativePath}` });
    }
  }

  const logPath = await writeStageLog(context, 'docs', findings.map(item => `${item.ruleId}: ${item.message}`).join('\n'));
  return {
    stage: 'docs',
    status: findings.length > 0 ? 'fail' : 'pass',
    durationMs: Date.now() - startedAt,
    findings,
    summary: findings.length > 0 ? `${findings.length} problemas` : 'documentación coherente',
    logPath,
  };
}
