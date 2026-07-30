/* wandori.us — About Page
 * Pagina "about me" con contenido editable desde settings. */

import { api } from '../api/client';
import { updateMeta, setPageJsonLd } from '../features/seo/meta';
import { showProfile } from '../store';
import { appendSanitizedHtml } from '../utils/sanitize-html';

export async function renderAbout(): Promise<HTMLElement> {
  showProfile.set(true);
  updateMeta({ title: 'about', description: 'sobre wandorius — diseño web, software, musica, escritura.' });
  setPageJsonLd('about', 'sobre wandorius — diseño web, software, musica, escritura.');

  const page = document.createElement('div');
  page.className = 'about-contenido';

  try {
    const settings = await api.get<Record<string, string>>('/api/settings');
    const content = settings.about_content || '';

    if (content) {
      appendSanitizedHtml(page, content);
    } else {
      /* API sin contenido — mostrar texto de ejemplo */
      const titulo = document.createElement('h1');
      titulo.textContent = 'about';

      const texto1 = document.createElement('p');
      texto1.textContent = 'soy wandorius. hago cosas con codigo y con palabras, aunque a veces no se cual de las dos es mas dificil.';

      const texto2 = document.createElement('p');
      texto2.textContent = 'me interesan los espacios entre las cosas: el silencio entre dos notas, el espacio en blanco entre dos lineas de codigo, el momento exacto en que una idea deja de ser tuya y empieza a ser de todos.';

      const texto3 = document.createElement('p');
      texto3.textContent = 'diseño web, software, musica, escritura. no me gusta definirme pero si tuviera que elegir una palabra seria: curioso.';

      page.append(titulo, texto1, texto2, texto3);
    }
  } catch {
    /* API no disponible — mostrar texto de ejemplo */
    const titulo = document.createElement('h1');
    titulo.textContent = 'about';

    const texto1 = document.createElement('p');
    texto1.textContent = 'soy wandorius. hago cosas con codigo y con palabras, aunque a veces no se cual de las dos es mas dificil.';

    const texto2 = document.createElement('p');
    texto2.textContent = 'me interesan los espacios entre las cosas: el silencio entre dos notas, el espacio en blanco entre dos lineas de codigo, el momento exacto en que una idea deja de ser tuya y empieza a ser de todos.';

    const texto3 = document.createElement('p');
    texto3.textContent = 'diseño web, software, musica, escritura. no me gusta definirme pero si tuviera que elegir una palabra seria: curioso.';

    page.append(titulo, texto1, texto2, texto3);
  }

  return page;
}
