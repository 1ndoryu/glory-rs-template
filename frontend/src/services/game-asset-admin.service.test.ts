import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  GameAssetAdminService,
  isValidAdminAssetId,
  isValidAdminAssetLabel,
  isValidAdminAssetCategory,
  isValidAdminAssetEntry,
  GAME_ASSET_CATEGORIES,
} from './game-asset-admin.service';

function assetEntry(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: 'oak',
    displayName: 'Roble',
    category: 'tree',
    isActive: true,
    createdAt: '2026-08-02T00:00:00Z',
    ...overrides,
  };
}

describe('GameAssetAdminService', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('loads the exact admin contract with state and creation date', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify([assetEntry(), assetEntry({ id: 'pond', category: 'water', isActive: false })]), { status: 200 }),
    );
    vi.stubGlobal('fetch', fetchMock);

    await expect(GameAssetAdminService.listAll()).resolves.toEqual([
      expect.objectContaining({ id: 'oak', isActive: true }),
      expect.objectContaining({ id: 'pond', category: 'water', isActive: false }),
    ]);
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/admin/game/assets');
  });

  it('rejects malformed, extra-field, and wrong-category admin entries', () => {
    expect(isValidAdminAssetEntry(assetEntry())).toBe(true);
    expect(isValidAdminAssetEntry(assetEntry({ script: 'alert(1)' }))).toBe(false);
    expect(isValidAdminAssetEntry(assetEntry({ category: 'bridge' }))).toBe(false);
    expect(isValidAdminAssetEntry(assetEntry({ isActive: 'yes' }))).toBe(false);
    expect(isValidAdminAssetEntry(assetEntry({ displayName: '  ' }))).toBe(false);
    expect(isValidAdminAssetEntry(assetEntry({ id: 'Upper-Case' }))).toBe(false);
  });

  it('validates id, label and category against the backend allowlist', () => {
    expect(isValidAdminAssetId('oak')).toBe(true);
    expect(isValidAdminAssetId('oak_tree')).toBe(false);
    expect(isValidAdminAssetId('A')).toBe(false);
    expect(isValidAdminAssetId('x'.repeat(49))).toBe(false);
    expect(isValidAdminAssetLabel('Roble')).toBe(true);
    expect(isValidAdminAssetLabel('linea\nnueva')).toBe(false);
    expect(isValidAdminAssetLabel('x'.repeat(65))).toBe(false);
    expect(isValidAdminAssetLabel('  ')).toBe(false);
    expect(isValidAdminAssetCategory('terrain')).toBe(true);
    expect(isValidAdminAssetCategory('tree')).toBe(true);
    expect(isValidAdminAssetCategory('rock')).toBe(true);
    expect(isValidAdminAssetCategory('water')).toBe(true);
    expect(isValidAdminAssetCategory('character')).toBe(true);
    expect(isValidAdminAssetCategory('generic')).toBe(true);
    expect(isValidAdminAssetCategory('sky')).toBe(false);
    expect(GAME_ASSET_CATEGORIES).toEqual(['terrain', 'tree', 'rock', 'water', 'character', 'generic']);
  });

  it('creates via POST with the exact admin body', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(assetEntry()), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);

    await GameAssetAdminService.create({
      id: 'oak',
      displayName: 'Roble',
      category: 'tree',
    });
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('/api/admin/game/assets');
    expect(init.method).toBe('POST');
    expect(JSON.parse(String(init.body))).toEqual({
      id: 'oak',
      displayName: 'Roble',
      category: 'tree',
    });
  });

  it('updates via PUT to the id path with state', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(assetEntry({ isActive: false })), { status: 200 }),
    );
    vi.stubGlobal('fetch', fetchMock);

    await GameAssetAdminService.update('oak', {
      displayName: 'Roble seco',
      category: 'tree',
      isActive: false,
    });
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('/api/admin/game/assets/oak');
    expect(init.method).toBe('PUT');
    expect(JSON.parse(String(init.body))).toEqual({
      displayName: 'Roble seco',
      category: 'tree',
      isActive: false,
    });
  });

  it('preserves the lifecycle abort signal on admin reads', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify([]), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    const controller = new AbortController();

    await GameAssetAdminService.listAll({ signal: controller.signal });
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(init.signal).toBe(controller.signal);
  });
});
