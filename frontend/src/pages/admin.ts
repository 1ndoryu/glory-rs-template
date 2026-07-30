/* wandori.us — Admin Panel
 * Panel de administracion. Gestion de articulos, proyectos, settings, stats.
 * Solo accesible si autenticado. */

import { api } from '../api/client';
import { authStore, showProfile } from '../store';
import { navigate } from '../router';
import { showToast } from '../components/ui/toast';
import { createModal } from '../components/ui/modal';
import { showConfirm } from '../components/ui/confirm';
import { createInput } from '../components/ui/input';
import { createTextarea } from '../components/ui/textarea';
import { createFontPanel } from '../features/settings/font-panel';
import { pickAndUpload } from '../utils/upload';
import { clearArticleCache } from '../components/layout/sidebar';
import type { Article, PaginatedArticles, Project, AnalyticsStats } from '../api/types';

function formatDate(iso: string): string {
  const d = new Date(iso);
  return `${d.getDate()}/${d.getMonth() + 1}/${d.getFullYear()}`;
}

/* === Articulos === */

async function renderArticleList(container: HTMLElement): Promise<void> {
  container.innerHTML = '<p class="cargando">cargando...</p>';

  try {
    const data = await api.get<PaginatedArticles>('/api/articles?per_page=50');
    container.innerHTML = '';

    for (const article of data.items) {
      const item = document.createElement('div');
      item.className = 'admin-item';

      const info = document.createElement('div');
      const titulo = document.createElement('span');
      titulo.textContent = article.title;
      if (article.status === 'draft') {
        titulo.textContent += ' (borrador)';
      }
      if (article.is_pinned) {
        titulo.textContent += ' · fijado';
      }

      const fecha = document.createElement('small');
      fecha.textContent = ` — ${formatDate(article.created_at)}`;
      fecha.style.marginLeft = 'var(--espacio-sm)';

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
          await api.delete(`/api/articles/${article.id}`);
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

/* === Editor de articulos === */

async function openEditor(article?: Article): Promise<void> {
  const { createSelect } = await import('../components/ui/select');

  const container = document.createElement('div');
  container.style.display = 'flex';
  container.style.flexDirection = 'column';
  container.style.gap = 'var(--espacio-lg)';
  container.style.minHeight = '400px';

  let title = article?.title || '';
  let excerpt = article?.excerpt || '';
  let status = article?.status || 'draft';
  let isPinned = article?.is_pinned || false;

  const titleInput = createInput({
    label: 'titulo',
    placeholder: 'titulo del articulo',
    value: title,
    onInput: (v) => { title = v; },
  });

  const excerptInput = createTextarea({
    label: 'extracto',
    placeholder: 'resumen breve del articulo',
    value: excerpt,
    rows: 3,
    onInput: (v) => { excerpt = v; },
  });

  const statusSelect = createSelect({
    label: 'estado',
    options: [
      { value: 'draft', label: 'borrador' },
      { value: 'published', label: 'publicado' },
    ],
    value: status,
    onChange: (v) => { status = v as 'draft' | 'published'; },
  });

  const pinBtn = document.createElement('button');
  pinBtn.className = 'boton';
  pinBtn.textContent = isPinned ? 'fijado ✓' : 'fijar articulo';
  pinBtn.addEventListener('click', () => {
    isPinned = !isPinned;
    pinBtn.textContent = isPinned ? 'fijado ✓' : 'fijar articulo';
  });

  /* Contenedor para el editor TipTap */
  const editorContainer = document.createElement('div');
  editorContainer.style.border = 'var(--borde)';
  editorContainer.style.minHeight = '300px';
  editorContainer.style.padding = 'var(--espacio-md)';

  /* Cargar TipTap dinamicamente */
  const { Editor } = await import('@tiptap/core');
  const StarterKit = (await import('@tiptap/starter-kit')).default;
  const Image = (await import('@tiptap/extension-image')).default;

  const editor = new Editor({
    element: editorContainer,
    extensions: [
      StarterKit,
      Image.configure({ inline: false }),
    ],
    content: article?.content || { type: 'doc', content: [{ type: 'paragraph' }] },
  });

  /* Toolbar simple */
  const toolbar = document.createElement('div');
  toolbar.style.display = 'flex';
  toolbar.style.gap = 'var(--espacio-sm)';
  toolbar.style.padding = 'var(--espacio-sm) 0';
  toolbar.style.borderBottom = 'var(--borde)';
  toolbar.style.marginBottom = 'var(--espacio-sm)';
  toolbar.style.flexWrap = 'wrap';

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
    { label: 'imagen', action: async () => {
      try {
        const result = await pickAndUpload('image/*', article?.id);
        if (result) editor.chain().focus().setImage({ src: result.url }).run();
      } catch { showToast('error al subir imagen'); }
    }},
    { label: 'audio', action: async () => {
      try {
        const result = await pickAndUpload('audio/*', article?.id);
        if (result) {
          const audioHtml = `<audio controls src="${result.url}"></audio>`;
          editor.chain().focus().insertContent(audioHtml).run();
        }
      } catch { showToast('error al subir audio'); }
    }},
    { label: 'video', action: async () => {
      try {
        const result = await pickAndUpload('video/*', article?.id);
        if (result) {
          const videoHtml = `<video controls src="${result.url}" style="width:100%"></video>`;
          editor.chain().focus().insertContent(videoHtml).run();
        }
      } catch { showToast('error al subir video'); }
    }},
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
  coverPreview.style.display = coverImage ? 'block' : 'none';
  if (coverImage) coverPreview.src = coverImage;
  const coverBtns = document.createElement('div');
  coverBtns.style.display = 'flex';
  coverBtns.style.gap = 'var(--espacio-md)';

  const coverBtn = document.createElement('button');
  coverBtn.className = 'boton';
  coverBtn.textContent = coverImage ? 'cambiar portada' : 'subir portada';
  coverBtn.addEventListener('click', async () => {
    try {
      const result = await pickAndUpload('image/*');
      if (result) {
        coverImage = result.url;
        coverPreview.src = result.url;
        coverPreview.style.display = 'block';
        coverBtn.textContent = 'cambiar portada';
        coverQuitar.style.display = 'inline';
      }
    } catch { showToast('error al subir portada'); }
  });

  const coverQuitar = document.createElement('button');
  coverQuitar.className = 'boton';
  coverQuitar.textContent = 'quitar';
  coverQuitar.style.display = coverImage ? 'inline' : 'none';
  coverQuitar.addEventListener('click', () => {
    coverImage = '';
    coverPreview.style.display = 'none';
    coverPreview.src = '';
    coverBtn.textContent = 'subir portada';
    coverQuitar.style.display = 'none';
  });

  coverBtns.append(coverBtn, coverQuitar);
  coverContainer.append(coverLabel, coverPreview, coverBtns);

  /* Guardar */
  const btnGuardar = document.createElement('button');
  btnGuardar.className = 'boton boton-grande';
  btnGuardar.textContent = article ? 'guardar' : 'crear';

  btnGuardar.addEventListener('click', async () => {
    if (!title.trim()) {
      showToast('el titulo es obligatorio');
      return;
    }

    const payload = {
      title,
      excerpt,
      content: editor.getJSON(),
      cover_image: coverImage || undefined,
      status,
      is_pinned: isPinned,
    };

    try {
      if (article) {
        await api.put(`/api/articles/${article.id}`, payload);
        showToast('articulo actualizado');
      } else {
        await api.post('/api/articles', payload);
        showToast('articulo creado');
      }
      clearArticleCache();
      modal.close();
      /* Refrescar lista si estamos en admin */
      const lista = document.getElementById('admin-articulos');
      if (lista) renderArticleList(lista);
    } catch {
      showToast('error al guardar');
    }
  });

  container.append(
    titleInput,
    excerptInput,
    coverContainer,
    toolbar,
    editorContainer,
    statusSelect,
    pinBtn,
    btnGuardar,
  );

  const modal = createModal({
    titulo: article ? 'editar articulo' : 'nuevo articulo',
    contenido: container,
    ancho: '720px',
    onClose: () => editor.destroy(),
  });
}

/* === Proyectos === */

async function renderProjectList(container: HTMLElement): Promise<void> {
  container.innerHTML = '<p class="cargando">cargando...</p>';

  try {
    const projects = await api.get<Project[]>('/api/projects');
    container.innerHTML = '';

    for (const project of projects) {
      const item = document.createElement('div');
      item.className = 'admin-item';

      const info = document.createElement('span');
      info.textContent = project.title;

      const acciones = document.createElement('div');
      acciones.className = 'admin-acciones';

      const btnEditar = document.createElement('button');
      btnEditar.className = 'boton boton-pequeno';
      btnEditar.textContent = 'editar';
      btnEditar.addEventListener('click', () => openProjectEditor(project, container));

      const btnEliminar = document.createElement('button');
      btnEliminar.className = 'boton boton-pequeno';
      btnEliminar.textContent = 'eliminar';
      btnEliminar.addEventListener('click', async () => {
        const ok = await showConfirm(`eliminar "${project.title}"?`);
        if (ok) {
          await api.delete(`/api/projects/${project.id}`);
          showToast('proyecto eliminado');
          renderProjectList(container);
        }
      });

      acciones.append(btnEditar, btnEliminar);
      item.append(info, acciones);
      container.appendChild(item);
    }

    /* Boton agregar */
    const btnNuevo = document.createElement('button');
    btnNuevo.className = 'boton';
    btnNuevo.textContent = '+ nuevo proyecto';
    btnNuevo.style.marginTop = 'var(--espacio-md)';
    btnNuevo.addEventListener('click', () => openProjectEditor(undefined, container));
    container.appendChild(btnNuevo);
  } catch {
    container.innerHTML = '<p class="vacio">error al cargar</p>';
  }
}

async function openProjectEditor(project: Project | undefined, listContainer: HTMLElement): Promise<void> {
  let title = project?.title || '';
  let description = project?.description || '';
  let url = project?.url || '';

  const container = document.createElement('div');
  container.style.display = 'flex';
  container.style.flexDirection = 'column';
  container.style.gap = 'var(--espacio-lg)';

  const titleInput = createInput({
    label: 'titulo',
    value: title,
    onInput: (v) => { title = v; },
  });

  const descInput = createTextarea({
    label: 'descripcion',
    value: description,
    rows: 3,
    onInput: (v) => { description = v; },
  });

  const urlInput = createInput({
    label: 'url',
    value: url,
    placeholder: 'https://...',
    onInput: (v) => { url = v; },
  });

  const btnGuardar = document.createElement('button');
  btnGuardar.className = 'boton';
  btnGuardar.textContent = project ? 'guardar' : 'crear';

  btnGuardar.addEventListener('click', async () => {
    if (!title.trim()) {
      showToast('el titulo es obligatorio');
      return;
    }
    try {
      if (project) {
        await api.put(`/api/projects/${project.id}`, { title, description, url });
        showToast('proyecto actualizado');
      } else {
        await api.post('/api/projects', { title, description, url });
        showToast('proyecto creado');
      }
      modal.close();
      renderProjectList(listContainer);
    } catch {
      showToast('error al guardar');
    }
  });

  container.append(titleInput, descInput, urlInput, btnGuardar);

  const modal = createModal({
    titulo: project ? 'editar proyecto' : 'nuevo proyecto',
    contenido: container,
    ancho: '480px',
  });
}

/* === Render principal del admin === */

export async function renderAdmin(): Promise<HTMLElement> {
  showProfile.set(false);

  const page = document.createElement('div');
  page.className = 'admin-pagina';

  const header = document.createElement('div');
  header.className = 'admin-header';

  const titulo = document.createElement('h1');
  titulo.textContent = 'admin';

  const btnLogout = document.createElement('button');
  btnLogout.className = 'boton';
  btnLogout.textContent = 'salir';
  btnLogout.addEventListener('click', () => {
    authStore.set({ token: null, isAuthenticated: false });
    showToast('sesion cerrada');
    navigate('/');
  });

  header.append(titulo, btnLogout);
  page.appendChild(header);

  /* Tabs */
  const tabs = document.createElement('div');
  tabs.style.display = 'flex';
  tabs.style.gap = 'var(--espacio-lg)';
  tabs.style.marginBottom = 'var(--espacio-lg)';
  tabs.style.borderBottom = 'var(--borde)';
  tabs.style.paddingBottom = 'var(--espacio-sm)';

  const contentArea = document.createElement('div');
  contentArea.id = 'admin-articulos';

  const tabNames = ['articulos', 'proyectos', 'fuentes', 'sitio', 'estadisticas'];
  function switchTab(name: string): void {
    tabs.querySelectorAll('.boton').forEach(b => {
      (b as HTMLElement).style.fontWeight = b.textContent === name ? 'var(--peso-medio)' : 'var(--peso-normal)';
    });

    contentArea.innerHTML = '';
    contentArea.id = `admin-${name}`;

    switch (name) {
      case 'articulos': {
        const btnNuevo = document.createElement('button');
        btnNuevo.className = 'boton';
        btnNuevo.textContent = '+ nuevo articulo';
        btnNuevo.style.marginBottom = 'var(--espacio-md)';
        btnNuevo.addEventListener('click', () => openEditor());
        contentArea.appendChild(btnNuevo);

        const lista = document.createElement('div');
        lista.className = 'admin-lista';
        contentArea.appendChild(lista);
        renderArticleList(lista);
        break;
      }
      case 'proyectos': {
        const lista = document.createElement('div');
        lista.className = 'admin-lista';
        contentArea.appendChild(lista);
        renderProjectList(lista);
        break;
      }
      case 'fuentes':
        contentArea.appendChild(createFontPanel());
        break;
      case 'sitio': {
        const sitioContainer = document.createElement('div');
        sitioContainer.style.display = 'flex';
        sitioContainer.style.flexDirection = 'column';
        sitioContainer.style.gap = 'var(--espacio-lg)';

        const sitioTitulo = document.createElement('h3');
        sitioTitulo.textContent = 'contenido del sitio';
        sitioContainer.appendChild(sitioTitulo);

        /* About content */
        let aboutContent = '';
        const aboutArea = createTextarea({
          label: 'contenido about (html)',
          placeholder: '<h1>about</h1><p>tu contenido...</p>',
          rows: 8,
          onInput: (v) => { aboutContent = v; },
        });
        sitioContainer.appendChild(aboutArea);

        /* Cargar contenido actual */
        api.get<Record<string, string>>('/api/settings').then(s => {
          aboutContent = s.about_content || '';
          const textarea = aboutArea.querySelector('textarea');
          if (textarea) textarea.value = aboutContent;
        }).catch(() => {});

        const btnGuardarSitio = document.createElement('button');
        btnGuardarSitio.className = 'boton';
        btnGuardarSitio.textContent = 'guardar';
        btnGuardarSitio.addEventListener('click', async () => {
          try {
            await api.post('/api/settings', { settings: { about_content: aboutContent } });
            showToast('contenido actualizado');
          } catch {
            showToast('error al guardar');
          }
        });
        sitioContainer.appendChild(btnGuardarSitio);

        contentArea.appendChild(sitioContainer);
        break;
      }
      case 'estadisticas': {
        const statsContainer = document.createElement('div');
        statsContainer.innerHTML = '<p class="cargando">cargando...</p>';
        contentArea.appendChild(statsContainer);

        api.get<AnalyticsStats>('/api/analytics/stats').then(stats => {
          statsContainer.innerHTML = '';

          const grid = document.createElement('div');
          grid.className = 'stats-grid';

          const metrics = [
            { valor: stats.total_page_views, etiqueta: 'page views' },
            { valor: stats.total_clicks, etiqueta: 'clicks' },
            { valor: stats.total_downloads, etiqueta: 'descargas' },
            { valor: stats.total_purchases, etiqueta: 'compras' },
          ];

          for (const m of metrics) {
            const item = document.createElement('div');
            item.className = 'stats-item';
            const valor = document.createElement('div');
            valor.className = 'stats-valor';
            valor.textContent = String(m.valor);
            const etiqueta = document.createElement('div');
            etiqueta.className = 'stats-etiqueta';
            etiqueta.textContent = m.etiqueta;
            item.append(valor, etiqueta);
            grid.appendChild(item);
          }

          statsContainer.appendChild(grid);

          if (stats.top_articles.length > 0) {
            const tituloTop = document.createElement('h3');
            tituloTop.textContent = 'articulos mas vistos';
            tituloTop.style.marginTop = 'var(--espacio-lg)';
            tituloTop.style.marginBottom = 'var(--espacio-md)';
            statsContainer.appendChild(tituloTop);

            for (const art of stats.top_articles) {
              const item = document.createElement('div');
              item.className = 'admin-item';
              const nombre = document.createElement('span');
              nombre.textContent = art.title;
              const views = document.createElement('span');
              views.textContent = `${art.views} views`;
              item.append(nombre, views);
              statsContainer.appendChild(item);
            }
          }
        }).catch(() => {
          statsContainer.innerHTML = '<p class="vacio">error al cargar estadisticas</p>';
        });
        break;
      }
    }
  }

  for (const name of tabNames) {
    const btn = document.createElement('button');
    btn.className = 'boton';
    btn.textContent = name;
    btn.addEventListener('click', () => switchTab(name));
    tabs.appendChild(btn);
  }

  page.append(tabs, contentArea);
  switchTab('articulos');

  return page;
}
