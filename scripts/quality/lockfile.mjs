import { createHash } from 'node:crypto';
import { access, readFile, realpath } from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { runProcess } from './runner.mjs';

const LOCK_SCHEMA_VERSION = 1;
const LOCK_FILE = 'sentinel.lock.json';
const SHA256_PATTERN = /^[a-f0-9]{64}$/u;
const TOOL_NAMES = new Set(['sentinel', 'varsense']);
const RUNTIME_STATUSES = new Set(['not-installed', 'project-adapter', 'installed']);
const INSTALL_METADATA_PATH = '.quality-install.json';
const COMMIT_PATTERN = /^[a-f0-9]{40}$/u;
const RUNTIME_ARTIFACT_STATUSES = new Set(['installed']);

function fail(message) {
  throw new Error(`sentinel.lock.json: ${message}`);
}

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function validateKeys(value, allowed, label) {
  const unknown = Object.keys(value).filter(key => !allowed.has(key));
  if (unknown.length > 0) fail(`${label}: claves desconocidas: ${unknown.join(', ')}`);
}

function validateSha(value, label) {
  if (typeof value !== 'string' || !SHA256_PATTERN.test(value)) fail(`${label}: SHA-256 inválido`);
}

function validateText(value, label) {
  if (typeof value !== 'string' || value.length === 0 || value.length > 160) fail(`${label}: texto inválido`);
}

function validateCommit(value, label, allowAliases = false) {
  if (allowAliases && ['not-installed', 'repo-scripts'].includes(value)) return;
  if (typeof value !== 'string' || !COMMIT_PATTERN.test(value)) fail(`${label}: commit inválido`);
}

function validateInstallRoot(value) {
  if (typeof value !== 'string' || value.length === 0 || path.isAbsolute(value)) fail('installRoot debe ser una ruta relativa');
  if (value.replace(/\\/g, '/').split('/').includes('..')) fail('installRoot no puede salir del workspace');
}

async function assertInsideWorkspace(workspaceRoot, target, label) {
  let rootReal;
  let targetReal;
  try {
    rootReal = await realpath(workspaceRoot);
    targetReal = await realpath(target);
  } catch {
    fail(`${label}: ruta inexistente o no resoluble`);
  }
  const relative = path.relative(rootReal, targetReal);
  if (relative === '..' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    fail(`${label}: ruta fuera del workspace`);
  }
  return targetReal;
}

export function untrustedCheckoutChanges(statusOutput) {
  return statusOutput
    .split(/\r?\n/)
    .map(line => line.trimEnd())
    .filter(Boolean)
    .filter(line => {
      const pathName = line.slice(3).replace(/\\\\/g, '/');
      return pathName !== INSTALL_METADATA_PATH;
    });
}

export function validateLock(lock, manifest) {
  if (!isRecord(lock)) fail('la raíz debe ser un objeto');
  validateKeys(lock, new Set(['schemaVersion', 'generatedAt', 'runtime', 'analyzers']), 'raíz');
  if (lock.schemaVersion !== LOCK_SCHEMA_VERSION) fail(`schemaVersion debe ser ${LOCK_SCHEMA_VERSION}`);
  if (typeof lock.generatedAt !== 'string' || Number.isNaN(Date.parse(lock.generatedAt))) fail('generatedAt inválido');

  validateInstallRoot(manifest?.installRoot);
  if (!isRecord(lock.runtime)) fail('runtime debe ser un objeto');
  validateKeys(lock.runtime, new Set(['status', 'version', 'commit', 'identitySha256', 'artifactSha256']), 'runtime');
  if (!RUNTIME_STATUSES.has(lock.runtime.status)) fail('runtime.status inválido');
  validateText(lock.runtime.version, 'runtime.version');
  validateCommit(lock.runtime.commit, 'runtime.commit', true);
  validateSha(lock.runtime.identitySha256, 'runtime.identitySha256');
  if (lock.runtime.artifactSha256 !== null && lock.runtime.artifactSha256 !== undefined) validateSha(lock.runtime.artifactSha256, 'runtime.artifactSha256');
  if (RUNTIME_ARTIFACT_STATUSES.has(lock.runtime.status) && !lock.runtime.artifactSha256) fail('runtime instalado debe declarar artifactSha256');
  if (lock.runtime.status === 'not-installed' && lock.runtime.commit !== 'not-installed') {
    fail('runtime not-installed debe declarar commit not-installed');
  }

  if (!isRecord(lock.analyzers)) fail('analyzers debe ser un objeto');
  const manifestTools = manifest?.tools;
  if (!isRecord(manifestTools)) fail('quality-tools.json.tools inválido');
  const lockNames = Object.keys(lock.analyzers);
  const manifestNames = Object.keys(manifestTools);
  if (lockNames.length !== manifestNames.length || lockNames.some(name => !manifestNames.includes(name))) {
    fail('analyzers no coincide con quality-tools.json.tools');
  }

  for (const name of manifestNames) {
    if (!TOOL_NAMES.has(name)) fail(`analyzer desconocido: ${name}`);
    const entry = lock.analyzers[name];
    const expected = manifestTools[name];
    if (!isRecord(entry)) fail(`analyzers.${name} debe ser un objeto`);
    validateKeys(entry, new Set(['version', 'protocolVersion', 'commit', 'sha256']), `analyzers.${name}`);
    validateText(entry.version, `analyzers.${name}.version`);
    if (entry.version !== expected.version) fail(`analyzers.${name}.version no coincide con quality-tools.json`);
    const protocolVersion = Number(expected.outputSchemaVersion);
    if (!Number.isInteger(entry.protocolVersion) || entry.protocolVersion !== protocolVersion) {
      fail(`analyzers.${name}.protocolVersion no coincide con quality-tools.json`);
    }
    validateCommit(entry.commit, `analyzers.${name}.commit`);
    if (entry.commit !== expected.commit) fail(`analyzers.${name}.commit no coincide con quality-tools.json`);
    validateSha(entry.sha256, `analyzers.${name}.sha256`);
  }
  return lock;
}

export async function readLock(workspaceRoot, manifest, lockFile = LOCK_FILE) {
  if (typeof lockFile !== 'string' || path.isAbsolute(lockFile) || lockFile.replace(/\\/g, '/').split('/').includes('..')) {
    fail('runtime.lockFile debe ser una ruta relativa dentro del workspace');
  }
  const lockPath = path.join(workspaceRoot, lockFile);
  try {
    await assertInsideWorkspace(workspaceRoot, lockPath, 'runtime.lockFile');
    const lock = JSON.parse(await readFile(lockPath, 'utf8'));
    validateLock(lock, manifest);
    return { lock, lockPath };
  } catch (error) {
    if (error?.code === 'ENOENT') throw new Error(`Falta ${lockFile}; ejecuta el generador/verificador de lock antes del gate`);
    throw error;
  }
}

export function runtimeLockHash(status, version, commit) {
  return createHash('sha256').update(`sentinel-runtime:${status}:${version}:${commit}`).digest('hex');
}

export function assertRuntimeLockHash(runtime) {
  const expected = runtimeLockHash(runtime.status, runtime.version, runtime.commit);
  if (runtime.identitySha256 !== expected) fail('runtime.identitySha256 no coincide con su identidad');
  if (runtime.status === 'installed' && !runtime.artifactSha256) fail('runtime instalado requiere artifactSha256');
}

export async function gitArchiveSha256(toolRoot) {
  return new Promise((resolve, reject) => {
    const child = spawn('git', ['-C', toolRoot, 'archive', '--format=tar', 'HEAD'], {
      cwd: toolRoot,
      shell: false,
      windowsHide: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const hash = createHash('sha256');
    let stderr = '';
    const timeout = setTimeout(() => {
      if (process.platform === 'win32' && child.pid) {
        spawn('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], { shell: false, stdio: 'ignore', windowsHide: true });
      } else child.kill('SIGTERM');
      reject(new Error('git archive excedió el timeout'));
    }, 30_000);
    child.stdout.on('data', chunk => hash.update(chunk));
    child.stderr.on('data', chunk => { stderr += chunk; });
    child.on('error', error => { clearTimeout(timeout); reject(error); });
    child.on('close', (code, signal) => {
      clearTimeout(timeout);
      if (signal || code !== 0) reject(new Error(`git archive falló (${signal ?? code}): ${stderr.trim()}`));
      else resolve(hash.digest('hex'));
    });
  });
}

export async function verifyInstalledAnalyzers(workspaceRoot, manifest, lock) {
  const results = {};
  for (const [name, config] of Object.entries(manifest.tools)) {
    validateInstallRoot(manifest.installRoot);
    const installRoot = await assertInsideWorkspace(workspaceRoot, path.resolve(workspaceRoot, manifest.installRoot), 'quality-tools.installRoot');
    const toolRoot = await assertInsideWorkspace(workspaceRoot, path.join(installRoot, name), `quality-tools.${name}`);
    const cliPath = path.join(toolRoot, config.cli);
    try {
      await access(cliPath);
    } catch {
      throw new Error(`Falta el CLI instalado de ${name}; ejecuta npm run quality:setup`);
    }
    const status = await runProcess('git', ['-C', toolRoot, 'status', '--porcelain', '--untracked-files=all'], { cwd: workspaceRoot, timeoutMs: 10_000 });
    if (status.code !== 0) throw new Error(`${name}: no se pudo inspeccionar el estado del checkout`);
    const untrustedChanges = untrustedCheckoutChanges(status.stdout);
    if (untrustedChanges.length > 0) throw new Error(`${name}: checkout modificado; no se puede confiar en sentinel.lock.json (${untrustedChanges.join(', ')})`);
    const version = await runProcess(process.execPath, [cliPath, '--version'], { cwd: workspaceRoot, timeoutMs: 10_000 });
    if (version.code !== 0 || version.stdout.trim() !== lock.analyzers[name].version) {
      throw new Error(`${name}: versión instalada no coincide con sentinel.lock.json`);
    }
    const revision = await runProcess('git', ['-C', toolRoot, 'rev-parse', 'HEAD'], { cwd: workspaceRoot, timeoutMs: 10_000 });
    if (revision.code !== 0 || revision.stdout.trim() !== lock.analyzers[name].commit) {
      throw new Error(`${name}: commit instalado no coincide con sentinel.lock.json`);
    }
    const sha256 = await gitArchiveSha256(toolRoot);
    if (sha256 !== lock.analyzers[name].sha256) {
      throw new Error(`${name}: SHA-256 del árbol instalado no coincide con sentinel.lock.json`);
    }
    results[name] = { version: version.stdout.trim(), commit: revision.stdout.trim(), sha256, cliPath };
  }
  return results;
}

export { LOCK_FILE, LOCK_SCHEMA_VERSION };
