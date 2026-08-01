/* wandori.us — Project Editor App
 * Programa editorial de proyectos.
 * Solo devuelve contenido; el shell crea la ventana y el chrome.
 * [297A-14] Extraído del modal Admin con lifecycle propio. */

import { ProjectService } from '../../../../services';
import { createInput } from '../../../../components/ui/input';
import { createTextarea } from '../../../../components/ui/textarea';
import { createSelect } from '../../../../components/ui/select';
import { createEl } from '../../../../utils/dom';
import { safeClick, safeRun } from '../../../../utils/safe-async';
import { showToast } from '../../../../components/ui/toast';
import { tryCatch } from '../../../../utils/result';
import { publishProjectEditorSaved } from '../../../runtime/project-editor-events';
import type { MountedView, RenderContext } from '../../../../core/lifecycle';
import type { Project } from '../../../../api/types';

function createLoadingView(): HTMLElement {
  return createEl('div', { className: 'project-editor flex-columna gap-lg' },
    createEl('p', { className: 'cargando', textContent: 'cargando editor...' }),
  );
}

async function loadProject(ctx: RenderContext): Promise<Project | undefined> {
  const projectId = ctx.params?.projectId;
  if (!projectId) return undefined;
  const result = await tryCatch(ProjectService.getById(projectId));
  if (!result.ok) throw new Error('No se pudo cargar el proyecto');
  return result.value;
}

/** Renderiza un editor de proyecto nuevo o existente como vista del OS. */
export function renderProjectEditor(ctx: RenderContext): MountedView {
  const container = createLoadingView();
  let disposed = false;
  let currentProjectId: string | undefined;

  const isActive = (): boolean => !disposed && !ctx.signal.aborted;

  const hydrate = async (): Promise<void> => {
    try {
      const project = await loadProject(ctx);
      if (!isActive()) return;

      let title = project?.title || '';
      let description = project?.description || '';
      let url = project?.url || '';
      let sortOrder = project?.sort_order ?? 0;
      let isVisible = project?.is_visible ?? false;
      currentProjectId = project?.id;

      const titleInput = createInput({
        label: 'titulo',
        placeholder: 'titulo del proyecto',
        value: title,
        onInput: value => { title = value; },
      });
      const descriptionInput = createTextarea({
        label: 'descripcion',
        placeholder: 'descripcion del proyecto',
        value: description,
        rows: 3,
        onInput: value => { description = value; },
      });
      const urlInput = createInput({
        label: 'url',
        placeholder: 'https://...',
        value: url,
        onInput: value => { url = value; },
      });
      const orderInput = createInput({
        label: 'orden',
        type: 'number',
        value: String(sortOrder),
        onInput: value => {
          const parsed = Number.parseInt(value, 10);
          sortOrder = Number.isFinite(parsed) ? parsed : 0;
        },
      });
      const visibilitySelect = createSelect({
        label: 'visibilidad',
        options: [
          { value: 'visible', label: 'visible' },
          { value: 'hidden', label: 'oculto' },
        ],
        value: isVisible ? 'visible' : 'hidden',
        onChange: value => { isVisible = value === 'visible'; },
      });
      const saveButton = createEl('button', {
        type: 'button',
        className: 'boton boton-grande',
        textContent: currentProjectId ? 'guardar' : 'crear',
      });

      saveButton.addEventListener('click', safeClick(async () => {
        if (!isActive()) return;
        if (!title.trim()) {
          showToast('el titulo es obligatorio');
          return;
        }

        const projectData = {
          title: title.trim(),
          description,
          url: url.trim() || null,
          sort_order: sortOrder,
          is_visible: isVisible,
        };
        const request = currentProjectId
          ? ProjectService.update(currentProjectId, projectData)
          : ProjectService.create({
            title: projectData.title,
            description: projectData.description,
            url: projectData.url || undefined,
            sort_order: projectData.sort_order,
          });
        const result = await safeRun(request, 'error al guardar proyecto');
        if (!isActive() || !result.ok) return;

        const operation = currentProjectId ? 'updated' : 'created';
        currentProjectId = result.value.id;
        saveButton.textContent = 'guardar';
        publishProjectEditorSaved({ projectId: currentProjectId, operation });
        showToast(operation === 'updated' ? 'proyecto actualizado' : 'proyecto creado');
      }));

      container.textContent = '';
      container.append(
        titleInput,
        descriptionInput,
        urlInput,
        orderInput,
        visibilitySelect,
        saveButton,
      );
    } catch {
      if (!isActive()) return;
      container.textContent = '';
      container.appendChild(createEl('p', {
        className: 'vacio',
        textContent: 'error al cargar el editor de proyectos',
      }));
    }
  };

  void hydrate();

  const abortHandler = (): void => {
    disposed = true;
  };
  ctx.signal.addEventListener('abort', abortHandler, { once: true });

  return {
    element: container,
    destroy: () => {
      disposed = true;
      ctx.signal.removeEventListener('abort', abortHandler);
    },
  };
}
