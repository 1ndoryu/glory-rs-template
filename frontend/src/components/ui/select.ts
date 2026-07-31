/* wandori.us — Select Component
 * Selector minimalista. B&W sin bordes redondeados. */

import { createEl } from '../../utils/dom';

export interface SelectOption {
  value: string;
  label: string;
}

export interface SelectOptions {
  label?: string;
  options: SelectOption[];
  value?: string;
  onChange?: (value: string) => void;
}

export function createSelect(options: SelectOptions): HTMLElement {
  const { label, options: items, value, onChange } = options;

  const children: (string | HTMLElement)[] = [];

  if (label) {
    children.push(createEl('label', { className: 'campo-etiqueta', textContent: label }));
  }

  const select = createEl('select', {
    className: 'campo-select',
    'data-transient': 'true',
  });

  for (const item of items) {
    const opt = createEl('option', { value: item.value, textContent: item.label });
    if (item.value === value) opt.selected = true;
    select.appendChild(opt);
  }

  select.addEventListener('change', () => { onChange?.(select.value); });

  children.push(select);

  return createEl('div', { className: 'campo' }, ...children);
}
