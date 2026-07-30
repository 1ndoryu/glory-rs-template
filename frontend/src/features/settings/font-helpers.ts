/* wandori.us — Font Helpers
 * Componentes reutilizables para el panel de configuración:
 * ArrowSelect (selector con flechas + dropdown) y SizeSlider. */

import { GOOGLE_FONTS } from './font-constants';

/* === ArrowSelect — selector con flechas ← → y dropdown === */
export function createArrowSelect(
  etiqueta: string,
  currentValue: string,
  onChange: (value: string) => void,
): HTMLElement {
  const container = document.createElement('div');
  container.className = 'campo';

  const label = document.createElement('label');
  label.className = 'campo-etiqueta';
  label.textContent = etiqueta;

  const row = document.createElement('div');
  row.className = 'arrow-select';

  const btnPrev = document.createElement('button');
  btnPrev.className = 'arrow-select-btn';
  btnPrev.textContent = '←';
  btnPrev.setAttribute('aria-label', 'fuente anterior');

  const nombre = document.createElement('span');
  nombre.className = 'arrow-select-nombre';
  nombre.style.fontFamily = `'${currentValue}', system-ui`;
  nombre.textContent = currentValue;

  const btnNext = document.createElement('button');
  btnNext.className = 'arrow-select-btn';
  btnNext.textContent = '→';
  btnNext.setAttribute('aria-label', 'fuente siguiente');

  row.append(btnPrev, nombre, btnNext);

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
  const dropdown = document.createElement('div');
  dropdown.className = 'arrow-select-dropdown oculto';

  for (const font of GOOGLE_FONTS) {
    const item = document.createElement('button');
    item.className = 'boton arrow-select-item';
    item.textContent = font;
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

  const selectWrapper = document.createElement('div');
  selectWrapper.className = 'arrow-select-wrapper';
  selectWrapper.append(row, dropdown);

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
  observer.observe(container.parentNode || document.body, { childList: true });

  container.append(label, selectWrapper);
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
  const container = document.createElement('div');
  container.className = 'campo';

  const etiqueta = document.createElement('label');
  etiqueta.className = 'campo-etiqueta';
  etiqueta.textContent = label;

  const valor = document.createElement('span');
  valor.className = 'slider-valor';
  valor.textContent = `${value}${suffix}`;

  const header = document.createElement('div');
  header.className = 'slider-header';
  header.append(etiqueta, valor);

  const input = document.createElement('input');
  input.type = 'range';
  input.className = 'slider-input';
  input.min = String(min);
  input.max = String(max);
  input.value = String(value);
  if (step !== undefined) input.step = String(step);

  input.addEventListener('input', () => {
    const v = Number(input.value);
    valor.textContent = `${v}${suffix}`;
    onChange(v);
  });

  container.append(header, input);
  return container;
}
