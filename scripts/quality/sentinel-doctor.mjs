import { lstat, readFile, realpath } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { migrateLegacyConfig, loadPolicy, policyIdentity } from './policy.mjs';
import { checkLock, writeLock } from './lock-generator.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

async function readJson(projectRoot, relativePath) {
  const filePath = path.join(projectRoot, relativePath);
  const metadata = await lstat(filePath);
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new Error(`${relativePath}: no puede ser symlink/junction y debe ser un archivo regular`);
  }
  const canonicalRoot = await realpath(projectRoot);
  const canonicalFile = await realpath(filePath);
  const relative = path.relative(canonicalRoot, canonicalFile);
  if (relative.startsWith('..') || path.isAbsolute(relative)) {
    throw new Error(`${relativePath}: su realpath debe permanecer dentro del workspace`);
  }
  return JSON.parse(await readFile(canonicalFile, 'utf8'));
}

export function resolveLegacyRoot(discovered) {
  if (!discovered.projectRoot) {
    throw new Error('No se encontró una raíz con sentinel.config.json; no se puede migrar este proyecto sin configuración legacy');
  }
  return discovered.projectRoot;
}

function parseArgs(argv) {
  const options = { migrate: false, dryRun: false, json: false, lock: false, write: false, cwd: process.cwd() };
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--migrate') options.migrate = true;
    else if (value === '--dry-run') options.dryRun = true;
    else if (value === '--json') options.json = true;
    else if (value === '--lock') options.lock = true;
    else if (value === '--write') options.write = true;
    else if (value === '--cwd') {
      options.cwd = argv[index + 1];
      index += 1;
    } else if (value.startsWith('--')) throw new Error(`Opción desconocida: ${value}`);
  }
  if (options.migrate && !options.dryRun) throw new Error('La migración solo está disponible como --dry-run en esta fase');
  if (options.write && !options.lock) throw new Error('--write requiere --lock');
  return options;
}

async function main(argv = process.argv.slice(2)) {
  const options = parseArgs(argv);
  if (options.lock) {
    const lockResult = options.write
      ? await writeLock(path.resolve(options.cwd))
      : await checkLock(path.resolve(options.cwd));
    const lockOutput = {
      schemaVersion: 1,
      command: 'sentinel doctor --lock',
      mode: options.write ? 'write' : 'check',
      status: options.write ? 'written' : lockResult.ok ? 'pass' : 'error',
      reason: options.write ? 'written' : lockResult.reason,
      lockPath: lockResult.lockPath,
      backupCreated: lockResult.backupCreated ?? false,
    };
    if (options.json) process.stdout.write(`${JSON.stringify(lockOutput, null, 2)}\\n`);
    else process.stdout.write(`[sentinel doctor] lock ${lockOutput.status}: ${lockOutput.reason}\\n`);
    if (!options.write && !lockResult.ok) process.exitCode = 1;
    return lockResult;
  }
  const discovered = await loadPolicy(options.cwd);
  const result = {
    schemaVersion: 1,
    command: 'sentinel doctor',
    ...discovered,
    decision: policyIdentity(discovered, null).decision,
  };
  if (options.migrate) {
    const legacyRoot = resolveLegacyRoot(discovered);
    const migrated = migrateLegacyConfig({
      sentinelConfig: await readJson(legacyRoot, 'sentinel.config.json'),
      qualityConfig: await readJson(legacyRoot, 'quality.config.json'),
      varsenseConfig: await readJson(legacyRoot, 'varsense.config.json'),
      toolManifest: await readJson(legacyRoot, 'quality-tools.json'),
    });
    result.migration = {
      mode: 'dry-run',
      writes: [],
      target: 'sentinel.config.v2.preview.json',
      policy: migrated.policy,
      mapped: migrated.mapped,
      legacyPreserved: migrated.legacy,
      note: 'El preview mapea los contratos legacy sin escribir; quality.config.json, varsense.config.json y quality-tools.json permanecen intactos hasta una migración global versionada.',
    };
  }
  const output = JSON.stringify(result, null, 2);
  if (options.json) process.stdout.write(`${output}\n`);
  else {
    process.stdout.write(`[sentinel doctor] ${result.status}\n`);
    if (result.policyPath) process.stdout.write(`[sentinel doctor] policy: ${result.policyPath}\n`);
    if (result.warning) process.stdout.write(`[sentinel doctor] warning: ${result.warning}\n`);
    if (result.error) process.stdout.write(`[sentinel doctor] error: ${result.error}\n`);
    if (result.migration) process.stdout.write('[sentinel doctor] migración dry-run: no se modificaron archivos\n');
  }
  if (result.status === 'invalid-policy') process.exitCode = 2;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch(error => {
    process.stderr.write(`[sentinel doctor] ERROR: ${error.message}\n`);
    process.exitCode = 2;
  });
}

export { main, parseArgs };
