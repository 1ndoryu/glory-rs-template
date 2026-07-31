import { describe, expect, it } from 'vitest';
import '../app-registration';
import { resolvePublicResourceTarget } from './public-resource-locator';

const baseNode = {
  id: 'article-node',
  parentId: 'desktop' as const,
  type: 'resource' as const,
  label: 'Artículo',
  refId: 'internal-resource-uuid',
  resourceKind: 'article' as const,
  origin: 'release' as const,
  publicLocator: undefined,
};

describe('public resource locator', () => {
  it('resuelve el slug público sin usar refId', () => {
    expect(resolvePublicResourceTarget({
      ...baseNode,
      publicLocator: { appId: 'reader', params: { slug: 'julio-2026' } },
    })).toEqual({
      appId: 'reader',
      params: { slug: 'julio-2026' },
    });
  });

  it('rechaza un locator con parámetros no allowlisted', () => {
    expect(resolvePublicResourceTarget({
      ...baseNode,
      publicLocator: { appId: 'reader', params: { resourceId: baseNode.refId } },
    })).toBeNull();
  });

  it('no convierte un recurso sin locator en una URL pública', () => {
    expect(resolvePublicResourceTarget(baseNode)).toBeNull();
  });

  it('rechaza una app inexistente o no pública', () => {
    expect(resolvePublicResourceTarget({
      ...baseNode,
      publicLocator: { appId: 'missing-app', params: { slug: 'julio-2026' } },
    })).toBeNull();
  });

  it('falla cerrado ante nodos no públicos o de tipo incorrecto', () => {
    expect(resolvePublicResourceTarget({
      ...baseNode,
      type: 'folder',
      publicLocator: { appId: 'reader', params: { slug: 'julio-2026' } },
    })).toBeNull();
    expect(resolvePublicResourceTarget({
      ...baseNode,
      requires: 'admin',
      publicLocator: { appId: 'reader', params: { slug: 'julio-2026' } },
    })).toBeNull();
  });
});
