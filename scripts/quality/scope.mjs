import { access, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { runProcess } from './runner.mjs';
import { validateExecutableProfiles } from './profile-contract.mjs';

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

async function existingPath(candidate) {
  try { await access(candidate); return candidate; } catch { return null; }
}

export async function expandLocalDependencies(root, files) {
  const resolved = new Set(files);
  const queue = [...files];
  while (queue.length > 0) {
    const relative = queue.shift();
    if (!/\.(?:ts|tsx|js|jsx|mjs)$/.test(relative)) continue;
    let source;
    try { source = await readFile(path.join(root, relative), 'utf8'); } catch { continue; }
    for (const match of source.matchAll(/from\s*['"](\.[^'"]+)['"]|import\s*\(['"](\.[^'"]+)['"]\)/g)) {
      const specifier = match[1] ?? match[2];
      const base = path.normalize(path.join(path.dirname(relative), specifier));
      const candidates = [base, `${base}.ts`, `${base}.tsx`, `${base}.js`, `${base}.jsx`, path.join(base, 'index.ts')];
      for (const candidate of candidates) {
        const normalized = candidate.replace(/\\/g, '/');
        if (await existingPath(path.join(root, normalized)) && !resolved.has(normalized)) {
          resolved.add(normalized);
          queue.push(normalized);
          break;
        }
      }
    }
  }
  return [...resolved].sort();
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

export function resolveExplicitProfiles(args, availableProfiles, env = process.env) {
  const cliProfiles = Array.isArray(args.profiles) ? args.profiles : [];
  const envProfiles = typeof env.GLORY_QUALITY_PROFILE === 'string' && env.GLORY_QUALITY_PROFILE.trim().length > 0
    ? env.GLORY_QUALITY_PROFILE.split(',').map(profile => profile.trim()).filter(Boolean)
    : [];
  const requested = cliProfiles.length > 0 ? cliProfiles : envProfiles;
  if (requested.length === 0) return { profiles: new Set(), explicit: false, source: null };
  const unique = [...new Set(requested)];
  const unknown = unique.filter(profile => !Object.prototype.hasOwnProperty.call(availableProfiles, profile));
  if (unknown.length > 0) {
    throw new Error(`Perfil no permitido: ${unknown.join(', ')}`);
  }
  validateExecutableProfiles(unique);
  return {
    profiles: new Set(unique),
    explicit: true,
    source: cliProfiles.length > 0 ? 'cli' : 'env',
  };
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
    : await expandLocalDependencies(context.projectRoot, files);
  const explicitProfiles = resolveExplicitProfiles(args, context.qualityConfig.profiles);
  const profiles = explicitProfiles.explicit
    ? explicitProfiles.profiles
    : new Set();

  if (!explicitProfiles.explicit) {
    for (const [profile, patterns] of Object.entries(context.qualityConfig.profiles)) {
      if (full || files.some(file => patterns.some(pattern => matches(file, pattern)))) profiles.add(profile);
    }
    if (full) ['rust', 'frontend', 'css', 'docs'].forEach(profile => profiles.add(profile));
  }

  const changedFilesPath = path.join(context.reportRoot, 'changed-files.txt');
  await writeFile(changedFilesPath, `${files.join('\n')}\n`, 'utf8');
  return {
    base,
    files,
    fingerprintFiles,
    profiles,
    full,
    executionFull: full && !explicitProfiles.explicit,
    profileOverride: explicitProfiles.explicit,
    profileSource: explicitProfiles.source,
    changedFilesPath,
  };
}
