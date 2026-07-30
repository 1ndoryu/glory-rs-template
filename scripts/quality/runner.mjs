import { spawn } from 'node:child_process';
import { truncate } from './redaction.mjs';

const ENV_ALLOWLIST = [
  'PATH', 'Path', 'PATHEXT', 'SystemRoot', 'WINDIR', 'COMSPEC', 'TEMP', 'TMP',
  'USERPROFILE', 'APPDATA', 'LOCALAPPDATA', 'PROGRAMFILES', 'PROGRAMFILES(X86)',
  'NUMBER_OF_PROCESSORS', 'CI', 'NO_COLOR', 'TERM', 'npm_execpath',
  'DATABASE_URL', 'CARGO_TARGET_DIR_BASE', 'GLORY_CARGO_TARGET_DIR',
];
const activeChildren = new Set();

export function safeEnvironment(extra = {}) {
  const env = {};
  for (const key of ENV_ALLOWLIST) {
    if (process.env[key] !== undefined) env[key] = process.env[key];
  }
  return { ...env, ...extra };
}

function terminateTree(child) {
  if (!child.pid) return;
  if (process.platform === 'win32') {
    spawn('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], {
      shell: false, stdio: 'ignore', windowsHide: true,
    });
  } else child.kill('SIGTERM');
}

export function cancelAll() {
  for (const child of activeChildren) terminateTree(child);
}

export function runProcess(executable, args, options = {}) {
  return new Promise(resolve => {
    const startedAt = Date.now();
    const child = spawn(executable, args, {
      cwd: options.cwd,
      env: safeEnvironment(options.env),
      shell: false,
      windowsHide: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    activeChildren.add(child);
    let stdout = '';
    let stderr = '';
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      terminateTree(child);
    }, options.timeoutMs ?? 120_000);

    child.stdout.on('data', chunk => { stdout += chunk; });
    child.stderr.on('data', chunk => { stderr += chunk; });
    child.on('error', error => {
      clearTimeout(timer);
      activeChildren.delete(child);
      resolve({ code: 2, signal: null, timedOut: false, durationMs: Date.now() - startedAt, stdout: '', stderr: error.message });
    });
    child.on('close', (code, signal) => {
      clearTimeout(timer);
      activeChildren.delete(child);
      resolve({
        code: timedOut ? 2 : code ?? 2,
        signal,
        timedOut,
        durationMs: Date.now() - startedAt,
        stdout: truncate(stdout),
        stderr: truncate(stderr),
      });
    });
  });
}
