import { beforeEach, describe, expect, it } from 'vitest';
import { authStore } from '../../../store';
import { createDesktopMenuBar } from './desktop-menu-bar';

beforeEach(() => {
  if (!window.matchMedia) {
    window.matchMedia = ((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      dispatchEvent: () => false,
    })) as typeof window.matchMedia;
  }
  authStore.set({ isAuthenticated: false, userId: null, capability: 'public' }, 'sync');
});

describe('desktop menu bar account status', () => {
  it('projects guest, authenticated and admin labels from authStore', () => {
    const bar = createDesktopMenuBar();
    document.body.appendChild(bar.element);
    const button = bar.element.querySelector<HTMLButtonElement>('.desktop-menu-bar__account');
    const label = bar.element.querySelector('.desktop-menu-bar__account-label');
    expect(button).not.toBeNull();
    expect(label?.textContent).toBe('Entrar');

    authStore.set({ isAuthenticated: true, userId: 'user-1', capability: 'authenticated' }, 'sync');
    expect(label?.textContent).toBe('Cuenta');
    authStore.set({ isAuthenticated: true, userId: 'admin-1', capability: 'admin' }, 'sync');
    expect(label?.textContent).toBe('Cuenta · admin');

    bar.destroy();
    document.body.innerHTML = '';
  });

  it('stops reacting after destroy', () => {
    const bar = createDesktopMenuBar();
    document.body.appendChild(bar.element);
    const label = bar.element.querySelector('.desktop-menu-bar__account-label');
    bar.destroy();

    authStore.set({ isAuthenticated: true, userId: 'user-1', capability: 'authenticated' }, 'sync');
    expect(label?.textContent).toBe('Entrar');
    document.body.innerHTML = '';
  });
});
