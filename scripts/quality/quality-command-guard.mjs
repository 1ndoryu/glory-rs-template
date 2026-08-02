import path from 'node:path';
import { existsSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

export const QUALITY_GUARD_EXIT_CODE = 78;

/* [028A-5] Direct validation commands must enter the task gate so agents
 * cannot bypass incremental scope, cooldowns or the compact quality report.
 * Gotcha: the root check is mandatory because these shims are global. */
const BLOCKED_NPM_SCRIPTS = new Set([
  'test',
  'test:changed',
  'test:full',
  'test:file',
  'test:watch',
  'type-check',
  'lint',
  'check',
  'check:back',
  'check:front',
  'fmt',
  'fmt:check',
  'build',
]);

/* Direct rustfmt is the same validation path as cargo fmt and must not be
 * able to bypass the task gate from a shell that does not expose cargo. */
const BLOCKED_TOOLS = new Set(['vitest', 'tsc', 'eslint', 'prettier', 'rustfmt']);
const BLOCKED_CARGO_COMMANDS = new Set(['check', 'clippy', 'test', 'bench', 'fmt']);

function normalizeExecutable(value = '') {
  return path.basename(String(value)).toLowerCase().replace(/\.(cmd|exe)$/u, '');
}

function firstNonOption(args = []) {
  return args.find(value => !String(value).startsWith('-'));
}

function findQualityRoot(startPath = process.cwd()) {
  let candidate = path.resolve(startPath);
  while (candidate) {
    if (
      existsSync(path.join(candidate, 'quality.config.json'))
      && existsSync(path.join(candidate, 'scripts', 'quality', 'heavy-run-guard.mjs'))
    ) return candidate;
    const parent = path.dirname(candidate);
    if (parent === candidate) break;
    candidate = parent;
  }
  return null;
}

function npmScript(args = []) {
  const values = args.map(String);
  const runIndex = values.findIndex(value => value === 'run' || value === 'run-script');
  if (runIndex >= 0) return values[runIndex + 1] ?? null;
  const direct = values.find(value => BLOCKED_NPM_SCRIPTS.has(value));
  return direct ?? null;
}

function npxTool(args = []) {
  const values = args.map(String);
  const index = values.findIndex(value => !value.startsWith('-'));
  return index >= 0 ? normalizeExecutable(values[index]) : null;
}

export function inspectDirectCommand({ executable, args = [], cwd = process.cwd(), projectRoot } = {}) {
  const root = findQualityRoot(projectRoot ? path.resolve(projectRoot) : cwd);
  if (!root) return { blocked: false, root: null };

  const command = normalizeExecutable(executable);
  const values = args.map(String);
  let reason = null;
  let category = null;

  if (command === 'npm') {
    const script = npmScript(values);
    if (script && BLOCKED_NPM_SCRIPTS.has(script)) {
      reason = `npm ${script}`;
      category = 'script';
    } else {
      const execIndex = values.findIndex(value => value === 'exec');
      const tool = execIndex >= 0 ? npxTool(values.slice(execIndex + 1)) : null;
      if (tool && BLOCKED_TOOLS.has(tool)) {
        reason = `npm exec ${tool}`;
        category = 'tool';
      }
    }
  } else if (command === 'npx' || command === 'npm exec') {
    const tool = npxTool(values);
    if (tool && BLOCKED_TOOLS.has(tool)) {
      reason = `${command} ${tool}`;
      category = 'tool';
    }
  } else if (BLOCKED_TOOLS.has(command)) {
    reason = command;
    category = 'tool';
  } else if (command === 'cargo') {
    const cargoCommand = firstNonOption(values)?.toLowerCase();
    if (cargoCommand && BLOCKED_CARGO_COMMANDS.has(cargoCommand)) {
      reason = `cargo ${cargoCommand}`;
      category = 'cargo';
    }
  }

  if (!reason) return { blocked: false, root };
  return {
    blocked: true,
    category,
    command: reason,
    root,
    exitCode: QUALITY_GUARD_EXIT_CODE,
  };
}

export function formatBlockMessage(decision) {
  return [
    '[glory-quality] BLOQUEADO: esta validación directa no está permitida.',
    `  Comando detectado: ${decision.command}`,
    '  Ejecuta el gate del proyecto para usar alcance incremental y límites:',
    '  npm run task:check -- <TareaId>',
    '  El gate decide type-check/tests/build según la tarea y el modo CI.',
  ].join('\n');
}

function cliArguments(argv) {
  const executableIndex = argv.indexOf('--executable');
  const projectRootIndex = argv.indexOf('--project-root');
  const separator = argv.indexOf('--');
  return {
    executable: executableIndex >= 0 ? argv[executableIndex + 1] : '',
    projectRoot: projectRootIndex >= 0 ? argv[projectRootIndex + 1] : undefined,
    args: separator >= 0 ? argv.slice(separator + 1) : [],
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const decision = inspectDirectCommand(cliArguments(process.argv.slice(2)));
  if (decision.blocked) {
    process.stderr.write(`${formatBlockMessage(decision)}\n`);
    process.exitCode = decision.exitCode;
  }
}
