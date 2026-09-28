// Hospedaje temporal de media entrante para que el backend la descargue (289A-1).
// El backend archiva en UPLOAD_DIR y valida tipo; aquí solo se sirve 30 min.
import { mkdir, writeFile, readdir, unlink, stat } from "node:fs/promises";
import { config } from "./config.mjs";

const EXT_POR_MIME = { "image/jpeg": "jpg", "image/png": "png", "image/webp": "webp" };

export async function guardarTemporal(buffer, mime) {
  const ext = EXT_POR_MIME[mime] ?? "bin";
  await mkdir(config.mediaDir, { recursive: true });
  const nombre = `${crypto.randomUUID()}.${ext}`;
  await writeFile(new URL(nombre, config.mediaDir), buffer);
  return `http://127.0.0.1:${config.puerto}/media/${nombre}`;
}

export async function purgar() {
  let archivos = [];
  try {
    archivos = await readdir(config.mediaDir);
  } catch {
    return 0;
  }
  const ahora = Date.now();
  let borrados = 0;
  for (const a of archivos) {
    if (a === ".gitkeep") continue;
    try {
      const info = await stat(new URL(a, config.mediaDir));
      if (ahora - info.mtimeMs > config.mediaTtlMs) {
        await unlink(new URL(a, config.mediaDir));
        borrados += 1;
      }
    } catch {
      // Archivo en uso o ya borrado: se reintenta en el próximo ciclo.
    }
  }
  return borrados;
}
