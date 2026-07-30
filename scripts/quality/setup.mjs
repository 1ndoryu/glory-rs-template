import { spawn } from 'node:child_process';
import { access, mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const manifestPath = path.join(projectRoot, 'quality-tools.json');
const npmCliPath = process.env.npm_execpath;

async function exists(target) {
  try {
    await access(target);
    return true;
  } catch {
    return false;
  }
}

function run(executable, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, args, {
      cwd: options.cwd ?? projectRoot,
      shell: false,
      stdio: options.capture ? ['ignore', 'pipe', 'pipe'] : 'inherit',
      windowsHide: true,
    });
    let stdout = '';
    let stderr = '';
    let timedOut = false;
    const timeout = setTimeout(() => {
      timedOut = true;
      if (process.platform === 'win32' && child.pid) {
        spawn('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], {
          shell: false,
          stdio: 'ignore',
          windowsHide: true,
        });
      } else {
        child.kill('SIGTERM');
      }
    }, options.timeoutMs ?? 300_000);
    child.stdout?.on('data', chunk => { stdout += chunk; });
    child.stderr?.on('data', chunk => { stderr += chunk; });
    child.on('error', error => {
      clearTimeout(timeout);
      reject(error);
    });
    child.on('close', (code, signal) => {
      clearTimeout(timeout);
      if (timedOut) {
        reject(new Error(`${executable} excedió el timeout`));
        return;
      }
      if (signal || code !== 0) {
        reject(new Error(`${executable} ${args.join(' ')} falló (${signal ?? code})\n${stderr.trim()}`));
        return;
      }
      resolve(stdout.trim());
    });
  });
}

async function readManifest() {
  const parsed = JSON.parse(await readFile(manifestPath, 'utf8'));
  if (parsed.schemaVersion !== 1 || !parsed.installRoot || !parsed.tools) {
    throw new Error('quality-tools.json no cumple schemaVersion 1');
  }
  return parsed;
}

async function installTool(name, config, installRoot) {
  const toolRoot = path.join(installRoot, name);
  const gitRoot = path.join(toolRoot, '.git');
  const markerPath = path.join(toolRoot, '.quality-install.json');
  process.stdout.write(`[quality:setup] ${name}: preparando ${config.version}\n`);

  if (!await exists(toolRoot)) {
    await run('git', ['clone', '--filter=blob:none', '--no-checkout', config.repository, toolRoot]);
    await run('git', ['checkout', '--detach', config.commit], { cwd: toolRoot });
  } else {
    if (!await exists(gitRoot)) {
      throw new Error(`${toolRoot} existe pero no es un checkout administrado; muévelo y repite quality:setup`);
    }
    const currentCommit = await run('git', ['rev-parse', 'HEAD'], { cwd: toolRoot, capture: true });
    if (currentCommit !== config.commit) {
      throw new Error(`${name} está en ${currentCommit}; se esperaba ${config.commit}. Reinstala .quality-tools/${name}`);
    }
  }

  const cliPath = path.join(toolRoot, config.cli);
  if (await exists(markerPath) && await exists(cliPath)) {
    const marker = JSON.parse(await readFile(markerPath, 'utf8'));
    const installedVersion = await run(process.execPath, [cliPath, '--version'], { capture: true });
    if (marker.commit === config.commit && installedVersion === config.version) {
      process.stdout.write(`[quality:setup] ${name}: PASS (cache)\n`);
      return { commit: config.commit, version: installedVersion, cli: path.relative(projectRoot, cliPath) };
    }
  }

  if (!npmCliPath) {
    throw new Error('npm_execpath no está disponible; ejecuta este setup mediante npm run quality:setup');
  }
  await run(process.execPath, [npmCliPath, 'ci', '--ignore-scripts'], { cwd: toolRoot });
  await run(process.execPath, [npmCliPath, 'run', config.buildScript], { cwd: toolRoot });
  const installedVersion = await run(process.execPath, [cliPath, '--version'], { capture: true });
  if (installedVersion !== config.version) {
    throw new Error(`${name} reporta ${installedVersion}; se esperaba ${config.version}`);
  }
  const markerTemporaryPath = `${markerPath}.tmp`;
  await writeFile(markerTemporaryPath, `${JSON.stringify({ commit: config.commit, version: installedVersion })}\n`, 'utf8');
  await rename(markerTemporaryPath, markerPath);
  process.stdout.write(`[quality:setup] ${name}: PASS\n`);
  return { commit: config.commit, version: installedVersion, cli: path.relative(projectRoot, cliPath) };
}

async function main() {
  const manifest = await readManifest();
  const installRoot = path.resolve(projectRoot, manifest.installRoot);
  await mkdir(installRoot, { recursive: true });
  const installed = {};

  for (const [name, config] of Object.entries(manifest.tools)) {
    installed[name] = await installTool(name, config, installRoot);
  }

  const statePath = path.join(installRoot, 'install-state.json');
  const temporaryPath = `${statePath}.tmp`;
  await writeFile(temporaryPath, `${JSON.stringify({ schemaVersion: 1, installed }, null, 2)}\n`, 'utf8');
  await rename(temporaryPath, statePath);
  process.stdout.write('[quality:setup] Herramientas listas. Próximo: npm run task:check -- <ID>\n');
}

main().catch(error => {
  process.stderr.write(`[quality:setup] ERROR: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 2;
});
