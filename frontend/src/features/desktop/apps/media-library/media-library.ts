/* wandori.us — Media Library App
 * Biblioteca de media como programa del OS (admin-only).
 * Solo devuelve contenido; el shell crea la ventana y el chrome.
 * [297A-14 F4] Estados de asset visibles, subida validada (backend autoridad),
 * selección, papelera (soft delete) y restauración; object URLs siempre revocadas. */

import { createElement, RotateCcw, Trash2, Link, Upload, Image as ImageIcon, FileText } from 'lucide';
import { createEl } from '../../../../utils/dom';
import { createVacio } from '../../../../components/ui/empty-state';
import { createSelect } from '../../../../components/ui/select';
import { MediaService } from '../../../../services';
import { tryCatch } from '../../../../utils/result';
import { safeClick, safeRun } from '../../../../utils/safe-async';
import { showToast } from '../../../../components/ui/toast';
import { showConfirm } from '../../../../components/ui/confirm';
import {
  assetStateLabel,
  classifyClientType,
  fileNameFromPath,
  formatFileSize,
  getFileExtension,
  isAllowedUpload,
} from './media-library-utils';
import type { Media } from '../../../../api/types';

type MediaFilter = 'all' | 'image' | 'audio' | 'video';

interface MediaLibraryOptions {
  readonly signal: AbortSignal;
}

export interface MediaLibraryView {
  readonly element: HTMLElement;
  readonly destroy: () => void;
}

/** Copiar una URL al portapapeles con fallback. */
async function copyToClipboard(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    /* fallback abajo */
  }
  try {
    const ta = createEl('textarea', { value: text });
    ta.style.position = 'fixed';
    ta.style.opacity = '0';
    document.body.appendChild(ta);
    ta.select();
    const ok = document.execCommand('copy');
    ta.remove();
    return ok;
  } catch {
    return false;
  }
}

function createThumbnail(item: Media): HTMLElement {
  const wrapper = createEl('div', { className: 'media-library__thumb' });
  if (item.file_type === 'image') {
    wrapper.appendChild(createEl('img', {
      className: 'media-library__thumb-img',
      src: item.file_path,
      alt: item.alt_text || fileNameFromPath(item.file_path),
      loading: 'lazy',
    }));
  } else {
    const icon = item.file_type === 'audio' ? FileText : ImageIcon;
    wrapper.appendChild(createEl('span', { className: 'media-library__thumb-icon' }, createElement(icon)));
  }
  return wrapper;
}

function createItemCard(
  item: Media,
  isTrashView: boolean,
  onAction: () => void,
): HTMLElement {
  const name = item.alt_text || fileNameFromPath(item.file_path);
  const label = createEl('span', { className: 'media-library__name', textContent: name, title: name });
  const meta = createEl('div', { className: 'media-library__meta' },
    createEl('span', { textContent: item.file_type }),
    createEl('span', { textContent: formatFileSize(item.file_size) }),
  );
  const badge = createEl('span', {
    className: `media-library__badge media-library__badge--${item.asset_state}`,
    textContent: assetStateLabel(item.asset_state),
  });
  const actions = createEl('div', { className: 'media-library__actions' });

  const copyBtn = createEl('button', { type: 'button', className: 'boton boton-pequeno', ariaLabel: 'Copiar URL' },
    createElement(Link));
  copyBtn.addEventListener('click', safeClick(async () => {
    const ok = await copyToClipboard(item.file_path);
    showToast(ok ? 'URL copiada' : 'no se pudo copiar la URL');
  }));
  actions.appendChild(copyBtn);

  if (isTrashView) {
    const restoreBtn = createEl('button', {
      type: 'button', className: 'boton boton-pequeno', ariaLabel: `Restaurar ${name}`,
    }, createElement(RotateCcw));
    restoreBtn.addEventListener('click', safeClick(async () => {
      const result = await safeRun(MediaService.restore(item.id), 'error al restaurar');
      if (result.ok) {
        showToast('media restaurado');
        onAction();
      }
    }));
    actions.appendChild(restoreBtn);
  } else {
    const deleteBtn = createEl('button', {
      type: 'button', className: 'boton boton-pequeno', ariaLabel: `Eliminar ${name}`,
    }, createElement(Trash2));
    deleteBtn.addEventListener('click', safeClick(async () => {
      const confirmed = await showConfirm(`mover "${name}" a la papelera?`);
      if (!confirmed) return;
      const result = await safeRun(MediaService.delete(item.id), 'error al eliminar');
      if (result.ok) {
        showToast('media movido a la papelera');
        onAction();
      }
    }));
    actions.appendChild(deleteBtn);
  }

  return createEl('div', { className: 'media-library__item' },
    createThumbnail(item), label, meta, badge, actions);
}

export function createMediaLibraryPreview(options: MediaLibraryOptions): MediaLibraryView {
  const container = createEl('div', { className: 'media-library' });
  const pendingObjectUrls = new Set<string>();
  let disposed = false;
  let filter: MediaFilter = 'all';
  let trashView = false;

  const isActive = (): boolean => !disposed && !options.signal.aborted;

  /* === Toolbar de la biblioteca === */
  const filterSelect = createSelect({
    label: 'tipo',
    options: [
      { value: 'all', label: 'todos' },
      { value: 'image', label: 'imágenes' },
      { value: 'audio', label: 'audio' },
      { value: 'video', label: 'video' },
    ],
    value: 'all',
    onChange: (value) => { filter = value as MediaFilter; void render(); },
  });
  const trashToggle = createEl('button', {
    type: 'button', className: 'boton', ariaPressed: 'false', textContent: 'papelera',
  });
  trashToggle.addEventListener('click', () => {
    trashView = !trashView;
    trashToggle.setAttribute('aria-pressed', String(trashView));
    trashToggle.textContent = trashView ? 'biblioteca' : 'papelera';
    void render();
  });
  const uploadBtn = createEl('button', { type: 'button', className: 'boton', textContent: 'subir archivo' },
    createElement(Upload));
  const fileInput = createEl('input', { type: 'file', accept: 'image/*,audio/*,video/*', className: 'oculto' });
  fileInput.addEventListener('change', safeClick(async () => {
    const file = fileInput.files?.[0];
    fileInput.value = '';
    if (!file || !isActive()) return;
    const check = isAllowedUpload(file);
    if (!check.ok) {
      showToast(check.reason || 'archivo no válido');
      return;
    }
    const ext = getFileExtension(file.name);
    if (filter !== 'all' && classifyClientType(ext) !== filter) {
      showToast('el archivo no coincide con el filtro actual');
      return;
    }

    /* Vista previa con object URL; se revoca al terminar o al desmontar. */
    let previewUrl = '';
    if (classifyClientType(ext) === 'image') {
      previewUrl = URL.createObjectURL(file);
      pendingObjectUrls.add(previewUrl);
    }

    const result = await safeRun(MediaService.upload(file), 'error al subir');
    if (previewUrl) {
      URL.revokeObjectURL(previewUrl);
      pendingObjectUrls.delete(previewUrl);
    }
    if (!result.ok) return;
    showToast('archivo subido');
    await render();
  }));
  uploadBtn.addEventListener('click', () => fileInput.click());

  const toolbar = createEl('div', { className: 'media-library__toolbar' },
    uploadBtn, fileInput, filterSelect, trashToggle);
  const list = createEl('div', { className: 'media-library__grid' });
  container.append(toolbar, list);

  /* === Listado con descarte de respuestas obsoletas === */
  let generation = 0;
  async function render(): Promise<void> {
    const current = ++generation;
    list.textContent = '';
    list.appendChild(createEl('p', { className: 'cargando', textContent: 'cargando...' }));
    const result = await tryCatch(
      trashView ? MediaService.listTrashed() : MediaService.listAdmin(),
    );
    if (!isActive() || current !== generation) return;
    if (!result.ok) {
      list.textContent = '';
      list.appendChild(createVacio('error al cargar la biblioteca'));
      return;
    }
    list.textContent = '';
    const items = result.value.filter(item => filter === 'all' || item.file_type === filter);
    if (items.length === 0) {
      list.appendChild(createVacio(
        trashView ? 'la papelera está vacía' : 'no hay archivos en la biblioteca',
      ));
      return;
    }
    for (const item of items) {
      list.appendChild(createItemCard(item, trashView, () => { void render(); }));
    }
  }

  void render();

  const destroy = (): void => {
    if (disposed) return;
    disposed = true;
    for (const url of pendingObjectUrls) URL.revokeObjectURL(url);
    pendingObjectUrls.clear();
  };
  options.signal.addEventListener('abort', destroy, { once: true });

  return { element: container, destroy };
}
