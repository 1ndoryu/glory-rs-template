import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createGameMapEditor } from './game-map-editor';
import { GameMapAdminService } from '../../../../services/game-map-admin.service';
import { GameAssetAdminService } from '../../../../services/game-asset-admin.service';
import { FIXTURE_MAP_VERSION } from './game-fixture-map';

function activeCatalog(): unknown[] {
  return [
    { id: 'tree', displayName: 'Árbol', category: 'tree', isActive: true, createdAt: '2026-08-02T00:00:00Z' },
    { id: 'rock', displayName: 'Roca', category: 'rock', isActive: true, createdAt: '2026-08-02T00:00:00Z' },
  ];
}

function mockCanvas(): void {
  vi.stubGlobal('ResizeObserver', class {
    observe(): void {}
    disconnect(): void {}
    unobserve(): void {}
  });
  HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
    clearRect: vi.fn(),
    fillRect: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    fillText: vi.fn(),
    arc: vi.fn(),
    strokeRect: vi.fn(),
    fillStyle: '',
    strokeStyle: '',
    lineWidth: 1,
    font: '',
    textAlign: 'left',
    textBaseline: 'alphabetic',
  })) as never;
}

describe('createGameMapEditor (297A-64)', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    mockCanvas();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.textContent = '';
  });

  it('monta el toolbar, el canvas y el footer', async () => {
    vi.spyOn(GameAssetAdminService, 'listAll').mockResolvedValue(activeCatalog() as never);
    vi.spyOn(GameMapAdminService, 'getActive').mockResolvedValue({
      document: FIXTURE_MAP_VERSION,
      activeVersion: 3,
    } as never);

    const host = document.createElement('div');
    document.body.appendChild(host);
    const handle = createGameMapEditor(host);
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(host.querySelector('canvas')).not.toBeNull();
    expect(host.textContent).toContain('seleccionar');
    expect(host.textContent).toContain('colocar');
    expect(host.textContent).toContain('spawn');
    expect(host.textContent).toContain('pintar');
    expect(host.textContent).toContain('publicar mapa');
    /* La paleta se puebla con assets activos. */
    expect(host.textContent).toContain('Árbol');
    expect(host.textContent).toContain('Roca');
    /* [297A-66] El pincel expone el selector de superficies suelo/agua. */
    expect(host.querySelector('select[aria-label="superficie del pincel"]')).not.toBeNull();
    handle.destroy();
  });

  it('usa el fixture como base cuando no hay mapa publicado', async () => {
    vi.spyOn(GameAssetAdminService, 'listAll').mockResolvedValue(activeCatalog() as never);
    vi.spyOn(GameMapAdminService, 'getActive').mockResolvedValue(null);

    const host = document.createElement('div');
    document.body.appendChild(host);
    const handle = createGameMapEditor(host);
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(host.textContent).toContain('sin publicar');
    expect(host.textContent).toContain(`${FIXTURE_MAP_VERSION.instances.length} instancias`);
    handle.destroy();
  });

  it('no monta la paleta si el catálogo falla pero mantiene el editor', async () => {
    vi.spyOn(GameAssetAdminService, 'listAll').mockRejectedValue(new Error('catalog down'));
    vi.spyOn(GameMapAdminService, 'getActive').mockResolvedValue(null);

    const host = document.createElement('div');
    document.body.appendChild(host);
    const handle = createGameMapEditor(host);
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(host.querySelector('canvas')).not.toBeNull();
    expect(host.textContent).toContain('mapa cargado sin catálogo');
    handle.destroy();
  });

  it('destroy retira el editor del DOM', async () => {
    vi.spyOn(GameAssetAdminService, 'listAll').mockResolvedValue(activeCatalog() as never);
    vi.spyOn(GameMapAdminService, 'getActive').mockResolvedValue(null);

    const host = document.createElement('div');
    document.body.appendChild(host);
    const handle = createGameMapEditor(host);
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(host.children.length).toBeGreaterThan(0);
    handle.destroy();
    expect(host.children.length).toBe(0);
  });
});
