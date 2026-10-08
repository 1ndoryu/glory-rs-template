/* Adaptador de plataforma (DOM): cualquier acceso directo al documento vive
 * aquí, nunca en componentes. Sentinel (dom-access-outside-platform). */

/* Descarga un dataURL como archivo con el nombre dado (via anchor temporal). */
export function descargarDataUrl(url: string, nombre: string): void {
  const a = document.createElement('a');
  a.href = url;
  a.download = nombre;
  document.body.appendChild(a);
  a.click();
  a.remove();
}

/* Descarga una URL http(s) como archivo (vía blob temporal): el atributo
 * `download` se ignora entre orígenes, así que no basta el anchor directo. */
export async function descargarUrl(url: string, nombre: string): Promise<void> {
  const respuesta = await fetch(url);
  if (!respuesta.ok) throw new Error(`No se pudo descargar la foto (HTTP ${respuesta.status}).`);
  const blob = await respuesta.blob();
  const objeto = URL.createObjectURL(blob);
  try {
    const a = document.createElement('a');
    a.href = objeto;
    a.download = nombre;
    document.body.appendChild(a);
    a.click();
    a.remove();
  } finally {
    URL.revokeObjectURL(objeto);
  }
}

/* Pausa en ms (evita que el navegador bloquee descargas múltiples seguidas). */
export function esperar(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}
