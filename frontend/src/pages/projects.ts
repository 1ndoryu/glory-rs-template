/* wandori.us — Projects Page
 * Lista minimalista de proyectos con links. */

import { ProjectService } from '../services';
import { updateMeta, setPageJsonLd } from '../features/seo/meta';
import { showProfile } from '../store';

export async function renderProjects(): Promise<HTMLElement> {
  showProfile.set(true);
  updateMeta({ title: 'proyectos', description: 'proyectos y trabajo de wandorius' });
  setPageJsonLd('proyectos', 'proyectos y trabajo de wandorius');

  const page = document.createElement('div');

  const titulo = document.createElement('h1');
  titulo.textContent = 'proyectos';
  titulo.style.marginBottom = 'var(--espacio-xl)';

  const cargando = document.createElement('p');
  cargando.className = 'cargando';
  cargando.textContent = 'cargando...';

  page.append(titulo, cargando);

  try {
    const projects = await ProjectService.list();
    page.innerHTML = '';
    page.appendChild(titulo);

    const visibles = projects.filter(p => p.is_visible).sort((a, b) => a.sort_order - b.sort_order);

    if (visibles.length === 0) {
      const vacio = document.createElement('p');
      vacio.className = 'vacio';
      vacio.textContent = 'no hay proyectos todavia';
      page.appendChild(vacio);
      return page;
    }

    const lista = document.createElement('div');

    for (const project of visibles) {
      const item = document.createElement('div');
      item.className = 'proyecto-item';

      const info = document.createElement('div');

      const nombre = document.createElement('span');
      nombre.className = 'proyecto-titulo';
      nombre.textContent = project.title;

      info.appendChild(nombre);

      if (project.description) {
        const desc = document.createElement('p');
        desc.className = 'proyecto-descripcion';
        desc.textContent = project.description;
        info.appendChild(desc);
      }

      item.appendChild(info);

      if (project.url) {
        const link = document.createElement('a');
        link.className = 'proyecto-link';
        link.href = project.url;
        link.textContent = 'ver';
        link.target = '_blank';
        link.rel = 'noopener noreferrer';
        link.setAttribute('data-external', 'true');
        item.appendChild(link);
      }

      lista.appendChild(item);
    }

    page.appendChild(lista);
  } catch {
    /* API no disponible — mostrar proyectos de ejemplo */
    page.innerHTML = '';
    page.appendChild(titulo);

    const demoProjects = [
      { title: 'wandori.us', description: 'este sitio. blog/portfolio minimalista construido con rust y vanilla ts.', url: 'https://wandori.us' },
      { title: 'glory-sentinel', description: 'extension vscode para deteccion de violaciones de diseño en tiempo real.', url: 'https://github.com/1ndoryu/glory-sentinel' },
      { title: 'coolify-manager-rs', description: 'cli en rust para gestionar deploys en coolify via api.', url: 'https://github.com/1ndoryu/coolify-manager-rs' },
    ];

    const lista = document.createElement('div');
    for (const project of demoProjects) {
      const item = document.createElement('div');
      item.className = 'proyecto-item';

      const info = document.createElement('div');
      const nombre = document.createElement('span');
      nombre.className = 'proyecto-titulo';
      nombre.textContent = project.title;
      const desc = document.createElement('p');
      desc.className = 'proyecto-descripcion';
      desc.textContent = project.description;
      info.append(nombre, desc);

      item.appendChild(info);

      if (project.url) {
        const link = document.createElement('a');
        link.className = 'proyecto-link';
        link.href = project.url;
        link.textContent = 'ver';
        link.target = '_blank';
        link.rel = 'noopener noreferrer';
        link.setAttribute('data-external', 'true');
        item.appendChild(link);
      }

      lista.appendChild(item);
    }
    page.appendChild(lista);
  }

  return page;
}
