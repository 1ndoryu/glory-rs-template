import { execFile, spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const frontendRoot = path.join(projectRoot, 'frontend');
const vitestBin = path.join(frontendRoot, 'node_modules', 'vitest', 'vitest.mjs');
const fullMarkers = new Set([
  'frontend/package.json',
  'frontend/package-lock.json',
  'frontend/tsconfig.json',
  'frontend/vite.config.ts',
  'frontend/vitest.config.ts',
  'frontend/orval.config.ts',
]);

async function gitLines(args) {
  const { stdout } = await execFileAsync('git', args, { cwd: projectRoot, windowsHide: true });
  return stdout.split(/\r?\n/).map(value => value.trim()).filter(Boolean);
}

async function changedFiles() {
  const [tracked, untracked] = await Promise.all([
    gitLines(['diff', '--name-status', '--diff-filter=ACMRD', 'HEAD']),
    gitLines(['ls-files', '--others', '--exclude-standard']),
  ]);
  const trackedFiles = tracked.flatMap(line => {
    const parts = line.split(/\t+/);
    const status = parts[0] ?? '';
    /* Renames expose old and new paths. Treat them as full because references
     * to the old module can remain in the dependency graph. */
    return status.startsWith('R')
      ? parts.slice(1).map(file => ({ file, status }))
      : parts[1] ? [{ file: parts[1], status }] : [];
  });
  return [...trackedFiles, ...untracked.map(file => ({ file, status: '??' }))]
    .filter(item => item.file)
    .reduce((items, item) => {
      const file = item.file.replace(/\\/g, '/');
      if (!items.some(existing => existing.file === file)) items.push({ ...item, file });
      return items;
    }, [])
    .sort((left, right) => left.file.localeCompare(right.file));
}

function isFrontendSource(file) {
  return /^frontend\/src\/.*\.(?:ts|tsx|js|jsx)$/i.test(file);
}

function isFrontendTest(file) {
  return /^frontend\/src\/.*\.(?:test|spec)\.(?:ts|tsx|js|jsx)$/i.test(file);
}

function runVitest(args) {
  return new Promise(resolve => {
    const child = spawn(process.execPath, [vitestBin, ...args], {
      cwd: frontendRoot,
      env: { ...process.env, VITEST_MAX_WORKERS: '1' },
      stdio: 'inherit',
      windowsHide: true,
    });
    child.once('error', error => {
      process.stderr.write(`[frontend-tests] No se pudo iniciar Vitest: ${error.message}\n`);
      resolve(2);
    });
    child.once('close', (code, signal) => resolve(signal ? 2 : code ?? 2));
  });
}

const flags = new Set(process.argv.slice(2));
const files = await changedFiles();
const frontendItems = files.filter(item => item.file.startsWith('frontend/'));
const forceFull = flags.has('--full')
  || files.some(item => fullMarkers.has(item.file))
  || frontendItems.some(item => item.status === 'D' || item.status.startsWith('R') || item.status === '??')
  || frontendItems.some(item => isFrontendSource(item.file) && !isFrontendTest(item.file));
const testFiles = frontendItems
  .filter(item => isFrontendTest(item.file))
  .map(item => item.file.replace(/^frontend\//, ''));

if (flags.has('--dry-run')) {
  process.stdout.write(`${JSON.stringify({ mode: forceFull ? 'full' : 'selected', files: testFiles }, null, 2)}\n`);
  process.exit(0);
}

if (!forceFull && testFiles.length === 0) {
  process.stdout.write('[frontend-tests] Sin archivos TypeScript relacionados; no se ejecutan tests.\n');
  process.exit(0);
}

/* [018A-4] Solo tests modificados se ejecutan de forma selectiva. Código,
 * configuración, borrados, renombres o untracked fuerzan full para evitar
 * falsos PASS por un grafo de dependencias incompleto. */
const vitestArgs = forceFull
  ? ['run', '--maxWorkers=1', '--no-file-parallelism']
  : ['run', '--maxWorkers=1', '--no-file-parallelism', ...testFiles];
process.exitCode = await runVitest(vitestArgs);
