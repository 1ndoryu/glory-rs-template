import { describe, expect, it, vi } from 'vitest';
import { ArticleService } from '../../services';
import { AppRegistry } from './app-registry';
import './app-registration';

describe('Account app registration', () => {
  it('registers the Bosque preview as a lazy public full-bleed app', async () => {
    const game = AppRegistry.get('game');
    expect(game).toBeDefined();
    expect(game?.singleton).toBe(true);
    expect(game?.requires).toBe('public');
    expect(game?.layout).toBe('full-bleed');
    expect(game?.deepLink?.stringify()).toBe('/forest-2d');
    expect(game?.routePatterns).toBeUndefined();

    const view = await AppRegistry.instantiate('game', {
      signal: new AbortController().signal,
    });
    expect(view?.element.classList.contains('bosqueBoceto')).toBe(true);
    expect(() => view?.destroy?.()).not.toThrow();
  });

  it('keeps the 3D alternative separate and lazy', () => {
    const game3d = AppRegistry.get('game-3d');
    expect(game3d).toBeDefined();
    expect(game3d?.singleton).toBe(true);
    expect(game3d?.requires).toBe('public');
    expect(game3d?.layout).toBe('full-bleed');
    expect(game3d?.deepLink?.stringify()).toBe('/forest-3d');
  });

  it('keeps the playable fixture lazy until its first instantiation', () => {
    expect(AppRegistry.isLazy('game-playable')).toBe(true);
    const playable = AppRegistry.get('game-playable');
    expect(playable).toBeDefined();
    expect(playable?.singleton).toBe(true);
    expect(playable?.requires).toBe('public');
    expect(playable?.layout).toBe('full-bleed');
    expect(playable?.deepLink?.stringify()).toBe('/forest-playable');
    expect(playable?.routePatterns).toBeUndefined();
    /* [297A-62] El toolbar real de la ventana del Bosque expone la
     * configuración del juego; el comando game:settings es adminOnly y el
     * shell oculta el grupo completo para no-admin. */
    expect(playable?.toolbar).toEqual([{ label: 'Configuración', items: ['game:settings'] }]);
  });

  it('registers a public singleton with the /login deep link', () => {
    const account = AppRegistry.get('account');
    expect(account).toBeDefined();
    expect(account?.singleton).toBe(true);
    expect(account?.requires).toBe('public');
    expect(account?.deepLink?.patterns).toEqual(['/login']);
    expect(account?.deepLink?.parse({})).toEqual({});
    expect(account?.deepLink?.stringify()).toBe('/login');
  });

  it('does not allow unexpected login URL parameters', () => {
    const account = AppRegistry.get('account');
    expect(account?.deepLink?.parse({ redirect: '/admin' })).toBeNull();
  });

  it('registers article editor as an internal admin app', () => {
    const editor = AppRegistry.get('article-editor');
    expect(editor).toBeDefined();
    expect(editor?.requires).toBe('admin');
    expect(editor?.singleton).toBe(false);
    expect(editor?.deepLink).toBeUndefined();
    expect(editor?.routePatterns).toBeUndefined();
  });

  it('registers project editor as an internal admin app', () => {
    const editor = AppRegistry.get('project-editor');
    expect(editor).toBeDefined();
    expect(editor?.requires).toBe('admin');
    expect(editor?.singleton).toBe(false);
    expect(editor?.deepLink).toBeUndefined();
    expect(editor?.routePatterns).toBeUndefined();
  });

  it('registers admin as an internal app without a legacy route', () => {
    const admin = AppRegistry.get('admin');
    expect(admin).toBeDefined();
    expect(admin?.requires).toBe('admin');
    expect(admin?.singleton).toBe(true);
    expect(admin?.deepLink).toBeUndefined();
    expect(admin?.routePatterns).toBeUndefined();
  });

  it('registers product editor as an internal admin app', () => {
    const editor = AppRegistry.get('product-editor');
    expect(editor).toBeDefined();
    expect(editor?.requires).toBe('admin');
    expect(editor?.singleton).toBe(false);
    expect(editor?.deepLink).toBeUndefined();
    expect(editor?.routePatterns).toBeUndefined();
  });

  it('registers media library as an admin-only singleton without public deep link', () => {
    const library = AppRegistry.get('media-library');
    expect(library).toBeDefined();
    expect(library?.requires).toBe('admin');
    expect(library?.singleton).toBe(true);
    expect(library?.deepLink).toBeUndefined();
    expect(library?.routePatterns).toBeUndefined();
  });

  it('does not reinterpret an internal resourceId as a public Reader slug', async () => {
    const reader = AppRegistry.get('reader');
    expect(reader).toBeDefined();

    const getBySlug = vi.spyOn(ArticleService, 'getBySlug');
    try {
      const view = await AppRegistry.instantiate('reader', {
        signal: new AbortController().signal,
        params: { resourceId: 'internal-resource-uuid' },
      });

      expect(view?.element.textContent).toContain('Selecciona un artículo para leer.');
      expect(getBySlug).not.toHaveBeenCalled();
    } finally {
      getBySlug.mockRestore();
    }
  });
});
