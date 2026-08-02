import { createHash } from 'node:crypto';
import { mkdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import { writeAtomic } from './atomic-file.mjs';

const CACHE_FORMAT_VERSION = 3;

async function hashFile(hash, root, relativePath) {
  try {
    hash.update(relativePath);
    hash.update(await readFile(path.join(root, relativePath)));
  } catch { hash.update(`[missing:${relativePath}]`); }
}

export async function fingerprint(context, scope, stage) {
  const hash = createHash('sha256');
  /* [018A-4] Un PASS no puede cruzar cambios de runtime, plataforma o formato
   * del runner aunque el conjunto de archivos permanezca igual. */
  hash.update(`quality-cache-v${CACHE_FORMAT_VERSION}\0`);
  hash.update(`${process.version}\0${process.platform}\0${process.arch}\0`);
  /* [018A-52] CI/full ejecutan validaciones ampliadas y local-light no; un
   * PASS de un modo nunca puede reutilizarse para afirmar cobertura del otro. */
  hash.update(`mode:${context.ci ? 'ci' : context.full ? 'full' : 'local-light'}\0`);
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
