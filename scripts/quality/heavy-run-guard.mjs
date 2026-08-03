import { lstat, mkdir, readFile, realpath, rename, unlink, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import crypto from 'node:crypto';
import os from 'node:os';
import path from 'node:path';

const DEFAULT_COOLDOWN_MS = 3 * 60 * 60 * 1000;
const DEFAULT_TARGET_BASE = process.platform === 'win32'
  ? 'C:\\tmp\\glory-target'
  : path.join(os.tmpdir(), 'glory-target');

function normalizeRoot(value) {
  return path.resolve(value).replace(/\\/g, '/').toLowerCase();
}

function projectKey(projectRoot) {
  return crypto.createHash('sha256').update(normalizeRoot(projectRoot)).digest('hex').slice(0, 16);
}

export function resolveTargetBase() {
  return path.resolve(process.env.CARGO_TARGET_DIR_BASE || DEFAULT_TARGET_BASE);
}

export function resolveGuardRoot(targetBase = resolveTargetBase()) {
  return path.join(path.dirname(path.resolve(targetBase)), 'glory-quality-guard');
}

function statePath(targetBase) {
  return path.join(resolveGuardRoot(targetBase), 'state.json');
}

function activePath(targetBase) {
  return path.join(resolveGuardRoot(targetBase), 'active.json');
}

async function readJson(filePath, fallback) {
  try { return JSON.parse(await readFile(filePath, 'utf8')); }
  catch { return fallback; }
}

async function writeJsonAtomic(filePath, value) {
  const temporary = `${filePath}.${process.pid}.${Date.now()}.tmp`;
  await writeFile(temporary, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
  try { await rename(temporary, filePath); }
  catch (error) {
    await unlink(temporary).catch(() => {});
    throw error;
  }
}

function processAlive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try { process.kill(pid, 0); return true; }
  catch { return false; }
}

function readCooldownMs(config = {}) {
  const minutes = Number(config.heavyRun?.cooldownMinutes);
  if (!Number.isFinite(minutes) || minutes < 0) return DEFAULT_COOLDOWN_MS;
  return minutes * 60 * 1000;
}

async function readProjectConfig(projectRoot) {
  return readJson(path.join(projectRoot, 'quality.config.json'), {});
}

export async function findQualityRoot(startPath = process.cwd()) {
  let candidate = path.resolve(startPath);
  try {
    candidate = await realpath(candidate);
  } catch (error) {
    if (error?.code === 'ENOENT') return path.resolve(startPath);
    throw error;
  }
  while (candidate) {
    const config = path.join(candidate, 'quality.config.json');
    const guard = path.join(candidate, 'scripts', 'quality', 'heavy-run-guard.mjs');
    try {
      const configMetadata = await lstat(config);
      const guardMetadata = await lstat(guard);
      if (!configMetadata.isFile() || !guardMetadata.isFile()) throw new Error('quality markers are not regular files');
      return candidate;
    } catch { /* Symlink/junction o marcador ausente: sube al padre. */ }
    const parent = path.dirname(candidate);
    if (parent === candidate) break;
    candidate = parent;
  }
  return path.resolve(startPath);
}

export function isHeavyCargoCommand(args) {
  const command = args.find(value => !String(value).startsWith('-'))?.toLowerCase();
  return command === 'test' || command === 'clippy' || command === 'bench';
}

export function isHeavyOverride(options = {}) {
  return Boolean(
    options.allowHeavy
    || options.ci
    || process.env.GLORY_QUALITY_ALLOW_HEAVY === '1'
    || process.env.GLORY_HEAVY_RUN_TOKEN,
  );
}

export async function inspectHeavyRun({ projectRoot, targetBase = resolveTargetBase(), mode = 'full', allowHeavy = false, now = Date.now() }) {
  const config = await readProjectConfig(projectRoot);
  const cooldownMs = readCooldownMs(config);
  const state = await readJson(statePath(targetBase), { version: 1, projects: {} });
  const entry = state.projects?.[projectKey(projectRoot)];
  const lastHeavyAt = Number(entry?.lastHeavyAt || 0);
  const elapsed = lastHeavyAt > 0 ? now - lastHeavyAt : Number.POSITIVE_INFINITY;
  const remainingMs = Math.max(0, cooldownMs - elapsed);
  const override = isHeavyOverride({ allowHeavy, ci: mode === 'ci' });
  if (!override && remainingMs > 0) {
    return {
      allowed: false,
      reason: 'cooldown',
      cooldownMs,
      remainingMs,
      nextAllowedAt: new Date(now + remainingMs).toISOString(),
      lastHeavyAt: new Date(lastHeavyAt).toISOString(),
    };
  }
  return { allowed: true, cooldownMs, remainingMs: 0, override };
}

async function clearStaleActiveLock(filePath, targetBase) {
  const active = await readJson(filePath, null);
  if (!active) return null;
  if (processAlive(Number(active.pid))) return active;
  await unlink(filePath).catch(() => {});
  return null;
}

export async function acquireHeavyRun({
  projectRoot,
  targetBase = resolveTargetBase(),
  mode = 'full',
  taskId = null,
  command = 'quality-full',
  allowHeavy = false,
}) {
  const decision = await inspectHeavyRun({ projectRoot, targetBase, mode, allowHeavy });
  if (!decision.allowed) return decision;

  const guardRoot = resolveGuardRoot(targetBase);
  await mkdir(guardRoot, { recursive: true });
  const lockPath = activePath(targetBase);
  const token = crypto.randomUUID();
  const lock = {
    version: 1,
    token,
    pid: process.pid,
    projectRoot: normalizeRoot(projectRoot),
    taskId,
    command,
    startedAt: new Date().toISOString(),
  };
  const existing = await clearStaleActiveLock(lockPath, targetBase);
  if (existing) {
    return {
      allowed: false,
      reason: 'active',
      message: `Ya existe una ejecución pesada activa (PID ${existing.pid}).`,
    };
  }
  try { await writeFile(lockPath, `${JSON.stringify(lock, null, 2)}\n`, { encoding: 'utf8', flag: 'wx' }); }
  catch (error) {
    if (error.code === 'EEXIST') return { allowed: false, reason: 'active', message: 'Otra ejecución pesada tomó el guard.' };
    throw error;
  }

  let released = false;
  return {
    ...decision,
    allowed: true,
    token,
    async release({ status = 'completed' } = {}) {
      if (released) return;
      released = true;
      const stateFile = statePath(targetBase);
      const state = await readJson(stateFile, { version: 1, projects: {} });
      state.version = 1;
      state.projects ??= {};
      state.projects[projectKey(projectRoot)] = {
        projectRoot: normalizeRoot(projectRoot),
        lastHeavyAt: Date.now(),
        lastStatus: status,
        taskId,
        command,
      };
      await writeJsonAtomic(stateFile, state);
      const current = await readJson(lockPath, null);
      if (current?.token === token) await unlink(lockPath).catch(() => {});
    },
  };
}

export function formatHeavyGuardMessage(decision) {
  if (decision.reason === 'cooldown') {
    const minutes = Math.ceil(decision.remainingMs / 60_000);
    return `Full diferido por cooldown: faltan aproximadamente ${minutes} min. Próxima ejecución: ${decision.nextAllowedAt}. Usa --allow-heavy solo si es imprescindible.`;
  }
  return decision.message || 'Full diferido porque ya hay otra ejecución pesada activa.';
}

async function executeCargo(argv) {
  const separator = argv.indexOf('--');
  const options = argv.slice(0, separator === -1 ? argv.length : separator);
  const cargoArgs = separator === -1 ? [] : argv.slice(separator + 1);
  const projectIndex = options.indexOf('--project-root');
  const cargoIndex = options.indexOf('--cargo-path');
  const requestedRoot = projectIndex >= 0 ? path.resolve(options[projectIndex + 1]) : process.cwd();
  const projectRoot = await findQualityRoot(requestedRoot);
  const cargoPath = cargoIndex >= 0 ? options[cargoIndex + 1] : (process.platform === 'win32' ? 'cargo.exe' : 'cargo');
  if (!isHeavyCargoCommand(cargoArgs)) {
    const light = spawn(cargoPath, cargoArgs, { cwd: projectRoot, env: process.env, stdio: 'inherit', shell: false, windowsHide: true });
    light.on('error', error => {
      console.error(`[glory-quality] Cargo no pudo iniciar: ${error.message}`);
      process.exitCode = 2;
    });
    light.on('exit', (code, signal) => { process.exitCode = signal ? 2 : code ?? 2; });
    return;
  }
  const lease = await acquireHeavyRun({
    projectRoot,
    mode: 'raw-cargo',
    command: `cargo ${cargoArgs.join(' ')}`,
    allowHeavy: options.includes('--allow-heavy'),
  });
  if (!lease.allowed) {
    console.error(`[glory-quality] BLOQUEADO: ${formatHeavyGuardMessage(lease)}`);
    process.exitCode = 75;
    return;
  }
  const child = spawn(cargoPath, cargoArgs, { cwd: projectRoot, env: process.env, stdio: 'inherit', shell: false, windowsHide: true });
  child.on('error', async error => {
    await lease.release({ status: 'error' });
    console.error(`[glory-quality] Cargo no pudo iniciar: ${error.message}`);
    process.exitCode = 2;
  });
  child.on('exit', async (code, signal) => {
    await lease.release({ status: signal ? 'signal' : code === 0 ? 'pass' : 'fail' });
    process.exitCode = signal ? 2 : code ?? 2;
  });
}

const argv = process.argv.slice(2);
if (argv.includes('--execute-cargo')) await executeCargo(argv);
else if (argv.includes('--status')) {
  const targetBase = resolveTargetBase();
  const state = await readJson(statePath(targetBase), { version: 1, projects: {} });
  const active = await readJson(activePath(targetBase), null);
  console.log(JSON.stringify({ targetBase, state, active }, null, 2));
}
