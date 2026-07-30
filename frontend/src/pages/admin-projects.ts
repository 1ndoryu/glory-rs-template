/* wandori.us — Admin Projects
 * Lista y editor de proyectos para el panel de administración. */

import { safeRun, safeClick } from '../utils/safe-async';
import { tryCatch } from '../utils/result';
import { createEl } from '../utils/dom';
import { ProjectService } from '../services';
import { showToast } from '../components/ui/toast';
import { createModal } from '../components/ui/modal';
import { showConfirm } from '../components/ui/confirm';
import { createInput } from '../components/ui/input';
import { createTextarea } from '../components/ui/textarea';
import type { Project } from '../api/types';

export async function renderProjectList(container: HTMLElement): Promise<void> {
  container.textContent = '';
  container.appendChild(createEl('p', { className: 'cargando', textContent: 'cargando...' }));

  const listResult = await tryCatch(ProjectService.listAll());
  if (!listResult.ok) {
    container.textContent = '';
    container.appendChild(createEl('p', { className: 'vacio', textContent: 'error al cargar' }));
    return;
  }
  const projects = listResult.value;
  container.textContent = '';

  for (const project of projects) {
    const btnEditar = createEl('button', { className: 'boton boton-pequeno', textContent: 'editar' });
    btnEditar.addEventListener('click', () => openProjectEditor(project, container));

    const btnEliminar = createEl('button', { className: 'boton boton-pequeno', textContent: 'eliminar' });
    btnEliminar.addEventListener('click', safeClick(async () => {
      const ok = await showConfirm(`eliminar "${project.title}"?`);
      if (ok) {
        const result = await safeRun(ProjectService.delete(project.id), 'error al eliminar');
        if (result.ok) {
          showToast('proyecto eliminado');
          renderProjectList(container);
        }
      }
    }));

    const acciones = createEl('div', { className: 'admin-acciones' }, btnEditar, btnEliminar);

    container.appendChild(createEl('div', { className: 'admin-item' },
      createEl('span', { textContent: project.title }),
      acciones,
    ));
  }

  const btnNuevo = createEl('button', { className: 'boton mt-md', textContent: '+ nuevo proyecto' });
  btnNuevo.addEventListener('click', () => openProjectEditor(undefined, container));
  container.appendChild(btnNuevo);
}

async function openProjectEditor(project: Project | undefined, listContainer: HTMLElement): Promise<void> {
  let title = project?.title || '';
  let description = project?.description || '';
  let url = project?.url || '';

  const titleInput = createInput({ label: 'titulo', value: title, onInput: (v) => { title = v; } });
  const descInput = createTextarea({ label: 'descripcion', value: description, rows: 3, onInput: (v) => { description = v; } });
  const urlInput = createInput({ label: 'url', value: url, placeholder: 'https://...', onInput: (v) => { url = v; } });

  const btnGuardar = createEl('button', { className: 'boton', textContent: project ? 'guardar' : 'crear' });
  btnGuardar.addEventListener('click', safeClick(async () => {
    if (!title.trim()) { showToast('el titulo es obligatorio'); return; }
    const fn = project
      ? ProjectService.update(project.id, { title, description, url })
      : ProjectService.create({ title, description, url });
    const result = await safeRun(fn, 'error al guardar');
    if (result.ok) {
      showToast(project ? 'proyecto actualizado' : 'proyecto creado');
      modal.close();
      renderProjectList(listContainer);
    }
  }));

  const container = createEl('div', { className: 'flex-columna gap-lg' },
    titleInput, descInput, urlInput, btnGuardar,
  );

  const modal = createModal({ titulo: project ? 'editar proyecto' : 'nuevo proyecto', contenido: container, ancho: '480px' });
}
