/* wandori.us — Trash (Papelera) App
 * Muestra nodos tombstoneados del workspace con opción de restaurar.
 * [Plan 297A-11 §9.3] Papelera personal separada de recursos. */

import { createElement, RotateCcw } from 'lucide';
import { getTombstonedNodes, restoreNode, workspaceStore } from '../../../runtime/workspace/workspace-store';

/* [297A-11] Contenido de la papelera — acciones en menú declarativo (app.menus).
 * Solo renderiza la lista de items tombstoneados. */
export function createTrashPreview(): HTMLElement {
  const container = document.createElement('div');
  container.className = 'trash-app';

  function render(): void {
    container.innerHTML = '';

    const tombstoned = getTombstonedNodes();

    if (tombstoned.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'trash-app__empty';
      empty.textContent = 'La papelera está vacía.';
      container.appendChild(empty);
      return;
    }

    const list = document.createElement('ul');
    list.className = 'trash-app__list';

    for (const node of tombstoned) {
      const item = document.createElement('li');
      item.className = 'trash-app__item';

      const label = document.createElement('span');
      label.className = 'trash-app__item-label';
      label.textContent = node.label;

      const type = document.createElement('span');
      type.className = 'trash-app__item-type';
      type.textContent = node.type;

      const restoreBtn = document.createElement('button');
      restoreBtn.type = 'button';
      restoreBtn.className = 'trash-app__restore-btn';
      restoreBtn.setAttribute('aria-label', `Restaurar ${node.label}`);
      restoreBtn.appendChild(createElement(RotateCcw));
      restoreBtn.addEventListener('click', () => {
        restoreNode(node.id);
      });

      item.append(label, type, restoreBtn);
      list.appendChild(item);
    }

    container.appendChild(list);
  }

  /* Suscribirse a workspaceStore — subscribe() dispara inmediatamente con valor actual. */
  workspaceStore.subscribe(() => {
    render();
  });

  return container;
}
