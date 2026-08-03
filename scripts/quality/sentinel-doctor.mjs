import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { migrateLegacyConfig, loadPolicy } from './policy.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

async function readJson(projectRoot, relativePath) {
  return JSON.parse(await readFile(path.join(projectRoot, relativePath), 'utf8'));
}

export function resolveLegacyRoot(discovered) {
  if (!discovered.projectRoot) {
    throw new Error('No se encontró una raíz con sentinel.config.json; no se puede migrar este proyecto sin configuración legacy');
  }
  return discovered.projectRoot;
}

function parseArgs(argv) {
  const options = { migrate: false, dryRun: false, json: false, cwd: process.cwd() };
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--migrate') options.migrate = true;
    else if (value === '--dry-run') options.dryRun = true;
    else if (value === '--json') options.json = true;
    else if (value === '--cwd') {
      options.cwd = argv[index + 1];
      index += 1;
    } else if (value.startsWith('--')) throw new Error(`Opción desconocida: ${value}`);
  }
  if (options.migrate && !options.dryRun) throw new Error('La migración solo está disponible como --dry-run en esta fase');
  return options;
}

async function main(argv = process.argv.slice(2)) {
  const options = parseArgs(argv);
  const discovered = await loadPolicy(options.cwd);
  const result = { schemaVersion: 1, command: 'sentinel doctor', ...discovered };
  if (options.migrate) {
    const legacyRoot = resolveLegacyRoot(discovered);
    const migrated = migrateLegacyConfig({
      sentinelConfig: await readJson(legacyRoot, 'sentinel.config.json'),
      qualityConfig: await readJson(legacyRoot, 'quality.config.json'),
      toolManifest: await readJson(legacyRoot, 'quality-tools.json'),
    });
    result.migration = {
      mode: 'dry-run',
      writes: [],
      target: 'sentinel.config.v2.preview.json',
      policy: migrated.policy,
      legacyPreserved: migrated.legacy,
      note: 'quality.config.json y quality-tools.json siguen siendo contratos legacy; no se aplicó una migración irreversible.',
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
