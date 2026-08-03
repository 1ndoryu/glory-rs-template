import { access, readFile } from 'node:fs/promises';
import path from 'node:path';
import { BLOCKED_CARGO_COMMANDS, BLOCKED_NPM_SCRIPTS, BLOCKED_TOOLS, DEFAULT_GATE_COMMAND } from './policy-defaults.mjs';

const POLICY_FILE = 'sentinel.config.json';
const MAX_STRING_LENGTH = 160;
const NAME_PATTERN = /^[A-Za-z0-9:_*.-]+$/u;
const MODES = new Set(['enforce', 'observe', 'pass-through']);
const ROOT_KEYS = new Set(['schemaVersion', 'mode', 'gate', 'guard', 'runtime', 'analyzers']);
const ANALYZER_KEYS = new Set(['enabled', 'profile', 'config']);
const RUNTIME_KEYS = new Set(['minimumVersion', 'protocolVersion', 'lockFile']);
const GATE_KEYS = new Set(['command', 'taskIdRequired']);
const GUARD_KEYS = new Set(['directCommands']);
const DIRECT_COMMAND_KEYS = new Set(['npmScripts', 'npxTools', 'cargoSubcommands', 'tools']);

function fail(message) {
  throw new Error(`sentinel.config.json: ${message}`);
}

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function validateKeys(value, allowed, label) {
  const unknown = Object.keys(value).filter(key => !allowed.has(key));
  if (unknown.length > 0) fail(`${label}: claves desconocidas: ${unknown.join(', ')}`);
}

function validateName(value, label) {
  if (typeof value !== 'string' || value.length === 0 || value.length > MAX_STRING_LENGTH || !NAME_PATTERN.test(value)) {
    fail(`${label}: nombre inválido`);
  }
}

function validateStringList(value, label) {
  if (!Array.isArray(value) || value.length > 128) fail(`${label}: debe ser una lista de nombres`);
  for (const item of value) validateName(item, label);
}

function validateRelativePath(value, label) {
  if (typeof value !== 'string' || value.length === 0 || value.length > MAX_STRING_LENGTH || path.isAbsolute(value)) {
    fail(`${label}: debe ser una ruta relativa`);
  }
  const normalized = value.replace(/\\/g, '/');
  if (normalized.split('/').includes('..')) fail(`${label}: no puede salir del workspace`);
}

function validateAnalyzer(value, label) {
  if (!isRecord(value)) fail(`${label}: debe ser un objeto`);
  validateKeys(value, ANALYZER_KEYS, label);
  if (typeof value.enabled !== 'boolean') fail(`${label}.enabled inválido`);
  if (value.profile !== undefined) validateName(value.profile, `${label}.profile`);
  if (value.config !== undefined) {
    if (typeof value.config === 'string') validateRelativePath(value.config, `${label}.config`);
    else if (!isRecord(value.config)) fail(`${label}.config inválido`);
  }
}

export function validatePolicy(policy) {
  if (!isRecord(policy)) fail('la raíz debe ser un objeto');
  validateKeys(policy, ROOT_KEYS, 'raíz');
  if (policy.schemaVersion !== 2) fail('schemaVersion debe ser 2');
  if (typeof policy.mode !== 'string' || !MODES.has(policy.mode)) fail('mode inválido');

  if (!isRecord(policy.gate)) fail('gate debe ser un objeto');
  validateKeys(policy.gate, GATE_KEYS, 'gate');
  if (!Array.isArray(policy.gate.command) || policy.gate.command.length < 2 || policy.gate.command.length > 8) {
    fail('gate.command inválido');
  }
  for (const item of policy.gate.command) validateName(item, 'gate.command');
  if (typeof policy.gate.taskIdRequired !== 'boolean') fail('gate.taskIdRequired inválido');

  if (!isRecord(policy.guard)) fail('guard debe ser un objeto');
  validateKeys(policy.guard, GUARD_KEYS, 'guard');
  if (!isRecord(policy.guard.directCommands)) fail('guard.directCommands debe ser un objeto');
  validateKeys(policy.guard.directCommands, DIRECT_COMMAND_KEYS, 'guard.directCommands');
  for (const key of DIRECT_COMMAND_KEYS) validateStringList(policy.guard.directCommands[key], `guard.directCommands.${key}`);

  if (!isRecord(policy.runtime)) fail('runtime debe ser un objeto');
  validateKeys(policy.runtime, RUNTIME_KEYS, 'runtime');
  validateName(policy.runtime.minimumVersion, 'runtime.minimumVersion');
  if (!Number.isInteger(policy.runtime.protocolVersion) || policy.runtime.protocolVersion < 1 || policy.runtime.protocolVersion > 100) {
    fail('runtime.protocolVersion inválido');
  }
  validateRelativePath(policy.runtime.lockFile, 'runtime.lockFile');

  if (!isRecord(policy.analyzers)) fail('analyzers debe ser un objeto');
  validateKeys(policy.analyzers, new Set(['sentinel', 'varsense']), 'analyzers');
  validateAnalyzer(policy.analyzers.sentinel, 'analyzers.sentinel');
  validateAnalyzer(policy.analyzers.varsense, 'analyzers.varsense');
  return policy;
}

export function defaultGuardPolicy() {
  return {
    npmScripts: [...BLOCKED_NPM_SCRIPTS],
    npxTools: [...BLOCKED_TOOLS],
    cargoSubcommands: [...BLOCKED_CARGO_COMMANDS],
    tools: ['rustfmt'],
  };
}

export function migrateLegacyConfig({ sentinelConfig, qualityConfig, toolManifest }) {
  if (!isRecord(sentinelConfig) || !isRecord(qualityConfig) || !isRecord(toolManifest)) {
    throw new Error('No se puede migrar una configuración legacy incompleta');
  }
  const sentinelTool = toolManifest.tools?.sentinel;
  const protocolVersion = Number.isInteger(Number(sentinelTool?.outputSchemaVersion))
    ? Number(sentinelTool.outputSchemaVersion)
    : 1;
  const migrated = {
    schemaVersion: 2,
    mode: 'enforce',
    gate: { command: [...DEFAULT_GATE_COMMAND], taskIdRequired: true },
    guard: { directCommands: defaultGuardPolicy() },
    runtime: {
      minimumVersion: String(sentinelTool?.version ?? '0.4.0'),
      protocolVersion,
      lockFile: 'sentinel.lock.json',
    },
    analyzers: {
      sentinel: { enabled: true, profile: 'project-default', config: sentinelConfig },
      varsense: { enabled: true, profile: 'project-default', config: 'varsense.config.json' },
    },
  };
  validatePolicy(migrated);
  return {
    policy: migrated,
    legacy: {
      qualityConfig,
      toolManifest: {
        schemaVersion: toolManifest.schemaVersion,
        tools: toolManifest.tools,
      },
    },
  };
}

async function exists(filePath) {
  try { await access(filePath); return true; } catch { return false; }
}

export async function discoverPolicy(startPath) {
  let candidate = path.resolve(startPath);
  while (true) {
    const policyPath = path.join(candidate, POLICY_FILE);
    if (await exists(policyPath)) return { projectRoot: candidate, policyPath };
    const parent = path.dirname(candidate);
    if (parent === candidate) return { projectRoot: null, policyPath: null };
    candidate = parent;
  }
}

export async function loadPolicy(startPath) {
  const discovered = await discoverPolicy(startPath);
  if (!discovered.policyPath) return { status: 'no-policy', ...discovered };
  let parsed;
  try { parsed = JSON.parse(await readFile(discovered.policyPath, 'utf8')); }
  catch (error) {
    return { status: 'invalid-policy', ...discovered, error: `JSON inválido: ${error.message}` };
  }
  if (parsed.schemaVersion === undefined) {
    return { status: 'legacy-v1', ...discovered, warning: 'sentinel.config.json usa el formato de analizador v1; no se activa como política v2' };
  }
  try {
    validatePolicy(parsed);
    return { status: 'policy', ...discovered, policy: parsed };
  } catch (error) {
    return { status: 'invalid-policy', ...discovered, error: error.message };
  }
}
