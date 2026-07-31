/* wandori.us — Focused Window URL Sync
 * Proyecta el foco actual del OS sobre la URL canónica.
 * El router sigue siendo la única fuente de navegación; este módulo solo
 * reemplaza la URL cuando cambia la presentación enfocada.
 */

import { getCurrentPath, replacePath } from '../../router';
import { getPresentationMode } from '../../utils/viewport';
import { mobileStackStore, type MobileStackEntry } from '../mobile/mobile-stack';
import { AppRegistry } from './app-registry';
import { getCanonicalAppPath } from './deep-links';
import { windowStore, type WindowEntry } from './window-store';
import type { StoreSource } from '../../store';

export interface FocusedEntrySnapshot {
  readonly appId: string;
  readonly focused?: boolean;
  readonly params?: Readonly<Record<string, string>>;
}

/** Resolver puro del path que representa la app actualmente enfocada. */
export function resolveFocusedPath(
  windows: readonly FocusedEntrySnapshot[],
  mobileStack: readonly FocusedEntrySnapshot[],
  presentation: 'desktop' | 'tablet' | 'mobile',
): string {
  const active = presentation === 'mobile'
    ? mobileStack.at(-1)
    : windows.find((entry) => entry.focused);
  if (!active) return '/';

  const app = AppRegistry.get(active.appId);
  return app ? getCanonicalAppPath(app, active.params) ?? '/' : '/';
}

/** Registrar el sincronizador de foco; devuelve teardown idempotente. */
export interface WindowUrlSyncHandle {
  readonly pause: () => void;
  readonly resume: () => void;
  readonly stop: () => void;
}

export function initWindowUrlSync(): WindowUrlSyncHandle {
  let windowsInitialized = false;
  let mobileInitialized = false;
  let paused = false;
  let stopped = false;

  const sync = (): void => {
    if (paused || stopped) return;
    const targetPath = resolveFocusedPath(
      windowStore.get(),
      mobileStackStore.get(),
      getPresentationMode(),
    );
    if (targetPath !== getCurrentPath()) replacePath(targetPath);
  };

  const stopWindows = windowStore.subscribe((_: readonly WindowEntry[], source: StoreSource) => {
    /* Una mutación coordinada con history.back() conserva la URL hasta que
     * popstate resuelva la entrada anterior. */
    if (source === 'sync') return;
    /* No sobrescribir una deep link antes de que el router monte su app inicial. */
    if (!windowsInitialized) {
      windowsInitialized = true;
      return;
    }
    sync();
  });
  const stopMobile = mobileStackStore.subscribe((_: readonly MobileStackEntry[], source: StoreSource) => {
    if (source === 'sync') return;
    if (!mobileInitialized) {
      mobileInitialized = true;
      return;
    }
    sync();
  });

  return {
    pause: () => { paused = true; },
    resume: () => {
      if (stopped) return;
      paused = false;
      sync();
    },
    stop: () => {
      if (stopped) return;
      stopped = true;
      stopWindows();
      stopMobile();
    },
  };
}
