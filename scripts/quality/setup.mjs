import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
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
        if (options.allowFailure) {
          resolve({ code: code ?? 1, signal, stdout: stdout.trim(), stderr: stderr.trim() });
          return;
        }
        reject(new Error(`${executable} ${args.join(' ')} falló (${signal ?? code})\n${stderr.trim()}`));
        return;
      }
      resolve(stdout.trim());
    });
  });
}

function captureBinary(executable, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, args, {
      cwd: options.cwd ?? projectRoot,
      shell: false,
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    });
    const stdout = [];
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
    child.stdout.on('data', chunk => stdout.push(Buffer.from(chunk)));
    child.stderr.on('data', chunk => { stderr += chunk; });
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
        reject(new Error(`${executable} ${args.join(' ')} falló (${signal ?? code})\\n${stderr.trim()}`));
        return;
      }
      resolve(Buffer.concat(stdout));
    });
  });
}

async function assertAppliedPatch(name, toolRoot, expectedSha256) {
  const currentPatch = await captureBinary('git', ['diff', '--binary', '--no-ext-diff'], {
    cwd: toolRoot,
  });
  const currentSha256 = createHash('sha256').update(currentPatch).digest('hex');
  if (currentSha256 !== expectedSha256) {
    throw new Error(
      `${name}: el árbol parcheado difiere del patch declarado; revisa cambios locales antes de continuar`,
    );
  }
}

async function readManifest() {
  const parsed = JSON.parse(await readFile(manifestPath, 'utf8'));
  if (parsed.schemaVersion !== 1 || !parsed.installRoot || !parsed.tools) {
    throw new Error('quality-tools.json no cumple schemaVersion 1');
  }
  return parsed;
}

async function applyDeclaredPatch(name, config, toolRoot) {
  if (!config.patch) return null;

  const patchPath = path.resolve(projectRoot, config.patch.path);
  const relativePatchPath = path.relative(projectRoot, patchPath);
  if (relativePatchPath.startsWith('..') || path.isAbsolute(relativePatchPath)) {
    throw new Error(`${name}: la ruta del patch debe permanecer dentro del workspace`);
  }
  if (!/^[a-f0-9]{64}$/i.test(config.patch.sha256)) {
    throw new Error(`${name}: el manifest contiene un SHA-256 inválido`);
  }
  const patchBytes = await readFile(patchPath);
  const patchSha256 = createHash('sha256').update(patchBytes).digest('hex');
  if (patchSha256 !== config.patch.sha256) {
    throw new Error(`${name}: SHA-256 del patch no coincide; revisa el patch y actualiza el manifest`);
  }

  const check = await run('git', ['apply', '--check', patchPath], {
    cwd: toolRoot,
    capture: true,
    allowFailure: true,
  });
  if (typeof check === 'string' || check.code === 0) {
    await run('git', ['apply', patchPath], { cwd: toolRoot });
    await assertAppliedPatch(name, toolRoot, patchSha256);
    process.stdout.write(`[quality:setup] ${name}: patch aplicado (${patchSha256.slice(0, 12)})\n`);
    return patchSha256;
  }

  const reverseCheck = await run('git', ['apply', '--reverse', '--check', patchPath], {
    cwd: toolRoot,
    capture: true,
    allowFailure: true,
  });
  if (typeof reverseCheck === 'string' || reverseCheck.code === 0) {
    await assertAppliedPatch(name, toolRoot, patchSha256);
    process.stdout.write(`[quality:setup] ${name}: patch ya aplicado (${patchSha256.slice(0, 12)})\n`);
    return patchSha256;
  }

  throw new Error(
    `${name}: no se pudo aplicar ni reconocer el patch ${config.patch.path}\n${check.stderr || reverseCheck.stderr}`,
  );
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

  const patchSha256 = await applyDeclaredPatch(name, config, toolRoot);
  const cliPath = path.join(toolRoot, config.cli);
  if (await exists(markerPath) && await exists(cliPath)) {
    const marker = JSON.parse(await readFile(markerPath, 'utf8'));
    const installedVersion = await run(process.execPath, [cliPath, '--version'], { capture: true });
    if (
      marker.commit === config.commit
      && marker.patchSha256 === patchSha256
      && marker.testScript === (config.testScript ?? null)
      && installedVersion === config.version
    ) {
      process.stdout.write(`[quality:setup] ${name}: PASS (cache)\n`);
      return {
        commit: config.commit,
        version: installedVersion,
        patchSha256,
        cli: path.relative(projectRoot, cliPath),
      };
    }
  }

  if (!npmCliPath) {
    throw new Error('npm_execpath no está disponible; ejecuta este setup mediante npm run quality:setup');
  }
  await run(process.execPath, [npmCliPath, 'ci', '--ignore-scripts'], { cwd: toolRoot });
  await run(process.execPath, [npmCliPath, 'run', config.buildScript], { cwd: toolRoot });
  if (config.testScript) {
    await run(process.execPath, [npmCliPath, 'run', config.testScript], { cwd: toolRoot });
  }
  const installedVersion = await run(process.execPath, [cliPath, '--version'], { capture: true });
  if (installedVersion !== config.version) {
    throw new Error(`${name} reporta ${installedVersion}; se esperaba ${config.version}`);
  }
  const markerTemporaryPath = `${markerPath}.tmp`;
  await writeFile(
    markerTemporaryPath,
    `${JSON.stringify({
      commit: config.commit,
      version: installedVersion,
      patchSha256,
      testScript: config.testScript ?? null,
    })}\n`,
    'utf8',
  );
  await rename(markerTemporaryPath, markerPath);
  process.stdout.write(`[quality:setup] ${name}: PASS\n`);
  return {
    commit: config.commit,
    version: installedVersion,
    patchSha256,
    cli: path.relative(projectRoot, cliPath),
  };
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
