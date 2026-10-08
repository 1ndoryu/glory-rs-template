/* Adaptador de plataforma (DOM/Canvas): carga y reescalado de imágenes.
 * Cualquier acceso directo a `document`, `Image` o `URL` vive aquí, nunca
 * en data. Sentinel (dom-access-outside-platform). */

/* Carga un `File` como imagen lista para dibujar (libera su object URL). */
export function cargarImagenArchivo(file: File): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      URL.revokeObjectURL(url);
      resolve(img);
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error(`No se pudo leer la imagen ${file.name}.`));
    };
    img.src = url;
  });
}

/* Dibuja `img` a `w×h` y la devuelve como JPEG dataURL con `calidad`. */
export function reescalarAJpeg(img: HTMLImageElement, w: number, h: number, calidad: number): string {  const canvas = document.createElement('canvas');
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('El navegador no permite procesar imágenes.');
  ctx.drawImage(img, 0, 0, w, h);
  return canvas.toDataURL('image/jpeg', calidad);
}

/* [08AA-23] Lienzo fuera de pantalla para componer (detalle público):
 * `document.createElement` vive aquí, nunca en features. */
export function crearLienzo(): HTMLCanvasElement {
  return document.createElement('canvas');
}
