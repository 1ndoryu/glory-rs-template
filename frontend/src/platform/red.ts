/* Adaptador de plataforma (red/DOM): descargar una URL a dataURL.
 * `FileReader` vive aquí, nunca en data ni componentes. */

export function urlADataUrl(url: string): Promise<string> {
  return fetch(url).then((r) => {
    if (!r.ok) throw new Error(`No se pudo descargar la foto (HTTP ${r.status}).`);
    return r.blob();
  }).then(
    (blob) =>
      new Promise<string>((resolve, reject) => {
        const lector = new FileReader();
        lector.onerror = () => reject(new Error('No se pudo leer la foto descargada.'));
        lector.onload = () => resolve(lector.result as string);
        lector.readAsDataURL(blob);
      }),
  );
}
