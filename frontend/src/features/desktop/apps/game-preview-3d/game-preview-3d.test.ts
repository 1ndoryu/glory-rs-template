import { describe, expect, it } from 'vitest';
import { createGame3dPreview } from './game-preview-3d';

describe('Bosque 3D visual preview', () => {
  it('builds an accessible WebGL host without mounting game logic', () => {
    const preview = createGame3dPreview();

    expect(preview.element.getAttribute('aria-label')).toBe('Segundo boceto visual Bosque 3D');
    expect(preview.sceneHost.getAttribute('aria-label')).toBe('Escena interactiva del Bosque 3D');
    expect(preview.resetButton.textContent).toBe('recentrar');
    expect(preview.element.querySelector('canvas')).toBeNull();
  });
});
