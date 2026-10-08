/* Adaptador de plataforma (document): raíz de montaje y portapapeles.
 * Cualquier acceso directo a `document`/`navigator` vive aquí, nunca en
 * hooks/componentes. Sentinel (dom-access-outside-platform). */

/* Raíz de montaje: falla explícito si falta (nunca null silencioso). */
export function obtenerElementoPorId(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error(`Falta el elemento raíz #${id} en index.html.`);
  return el;
}

/* Copia texto al portapapeles; `false` = no disponible (permisos o
 * contexto no seguro). El llamador decide qué mostrar. */
export async function copiarTexto(texto: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(texto);
    return true;
  } catch {
    return false;
  }
}
