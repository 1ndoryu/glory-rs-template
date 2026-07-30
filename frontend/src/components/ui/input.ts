/* wandori.us — Input Component
 * Campo de entrada minimalista. Solo borde inferior 1px. */

export interface InputOptions {
  label?: string;
  type?: string;
  placeholder?: string;
  value?: string;
  required?: boolean;
  error?: string;
  onInput?: (value: string) => void;
}

export function createInput(options: InputOptions): HTMLElement {
  const { label, type = 'text', placeholder, value = '', required, error, onInput } = options;

  const campo = document.createElement('div');
  campo.className = 'campo' + (error ? ' campo-error' : '');

  if (label) {
    const etiqueta = document.createElement('label');
    etiqueta.className = 'campo-etiqueta';
    etiqueta.textContent = label;
    campo.appendChild(etiqueta);
  }

  const entrada = document.createElement('input');
  entrada.className = 'campo-entrada';
  entrada.type = type;
  entrada.value = value;
  if (placeholder) entrada.placeholder = placeholder;
  if (required) entrada.required = true;

  entrada.addEventListener('input', () => {
    onInput?.(entrada.value);
  });

  campo.appendChild(entrada);

  if (error) {
    const msg = document.createElement('span');
    msg.className = 'campo-mensaje-error';
    msg.textContent = error;
    campo.appendChild(msg);
  }

  return campo;
}
