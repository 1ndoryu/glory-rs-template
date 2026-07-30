/* wandori.us — Admin Articles
 * Lista y editor de artículos para el panel de administración.
 * [Auditoría v4 §1.2] Migrado a createEl(). */

import { ArticleService } from '../services';
import { showToast } from '../components/ui/toast';
import { createModal } from '../components/ui/modal';
import { showConfirm } from '../components/ui/confirm';
import { createInput } from '../components/ui/input';
import { createTextarea } from '../components/ui/textarea';
import { pickAndUpload } from '../utils/upload';
import { clearArticleCache } from '../components/layout/sidebar';
import { createEl } from '../utils/dom';
import type { Article } from '../api/types';

function formatDate(iso: string): string {
  const d = new Date(iso);
  return `${d.getDate()}/${d.getMonth() + 1}/${d.getFullYear()}`;
}

export async function renderArticleList(container: HTMLElement): Promise<void> {
  container.textContent = '';
  container.appendChild(createEl('p', { className: 'cargando', textContent: 'cargando...' }));

  try {
    const data = await ArticleService.listByStatus('all');
    container.textContent = '';

    for (const article of data.items) {
      const titulo = createEl('span', { textContent: article.title + (article.status === 'draft' ? ' (borrador)' : '') + (article.is_pinned ? ' · fijado' : '') });
      const fecha = createEl('small', { className: 'ml-sm', textContent: ` — ${formatDate(article.created_at)}` });
      const info = createEl('div', {}, titulo, fecha);

      const btnEditar = createEl('button', { className: 'boton boton-pequeno', textContent: 'editar' });
      btnEditar.addEventListener('click', () => openEditor(article));

      const btnEliminar = createEl('button', { className: 'boton boton-pequeno', textContent: 'eliminar' });
      btnEliminar.addEventListener('click', async () => {
        const ok = await showConfirm(`eliminar "${article.title}"?`);
        if (ok) {
          await ArticleService.delete(article.id);
          showToast('articulo eliminado');
          clearArticleCache();
          renderArticleList(container);
        }
      });

      const acciones = createEl('div', { className: 'admin-acciones' }, btnEditar, btnEliminar);
      container.appendChild(createEl('div', { className: 'admin-item' }, info, acciones));
    }

    if (data.items.length === 0) {
      container.appendChild(createEl('p', { className: 'vacio', textContent: 'no hay articulos' }));
    }
  } catch {
    container.textContent = '';
    container.appendChild(createEl('p', { className: 'vacio', textContent: 'error al cargar' }));
  }
}

export async function openEditor(article?: Article): Promise<void> {
  const { createSelect } = await import('../components/ui/select');

  const container = createEl('div', { className: 'flex-columna gap-lg' });
  container.style.minHeight = '400px';

  let title = article?.title || '';
  let excerpt = article?.excerpt || '';
  let status = article?.status || 'draft';
  let isPinned = article?.is_pinned || false;

  const titleInput = createInput({ label: 'titulo', placeholder: 'titulo del articulo', value: title, onInput: v => { title = v; } });
  const excerptInput = createTextarea({ label: 'extracto', placeholder: 'resumen breve del articulo', value: excerpt, rows: 3, onInput: v => { excerpt = v; } });
  const statusSelect = createSelect({ label: 'estado', options: [{ value: 'draft', label: 'borrador' }, { value: 'published', label: 'publicado' }], value: status, onChange: v => { status = v as 'draft' | 'published'; } });

  const pinBtn = createEl('button', { className: 'boton', textContent: isPinned ? 'fijado ✓' : 'fijar articulo' });
  pinBtn.addEventListener('click', () => {
    isPinned = !isPinned;
    pinBtn.textContent = isPinned ? 'fijado ✓' : 'fijar articulo';
  });

  const editorContainer = createEl('div', { className: 'border-bottom' });
  editorContainer.style.minHeight = '300px';
  editorContainer.style.padding = 'var(--espacio-md)';

  const { Editor } = await import('@tiptap/core');
  const StarterKit = (await import('@tiptap/starter-kit')).default;
  const Image = (await import('@tiptap/extension-image')).default;
  const editor = new Editor({
    element: editorContainer,
    extensions: [StarterKit, Image.configure({ inline: false })],
    content: article?.content || { type: 'doc', content: [{ type: 'paragraph' }] },
  });

  const toolbar = createEl('div', { className: 'flex-wrap gap-sm mb-sm border-bottom' });
  const toolbarButtons: Array<{ label: string; action: () => void }> = [
    { label: 'negrita', action: () => { editor.chain().focus().toggleBold().run(); } },
    { label: 'italica', action: () => { editor.chain().focus().toggleItalic().run(); } },
    { label: 'codigo', action: () => { editor.chain().focus().toggleCode().run(); } },
    { label: 'h2', action: () => { editor.chain().focus().toggleHeading({ level: 2 }).run(); } },
    { label: 'h3', action: () => { editor.chain().focus().toggleHeading({ level: 3 }).run(); } },
    { label: 'lista', action: () => { editor.chain().focus().toggleBulletList().run(); } },
    { label: 'lista ordenada', action: () => { editor.chain().focus().toggleOrderedList().run(); } },
    { label: 'cita', action: () => { editor.chain().focus().toggleBlockquote().run(); } },
    { label: 'linea', action: () => { editor.chain().focus().setHorizontalRule().run(); } },
    { label: 'imagen', action: async () => { try { const r = await pickAndUpload('image/*', article?.id); if (r) editor.chain().focus().setImage({ src: r.url }).run(); } catch { showToast('error al subir imagen'); } } },
    { label: 'audio', action: async () => { try { const r = await pickAndUpload('audio/*', article?.id); if (r) editor.chain().focus().insertContent(`<audio controls src="${r.url}"></audio>`).run(); } catch { showToast('error al subir audio'); } } },
    { label: 'video', action: async () => { try { const r = await pickAndUpload('video/*', article?.id); if (r) editor.chain().focus().insertContent(`<video controls src="${r.url}" style="width:100%"></video>`).run(); } catch { showToast('error al subir video'); } } },
  ];

  for (const btn of toolbarButtons) {
    const el = createEl('button', { className: 'boton boton-pequeno', textContent: btn.label });
    el.addEventListener('click', btn.action);
    toolbar.appendChild(el);
  }

  let coverImage = article?.cover_image || '';
  const coverContainer = createEl('div', { className: 'campo' });
  const coverLabel = createEl('label', { className: 'campo-etiqueta', textContent: 'imagen de portada' });
  const coverPreview = createEl('img', { className: 'config-imagen-preview' });
  if (!coverImage) coverPreview.classList.add('oculto');
  if (coverImage) coverPreview.src = coverImage;

  const coverQuitar = createEl('button', { className: 'boton', textContent: 'quitar' });

  const coverBtn = createEl('button', { className: 'boton', textContent: coverImage ? 'cambiar portada' : 'subir portada' });
  coverBtn.addEventListener('click', async () => {
    try {
      const result = await pickAndUpload('image/*');
      if (result) {
        coverImage = result.url;
        coverPreview.src = result.url;
        coverPreview.classList.remove('oculto');
        coverBtn.textContent = 'cambiar portada';
        coverQuitar.classList.remove('oculto');
      }
    } catch { showToast('error al subir portada'); }
  });
  if (!coverImage) coverQuitar.classList.add('oculto');
  coverQuitar.addEventListener('click', () => {
    coverImage = '';
    coverPreview.classList.add('oculto');
    coverPreview.src = '';
    coverBtn.textContent = 'subir portada';
    coverQuitar.classList.add('oculto');
  });

  const coverBtns = createEl('div', { className: 'flex-fila gap-md' }, coverBtn, coverQuitar);
  coverContainer.append(coverLabel, coverPreview, coverBtns);

  const btnGuardar = createEl('button', { className: 'boton boton-grande', textContent: article ? 'guardar' : 'crear' });
  btnGuardar.addEventListener('click', async () => {
    if (!title.trim()) { showToast('el titulo es obligatorio'); return; }
    const payload = { title, excerpt, content: editor.getJSON(), cover_image: coverImage || undefined, status, is_pinned: isPinned };
    try {
      if (article) { await ArticleService.update(article.id, payload); showToast('articulo actualizado'); }
      else { await ArticleService.create(payload as any); showToast('articulo creado'); }
      clearArticleCache();
      modal.close();
      const lista = document.getElementById('admin-articulos');
      if (lista) renderArticleList(lista);
    } catch { showToast('error al guardar'); }
  });

  container.append(titleInput, excerptInput, coverContainer, toolbar, editorContainer, statusSelect, pinBtn, btnGuardar);
  const modal = createModal({ titulo: article ? 'editar articulo' : 'nuevo articulo', contenido: container, ancho: '720px', onClose: () => editor.destroy() });
}
