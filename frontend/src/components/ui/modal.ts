/* wandori.us — Modal
 * Modal overlay B&W. Cierra con click fuera o Escape. */

export interface ModalOptions {
  titulo?: string;
  contenido: HTMLElement | HTMLElement[];
  ancho?: string;
  onClose?: () => void;
}

export function createModal(options: ModalOptions): { close: () => void } {
  const { contenido, ancho = '560px', onClose } = options;

  const overlay = document.createElement('div');
  overlay.className = 'modal-overlay';

  const modal = document.createElement('div');
  modal.className = 'modal-contenido';
  modal.style.maxWidth = ancho;

  /* Cuerpo */
  const cuerpo = document.createElement('div');
  cuerpo.className = 'modal-cuerpo';
  if (Array.isArray(contenido)) {
    cuerpo.append(...contenido);
  } else {
    cuerpo.appendChild(contenido);
  }

  modal.appendChild(cuerpo);
  overlay.appendChild(modal);
  document.body.appendChild(overlay);
  document.body.style.overflow = 'hidden';

  const close = () => {
    overlay.remove();
    document.body.style.overflow = '';
    onClose?.();
  };

  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) close();
  });

  const handleEsc = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      document.removeEventListener('keydown', handleEsc);
      close();
    }
  };
  document.addEventListener('keydown', handleEsc);

  return { close };
}
