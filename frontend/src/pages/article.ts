/* wandori.us — Article Page
 * Renderiza un articulo individual a partir de su slug.
 * Convierte TipTap JSON a HTML.
 * Oculta el profile header cuando se esta viendo un articulo. */

import { api } from '../api/client';
import { trackImageDownload } from '../features/analytics/tracker';
import { updateArticleMeta, setArticleJsonLd, resetMeta } from '../features/seo/meta';
import { showProfile } from '../store';
import { appendSanitizedHtml } from '../utils/sanitize-html';
import type { Article, Product } from '../api/types';

function formatDate(iso: string): string {
  const d = new Date(iso);
  const meses = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  return `${d.getDate()} ${meses[d.getMonth()]} ${d.getFullYear()}`;
}

/* Convertir TipTap JSON a HTML basico */
function tiptapToHtml(content: Record<string, unknown>): string {
  if (!content || typeof content !== 'object') return '';
  const doc = content as { type?: string; content?: Array<Record<string, unknown>> };
  if (!doc.content) return '';

  function renderNode(node: Record<string, unknown>): string {
    const type = node.type as string;
    const attrs = (node.attrs || {}) as Record<string, string>;
    const children = (node.content || []) as Array<Record<string, unknown>>;
    const marks = (node.marks || []) as Array<Record<string, unknown>>;
    const text = node.text as string | undefined;

    if (type === 'text' && text !== undefined) {
      let html = text;
      for (const mark of marks) {
        if (mark.type === 'bold') html = `<strong>${html}</strong>`;
        if (mark.type === 'italic') html = `<em>${html}</em>`;
        if (mark.type === 'code') html = `<code>${html}</code>`;
        if (mark.type === 'link') {
          const href = (mark.attrs as Record<string, string>)?.href || '#';
          html = `<a href="${href}" target="_blank" rel="noopener noreferrer">${html}</a>`;
        }
      }
      return html;
    }

    const inner = children.map(renderNode).join('');

    switch (type) {
      case 'paragraph': return `<p>${inner}</p>`;
      case 'heading': {
        const level = attrs.level || '2';
        return `<h${level}>${inner}</h${level}>`;
      }
      case 'bulletList': return `<ul>${inner}</ul>`;
      case 'orderedList': return `<ol>${inner}</ol>`;
      case 'listItem': return `<li>${inner}</li>`;
      case 'blockquote': return `<blockquote>${inner}</blockquote>`;
      case 'codeBlock': return `<pre><code>${inner}</code></pre>`;
      case 'horizontalRule': return '<hr>';
      case 'hardBreak': return '<br>';
      case 'image':
        return `<img src="${attrs.src}" alt="${attrs.alt || ''}" loading="lazy" />`;
      default:
        return inner;
    }
  }

  return doc.content.map(renderNode).join('');
}

export async function renderArticle(params: Record<string, string>): Promise<HTMLElement> {
  resetMeta();
  /* Ocultar profile header al ver un articulo */
  showProfile.set(false);

  const { slug } = params;
  const page = document.createElement('div');
  page.className = 'articulo';

  const cargando = document.createElement('p');
  cargando.className = 'cargando';
  cargando.textContent = 'cargando...';
  page.appendChild(cargando);

  try {
    const article = await api.get<Article>(`/api/articles/slug/${slug}`);
    page.innerHTML = '';

    updateArticleMeta({
      title: article.title,
      excerpt: article.excerpt,
      cover_image: article.cover_image || undefined,
      slug: article.slug,
    });

    setArticleJsonLd({
      title: article.title,
      excerpt: article.excerpt,
      cover_image: article.cover_image || undefined,
      slug: article.slug,
      published_at: article.published_at,
      created_at: article.created_at,
    });

    const titulo = document.createElement('h1');
    titulo.className = 'articulo-titulo';
    titulo.textContent = article.title;

    const fecha = document.createElement('time');
    fecha.className = 'articulo-fecha';
    const dateStr = article.published_at || article.created_at;
    fecha.textContent = formatDate(dateStr);

    const contenido = document.createElement('div');
    contenido.className = 'articulo-contenido';
    appendSanitizedHtml(contenido, tiptapToHtml(article.content));

    contenido.querySelectorAll('img').forEach((img) => {
      img.addEventListener('contextmenu', () => {
        trackImageDownload(img.src);
      });
    });

    page.append(titulo, fecha, contenido);

    /* Producto asociado */
    try {
      const products = await api.get<Product[]>(`/api/articles/${article.id}/products`);
      for (const product of products) {
        if (!product.is_active) continue;
        const btnCompra = document.createElement('button');
        btnCompra.className = 'articulo-boton-compra boton';
        btnCompra.textContent = `comprar — $${(product.price_cents / 100).toFixed(2)} ${product.currency}`;
        btnCompra.addEventListener('click', () => openCheckoutModal(product));
        page.appendChild(btnCompra);
      }
    } catch { /* No hay productos */ }

  } catch {
    /* Articulo no encontrado — demo */
    page.innerHTML = '';
    const demoTitle = slug.replace(/-/g, ' ');
    updateArticleMeta({ title: demoTitle, slug });

    const titulo = document.createElement('h1');
    titulo.className = 'articulo-titulo';
    titulo.textContent = demoTitle;

    const fecha = document.createElement('time');
    fecha.className = 'articulo-fecha';
    fecha.textContent = '15 jul 2026';

    const contenido = document.createElement('div');
    contenido.className = 'articulo-contenido';
    contenido.innerHTML = '<p>hay algo en el ruido blanco de los servidores que me recuerda al mar. no el mar turistico de postal, sino el otro, el que nadie ve de madrugada cuando la ciudad duerme y solo quedan las luces del puerto.</p><p>escribi este texto pensando en eso. en como las maquinas tienen su propio silencio, y como ese silencio a veces dice mas que cualquier palabra.</p>';

    page.append(titulo, fecha, contenido);
  }

  return page;
}

async function openCheckoutModal(product: Product): Promise<void> {
  const { createModal } = await import('../components/ui/modal');
  const { createInput } = await import('../components/ui/input');
  const { showToast } = await import('../components/ui/toast');

  const container = document.createElement('div');
  container.style.display = 'flex';
  container.style.flexDirection = 'column';
  container.style.gap = 'var(--espacio-lg)';

  const desc = document.createElement('p');
  desc.textContent = product.description || product.name;

  let email = '';
  const emailInput = createInput({
    label: 'email para recibir el archivo',
    type: 'email',
    placeholder: 'tu@email.com',
    onInput: (v) => { email = v; },
  });

  const btnPagar = document.createElement('button');
  btnPagar.className = 'boton boton-grande';
  btnPagar.textContent = 'proceder al pago';
  btnPagar.addEventListener('click', async () => {
    if (!email) { showToast('ingresa tu email'); return; }
    try {
      const { checkout_url } = await api.post<{ checkout_url: string }>(
        `/api/products/${product.id}/checkout`,
        { email },
      );
      window.location.href = checkout_url;
    } catch { showToast('error al iniciar el pago'); }
  });

  container.append(desc, emailInput, btnPagar);
  createModal({ titulo: product.name, contenido: container, ancho: '440px' });
}
