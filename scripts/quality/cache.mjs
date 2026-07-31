import { createHash } from 'node:crypto';
import { mkdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import { writeAtomic } from './atomic-file.mjs';

async function hashFile(hash, root, relativePath) {
  try {
    hash.update(relativePath);
    hash.update(await readFile(path.join(root, relativePath)));
  } catch { hash.update(`[missing:${relativePath}]`); }
}

export async function fingerprint(context, scope, stage) {
  const hash = createHash('sha256');
  hash.update(stage);
  hash.update(JSON.stringify(context.qualityConfig));
  hash.update(JSON.stringify(context.toolManifest));
  for (const file of scope.fingerprintFiles ?? scope.files) {
    await hashFile(hash, context.projectRoot, file);
  }
  return hash.digest('hex');
}

function cachePath(context, stage) {
  return path.join(context.projectRoot, '.quality-reports', 'cache', `${stage}.json`);
}

export async function readCachedPass(context, stage, expectedFingerprint) {
  try {
    const cached = JSON.parse(await readFile(cachePath(context, stage), 'utf8'));
    if (cached.fingerprint === expectedFingerprint && cached.result?.status === 'pass') {
      return { ...cached.result, cached: true };
    }
  } catch { /* Cache ausente o inválida: ejecutar la etapa. */ }
  return null;
}

export async function writeCachedPass(context, stage, stageFingerprint, result) {
  if (result.status !== 'pass') return;
  const target = cachePath(context, stage);
  await mkdir(path.dirname(target), { recursive: true });
  await writeAtomic(target, `${JSON.stringify({ fingerprint: stageFingerprint, result }, null, 2)}\n`);
}
