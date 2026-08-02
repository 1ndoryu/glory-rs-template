import { access, mkdir, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { resolveTargetBase } from './heavy-run-guard.mjs';

const DEFAULT_MAX_BYTES = 15 * 1024 ** 3;
const DEFAULT_MAX_AGE_MS = 7 * 24 * 60 * 60 * 1000;

async function readJson(filePath, fallback) {
  try { return JSON.parse(await readFile(filePath, 'utf8')); }
  catch { return fallback; }
}

async function pathSize(root) {
  let total = 0;
  let entries;
  try { entries = await readdir(root, { withFileTypes: true }); }
  catch { return 0; }
  for (const entry of entries) {
    const target = path.join(root, entry.name);
    if (entry.isDirectory()) total += await pathSize(target);
    else {
      try { total += (await stat(target)).size; }
      catch { /* Archivo concurrente: se medirá en la siguiente ejecución. */ }
    }
  }
  return total;
}

function pidAlive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try { process.kill(pid, 0); return true; }
  catch { return false; }
}

async function activePids(target) {
  const markers = [];
  const entries = await readdir(target, { withFileTypes: true }).catch(() => []);
  for (const entry of entries.filter(item => item.isDirectory())) {
    const children = await readdir(path.join(target, entry.name), { withFileTypes: true }).catch(() => []);
    for (const child of children.filter(item => item.name.startsWith('.glory-cargo-active-') && item.name.endsWith('.json'))) {
      const markerPath = path.join(target, entry.name, child.name);
      const marker = await readJson(markerPath, null);
      if (pidAlive(Number(marker?.pid))) markers.push({ target: entry.name, pid: Number(marker.pid), path: markerPath });
      else await rm(markerPath, { force: true }).catch(() => {});
    }
  }
  return markers;
}

function assertSafeTargetRoot(targetRoot) {
  const resolved = path.resolve(targetRoot);
  const normalized = resolved.replace(/\\/g, '/').toLowerCase();
  const safeSuffix = normalized.endsWith('/glory-target');
  const parentIsTmp = path.basename(path.dirname(resolved)).toLowerCase() === 'tmp';
  if (!safeSuffix || !parentIsTmp || normalized === '/' || normalized.length < 12) {
    throw new Error(`Target root rechazado por seguridad: ${resolved}`);
  }
  return resolved;
}

async function loadPolicy(projectRoot) {
  const config = await readJson(path.join(projectRoot, 'quality.config.json'), {});
  const policy = config.heavyRun ?? {};
  const maxTargetBytes = Number(policy.maxTargetGb) > 0 ? Number(policy.maxTargetGb) * 1024 ** 3 : DEFAULT_MAX_BYTES;
  const maxAgeMs = Number(policy.maxTargetAgeDays) > 0 ? Number(policy.maxTargetAgeDays) * 24 * 60 * 60 * 1000 : DEFAULT_MAX_AGE_MS;
  return { maxTargetBytes, maxAgeMs };
}

export async function cleanupTargets({ projectRoot = process.cwd(), targetRoot = resolveTargetBase(), now = Date.now(), dryRun = false } = {}) {
  const safeRoot = assertSafeTargetRoot(targetRoot);
  const policy = await loadPolicy(projectRoot);
  await mkdir(safeRoot, { recursive: true });
  const ownershipPath = path.join(safeRoot, '.glory-target-root.json');
  const ownership = await readJson(ownershipPath, null);
  if (!ownership && !dryRun) {
    await writeFile(ownershipPath, `${JSON.stringify({ version: 1, managedBy: 'glory-quality', projectRoot: path.resolve(projectRoot), createdAt: new Date().toISOString() }, null, 2)}\n`, 'utf8');
  }
  const active = new Set((await activePids(safeRoot)).map(item => item.target));
  const entries = await readdir(safeRoot, { withFileTypes: true });
  const candidates = [];
  for (const entry of entries.filter(item => item.isDirectory())) {
    const fullPath = path.join(safeRoot, entry.name);
    const details = await stat(fullPath).catch(() => null);
    if (!details) continue;
    candidates.push({ name: entry.name, path: fullPath, bytes: await pathSize(fullPath), lastWriteMs: details.mtimeMs, active: active.has(entry.name) });
  }
  let totalBytes = candidates.reduce((sum, item) => sum + item.bytes, 0);
  const removed = [];
  for (const candidate of candidates
    .filter(item => !item.active)
    .sort((left, right) => left.lastWriteMs - right.lastWriteMs)) {
    const tooOld = now - candidate.lastWriteMs > policy.maxAgeMs;
    const overQuota = totalBytes > policy.maxTargetBytes;
    if (!tooOld && !overQuota) continue;
    removed.push({ name: candidate.name, bytes: candidate.bytes, reason: tooOld ? 'age' : 'quota' });
    totalBytes -= candidate.bytes;
    if (!dryRun) await rm(candidate.path, { recursive: true, force: true });
  }
  return {
    targetRoot: safeRoot,
    maxTargetBytes: policy.maxTargetBytes,
    totalBytes,
    active: [...active],
    removed,
    dryRun,
  };
}

const argv = process.argv.slice(2);
if (argv.includes('--cleanup') || argv.includes('--dry-run')) {
  const result = await cleanupTargets({ dryRun: argv.includes('--dry-run') && !argv.includes('--cleanup') });
  console.log(JSON.stringify(result, null, 2));
}
