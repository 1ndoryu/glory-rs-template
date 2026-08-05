import { mkdir, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import path from 'node:path';
import { resolveTargetBase } from './heavy-run-guard.mjs';

const execFileAsync = promisify(execFile);

const DEFAULT_MAX_BYTES = 15 * 1024 ** 3;
const DEFAULT_MAX_AGE_MS = 7 * 24 * 60 * 60 * 1000;
/* [028A-6] La supervisión automática del gate no debe caminar 15 GB de
 * artefactos en cada ejecución: una vez por ventana es suficiente y el
 * comando manual (quality:cleanup) siempre fuerza el pase completo. */
const DEFAULT_MAINTENANCE_INTERVAL_MS = 6 * 60 * 60 * 1000;
/* [028A-6] Presupuesto del pase automático: si excede, se informa
 * `truncated` y el gate nunca se cuelga en el mantenimiento. */
export const DEFAULT_MAINTENANCE_BUDGET_MS = 60_000;
/* [028A-6] Escritura reciente: un target que cargo/rustc está recompilando
 * ahora no se protege por ruta de ejecutable (cargo.exe vive en .rustup),
 * solo por mtime. La poda por cuota nunca toca targets con escritura en la
 * última media hora; la poda por edad sigue su criterio de días. */
export const RECENT_WRITE_MS = 30 * 60 * 1000;

async function readJson(filePath, fallback) {
  try { return JSON.parse(await readFile(filePath, 'utf8')); }
  catch { return fallback; }
}

function normalizePath(filePath) {
  return path.resolve(filePath).replace(/\\/g, '/').toLowerCase();
}

/* [028A-6] Rutas de ejecutables de procesos vivos (Windows). Un target en
 * uso — p. ej. un `cargo run` lanzado directamente, sin marcador del guard —
 * se protege comparando el prefijo del ejecutable: borrar `debug/` mientras
 * `glory-backend.exe` corre desde ahí rompería el proceso en ejecución. */
async function runningProcessPaths() {
  const paths = new Set();
  if (process.platform === 'win32') {
    try {
      const { stdout } = await execFileAsync('powershell', [
        '-NoProfile', '-Command',
        'Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath } | ForEach-Object { $_.ExecutablePath }',
      ], { timeout: 10_000, windowsHide: true });
      for (const line of stdout.split(/\r?\n/)) {
        const candidate = line.trim();
        if (candidate) paths.add(normalizePath(candidate));
      }
    } catch { /* Sin acceso a WMI: se depende de los marcadores del guard. */ }
  } else {
    try {
      const { stdout } = await execFileAsync('ps', ['-eo', 'comm'], { timeout: 10_000 });
      for (const line of stdout.split(/\r?\n/)) {
        const candidate = line.trim();
        if (candidate) paths.add(normalizePath(candidate));
      }
    } catch { /* Fallback sin procesos: se depende de los marcadores. */ }
  }
  return paths;
}

async function pathSize(root, budgetDeadlineMs = Infinity) {
  let total = 0;
  let entries;
  try { entries = await readdir(root, { withFileTypes: true }); }
  catch { return total; }
  for (const entry of entries) {
    if (Date.now() > budgetDeadlineMs) return { size: total, truncated: true };
    const target = path.join(root, entry.name);
    if (entry.isDirectory()) {
      const nested = await pathSize(target, budgetDeadlineMs);
      total += nested.size;
      if (nested.truncated) return { size: total, truncated: true };
    } else {
      try { total += (await stat(target)).size; }
      catch { /* Archivo concurrente: se medirá en la siguiente ejecución. */ }
    }
  }
  return { size: total, truncated: false };
}

function pidAlive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try { process.kill(pid, 0); return true; }
  catch { return false; }
}

async function activeMarkers(target) {
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

export async function lastMaintenanceAt(targetRoot = resolveTargetBase()) {
  const marker = await readJson(path.join(targetRoot, '.glory-target-maintenance.json'), null);
  return Number(marker?.lastRunAt) || 0;
}

export async function shouldRunMaintenance({ targetRoot = resolveTargetBase(), now = Date.now(), intervalMs = DEFAULT_MAINTENANCE_INTERVAL_MS } = {}) {
  return now - await lastMaintenanceAt(targetRoot) >= intervalMs;
}

export async function markMaintenanceRun(targetRoot = resolveTargetBase(), now = Date.now()) {
  await writeFile(path.join(targetRoot, '.glory-target-maintenance.json'), `${JSON.stringify({ lastRunAt: now }, null, 2)}\n`, 'utf8');
}

export async function cleanupTargets({
  projectRoot = process.cwd(),
  targetRoot = resolveTargetBase(),
  now = Date.now(),
  dryRun = false,
  budgetMs,
  processPaths = null,
} = {}) {
  const safeRoot = assertSafeTargetRoot(targetRoot);
  const policy = await loadPolicy(projectRoot);
  await mkdir(safeRoot, { recursive: true });
  const ownershipPath = path.join(safeRoot, '.glory-target-root.json');
  const ownership = await readJson(ownershipPath, null);
  if (!ownership && !dryRun) {
    await writeFile(ownershipPath, `${JSON.stringify({ version: 1, managedBy: 'glory-quality', projectRoot: path.resolve(projectRoot), createdAt: new Date().toISOString() }, null, 2)}\n`, 'utf8');
  }
  const markers = await activeMarkers(safeRoot);
  const activeFromMarkers = new Set(markers.map(item => item.target));
  /* [028A-6] Protección doble: marcadores del guard + ejecutables en uso.
   * Un binario corriendo desde `candidate` lo marca activo sin importar su
   * mtime; nunca se borra un target del que un proceso vivo está cargado. */
  const liveProcesses = processPaths ?? await runningProcessPaths();
  const budgetDeadline = budgetMs ? Date.now() + budgetMs : Infinity;
  const entries = await readdir(safeRoot, { withFileTypes: true });
  const candidates = [];
  let truncated = false;
  for (const entry of entries.filter(item => item.isDirectory())) {
    const fullPath = path.join(safeRoot, entry.name);
    const details = await stat(fullPath).catch(() => null);
    if (!details) continue;
    const measurement = await pathSize(fullPath, budgetDeadline);
    if (measurement.truncated) { truncated = true; break; }
    const runningFrom = [...liveProcesses].some(executable => executable.startsWith(normalizePath(fullPath) + '/'));
    candidates.push({
      name: entry.name,
      path: fullPath,
      bytes: measurement.size,
      lastWriteMs: details.mtimeMs,
      active: activeFromMarkers.has(entry.name) || runningFrom,
    });
  }
  let totalBytes = candidates.reduce((sum, item) => sum + item.bytes, 0);
  const removed = [];
  for (const candidate of candidates
    .filter(item => !item.active)
    .sort((left, right) => left.lastWriteMs - right.lastWriteMs)) {
    if (Date.now() > budgetDeadline) { truncated = true; break; }
    const tooOld = now - candidate.lastWriteMs > policy.maxAgeMs;
    /* [028A-6] Un target recién escrito está siendo usado aunque cargo/rustc
     * no cuelguen de él por ruta: la cuota no lo toca, la edad sí. */
    const recentlyWritten = now - candidate.lastWriteMs < RECENT_WRITE_MS;
    const overQuota = totalBytes > policy.maxTargetBytes && !recentlyWritten;
    if (!tooOld && !overQuota) continue;
    removed.push({ name: candidate.name, bytes: candidate.bytes, reason: tooOld ? 'age' : 'quota' });
    totalBytes -= candidate.bytes;
    if (!dryRun) await rm(candidate.path, { recursive: true, force: true });
  }
  return {
    targetRoot: safeRoot,
    maxTargetBytes: policy.maxTargetBytes,
    totalBytes,
    active: [...activeFromMarkers, ...candidates.filter(item => item.active).map(item => item.name)],
    removed,
    dryRun,
    truncated,
  };
}

const argv = process.argv.slice(2);
if (argv.includes('--cleanup') || argv.includes('--dry-run')) {
  /* [028A-6] El comando manual siempre fuerza el pase completo (sin throttle
   * ni presupuesto): el usuario pidió una revisión explícita. Un pase real
   * también marca el throttle para que el gate no vuelva a caminar el target
   * dentro de la ventana. */
  const dryRun = argv.includes('--dry-run') && !argv.includes('--cleanup');
  const result = await cleanupTargets({ dryRun });
  if (!dryRun) await markMaintenanceRun(result.targetRoot);
  console.log(JSON.stringify(result, null, 2));
}
