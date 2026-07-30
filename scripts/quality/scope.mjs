import { writeFile } from 'node:fs/promises';
import path from 'node:path';
import { runProcess } from './runner.mjs';

function normalize(value) { return value.replace(/\\/g, '/'); }

async function gitLines(root, args) {
  const result = await runProcess('git', args, { cwd: root, timeoutMs: 20_000 });
  if (result.code !== 0) throw new Error(`Git no pudo calcular alcance: ${result.stderr || result.stdout}`);
  return result.stdout.split(/\r?\n/).map(item => normalize(item.trim())).filter(Boolean);
}

function matches(pathName, pattern) {
  const lowerPath = pathName.toLowerCase();
  const lowerPattern = pattern.toLowerCase();
  return lowerPattern.startsWith('.') ? lowerPath.endsWith(lowerPattern) : lowerPath.includes(lowerPattern);
}

export async function detectScope(context, args) {
  const base = args.base ?? 'HEAD';
  const [changed, untracked, tracked] = await Promise.all([
    gitLines(context.projectRoot, ['diff', '--name-only', '--diff-filter=ACMR', base]),
    gitLines(context.projectRoot, ['ls-files', '--others', '--exclude-standard']),
    gitLines(context.projectRoot, ['ls-files']),
  ]);
  const files = [...new Set([...changed, ...untracked])].sort();
  const automaticFull = files.length === 0 || files.some(file =>
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
