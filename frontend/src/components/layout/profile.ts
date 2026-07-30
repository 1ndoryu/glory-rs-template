/* wandori.us — Profile Component
 * Foto de perfil, nombre y redes sociales.
 * Se renderiza como cabecera de la columna derecha. */

import { profileImage, socialLinksStore, redesLayoutStore } from '../../store';

export function createProfile(): HTMLElement {
  const profile = document.createElement('header');
  profile.className = 'profile';

  /* Foto de perfil */
  const foto = document.createElement('img');
  foto.className = 'profile-foto';
  foto.alt = 'wandorius';
  const inicial = document.createElement('div');
  inicial.className = 'profile-foto profile-foto-fallback';
  inicial.textContent = 'w';
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

  /* Nombre */
  const nombre = document.createElement('h1');
  nombre.className = 'profile-nombre';
  nombre.textContent = 'wandorius';

  /* Redes sociales — reactivas al store */
  const redes = document.createElement('div');
  redes.className = 'profile-redes';

  function renderRedes(): void {
    redes.innerHTML = '';
    for (const link of socialLinksStore.get()) {
      const a = document.createElement('a');
      a.href = link.url;
      a.textContent = link.nombre;
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
      a.setAttribute('data-external', 'true');
      redes.appendChild(a);
    }
  }

  socialLinksStore.subscribe(() => renderRedes());
  redesLayoutStore.subscribe((layout) => {
    redes.classList.toggle('profile-redes--stacked', layout === 'stacked');
  });

  profile.append(inicial, foto, nombre, redes);
  return profile;
}
