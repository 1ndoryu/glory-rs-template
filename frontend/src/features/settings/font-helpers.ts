/* wandori.us — Font Helpers
 * Componentes reutilizables para el panel de configuración:
 * ArrowSelect (selector con flechas + dropdown) y SizeSlider.
 * [Auditoría v4 §1.2] Migrado a createEl(). */

import { GOOGLE_FONTS } from './font-constants';
import { createEl } from '../../utils/dom';

/* === ArrowSelect — selector con flechas ← → y dropdown === */
export function createArrowSelect(
  etiqueta: string,
  currentValue: string,
  onChange: (value: string) => void,
): HTMLElement {
  const label = createEl('label', { className: 'campo-etiqueta', textContent: etiqueta });

  const btnPrev = createEl('button', { className: 'arrow-select-btn', textContent: '←', 'aria-label': 'fuente anterior' });
  const nombre = createEl('span', { className: 'arrow-select-nombre', textContent: currentValue });
  const btnNext = createEl('button', { className: 'arrow-select-btn', textContent: '→', 'aria-label': 'fuente siguiente' });

  nombre.style.fontFamily = `'${currentValue}', system-ui`;

  const row = createEl('div', { className: 'arrow-select' }, btnPrev, nombre, btnNext);

  let idx = GOOGLE_FONTS.indexOf(currentValue);
  if (idx === -1) idx = 0;

  function updateDisplay(): void {
    const font = GOOGLE_FONTS[idx];
    nombre.textContent = font;
    nombre.style.fontFamily = `'${font}', system-ui`;
    onChange(font);
    dropdown.querySelectorAll('.arrow-select-item').forEach((item, i) => {
      item.classList.toggle('activo', i === idx);
    });
  }

  btnPrev.addEventListener('click', () => {
    idx = (idx - 1 + GOOGLE_FONTS.length) % GOOGLE_FONTS.length;
    updateDisplay();
  });

  btnNext.addEventListener('click', () => {
    idx = (idx + 1) % GOOGLE_FONTS.length;
    updateDisplay();
  });

  /* Dropdown de fuentes */
  const dropdown = createEl('div', { className: 'arrow-select-dropdown oculto' });

  for (const font of GOOGLE_FONTS) {
    const item = createEl('button', { className: 'boton arrow-select-item', textContent: font });
    item.style.fontFamily = `'${font}', system-ui`;
    if (font === currentValue) item.classList.add('activo');
    item.addEventListener('click', () => {
      idx = GOOGLE_FONTS.indexOf(font);
      updateDisplay();
      dropdown.classList.add('oculto');
      dropdown.querySelectorAll('.arrow-select-item').forEach(i => i.classList.remove('activo'));
      item.classList.add('activo');
    });
    dropdown.appendChild(item);
  }

  const selectWrapper = createEl('div', { className: 'arrow-select-wrapper' }, row, dropdown);

  nombre.addEventListener('click', () => {
    dropdown.classList.toggle('oculto');
  });

  /* Cerrar dropdown al hacer click fuera */
  const cerrarDropdown = (e: MouseEvent) => {
    if (!selectWrapper.contains(e.target as Node)) {
      dropdown.classList.add('oculto');
    }
  };
  document.addEventListener('click', cerrarDropdown, true);
  const observer = new MutationObserver(() => {
    if (!container.isConnected) {
      document.removeEventListener('click', cerrarDropdown, true);
      observer.disconnect();
    }
  });
  const container = createEl('div', { className: 'campo' }, label, selectWrapper);
  observer.observe(container.parentNode || document.body, { childList: true });

  return container;
}

/* === SizeSlider — slider de tamaño con valor === */
export function createSizeSlider(
  label: string,
  min: number,
  max: number,
  value: number,
  onChange: (v: number) => void,
  suffix = 'px',
  step?: number,
): HTMLElement {
  const etiqueta = createEl('label', { className: 'campo-etiqueta', textContent: label });
  const valor = createEl('span', { className: 'slider-valor', textContent: `${value}${suffix}` });
  const header = createEl('div', { className: 'slider-header' }, etiqueta, valor);

  const input = createEl('input', {
    type: 'range',
    className: 'slider-input',
    min: String(min),
    max: String(max),
  });
  input.value = String(value);
  if (step !== undefined) input.step = String(step);

  input.addEventListener('input', () => {
    const v = Number(input.value);
    valor.textContent = `${v}${suffix}`;
    onChange(v);
  });

  return createEl('div', { className: 'campo' }, header, input);
}
