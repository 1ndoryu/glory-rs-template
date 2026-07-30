/* wandori.us — Select Component
 * Selector minimalista. B&W sin bordes redondeados. */

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

  const campo = document.createElement('div');
  campo.className = 'campo';

  if (label) {
    const etiqueta = document.createElement('label');
    etiqueta.className = 'campo-etiqueta';
    etiqueta.textContent = label;
    campo.appendChild(etiqueta);
  }

  const select = document.createElement('select');
  select.className = 'campo-select';

  for (const item of items) {
    const opt = document.createElement('option');
    opt.value = item.value;
    opt.textContent = item.label;
    if (item.value === value) opt.selected = true;
    select.appendChild(opt);
  }

  select.addEventListener('change', () => {
    onChange?.(select.value);
  });

  campo.appendChild(select);
  return campo;
}
