/* wandori.us — Admin Articles
 * Lista y editor de artículos para el panel de administración. */

import { api } from '../api/client';
import { showToast } from '../components/ui/toast';
import { createModal } from '../components/ui/modal';
import { showConfirm } from '../components/ui/confirm';
import { createInput } from '../components/ui/input';
import { createTextarea } from '../components/ui/textarea';
import { pickAndUpload } from '../utils/upload';
import { clearArticleCache } from '../components/layout/sidebar';
import type { Article, PaginatedArticles } from '../api/types';

function formatDate(iso: string): string {
  const d = new Date(iso);
  return `${d.getDate()}/${d.getMonth() + 1}/${d.getFullYear()}`;
}

/* === Lista de artículos === */
export async function renderArticleList(container: HTMLElement): Promise<void> {
  container.innerHTML = '<p class="cargando">cargando...</p>';

  try {
    const data = await api.get<PaginatedArticles>('/api/admin/articles?per_page=50');
    container.innerHTML = '';

    for (const article of data.items) {
      const item = document.createElement('div');
      item.className = 'admin-item';

      const info = document.createElement('div');
      const titulo = document.createElement('span');
      titulo.textContent = article.title;
      if (article.status === 'draft') titulo.textContent += ' (borrador)';
      if (article.is_pinned) titulo.textContent += ' · fijado';

      const fecha = document.createElement('small');
      fecha.textContent = ` — ${formatDate(article.created_at)}`;
      fecha.className = 'ml-sm';
      info.append(titulo, fecha);

      const acciones = document.createElement('div');
      acciones.className = 'admin-acciones';

      const btnEditar = document.createElement('button');
      btnEditar.className = 'boton boton-pequeno';
      btnEditar.textContent = 'editar';
      btnEditar.addEventListener('click', () => openEditor(article));

      const btnEliminar = document.createElement('button');
      btnEliminar.className = 'boton boton-pequeno';
      btnEliminar.textContent = 'eliminar';
      btnEliminar.addEventListener('click', async () => {
        const ok = await showConfirm(`eliminar "${article.title}"?`);
        if (ok) {
          await api.delete(`/api/admin/articles/${article.id}`);
          showToast('articulo eliminado');
          clearArticleCache();
          renderArticleList(container);
        }
      });

      acciones.append(btnEditar, btnEliminar);
      item.append(info, acciones);
      container.appendChild(item);
    }

    if (data.items.length === 0) {
      const vacio = document.createElement('p');
      vacio.className = 'vacio';
      vacio.textContent = 'no hay articulos';
      container.appendChild(vacio);
    }
  } catch {
    container.innerHTML = '<p class="vacio">error al cargar</p>';
  }
}

/* === Editor de artículos === */
export async function openEditor(article?: Article): Promise<void> {
  const { createSelect } = await import('../components/ui/select');

  const container = document.createElement('div');
  container.className = 'flex-columna gap-lg';
  container.style.minHeight = '400px';

  let title = article?.title || '';
  let excerpt = article?.excerpt || '';
  let status = article?.status || 'draft';
  let isPinned = article?.is_pinned || false;

  const titleInput = createInput({ label: 'titulo', placeholder: 'titulo del articulo', value: title, onInput: (v) => { title = v; } });
  const excerptInput = createTextarea({ label: 'extracto', placeholder: 'resumen breve del articulo', value: excerpt, rows: 3, onInput: (v) => { excerpt = v; } });
  const statusSelect = createSelect({ label: 'estado', options: [{ value: 'draft', label: 'borrador' }, { value: 'published', label: 'publicado' }], value: status, onChange: (v) => { status = v as 'draft' | 'published'; } });

  const pinBtn = document.createElement('button');
  pinBtn.className = 'boton';
  pinBtn.textContent = isPinned ? 'fijado ✓' : 'fijar articulo';
  pinBtn.addEventListener('click', () => { isPinned = !isPinned; pinBtn.textContent = isPinned ? 'fijado ✓' : 'fijar articulo'; });

  /* Editor TipTap */
  const editorContainer = document.createElement('div');
  editorContainer.className = 'border-bottom';
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

  /* Toolbar */
  const toolbar = document.createElement('div');
  toolbar.className = 'flex-wrap gap-sm mb-sm border-bottom';
  toolbar.style.padding = 'var(--espacio-sm) 0';

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
    const el = document.createElement('button');
    el.className = 'boton boton-pequeno';
    el.textContent = btn.label;
    el.addEventListener('click', btn.action);
    toolbar.appendChild(el);
  }

  /* Cover image */
  let coverImage = article?.cover_image || '';
  const coverContainer = document.createElement('div');
  coverContainer.className = 'campo';
  const coverLabel = document.createElement('label');
  coverLabel.className = 'campo-etiqueta';
  coverLabel.textContent = 'imagen de portada';
  const coverPreview = document.createElement('img');
  coverPreview.className = 'config-imagen-preview';
  if (!coverImage) coverPreview.classList.add('oculto');
  if (coverImage) coverPreview.src = coverImage;
  const coverBtns = document.createElement('div');
  coverBtns.className = 'flex-fila gap-md';

  const coverBtn = document.createElement('button');
  coverBtn.className = 'boton';
  coverBtn.textContent = coverImage ? 'cambiar portada' : 'subir portada';
  coverBtn.addEventListener('click', async () => {
    try {
      const result = await pickAndUpload('image/*');
      if (result) { coverImage = result.url; coverPreview.src = result.url; coverPreview.classList.remove('oculto'); coverBtn.textContent = 'cambiar portada'; coverQuitar.classList.remove('oculto'); }
    } catch { showToast('error al subir portada'); }
  });

  const coverQuitar = document.createElement('button');
  coverQuitar.className = 'boton';
  coverQuitar.textContent = 'quitar';
  if (!coverImage) coverQuitar.classList.add('oculto');
  coverQuitar.addEventListener('click', () => { coverImage = ''; coverPreview.classList.add('oculto'); coverPreview.src = ''; coverBtn.textContent = 'subir portada'; coverQuitar.classList.add('oculto'); });

  coverBtns.append(coverBtn, coverQuitar);
  coverContainer.append(coverLabel, coverPreview, coverBtns);

  /* Guardar */
  const btnGuardar = document.createElement('button');
  btnGuardar.className = 'boton boton-grande';
  btnGuardar.textContent = article ? 'guardar' : 'crear';
  btnGuardar.addEventListener('click', async () => {
    if (!title.trim()) { showToast('el titulo es obligatorio'); return; }
    const payload = { title, excerpt, content: editor.getJSON(), cover_image: coverImage || undefined, status, is_pinned: isPinned };
    try {
      if (article) { await api.put(`/api/admin/articles/${article.id}`, payload); showToast('articulo actualizado'); }
      else { await api.post('/api/admin/articles', payload); showToast('articulo creado'); }
      clearArticleCache(); modal.close();
      const lista = document.getElementById('admin-articulos');
      if (lista) renderArticleList(lista);
    } catch { showToast('error al guardar'); }
  });

  container.append(titleInput, excerptInput, coverContainer, toolbar, editorContainer, statusSelect, pinBtn, btnGuardar);

  const modal = createModal({ titulo: article ? 'editar articulo' : 'nuevo articulo', contenido: container, ancho: '720px', onClose: () => editor.destroy() });
}
