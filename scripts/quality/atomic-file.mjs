import { randomUUID } from 'node:crypto';
import { readdir, rename, stat, unlink, writeFile } from 'node:fs/promises';
import path from 'node:path';

const STALE_TEMP_MS = 600_000;

export async function cleanupStaleAtomicTemps(target) {
  const directory = path.dirname(target);
  const prefix = `${path.basename(target)}.tmp.`;
  let entries;
  try {
    entries = await readdir(directory);
  } catch {
    return;
  }

  await Promise.all(entries.filter(entry => entry.startsWith(prefix)).map(async entry => {
    const candidate = path.join(directory, entry);
    try {
      const metadata = await stat(candidate);
      if (Date.now() - metadata.mtimeMs > STALE_TEMP_MS) await unlink(candidate);
    } catch {
      /* Otro proceso puede haberlo retirado; no interrumpir el reporte. */
    }
  }));
}

export async function writeAtomic(target, content) {
  await cleanupStaleAtomicTemps(target);
  const temporary = `${target}.tmp.${process.pid}.${randomUUID()}`;
  await writeFile(temporary, content, 'utf8');
  await rename(temporary, target);
}
