/* wandori.us — Admin Projects
 * Lista y editor de proyectos para el panel de administración. */

import { ProjectService } from '../services';
import { showToast } from '../components/ui/toast';
import { createModal } from '../components/ui/modal';
import { showConfirm } from '../components/ui/confirm';
import { createInput } from '../components/ui/input';
import { createTextarea } from '../components/ui/textarea';
import type { Project } from '../api/types';

/* === Lista de proyectos === */
export async function renderProjectList(container: HTMLElement): Promise<void> {
  container.textContent = '';
  const loading = document.createElement('p');
  loading.className = 'cargando';
  loading.textContent = 'cargando...';
  container.appendChild(loading);

  try {
    const projects = await ProjectService.listAll();
    container.textContent = '';

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
          await ProjectService.delete(project.id);
          showToast('proyecto eliminado');
          renderProjectList(container);
        }
      });

      acciones.append(btnEditar, btnEliminar);
      item.append(info, acciones);
      container.appendChild(item);
    }

    const btnNuevo = document.createElement('button');
    btnNuevo.className = 'boton mt-md';
    btnNuevo.textContent = '+ nuevo proyecto';
    btnNuevo.addEventListener('click', () => openProjectEditor(undefined, container));
    container.appendChild(btnNuevo);
  } catch {
    container.textContent = '';
    const error = document.createElement('p');
    error.className = 'vacio';
    error.textContent = 'error al cargar';
    container.appendChild(error);
  }
}

/* === Editor de proyectos === */
async function openProjectEditor(project: Project | undefined, listContainer: HTMLElement): Promise<void> {
  let title = project?.title || '';
  let description = project?.description || '';
  let url = project?.url || '';

  const container = document.createElement('div');
  container.className = 'flex-columna gap-lg';

  const titleInput = createInput({ label: 'titulo', value: title, onInput: (v) => { title = v; } });
  const descInput = createTextarea({ label: 'descripcion', value: description, rows: 3, onInput: (v) => { description = v; } });
  const urlInput = createInput({ label: 'url', value: url, placeholder: 'https://...', onInput: (v) => { url = v; } });

  const btnGuardar = document.createElement('button');
  btnGuardar.className = 'boton';
  btnGuardar.textContent = project ? 'guardar' : 'crear';
  btnGuardar.addEventListener('click', async () => {
    if (!title.trim()) { showToast('el titulo es obligatorio'); return; }
    try {
      if (project) { await ProjectService.update(project.id, { title, description, url }); showToast('proyecto actualizado'); }
      else { await ProjectService.create({ title, description, url }); showToast('proyecto creado'); }
      modal.close();
      renderProjectList(listContainer);
    } catch { showToast('error al guardar'); }
  });

  container.append(titleInput, descInput, urlInput, btnGuardar);

  const modal = createModal({ titulo: project ? 'editar proyecto' : 'nuevo proyecto', contenido: container, ancho: '480px' });
}
