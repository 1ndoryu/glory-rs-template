import { describe, expect, it } from 'vitest';
import { AppRegistry } from './app-registry';
import { createPathDeepLink } from './deep-links';
import { resolveFocusedPath } from './window-url-sync';

const publicAppId = 'window-url-sync-public-test';
const localAppId = 'window-url-sync-local-test';

AppRegistry.register({
  id: publicAppId,
  title: 'Artículo',
  icon: [],
  singleton: false,
  requires: 'public',
  deepLink: createPathDeepLink('/article/:slug', ['slug']),
  render: () => ({ element: document.createElement('div') }),
});

AppRegistry.register({
  id: localAppId,
  title: 'Finder local',
  icon: [],
  singleton: false,
  requires: 'public',
  render: () => ({ element: document.createElement('div') }),
});

describe('resolveFocusedPath', () => {
  it('representa la ventana desktop enfocada con su ruta pública', () => {
    expect(resolveFocusedPath(
      [
        { appId: 'other', focused: false },
        { appId: publicAppId, focused: true, params: { slug: 'julio' } },
      ],
      [],
      'desktop',
    )).toBe('/article/julio');
  });

  it('no serializa parámetros locales sin deepLink', () => {
    expect(resolveFocusedPath(
      [{ appId: localAppId, focused: true, params: { folderId: 'internal-folder' } }],
      [],
      'desktop',
    )).toBe('/');
  });

  it('usa la vista superior móvil y no la ventana desktop', () => {
    expect(resolveFocusedPath(
      [{ appId: publicAppId, focused: true, params: { slug: 'desktop' } }],
      [{ appId: publicAppId, params: { slug: 'mobile' } }],
      'mobile',
    )).toBe('/article/mobile');
  });

  it('vuelve a la raíz sin app activa', () => {
    expect(resolveFocusedPath([], [], 'tablet')).toBe('/');
  });
});
