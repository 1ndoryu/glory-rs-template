import { describe, expect, it } from 'vitest';
import { createGamePreview, renderGamePreview } from './game-preview';

describe('game visual preview', () => {
  it('renderiza una escena estática accesible', () => {
    const preview = createGamePreview();
    const image = preview.querySelector('img');

    expect(preview.className).toBe('bosqueBoceto');
    expect(preview.getAttribute('aria-label')).toBe('Boceto visual del Bosque');
    expect(image?.getAttribute('src')).toContain('bosque-boceto.svg');
    expect(image?.getAttribute('alt')).toContain('Bosque cenital');
    expect(preview.textContent).toContain('sin lógica');
  });

  it('expone MountedView sin estado de juego', () => {
    const view = renderGamePreview();
    expect(view.element.classList.contains('bosqueBoceto')).toBe(true);
    expect(() => view.destroy?.()).not.toThrow();
  });
});
