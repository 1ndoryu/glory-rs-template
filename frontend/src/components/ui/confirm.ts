/* wandori.us — Confirm Dialog
 * Dialogo de confirmacion minimalista B&W. */

export function showConfirm(message: string): Promise<boolean> {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';

    const contenido = document.createElement('div');
    contenido.className = 'confirm-contenido';

    const msg = document.createElement('p');
    msg.className = 'confirm-mensaje';
    msg.textContent = message;

    const acciones = document.createElement('div');
    acciones.className = 'confirm-acciones';

    const btnSi = document.createElement('button');
    btnSi.className = 'boton';
    btnSi.textContent = 'confirmar';

    const btnNo = document.createElement('button');
    btnNo.className = 'boton';
    btnNo.textContent = 'cancelar';

    const cleanup = (result: boolean) => {
      overlay.remove();
      resolve(result);
    };

    btnSi.addEventListener('click', () => cleanup(true));
    btnNo.addEventListener('click', () => cleanup(false));
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) cleanup(false);
    });

    document.addEventListener('keydown', function handler(e) {
      if (e.key === 'Escape') {
        document.removeEventListener('keydown', handler);
        cleanup(false);
      }
    });

    acciones.append(btnNo, btnSi);
    contenido.append(msg, acciones);
    overlay.appendChild(contenido);
    document.body.appendChild(overlay);
    btnNo.focus();
  });
}
