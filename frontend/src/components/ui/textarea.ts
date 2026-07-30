/* wandori.us — Textarea Component
 * Area de texto minimalista. B&W. */

export interface TextareaOptions {
  label?: string;
  placeholder?: string;
  value?: string;
  rows?: number;
  onInput?: (value: string) => void;
}

export function createTextarea(options: TextareaOptions): HTMLElement {
  const { label, placeholder, value = '', rows = 5, onInput } = options;

  const campo = document.createElement('div');
  campo.className = 'campo';

  if (label) {
    const etiqueta = document.createElement('label');
    etiqueta.className = 'campo-etiqueta';
    etiqueta.textContent = label;
    campo.appendChild(etiqueta);
  }

  const textarea = document.createElement('textarea');
  textarea.className = 'campo-textarea';
  textarea.value = value;
  textarea.rows = rows;
  if (placeholder) textarea.placeholder = placeholder;

  textarea.addEventListener('input', () => {
    onInput?.(textarea.value);
  });

  campo.appendChild(textarea);
  return campo;
}
