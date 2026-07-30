/* wandori.us — Admin Projects
 * Lista y editor de proyectos para el panel de administración. */

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

  try {
    const projects = await ProjectService.listAll();
    container.textContent = '';

    for (const project of projects) {
      const btnEditar = createEl('button', { className: 'boton boton-pequeno', textContent: 'editar' });
      btnEditar.addEventListener('click', () => openProjectEditor(project, container));

      const btnEliminar = createEl('button', { className: 'boton boton-pequeno', textContent: 'eliminar' });
      btnEliminar.addEventListener('click', async () => {
        const ok = await showConfirm(`eliminar "${project.title}"?`);
        if (ok) {
          await ProjectService.delete(project.id);
          showToast('proyecto eliminado');
          renderProjectList(container);
        }
      });

      const acciones = createEl('div', { className: 'admin-acciones' }, btnEditar, btnEliminar);

      container.appendChild(createEl('div', { className: 'admin-item' },
        createEl('span', { textContent: project.title }),
        acciones,
      ));
    }

    const btnNuevo = createEl('button', { className: 'boton mt-md', textContent: '+ nuevo proyecto' });
    btnNuevo.addEventListener('click', () => openProjectEditor(undefined, container));
    container.appendChild(btnNuevo);
  } catch {
    container.textContent = '';
    container.appendChild(createEl('p', { className: 'vacio', textContent: 'error al cargar' }));
  }
}

async function openProjectEditor(project: Project | undefined, listContainer: HTMLElement): Promise<void> {
  let title = project?.title || '';
  let description = project?.description || '';
  let url = project?.url || '';

  const titleInput = createInput({ label: 'titulo', value: title, onInput: (v) => { title = v; } });
  const descInput = createTextarea({ label: 'descripcion', value: description, rows: 3, onInput: (v) => { description = v; } });
  const urlInput = createInput({ label: 'url', value: url, placeholder: 'https://...', onInput: (v) => { url = v; } });

  const btnGuardar = createEl('button', { className: 'boton', textContent: project ? 'guardar' : 'crear' });
  btnGuardar.addEventListener('click', async () => {
    if (!title.trim()) { showToast('el titulo es obligatorio'); return; }
    try {
      if (project) { await ProjectService.update(project.id, { title, description, url }); showToast('proyecto actualizado'); }
      else { await ProjectService.create({ title, description, url }); showToast('proyecto creado'); }
      modal.close();
      renderProjectList(listContainer);
    } catch { showToast('error al guardar'); }
  });

  const container = createEl('div', { className: 'flex-columna gap-lg' },
    titleInput, descInput, urlInput, btnGuardar,
  );

  const modal = createModal({ titulo: project ? 'editar proyecto' : 'nuevo proyecto', contenido: container, ancho: '480px' });
}
