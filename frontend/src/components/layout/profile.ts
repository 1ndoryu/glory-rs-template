/* wandori.us — Profile Component
 * Foto de perfil, nombre y redes sociales.
 * Se renderiza como cabecera de la columna derecha. */

import { createEl } from '../../utils/dom';
import { profileImage, socialLinksStore, redesLayoutStore } from '../../store';
import { reconcileChildren } from '../../utils/reconcile';

export function createProfile(): HTMLElement {
  const profile = createEl('header', { className: 'profile' });

  const foto = createEl('img', { className: 'profile-foto', alt: 'wandorius' });
  const inicial = createEl('div', { className: 'profile-foto profile-foto-fallback', textContent: 'w' });
  inicial.style.display = 'none';

  foto.onerror = () => {
    foto.style.display = 'none';
    inicial.style.display = 'flex';
  };

  profileImage.subscribe((src) => {
    foto.src = src;
    foto.style.display = '';
    inicial.style.display = 'none';
  });

  const nombre = createEl('h1', { className: 'profile-nombre', textContent: 'wandorius' });

  const redes = createEl('div', { className: 'profile-redes' });

  function renderRedes(): void {
    const links = socialLinksStore.get();
    reconcileChildren(
      redes,
      links,
      (link) => link.nombre,
      (link) => {
        return createEl('a', {
          href: link.url, textContent: link.nombre,
          target: '_blank', rel: 'noopener noreferrer',
          'data-external': 'true',
        });
      },
      (el, link) => {
        if (el.getAttribute('href') !== link.url) el.setAttribute('href', link.url);
      },
    );
  }

  socialLinksStore.subscribe(() => renderRedes());
  redesLayoutStore.subscribe((layout) => {
    redes.classList.toggle('profile-redes--stacked', layout === 'stacked');
  });

  profile.append(inicial, foto, nombre, redes);
  return profile;
}
