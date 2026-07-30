/* wandori.us — DOM Helper
 * Abstracción ligera sobre document.createElement para reducir las 215+ llamadas
 * directas esparcidas por el código.
 * [Auditoría v4 §1.2/§2.1] Primer paso hacia una capa de abstracción DOM. */

/** Atributos planos (solo strings) para simplificar el tipado.
 *  Event listeners y estilos se asignan post-creación. */
export interface DomAttrs {
  className?: string;
  id?: string;
  textContent?: string;
  innerHTML?: string;
  href?: string;
  target?: string;
  rel?: string;
  type?: string;
  placeholder?: string;
  alt?: string;
  src?: string;
  loading?: string;
  value?: string;
  name?: string;
  disabled?: string;
  'aria-label'?: string;
  'aria-haspopup'?: string;
  'aria-expanded'?: string;
  'role'?: string;
  title?: string;
  'data-external'?: string;
  [key: `data-${string}`]: string | undefined;
}

/** Crear un elemento HTML con atributos e hijos.
 *  @param tag - Tag del elemento (ej: 'div', 'span', 'h1')
 *  @param attrs - Atributos opcionales (solo strings)
 *  @param children - Hijos (HTMLElement o string) opcionales
 *
 *  @example
 *  createEl('div', { className: 'container' },
 *    createEl('h1', { textContent: 'Título' }),
 *    'texto directo',
 *  ) */
export function createEl<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs?: DomAttrs,
  ...children: (HTMLElement | string)[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);

  if (attrs) {
    if (attrs.className) el.className = attrs.className;
    if (attrs.id) el.id = attrs.id;
    if (attrs.textContent !== undefined) el.textContent = attrs.textContent;
    if (attrs.innerHTML) el.innerHTML = attrs.innerHTML;
    if (attrs.href) el.setAttribute('href', attrs.href);
    if (attrs.target) el.setAttribute('target', attrs.target);
    if (attrs.rel) el.setAttribute('rel', attrs.rel);
    if (attrs.type) el.setAttribute('type', attrs.type);
    if (attrs.placeholder) el.setAttribute('placeholder', attrs.placeholder);
    if (attrs.alt) el.setAttribute('alt', attrs.alt);
    if (attrs.src) el.setAttribute('src', attrs.src);
    if (attrs.loading) el.setAttribute('loading', attrs.loading);
    if (attrs.value) el.setAttribute('value', attrs.value);
    if (attrs.name) el.setAttribute('name', attrs.name);
    if (attrs.disabled) el.setAttribute('disabled', attrs.disabled);
    if (attrs['aria-label']) el.setAttribute('aria-label', attrs['aria-label']);
    if (attrs['aria-haspopup']) el.setAttribute('aria-haspopup', attrs['aria-haspopup']);
    if (attrs['aria-expanded']) el.setAttribute('aria-expanded', attrs['aria-expanded']);
    if (attrs['role']) el.setAttribute('role', attrs['role']);
    if (attrs['title']) el.setAttribute('title', attrs['title']);
    if (attrs['data-external']) el.setAttribute('data-external', attrs['data-external']);

    /* Atributos data-* dinámicos */
    for (const key of Object.keys(attrs)) {
      if (key.startsWith('data-') && key !== 'data-external') {
        const val = (attrs as Record<string, string | undefined>)[key];
        if (val !== undefined) {
          el.setAttribute(key, val);
        }
      }
    }
  }

  for (const child of children) {
    if (typeof child === 'string') {
      el.appendChild(document.createTextNode(child));
    } else {
      el.appendChild(child);
    }
  }

  return el;
}

/** Crear un contenedor con clase. Atajo para casos simples. */
export function createContainer(className: string, ...children: (HTMLElement | string)[]): HTMLDivElement {
  return createEl('div', { className }, ...children);
}

/** Crear un párrafo de texto. */
export function createText(text: string, className?: string): HTMLParagraphElement {
  return createEl('p', { className, textContent: text });
}

/** Crear un enlace externo. */
export function createExternalLink(href: string, text: string, className?: string): HTMLAnchorElement {
  return createEl('a', {
    href,
    textContent: text,
    className,
    target: '_blank',
    rel: 'noopener noreferrer',
    'data-external': 'true',
  });
}
