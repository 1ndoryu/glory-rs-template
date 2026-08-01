/* wandori.us — Segmented Control
 * Receta de toolbar de contenido (Mac clásico): grupo de opciones pequeñas
 * con el estado activo invertido (fondo negro, texto claro). Para filtros y
 * modos de vista dentro de una toolbar de app. NO es un formulario: sustituye
 * a `.campo`/`.campo-select` y a los toggles con borde de superficie en ese
 * contexto (regla 018A-68).
 * El componente gestiona su propio estado activo; el consumidor solo recibe
 * onChange. El nombre accesible del grupo lo aporta el consumidor (ariaLabel). */

import { createEl } from '../../utils/dom';

export interface SegmentedOption {
  readonly value: string;
  readonly label: string;
}

export interface SegmentedOptions {
  readonly ariaLabel: string;
  readonly options: readonly SegmentedOption[];
  readonly value: string;
  readonly onChange: (value: string) => void;
}

export function createSegmentedControl(options: SegmentedOptions): HTMLElement {
  const { ariaLabel, options: items, value, onChange } = options;

  const group = createEl('div', {
    className: 'control-segmentado',
    role: 'group',
    ariaLabel,
  });

  const setActive = (next: string): void => {
    for (const button of group.querySelectorAll<HTMLButtonElement>('.control-segmentado__opcion')) {
      const active = button.dataset.value === next;
      button.classList.toggle('control-segmentado__opcion--activa', active);
      button.setAttribute('aria-pressed', String(active));
    }
  };

  for (const item of items) {
    const active = item.value === value;
    const button = createEl('button', {
      type: 'button',
      className: `control-segmentado__opcion${active ? ' control-segmentado__opcion--activa' : ''}`,
      'data-value': item.value,
      ariaPressed: String(active),
      textContent: item.label,
    });
    button.addEventListener('click', () => {
      if (button.getAttribute('aria-pressed') === 'true') return;
      setActive(item.value);
      onChange(item.value);
    });
    group.appendChild(button);
  }

  return group;
}
