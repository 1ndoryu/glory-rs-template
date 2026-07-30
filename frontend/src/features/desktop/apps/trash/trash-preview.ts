/* wandori.us — Trash (Papelera) App
 * Muestra nodos tombstoneados del workspace con opción de restaurar.
 * [Plan 297A-11 §9.3] Papelera personal separada de recursos. */

import { createElement, RotateCcw } from 'lucide';
import { getTombstonedNodes, restoreNode, workspaceStore } from '../../../runtime/workspace/workspace-store';
import { reconcileChildren } from '../../../../utils/reconcile';

/* [297A-11] Contenido de la papelera — acciones en menú declarativo (app.menus).
 * Solo renderiza la lista de items tombstoneados. */
export function createTrashPreview(): HTMLElement {
  const container = document.createElement('div');
  container.className = 'trash-app';

  /* Contenedor para la lista reconciliada */
  const list = document.createElement('ul');
  list.className = 'trash-app__list';

  /* Mensaje de vacío (fuera del reconcile) */
  const emptyMsg = document.createElement('div');
  emptyMsg.className = 'trash-app__empty';
  emptyMsg.textContent = 'La papelera está vacía.';
  emptyMsg.style.display = 'none';
  container.append(emptyMsg, list);

  function render(): void {
    const tombstoned = getTombstonedNodes();

    if (tombstoned.length === 0) {
      emptyMsg.style.display = '';
      list.style.display = 'none';
      return;
    }

    emptyMsg.style.display = 'none';
    list.style.display = '';

    reconcileChildren(
      list,
      tombstoned,
      (node) => node.id,
      /* createElement */
      (node) => {
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
        return item;
      },
      /* updateElement */
      (el, node) => {
        const label = el.querySelector('.trash-app__item-label');
        if (label && label.textContent !== node.label) label.textContent = node.label;
        const type = el.querySelector('.trash-app__item-type');
        if (type && type.textContent !== node.type) type.textContent = node.type;
      },
    );
  }

  /* Suscribirse a workspaceStore — subscribe() dispara inmediatamente con valor actual. */
  workspaceStore.subscribe(() => {
    render();
  });

  return container;
}
