import { writeFile } from 'node:fs/promises';
import path from 'node:path';
import { runProcess } from './runner.mjs';

function normalize(value) { return value.replace(/\\/g, '/'); }

async function gitLines(root, args) {
  const result = await runProcess('git', args, { cwd: root, timeoutMs: 20_000 });
  if (result.code !== 0) throw new Error(`Git no pudo calcular alcance: ${result.stderr || result.stdout}`);
  return result.stdout.split(/\r?\n/).map(item => normalize(item.trim())).filter(Boolean);
}

function parseChangedStatus(lines) {
  const files = [];
  let ambiguous = false;
  for (const line of lines) {
    const parts = line.split(/\t+/);
    const status = parts[0] ?? '';
    if (status === 'D' || status.startsWith('R')) ambiguous = true;
    for (const file of parts.slice(1)) if (file) files.push(normalize(file));
  }
  return { files, ambiguous };
}

function escapeRegex(value) {
  return value.replace(/[|\\{}()[\]^$+?.]/g, '\\$&');
}

function globToRegex(pattern) {
  const normalized = pattern.replace(/\\/g, '/').toLowerCase();
  let expression = '^';
  for (let index = 0; index < normalized.length; index += 1) {
    const character = normalized[index];
    if (character === '*' && normalized[index + 1] === '*') {
      if (normalized[index + 2] === '/') {
        expression += '(?:.*/)?';
        index += 2;
      } else {
        expression += '.*';
        index += 1;
      }
    } else if (character === '*') {
      expression += '[^/]*';
    } else if (character === '?') {
      expression += '[^/]';
    } else {
      expression += escapeRegex(character);
    }
  }
  return new RegExp(`${expression}$`, 'i');
}

export function matches(pathName, pattern) {
  const lowerPath = pathName.replace(/\\/g, '/').toLowerCase();
  const lowerPattern = pattern.replace(/\\/g, '/').toLowerCase();
  if (lowerPattern.startsWith('.') && !lowerPattern.includes('/')) return lowerPath.endsWith(lowerPattern);
  if (lowerPattern.endsWith('/')) return lowerPath.startsWith(lowerPattern);
  if (!/[?*]/.test(lowerPattern)) {
    return lowerPath === lowerPattern || lowerPath.endsWith(`/${lowerPattern}`);
  }
  return globToRegex(lowerPattern).test(lowerPath);
}

export async function detectScope(context, args) {
  const base = args.base ?? 'HEAD';
  const [changedStatus, untracked, tracked] = await Promise.all([
    gitLines(context.projectRoot, ['diff', '--name-status', '--diff-filter=ACMRD', base]),
    gitLines(context.projectRoot, ['ls-files', '--others', '--exclude-standard']),
    gitLines(context.projectRoot, ['ls-files']),
  ]);
  const parsedChanged = parseChangedStatus(changedStatus);
  const files = [...new Set([...parsedChanged.files, ...untracked])].sort();
  const automaticFull = files.length === 0 || parsedChanged.ambiguous || files.some(file =>
    context.qualityConfig.fullPatterns.some(pattern => matches(file, pattern))
  );
  const full = args.full || args.ci || automaticFull;
  const fingerprintFiles = full
    ? [...new Set([...tracked, ...untracked])].sort()
    : files;
  const profiles = new Set();

  for (const [profile, patterns] of Object.entries(context.qualityConfig.profiles)) {
    if (full || files.some(file => patterns.some(pattern => matches(file, pattern)))) profiles.add(profile);
  }
  if (full) ['rust', 'frontend', 'css', 'docs'].forEach(profile => profiles.add(profile));

  const changedFilesPath = path.join(context.reportRoot, 'changed-files.txt');
  await writeFile(changedFilesPath, `${files.join('\n')}\n`, 'utf8');
  return { base, files, fingerprintFiles, profiles, full, changedFilesPath };
}
