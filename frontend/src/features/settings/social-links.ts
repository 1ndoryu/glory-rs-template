/* wandori.us — Social Links Editor
 * Editor de enlaces sociales con guardado debounced. */

import { socialLinksStore, redesLayoutStore, type RedesLayout } from '../../store';
import { SettingsService } from '../../services';
import { createSizeSlider } from './font-helpers';

let socialSaveTimer: ReturnType<typeof setTimeout> | null = null;

function saveSocialLinks(): void {
  const links = socialLinksStore.get();
  SettingsService.save({
      social_links: JSON.stringify(links),
      redes_layout: redesLayoutStore.get(),
  }).catch((err) => {
    console.error('Error guardando enlaces:', err);
  });
}

function debouncedSaveSocial(): void {
  if (socialSaveTimer) clearTimeout(socialSaveTimer);
  socialSaveTimer = setTimeout(saveSocialLinks, 500);
}

type SizeUpdateFn = (key: string, value: number) => void;

/* Renderiza la sección de enlaces sociales completa */
export function renderSocialLinksSection(
  cfg: { redesSize: number; redesGap: number },
  updateSize: SizeUpdateFn,
): HTMLElement[] {
  const enlacesLabel = document.createElement('label');
  enlacesLabel.className = 'campo-etiqueta';
  enlacesLabel.textContent = 'Enlaces';

  const redesSizeSlider = createSizeSlider('Tamaño enlaces', 8, 24, cfg.redesSize, (v) => updateSize('redesSize', v));
  const redesGapSlider = createSizeSlider('Separación enlaces', 0, 20, cfg.redesGap, (v) => updateSize('redesGap', v));

  const enlacesContainer = document.createElement('div');
  enlacesContainer.className = 'enlaces-editor';

  /* Toggle layout inline/stacked */
  const layoutLabel = document.createElement('label');
  layoutLabel.className = 'checkbox-personalizado';
  const layoutCheck = document.createElement('input');
  layoutCheck.type = 'checkbox';
  layoutCheck.checked = redesLayoutStore.get() === 'stacked';
  layoutCheck.addEventListener('change', () => {
    const layout: RedesLayout = layoutCheck.checked ? 'stacked' : 'inline';
    redesLayoutStore.set(layout);
    saveSocialLinks();
  });
  const layoutTexto = document.createElement('span');
  layoutTexto.textContent = 'uno por linea';
  layoutLabel.append(layoutCheck, layoutTexto);

  function renderEnlaces(): void {
    enlacesContainer.innerHTML = '';
    const links = socialLinksStore.get();

    for (let i = 0; i < links.length; i++) {
      const row = document.createElement('div');
      row.className = 'enlace-row';

      const nombreInput = document.createElement('input');
      nombreInput.className = 'campo-entrada enlace-nombre';
      nombreInput.value = links[i].nombre;
      nombreInput.placeholder = 'nombre';
      nombreInput.addEventListener('input', () => {
        socialLinksStore.update(arr => {
          const copy = [...arr];
          copy[i] = { ...copy[i], nombre: nombreInput.value };
          return copy;
        });
        debouncedSaveSocial();
      });

      const urlInput = document.createElement('input');
      urlInput.className = 'campo-entrada enlace-url';
      urlInput.value = links[i].url;
      urlInput.placeholder = 'https://...';
      urlInput.addEventListener('input', () => {
        socialLinksStore.update(arr => {
          const copy = [...arr];
          copy[i] = { ...copy[i], url: urlInput.value };
          return copy;
        });
        debouncedSaveSocial();
      });

      const btnQuitar = document.createElement('button');
      btnQuitar.className = 'boton enlace-quitar';
      btnQuitar.textContent = '×';
      btnQuitar.title = 'quitar enlace';
      btnQuitar.addEventListener('click', () => {
        socialLinksStore.update(arr => arr.filter((_, idx) => idx !== i));
        renderEnlaces();
        saveSocialLinks();
      });

      row.append(nombreInput, urlInput, btnQuitar);
      enlacesContainer.appendChild(row);
    }

    const btnAgregar = document.createElement('button');
    btnAgregar.className = 'boton mt-sm';
    btnAgregar.textContent = '+ agregar enlace';
    btnAgregar.addEventListener('click', () => {
      socialLinksStore.update(arr => [...arr, { nombre: '', url: '' }]);
      renderEnlaces();
    });
    enlacesContainer.appendChild(btnAgregar);
  }

  renderEnlaces();

  return [enlacesLabel, redesSizeSlider, redesGapSlider, layoutLabel, enlacesContainer];
}
