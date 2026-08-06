/* [028A-6 Fase 5] LEGACY — adaptador de migración del guard. El runtime
 * global de Sentinel (sentinel install, %LOCALAPPDATA%\GlorySentinel) ya
 * decide política y comandos directos (sentinel guard); este módulo del repo
 * se conserva SOLO para ramas antiguas y se retirará tras dos releases con
 * rollback probado (plan 028A-6 Fase 5). No modificar su lógica sin razón: la
 * fuente canónica es el runtime. */
import path from 'node:path';
import { existsSync, lstatSync, readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { validatePolicy } from './policy.mjs';
import { decisionForGuard } from './policy-decision.mjs';
import { BLOCKED_CARGO_COMMANDS, BLOCKED_NPM_SCRIPTS, BLOCKED_TOOLS } from './policy-defaults.mjs';

export const QUALITY_GUARD_EXIT_CODE = 78;

/* [028A-5] Direct validation commands must enter the task gate so agents
 * cannot bypass incremental scope, cooldowns or the compact quality report.
 * Gotcha: the root check is mandatory because these shims are global. */
const BLOCKED_NPM_SCRIPT_SET = new Set(BLOCKED_NPM_SCRIPTS);
const BLOCKED_TOOL_SET = new Set(BLOCKED_TOOLS);
const BLOCKED_CARGO_COMMAND_SET = new Set(BLOCKED_CARGO_COMMANDS);

/* [SNT-10/028A-16] Entrypoints directos de herramientas validadas: `node
 * node_modules/vitest/vitest.mjs run` elude el shim/función de `vitest`
 * invocando el runtime directamente. El guard intercepta `node` SOLO cuando el
 * primer argumento no-flag es el entrypoint de una herramienta bloqueada
 * (misma allowlist que BLOCKED_TOOLS); cualquier otro script, eval, REPL o
 * flag de node pasa intacto. Basename en minúsculas para cubrir rutas
 * relativas, absolutas y con barras mezcladas. */
const NODE_TOOL_ENTRYPOINTS = Object.freeze({
  'vitest.mjs': 'vitest', 'vitest.js': 'vitest', vitest: 'vitest',
  'tsc.js': 'tsc', tsc: 'tsc',
  'eslint.js': 'eslint', 'eslint.cjs': 'eslint',
  'prettier.cjs': 'prettier', 'prettier.js': 'prettier',
});

/* Flags de node que CONSUMEN su valor como argumento (código/módulo/condición):
 * el siguiente argumento no es un script y no puede clasificarse como
 * entrypoint de herramienta. Los flags sin valor (--version, --help, --watch…)
 * simplemente se ignoran al recorrer los argumentos. */
const NODE_VALUE_FLAGS = new Set([
  '-e', '--eval', '-p', '--print', '-r', '--require', '--import',
  '--loader', '-C', '--conditions', '--inspect', '--inspect-brk',
  '--experimental-loader', '--env-file',
]);

function nodeToolFromArgs(args) {
  const values = args.map(String);
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (value.startsWith('-')) {
      if (NODE_VALUE_FLAGS.has(value)) index += 1;
      continue;
    }
    /* [SNT-10] Barras normalizadas antes del basename: en win32 path.basename
     * ya colapsa backslashes, pero en un host POSIX no — el patrón es el mismo
     * que normalize() de scope.mjs. */
    const tool = NODE_TOOL_ENTRYPOINTS[path.basename(value.replace(/\\/g, '/')).toLowerCase()];
    return tool ?? null;
  }
  return null;
}

/* [SNT-10/028A-16] `node --run <script>` (task runner de Node) ejecuta los
 * scripts de package.json sin pasar por el shim de npm — un bypass de la misma
 * clase que el entrypoint directo (en cmd no existe shim de vitest, así que el
 * spawn interno no se intercepta). Devuelve el nombre del script si `--run` /
 * `--run-script` aparece ANTES de un script path; se bloquea cuando coincide
 * con npmScripts (test/test:full/type-check/build…). */
function nodeRunScript(args) {
  const values = args.map(String);
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (value === '--run' || value === '--run-script') {
      const script = values[index + 1];
      return script && !script.startsWith('-') ? script : null;
    }
    if (value.startsWith('--run')) {
      const equals = value.indexOf('=');
      if (equals >= 0) return value.slice(equals + 1) || null;
    }
    if (value.startsWith('-')) {
      if (NODE_VALUE_FLAGS.has(value)) index += 1;
      continue;
    }
    return null; /* Un script path llegó antes de --run: no es task runner. */
  }
  return null;
}

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

function readV2GuardPolicy(root) {
  const policyPath = path.join(root, 'sentinel.config.json');
  let metadata;
  try {
    metadata = lstatSync(policyPath);
  } catch (error) {
    if (error?.code === 'ENOENT') return { status: 'no-policy' };
    return { status: 'invalid-policy' };
  }
  if (metadata.isSymbolicLink() || !metadata.isFile()) return { status: 'invalid-policy' };
  let raw;
  try { raw = JSON.parse(readFileSync(policyPath, 'utf8')); }
  catch { return { status: 'invalid-policy' }; }
  if (raw?.schemaVersion !== 2) return { status: 'legacy-v1' };
  try { validatePolicy(raw); }
  catch { return { status: 'invalid-policy' }; }
  const directCommands = raw.guard.directCommands;
  const mode = raw.mode;
  if (!['enforce', 'observe', 'pass-through'].includes(mode) || !directCommands || typeof directCommands !== 'object') {
    return { status: 'invalid-policy' };
  }
  const lists = ['npmScripts', 'npxTools', 'cargoSubcommands', 'tools'];
  if (!lists.every(key => Array.isArray(directCommands[key]) && directCommands[key].every(value => typeof value === 'string'))) {
    return { status: 'invalid-policy' };
  }
  return {
    status: 'policy',
    mode,
    npmScripts: new Set(directCommands.npmScripts),
    npxTools: new Set(directCommands.npxTools.map(normalizeExecutable)),
    cargoSubcommands: new Set(directCommands.cargoSubcommands.map(value => value.toLowerCase())),
    tools: new Set(directCommands.tools.map(normalizeExecutable)),
  };
}

function matchesPolicyName(value, patterns) {
  if (patterns.has(value)) return true;
  return [...patterns].some(pattern => pattern.includes('*') && new RegExp(`^${pattern.split('*').map(part => part.replace(/[.*+?^${}()|[\\]\\\\]/g, '\\\\$&')).join('.*')}$`, 'u').test(value));
}

function npmScript(args = [], allowedScripts = BLOCKED_NPM_SCRIPT_SET) {
  const values = args.map(String);
  const runIndex = values.findIndex(value => value === 'run' || value === 'run-script');
  if (runIndex >= 0) return values[runIndex + 1] ?? null;
  const direct = values.find(value => matchesPolicyName(value, allowedScripts));
  return direct ?? null;
}

function npxTool(args = []) {
  const values = args.map(String);
  const index = values.findIndex(value => !value.startsWith('-'));
  return index >= 0 ? normalizeExecutable(values[index]) : null;
}

export function inspectDirectCommand({ executable, args = [], cwd = process.cwd(), projectRoot } = {}) {
  /* [297A-58] Las etapas internas del gate (fmt/type-check/tests) son la vía
   * sancionada de validación: el guard solo debe interceptar invocaciones
   * directas del agente. El gate establece un token aleatorio por ejecución
   * (GLORY_QUALITY_GATE_TOKEN) que se hereda únicamente por su árbol de
   * procesos; fuera de él, el token no existe y el bloqueo sigue vigente. */
  if (process.env.GLORY_QUALITY_GATE_TOKEN) return { blocked: false, root: null };
  const root = findQualityRoot(projectRoot ? path.resolve(projectRoot) : cwd);
  if (!root) return { blocked: false, root: null };

  const command = normalizeExecutable(executable);
  const values = args.map(String);
  const policy = readV2GuardPolicy(root);
  const legacyFallback = policy.status === 'legacy-v1';
  const npmScripts = policy.status === 'policy' || legacyFallback ? (policy.npmScripts ?? BLOCKED_NPM_SCRIPT_SET) : BLOCKED_NPM_SCRIPT_SET;
  const npxTools = policy.status === 'policy' || legacyFallback ? (policy.npxTools ?? BLOCKED_TOOL_SET) : BLOCKED_TOOL_SET;
  const cargoCommands = policy.status === 'policy' || legacyFallback ? (policy.cargoSubcommands ?? BLOCKED_CARGO_COMMAND_SET) : BLOCKED_CARGO_COMMAND_SET;
  const tools = policy.status === 'policy' || legacyFallback ? (policy.tools ?? BLOCKED_TOOL_SET) : BLOCKED_TOOL_SET;
  let reason = null;
  let category = null;

  if (command === 'npm') {
    const script = npmScript(values, npmScripts);
    if (script && matchesPolicyName(script, npmScripts)) {
      reason = `npm ${script}`;
      category = 'script';
    } else {
      const execIndex = values.findIndex(value => value === 'exec');
      const tool = execIndex >= 0 ? npxTool(values.slice(execIndex + 1)) : null;
      if (tool && matchesPolicyName(tool, npxTools)) {
        reason = `npm exec ${tool}`;
        category = 'tool';
      }
    }
  } else if (command === 'npx' || command === 'npm exec') {
    const tool = npxTool(values);
    if (tool && matchesPolicyName(tool, npxTools)) {
      reason = `${command} ${tool}`;
      category = 'tool';
    }
  } else if (command === 'node') {
    /* [SNT-10/028A-16] Bypass por runtime: node node_modules/vitest/vitest.mjs
     * no pasa por el shim de vitest. Se bloquea solo si el script directo es
     * el entrypoint de una herramienta de la allowlist, o si --run invoca un
     * script de validación del guard. */
    const runScript = nodeRunScript(values);
    if (runScript && matchesPolicyName(runScript, npmScripts)) {
      reason = `node --run ${runScript}`;
      category = 'script';
    } else {
      const tool = nodeToolFromArgs(values);
      if (tool && matchesPolicyName(tool, tools)) {
        reason = `node ${tool} (entrypoint directo)`;
        category = 'tool';
      }
    }
  } else if (matchesPolicyName(command, tools)) {
    reason = command;
    category = 'tool';
  } else if (command === 'cargo') {
    const cargoCommand = firstNonOption(values)?.toLowerCase();
    if (cargoCommand && matchesPolicyName(cargoCommand, cargoCommands)) {
      reason = `cargo ${cargoCommand}`;
      category = 'cargo';
    }
  }

  const discovered = policy.status === 'policy'
    ? { status: 'policy', policy: { mode: policy.mode } }
    : { status: policy.status };
  const baseDecision = decisionForGuard(discovered, reason);
  const decision = policy.status === 'invalid-policy' && reason
    ? { ...baseDecision, blocked: true, observed: false, reason }
    : baseDecision;
  if (!reason || !decision.blocked && !decision.observed) {
    return { ...decision, blocked: false, root, policyStatus: policy.status };
  }
  return {
    ...decision,
    category,
    command: reason,
    root,
    exitCode: decision.blocked ? QUALITY_GUARD_EXIT_CODE : undefined,
    policyStatus: policy.status,
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
