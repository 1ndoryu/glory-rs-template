/* wandori.us — Article Editor App
 * Programa editorial de artículos/About.
 * No crea ventanas ni chrome; devuelve solo contenido para AppRegistry.
 * [297A-14] El editor sale del monolito Admin y recibe articleId interno por params.
 * [297A-14] La ventana monta loading inmediatamente y luego hidrata Tiptap. */

import { ArticleService } from '../../../../services';
import { createInput } from '../../../../components/ui/input';
import { createTextarea } from '../../../../components/ui/textarea';
import { createEl } from '../../../../utils/dom';
import { createVacio } from '../../../../components/ui/empty-state';
import { pickAndUpload } from '../../../../utils/upload';
import { safeClick, safeRun } from '../../../../utils/safe-async';
import { showToast } from '../../../../components/ui/toast';
import { tryCatch } from '../../../../utils/result';
import { createSelect } from '../../../../components/ui/select';
import { publishArticleEditorSaved } from '../../../runtime/article-editor-events';
import type { MountedView, RenderContext } from '../../../../core/lifecycle';
import type { Article } from '../../../../api/types';

type EditorInstance = {
  getJSON: () => unknown;
  chain: () => {
    focus: () => EditorChain;
  };
  destroy: () => void;
};

type EditorChain = {
  toggleBold: () => EditorChain;
  toggleItalic: () => EditorChain;
  toggleCode: () => EditorChain;
  toggleHeading: (options: { level: 2 | 3 }) => EditorChain;
  toggleBulletList: () => EditorChain;
  toggleOrderedList: () => EditorChain;
  toggleBlockquote: () => EditorChain;
  setHorizontalRule: () => EditorChain;
  setImage: (options: { src: string }) => EditorChain;
  insertContent: (content: string) => EditorChain;
  run: () => boolean;
};

function createToolbar(
  editor: EditorInstance,
  getArticleId: () => string | undefined,
  isActive: () => boolean,
): HTMLElement {
  const toolbar = createEl('div', {
    className: 'article-editor__toolbar flex-wrap gap-sm mb-sm border-bottom',
    ariaLabel: 'Herramientas de edición',
  });
  const buttons: Array<{ label: string; action: () => void }> = [
    { label: 'negrita', action: () => { editor.chain().focus().toggleBold().run(); } },
    { label: 'italica', action: () => { editor.chain().focus().toggleItalic().run(); } },
    { label: 'codigo', action: () => { editor.chain().focus().toggleCode().run(); } },
    { label: 'h2', action: () => { editor.chain().focus().toggleHeading({ level: 2 }).run(); } },
    { label: 'h3', action: () => { editor.chain().focus().toggleHeading({ level: 3 }).run(); } },
    { label: 'lista', action: () => { editor.chain().focus().toggleBulletList().run(); } },
    { label: 'lista ordenada', action: () => { editor.chain().focus().toggleOrderedList().run(); } },
    { label: 'cita', action: () => { editor.chain().focus().toggleBlockquote().run(); } },
    { label: 'linea', action: () => { editor.chain().focus().setHorizontalRule().run(); } },
    {
      label: 'imagen',
      action: () => {
        void safeRun(pickAndUpload('image/*', getArticleId()), 'error al subir imagen').then(result => {
          if (isActive() && result.ok && result.value) {
            editor.chain().focus().setImage({ src: result.value.url }).run();
          }
        });
      },
    },
    {
      label: 'audio',
      action: () => {
        void safeRun(pickAndUpload('audio/*', getArticleId()), 'error al subir audio').then(result => {
          if (isActive() && result.ok && result.value) {
            editor.chain().focus().insertContent(`<audio controls src="${result.value.url}"></audio>`).run();
          }
        });
      },
    },
    {
      label: 'video',
      action: () => {
        void safeRun(pickAndUpload('video/*', getArticleId()), 'error al subir video').then(result => {
          if (isActive() && result.ok && result.value) {
            editor.chain().focus().insertContent(`<video controls src="${result.value.url}" style="width:100%"></video>`).run();
          }
        });
      },
    },
  ];

  for (const button of buttons) {
    const element = createEl('button', {
      type: 'button',
      className: 'boton boton-pequeno',
      textContent: button.label,
    });
    element.addEventListener('click', button.action);
    toolbar.appendChild(element);
  }
  return toolbar;
}

function createCoverField(
  article: Article | undefined,
  isActive: () => boolean,
): {
  element: HTMLElement;
  getValue: () => string | undefined;
} {
  let coverImage = article?.cover_image || '';
  const container = createEl('div', { className: 'campo article-editor__cover' });
  const label = createEl('label', { className: 'campo-etiqueta', textContent: 'imagen de portada' });
  const preview = createEl('img', {
    className: `config-imagen-preview${coverImage ? '' : ' oculto'}`,
    alt: 'Vista previa de portada',
  });
  if (coverImage) preview.src = coverImage;

  const removeButton = createEl('button', {
    type: 'button',
    className: `boton${coverImage ? '' : ' oculto'}`,
    textContent: 'quitar',
  });
  const uploadButton = createEl('button', {
    type: 'button',
    className: 'boton',
    textContent: coverImage ? 'cambiar portada' : 'subir portada',
  });
  uploadButton.addEventListener('click', safeClick(async () => {
    const result = await safeRun(pickAndUpload('image/*'), 'error al subir portada');
    if (!isActive() || !result.ok || !result.value) return;
    coverImage = result.value.url;
    preview.src = coverImage;
    preview.classList.remove('oculto');
    uploadButton.textContent = 'cambiar portada';
    removeButton.classList.remove('oculto');
  }));
  removeButton.addEventListener('click', () => {
    if (!isActive()) return;
    coverImage = '';
    preview.src = '';
    preview.classList.add('oculto');
    removeButton.classList.add('oculto');
    uploadButton.textContent = 'subir portada';
  });

  container.append(label, preview, createEl('div', { className: 'flex-fila gap-md' }, uploadButton, removeButton));
  return { element: container, getValue: () => coverImage || undefined };
}

async function loadArticle(ctx: RenderContext): Promise<Article | undefined> {
  const articleId = ctx.params?.articleId;
  if (!articleId) return undefined;
  const result = await tryCatch(ArticleService.getById(articleId));
  if (!result.ok) throw new Error('No se pudo cargar el artículo');
  return result.value;
}

function createLoadingView(): HTMLElement {
  return createEl('div', { className: 'article-editor flex-columna gap-lg' },
    createEl('p', { className: 'cargando', textContent: 'cargando editor...' }),
  );
}

/**
 * Renderizar el editor para una instancia nueva o un artículo existente.
 * El contrato es síncrono para que WindowManager publique la ventana sin
 * esperar red ni chunks; la hidratación vive dentro del lifecycle de la app.
 */
export function renderArticleEditor(ctx: RenderContext): MountedView {
  const container = createLoadingView();
  let editor: EditorInstance | null = null;
  let disposed = false;

  const isActive = (): boolean => !disposed && !ctx.signal.aborted;
  const destroyEditor = (): void => {
    editor?.destroy();
    editor = null;
  };

  const hydrate = async (): Promise<void> => {
    try {
      const article = await loadArticle(ctx);
      if (!isActive()) return;

      const [{ Editor }, StarterKitModule, ImageModule] = await Promise.all([
        import('@tiptap/core'),
        import('@tiptap/starter-kit'),
        import('@tiptap/extension-image'),
      ]);
      if (!isActive()) return;

      let title = article?.title || '';
      let excerpt = article?.excerpt || '';
      let status = article?.status || 'draft';
      let isPinned = article?.is_pinned || false;
      let currentArticleId = article?.id;

      const titleInput = createInput({
        label: 'titulo',
        placeholder: 'titulo del articulo',
        value: title,
        onInput: value => { title = value; },
      });
      const excerptInput = createTextarea({
        label: 'extracto',
        placeholder: 'resumen breve del articulo',
        value: excerpt,
        rows: 3,
        onInput: value => { excerpt = value; },
      });
      const statusSelect = createSelect({
        label: 'estado',
        options: [
          { value: 'draft', label: 'borrador' },
          { value: 'published', label: 'publicado' },
        ],
        value: status,
        onChange: value => { status = value as 'draft' | 'published'; },
      });
      const pinButton = createEl('button', {
        type: 'button',
        className: 'boton',
        textContent: isPinned ? 'fijado ✓' : 'fijar articulo',
      });
      pinButton.addEventListener('click', () => {
        if (!isActive()) return;
        isPinned = !isPinned;
        pinButton.textContent = isPinned ? 'fijado ✓' : 'fijar articulo';
      });

      const editorContainer = createEl('div', {
        className: 'article-editor__content border-bottom',
        ariaLabel: 'Contenido del artículo',
      });
      const cover = createCoverField(article, isActive);
      const StarterKit = StarterKitModule.default;
      const Image = ImageModule.default;

      editor = new Editor({
        element: editorContainer,
        extensions: [StarterKit, Image.configure({ inline: false })],
        content: article?.content || { type: 'doc', content: [{ type: 'paragraph' }] },
      }) as unknown as EditorInstance;

      const toolbar = createToolbar(editor, () => currentArticleId, isActive);
      const saveButton = createEl('button', {
        type: 'button',
        className: 'boton boton-grande',
        textContent: currentArticleId ? 'guardar' : 'crear',
      });
      saveButton.addEventListener('click', safeClick(async () => {
        if (!isActive() || !title.trim() || !editor) {
          if (isActive() && !title.trim()) showToast('el titulo es obligatorio');
          return;
        }
        const payload = {
          title,
          excerpt,
          content: editor.getJSON() as Record<string, unknown>,
          cover_image: cover.getValue(),
          status,
          is_pinned: isPinned,
        };
        const request = currentArticleId
          ? ArticleService.update(currentArticleId, payload)
          : ArticleService.create(payload);
        const result = await safeRun(request, 'error al guardar');
        if (!isActive() || !result.ok) return;
        const operation = currentArticleId ? 'updated' : 'created';
        currentArticleId = result.value.id;
        saveButton.textContent = 'guardar';
        publishArticleEditorSaved({
          articleId: currentArticleId,
          operation,
        });
        showToast(operation === 'updated' ? 'articulo actualizado' : 'articulo creado');
      }));

      container.textContent = '';
      container.append(titleInput, excerptInput, cover.element, toolbar, editorContainer, statusSelect, pinButton, saveButton);
    } catch {
      if (!isActive()) return;
      destroyEditor();
      container.textContent = '';
      container.appendChild(createVacio('error al cargar el editor'));
    }
  };

  void hydrate();

  const abortHandler = (): void => {
    disposed = true;
    destroyEditor();
  };
  ctx.signal.addEventListener('abort', abortHandler, { once: true });

  return {
    element: container,
    destroy: () => {
      disposed = true;
      ctx.signal.removeEventListener('abort', abortHandler);
      destroyEditor();
    },
  };
}
