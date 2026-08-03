import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { openGameSettings } from './game-settings';
import { GameCharacterAdminService } from '../../../../services/game-character-admin.service';
import { GameAssetAdminService } from '../../../../services/game-asset-admin.service';
import { GameAuditService, isValidAuditEvent } from '../../../../services/game-audit.service';

function characterEntry(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: 'forest-scout',
    displayName: 'Explorador',
    bodyTone: 'ink',
    isActive: true,
    createdAt: '2026-08-01T00:00:00Z',
    ...overrides,
  };
}

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

function auditEvent(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: 1,
    actorKind: 'admin',
    action: 'character.created',
    entityKind: 'character',
    entityId: 'forest-scout',
    payload: { displayName: 'Explorador', bodyTone: 'ink', isActive: true },
    createdAt: '2026-08-02T00:00:00Z',
    ...overrides,
  };
}

describe('openGameSettings (297A-62)', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.textContent = '';
    document.body.style.overflow = '';
  });

  it('abre el modal de configuración con ambas secciones de catálogo', async () => {
    vi.spyOn(GameCharacterAdminService, 'listAll').mockResolvedValue([
      characterEntry() as never,
    ]);
    vi.spyOn(GameAssetAdminService, 'listAll').mockResolvedValue([
      assetEntry() as never,
    ]);
    vi.spyOn(GameAuditService, 'listCharacterEvents').mockResolvedValue([] as never);
    vi.spyOn(GameAuditService, 'listAssetEvents').mockResolvedValue([] as never);

    openGameSettings();
    await new Promise((resolve) => setTimeout(resolve, 0));

    const dialog = document.querySelector<HTMLElement>('.modal-contenido');
    expect(dialog).not.toBeNull();
    expect(dialog?.textContent).toContain('personajes');
    expect(dialog?.textContent).toContain('assets');
    expect(dialog?.textContent).toContain('Explorador');
    expect(dialog?.textContent).toContain('Roble');
    expect(dialog?.textContent).toContain('cerrar');
  });

  it('muestra el estado vacío si un catálogo no tiene entradas', async () => {
    vi.spyOn(GameCharacterAdminService, 'listAll').mockResolvedValue([] as never);
    vi.spyOn(GameAssetAdminService, 'listAll').mockResolvedValue([] as never);
    vi.spyOn(GameAuditService, 'listCharacterEvents').mockResolvedValue([] as never);
    vi.spyOn(GameAuditService, 'listAssetEvents').mockResolvedValue([] as never);

    openGameSettings();
    await new Promise((resolve) => setTimeout(resolve, 0));

    const dialog = document.querySelector<HTMLElement>('.modal-contenido');
    /* [317A-2] createVacio capitaliza la primera letra del estado vacío. */
    expect(dialog?.textContent).toContain('No hay personajes en el catálogo');
    expect(dialog?.textContent).toContain('No hay assets en el catálogo');
  });

  it('mantiene los catálogos operativos si la auditoría falla (aislamiento)', async () => {
    vi.spyOn(GameCharacterAdminService, 'listAll').mockResolvedValue([
      characterEntry() as never,
    ]);
    vi.spyOn(GameAssetAdminService, 'listAll').mockResolvedValue([
      assetEntry() as never,
    ]);
    vi.spyOn(GameAuditService, 'listCharacterEvents').mockRejectedValue(new Error('audit down'));
    vi.spyOn(GameAuditService, 'listAssetEvents').mockRejectedValue(new Error('audit down'));

    openGameSettings();
    await new Promise((resolve) => setTimeout(resolve, 0));

    const dialog = document.querySelector<HTMLElement>('.modal-contenido');
    expect(dialog?.textContent).toContain('Explorador');
    expect(dialog?.textContent).toContain('Roble');
    expect(dialog?.textContent).toContain('No se pudo cargar la actividad');
  });

  it('permite cerrar el modal con el botón inferior', () => {
    vi.spyOn(GameCharacterAdminService, 'listAll').mockResolvedValue([] as never);
    vi.spyOn(GameAssetAdminService, 'listAll').mockResolvedValue([] as never);
    vi.spyOn(GameAuditService, 'listCharacterEvents').mockResolvedValue([] as never);
    vi.spyOn(GameAuditService, 'listAssetEvents').mockResolvedValue([] as never);

    openGameSettings();

    const cerrar = Array.from(document.querySelectorAll<HTMLButtonElement>('button'))
      .find((button) => button.textContent === 'cerrar');
    expect(cerrar).toBeDefined();
    cerrar?.click();
    expect(document.querySelector('.modal-overlay')).toBeNull();
  });
});

describe('game-settings audit event rendering', () => {
  it('isValidAuditEvent del servicio de auditoría acepta eventos de assets', () => {
    /* [297A-61] Los pares acción-entidad asset.*↔asset ya están en el validador
     * compartido; este test lo confirma de forma directa sin fetch. */
    expect(isValidAuditEvent(auditEvent({ action: 'asset.created', entityKind: 'asset', id: 9 }))).toBe(true);
  });
});
