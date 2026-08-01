/* wandori.us — Notification Store
 * La versión publicada del workspace es la fuente canónica de novedades.
 * El estado leído se guarda por navegador hasta que 297A-13 habilite la
 * sincronización por cuenta; no se inventa una segunda API de publicación. */

import { createStore, type Store } from '../../store';
import { WorkspaceService } from '../../services/workspace.service';

export interface NotificationItem {
  readonly id: string;
  readonly title: string;
  readonly body: string;
  readonly releaseVersion: number;
  readonly publishedAt: string;
  readonly read: boolean;
}

export interface NotificationsState {
  readonly items: readonly NotificationItem[];
  readonly loading: boolean;
  readonly error: string | null;
}

const READ_KEY = 'wandorius:notifications-read';

function readIds(): Set<string> {
  try {
    const value = JSON.parse(localStorage.getItem(READ_KEY) ?? '[]');
    return new Set(Array.isArray(value) ? value.filter((id): id is string => typeof id === 'string') : []);
  } catch {
    return new Set();
  }
}

function persistRead(ids: Set<string>): void {
  localStorage.setItem(READ_KEY, JSON.stringify(Array.from(ids).slice(-100)));
}

export const notificationsStore: Store<NotificationsState> = createStore({
  items: [],
  loading: false,
  error: null,
});

export async function loadNotifications(): Promise<void> {
  notificationsStore.set({ ...notificationsStore.get(), loading: true, error: null }, 'api');
  try {
    const release = await WorkspaceService.getActiveRelease();
    const ids = readIds();
    const items = release ? [{
      id: `workspace-release:${release.version}`,
      title: 'Novedades del escritorio',
      body: `El escritorio público está disponible en la versión ${release.version}.`,
      releaseVersion: release.version,
      publishedAt: release.published_at,
      read: ids.has(`workspace-release:${release.version}`),
    }] : [];
    notificationsStore.set({ items, loading: false, error: null }, 'api');
  } catch {
    notificationsStore.set({ ...notificationsStore.get(), loading: false, error: 'No se pudieron cargar las novedades.' }, 'api');
  }
}

export function markNotificationRead(id: string): void {
  const ids = readIds();
  ids.add(id);
  persistRead(ids);
  notificationsStore.update(state => ({
    ...state,
    items: state.items.map(item => item.id === id ? { ...item, read: true } : item),
  }), 'user');
}

export function unreadNotificationCount(state: NotificationsState = notificationsStore.get()): number {
  return state.items.reduce((count, item) => count + (item.read ? 0 : 1), 0);
}
