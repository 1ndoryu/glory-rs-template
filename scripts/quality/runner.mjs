import { spawn } from 'node:child_process';
import { truncate } from './redaction.mjs';

export const DEFAULT_ENV_ALLOWLIST = Object.freeze([
  'PATH', 'Path', 'PATHEXT', 'SystemRoot', 'WINDIR', 'COMSPEC', 'TEMP', 'TMP',
  'USERPROFILE', 'APPDATA', 'LOCALAPPDATA', 'PROGRAMFILES', 'PROGRAMFILES(X86)',
  'NUMBER_OF_PROCESSORS', 'CI', 'NO_COLOR', 'TERM', 'npm_execpath',
]);
const MAX_CAPTURE_BYTES = 64 * 1024;
const activeChildren = new Set();

function appendOutput(current, chunk) {
  const value = String(chunk);
  if (current.text.length >= MAX_CAPTURE_BYTES) { current.truncated = true; return current; }
  const remaining = MAX_CAPTURE_BYTES - current.text.length;
  current.text += value.slice(0, remaining);
  current.truncated ||= value.length > remaining;
  return current;
}
function outputText(capture) { return capture.truncated ? `${capture.text}\n...[quality output truncated at ${MAX_CAPTURE_BYTES} bytes]` : capture.text; }

export function safeEnvironment(extra = {}, allowlist = DEFAULT_ENV_ALLOWLIST) {
  const permitted = new Set(allowlist ?? DEFAULT_ENV_ALLOWLIST);
  const env = {};
  for (const key of permitted) if (process.env[key] !== undefined) env[key] = process.env[key];
  for (const [key, value] of Object.entries(extra ?? {})) if (permitted.has(key) && value !== undefined) env[key] = value;
  return env;
}

function terminateTree(child) {
  if (!child.pid) return;
  if (process.platform === 'win32') spawn('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], { shell: false, stdio: 'ignore', windowsHide: true });
  else child.kill('SIGTERM');
}
export function cancelAll() { for (const child of activeChildren) terminateTree(child); }

export function runProcess(executable, args, options = {}) {
  return new Promise(resolve => {
    const startedAt = Date.now();
    if (options.isCancelled?.()) { resolve({ code: 130, signal: null, timedOut: false, cancelled: true, durationMs: 0, stdout: '', stderr: '' }); return; }
    const isWindowsShim = process.platform === 'win32' && /\.(cmd|bat)$/i.test(executable);
    let spawnTarget = executable;
    let spawnArgs = args;
    if (isWindowsShim) {
      spawnTarget = [executable, ...args].map(part => /\s|"/.test(part) ? `"${part.replace(/"/g, '""')}"` : part).join(' ');
      spawnArgs = [];
    }
    const child = spawn(spawnTarget, spawnArgs, { cwd: options.cwd, env: safeEnvironment(options.env, options.envAllowlist), shell: isWindowsShim, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    activeChildren.add(child);
    const stdout = { text: '', truncated: false };
    const stderr = { text: '', truncated: false };
    let timedOut = false;
    let cancellationObserved = false;
    const timer = setTimeout(() => { timedOut = true; terminateTree(child); }, options.timeoutMs ?? 120_000);
    const cancellationTimer = setInterval(() => { if (!timedOut && !cancellationObserved && options.isCancelled?.()) { cancellationObserved = true; terminateTree(child); } }, 10);
    child.stdout.on('data', chunk => appendOutput(stdout, chunk));
    child.stderr.on('data', chunk => appendOutput(stderr, chunk));
    child.on('error', error => {
      clearTimeout(timer); clearInterval(cancellationTimer); activeChildren.delete(child);
      resolve({ code: 2, signal: null, timedOut: false, cancelled: cancellationObserved, durationMs: Date.now() - startedAt, stdout: '', stderr: error.message });
    });
    child.on('close', (code, signal) => {
      clearTimeout(timer); clearInterval(cancellationTimer); activeChildren.delete(child);
      const cancelled = !timedOut && cancellationObserved;
      resolve({ code: timedOut ? 2 : cancelled ? 130 : code ?? 2, signal, timedOut, cancelled, durationMs: Date.now() - startedAt, stdout: truncate(outputText(stdout)), stderr: truncate(outputText(stderr)) });
    });
  });
}
