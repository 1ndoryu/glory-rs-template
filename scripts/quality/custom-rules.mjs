import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';

const SOURCE_EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.jsx']);
const EXCLUDED_NAMES = new Set(['dom.ts', 'sanitize-html.ts']);

async function collectFiles(root, files = []) {
  for (const entry of await readdir(root, { withFileTypes: true })) {
    if (['node_modules', 'api', 'generated', 'dist', 'out'].includes(entry.name)) continue;
    const target = path.join(root, entry.name);
    if (entry.isDirectory()) await collectFiles(target, files);
    else if (entry.isFile() && SOURCE_EXTENSIONS.has(path.extname(entry.name))
      && !entry.name.endsWith('.d.ts') && !entry.name.endsWith('.test.ts') && !entry.name.endsWith('.spec.ts')
      && !EXCLUDED_NAMES.has(entry.name)) files.push(target);
  }
  return files;
}

function lineNumber(source, offset) {
  return source.slice(0, offset).split('\n').length;
}

function addFinding(findings, ruleId, severity, file, source, offset, message, remediation) {
  findings.push({
    ruleId, severity, file: file.replace(/\\/g, '/'), line: lineNumber(source, offset),
    message, remediation,
  });
}

function scanSource(file, source, findings) {
  const checks = [
    ['dom-access-outside-platform', /document\.createElement\s*\(/g, 'Acceso DOM directo fuera del adaptador de plataforma.', 'Usa la abstracción createEl o document adapter.'],
    ['window-reference-outside-platform', /window\.(?:location|history|scrollTo|innerWidth|innerHeight|addEventListener)\b/g, 'Referencia window directa fuera del boundary de plataforma.', 'Usa el adapter de navegación, viewport o lifecycle.'],
    ['unsafe-any', /(?:\bas any\b|:\s*any\b|@ts-(?:ignore|expect-error))/g, 'Uso de any o suppressions de TypeScript.', 'Tipa el valor o documenta una excepción acotada.'],
    ['default-export', /^\s*export\s+default\b/gm, 'Default export en módulo de aplicación.', 'Prefiere named exports para facilitar composición y análisis.'],
    ['console-production', /\bconsole\.(?:log|error|warn|debug)\s*\(/g, 'Console directa en código de producción.', 'Usa el logger o feedback visible del proyecto.'],
    ['api-call-outside-service', /\bapi\.(?:get|post|put|patch|delete)\s*\(/g, 'API llamada fuera de un service boundary.', 'Mueve la llamada a services/adapters.'],
    ['catch-vacio', /catch\s*(?:\([^)]*\))?\s*\{\s*\}/g, 'Catch vacío: el error queda silenciado.', 'Propaga, registra o muestra feedback accionable.'],
    ['mixed-barrel-logic', /export\s*\{[^}]+\}\s*from[\s\S]*export\s+(?:function|const|class|async)\b/g, 'Módulo mezcla re-export y lógica ejecutable.', 'Separa barrel y módulo de implementación.'],
    ['unsafe-process-shell', /(?:shell\s*:\s*true|exec\s*\(\s*[^)]*\+|execSync\s*\(\s*[^)]*\+)/g, 'Proceso externo con shell o argumentos concatenados.', 'Usa argumentos separados y shell:false.'],
    ['hardcoded-secret-context', /\b(?:api[_-]?key|secret|password|token|authorization)\b\s*[:=]\s*[`'\"][^`'\"]{8,}[`'\"]/gi, 'Posible secreto hardcodeado en código o configuración.', 'Mueve el secreto a variables de entorno y redáctalo en logs.'],
    ['open-redirect', /(?:window\.)?location(?:\.href)?\s*=\s*(?!['"`])/g, 'Redirección basada en valor no constante.', 'Valida el destino contra una allowlist server-side.'],
    ['async-without-abort', /\bfetch\s*\(/g, 'Fetch async: verificar AbortSignal y teardown.', 'Pasa AbortSignal y cancela el trabajo obsoleto en teardown.'],
    ['subscription-without-dispose', /\.subscribe\s*\(/g, 'Suscripción detectada: requiere cleanup verificable.', 'Conserva unsubscribe/dispose en el lifecycle dueño.'],
    ['innerhtml-variable', /\.innerHTML\s*=\s*(?!['"`])/g, 'innerHTML recibe contenido dinámico.', 'Usa textContent, DOM seguro o sanitización explícita.'],
  ];
  for (const [ruleId, pattern, message, remediation] of checks) {
    for (const match of source.matchAll(pattern)) addFinding(findings, ruleId, ruleId === 'catch-vacio' ? 'error' : 'warning', file, source, match.index, message, remediation);
  }
  const lines = source.split('\n');
  if (lines.length > 300) addFinding(findings, 'file-size-budget', 'warning', file, source, 0, `Archivo excede 300 líneas (${lines.length}).`, 'Divide por responsabilidad antes de añadir más lógica.');
  for (const match of source.matchAll(/^\s*let\s+\w+/gm)) addFinding(findings, 'singleton-mutable-state', 'warning', file, source, match.index, 'Estado mutable declarado a nivel de módulo.', 'Mueve el estado a un store o ciclo de vida explícito.');
  for (const match of source.matchAll(/interface\s+\w+\s*\{([\s\S]*?)\}/g)) {
    const fields = (match[1].match(/^\s*[A-Za-z_$][\w$]*\??\s*:/gm) ?? []).length;
    if (fields > 10) addFinding(findings, 'large-interface-isp', 'info', file, source, match.index, `Interface con ${fields} campos.`, 'Divide en subinterfaces cohesivas.');
  }
}

export async function analyzeWorkspace(sourceRoot, selectedFiles = null) {
  const findings = [];
  const files = selectedFiles?.length ? selectedFiles : await collectFiles(sourceRoot);
  for (const file of files) scanSource(file, await readFile(file, 'utf8'), findings);
  return findings.sort((left, right) => left.file.localeCompare(right.file) || left.line - right.line || left.ruleId.localeCompare(right.ruleId));
}

export { collectFiles, scanSource };
