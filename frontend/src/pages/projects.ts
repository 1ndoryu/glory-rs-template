/* wandori.us — Projects Page
 * Lista minimalista de proyectos con links.
 * [Auditoría v4 §1.2] Migrado a createEl(). */

import { ProjectService } from '../services';
import { updateMeta, setPageJsonLd } from '../features/seo/meta';
import { showProfile } from '../store';
import { createEl, createExternalLink } from '../utils/dom';
import { tryCatch } from '../utils/result';

export async function renderProjects(): Promise<HTMLElement> {
  showProfile.set(true);
  updateMeta({ title: 'proyectos', description: 'proyectos y trabajo de wandorius' });
  setPageJsonLd('proyectos', 'proyectos y trabajo de wandorius');

  const page = createEl('div');
  const titulo = createEl('h1', { textContent: 'proyectos' });
  const cargando = createEl('p', { className: 'cargando', textContent: 'cargando...' });

  page.append(titulo, cargando);

  const projectsResult = await tryCatch(ProjectService.list());
  if (!projectsResult.ok) {
    /* API no disponible — mostrar proyectos de ejemplo */
    page.innerHTML = '';
    page.appendChild(titulo);

    const demoProjects = [
      { title: 'wandori.us', description: 'este sitio. blog/portfolio minimalista construido con rust y vanilla ts.', url: 'https://wandori.us' },
      { title: 'glory-sentinel', description: 'extension vscode para deteccion de violaciones de diseno en tiempo real.', url: 'https://github.com/1ndoryu/glory-sentinel' },
      { title: 'coolify-manager-rs', description: 'cli en rust para gestionar deploys en coolify via api.', url: 'https://github.com/1ndoryu/coolify-manager-rs' },
    ];

    const lista = createEl('div');
    for (const project of demoProjects) {
      const info = createEl('div', {},
        createEl('span', { className: 'proyecto-titulo', textContent: project.title }),
        createEl('p', { className: 'proyecto-descripcion', textContent: project.description }),
      );
      const item = createEl('div', { className: 'proyecto-item' }, info);
      if (project.url) {
        item.appendChild(createExternalLink(project.url, 'ver', 'proyecto-link'));
      }
      lista.appendChild(item);
    }
    page.appendChild(lista);
    return page;
  }

  const projects = projectsResult.value;
  page.innerHTML = '';
  page.appendChild(titulo);

  const visibles = projects.filter(p => p.is_visible).sort((a, b) => a.sort_order - b.sort_order);

  if (visibles.length === 0) {
    page.appendChild(createEl('p', { className: 'vacio', textContent: 'no hay proyectos todavia' }));
    return page;
  }

  const lista = createEl('div');

  for (const project of visibles) {
    const info = createEl('div', {},
      createEl('span', { className: 'proyecto-titulo', textContent: project.title }),
    );

    if (project.description) {
      info.appendChild(createEl('p', { className: 'proyecto-descripcion', textContent: project.description }));
    }

    const item = createEl('div', { className: 'proyecto-item' }, info);

    if (project.url) {
      item.appendChild(createExternalLink(project.url, 'ver', 'proyecto-link'));
    }

    lista.appendChild(item);
  }

  page.appendChild(lista);

  return page;
}
