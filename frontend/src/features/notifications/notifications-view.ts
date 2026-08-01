/* wandori.us — Notifications app
 * AppRegistry monta esta vista tanto en ventana como en pantalla completa;
 * la app no crea chrome ni decide cómo se presenta. */

import { Bell, createElement, RefreshCw } from 'lucide';
import { createEl } from '../../utils/dom';
import { createVacio } from '../../components/ui/empty-state';
import {
  loadNotifications,
  markNotificationRead,
  notificationsStore,
  type NotificationsState,
} from './notifications-store';

export function createNotificationsView(): { element: HTMLElement; destroy: () => void } {
  const root = createEl('section', { className: 'notificaciones', ariaLabel: 'Novedades' });
  const title = createEl('h2', { className: 'notificaciones__titulo', textContent: 'Novedades' });
  const list = createEl('div', { className: 'notificaciones__lista', role: 'list' });
  const reload = createEl('button', {
    type: 'button', className: 'boton notificaciones__recargar', ariaLabel: 'Recargar novedades',
  }, createElement(RefreshCw), createEl('span', { textContent: 'Recargar' }));
  reload.addEventListener('click', () => { void loadNotifications(); });
  root.append(title, reload, list);

  const render = (state: NotificationsState): void => {
    list.replaceChildren();
    if (state.loading) {
      list.appendChild(createVacio('Cargando novedades…'));
      return;
    }
    if (state.error) {
      list.appendChild(createVacio(state.error));
      return;
    }
    if (state.items.length === 0) {
      list.appendChild(createVacio('No hay novedades.'));
      return;
    }
    for (const item of state.items) {
      const entry = createEl('article', {
        className: `notificaciones__item${item.read ? ' notificaciones__item--leida' : ''}`,
        role: 'listitem',
      },
      createEl('span', { className: 'notificaciones__icono', ariaHidden: 'true' }, createElement(Bell)),
      createEl('div', { className: 'notificaciones__contenido' },
        createEl('h3', { className: 'notificaciones__itemTitulo', textContent: item.title }),
        createEl('p', { className: 'notificaciones__itemTexto', textContent: item.body }),
      ));
      if (!item.read) {
        entry.tabIndex = 0;
        entry.addEventListener('click', () => markNotificationRead(item.id));
        entry.addEventListener('keydown', event => {
          if (event.key === 'Enter' || event.key === ' ') markNotificationRead(item.id);
        });
      }
      list.appendChild(entry);
    }
  };

  const stop = notificationsStore.subscribeSimple(render);
  void loadNotifications();
  return { element: root, destroy: stop };
}
