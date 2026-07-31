import { beforeEach, describe, expect, it, vi } from 'vitest';
import { bindLongPress } from './mobile-gestures';

function pointerEvent(
  type: string,
  pointerId: number,
  x: number,
  y: number,
): Event {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperties(event, {
    button: { value: 0 },
    clientX: { value: x },
    clientY: { value: y },
    pointerId: { value: pointerId },
  });
  return event;
}

describe('bindLongPress', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  it('ejecuta una sola vez después del umbral y bloquea el click sintético', () => {
    const element = document.createElement('button');
    const onLongPress = vi.fn();
    const click = vi.fn();
    element.addEventListener('click', click);
    const binding = bindLongPress(element, { onLongPress });

    element.dispatchEvent(pointerEvent('pointerdown', 1, 10, 10));
    vi.advanceTimersByTime(500);
    expect(onLongPress).toHaveBeenCalledTimes(1);

    element.dispatchEvent(pointerEvent('pointerup', 1, 10, 10));
    const syntheticClick = new MouseEvent('click', { bubbles: true, cancelable: true });
    expect(element.dispatchEvent(syntheticClick)).toBe(false);
    expect(click).not.toHaveBeenCalled();

    binding.destroy();
  });

  it('cancela el gesto si el puntero se mueve más del umbral', () => {
    const element = document.createElement('button');
    const onLongPress = vi.fn();
    const binding = bindLongPress(element, { onLongPress });

    element.dispatchEvent(pointerEvent('pointerdown', 2, 10, 10));
    element.dispatchEvent(pointerEvent('pointermove', 2, 25, 10));
    vi.advanceTimersByTime(500);

    expect(onLongPress).not.toHaveBeenCalled();
    binding.destroy();
  });

  it('permite el siguiente click si no hubo click sintético tras el long press', () => {
    const element = document.createElement('button');
    const onLongPress = vi.fn();
    const click = vi.fn();
    element.addEventListener('click', click);
    const binding = bindLongPress(element, { onLongPress });

    element.dispatchEvent(pointerEvent('pointerdown', 4, 10, 10));
    vi.advanceTimersByTime(500);
    element.dispatchEvent(pointerEvent('pointerup', 4, 10, 10));
    element.dispatchEvent(pointerEvent('pointerdown', 5, 10, 10));
    element.dispatchEvent(pointerEvent('pointerup', 5, 10, 10));
    element.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));

    expect(click).toHaveBeenCalledTimes(1);
    binding.destroy();
  });

  it('cancela el timer y listeners al destruirse', () => {
    const element = document.createElement('button');
    const onLongPress = vi.fn();
    const binding = bindLongPress(element, { onLongPress });

    element.dispatchEvent(pointerEvent('pointerdown', 3, 10, 10));
    binding.destroy();
    vi.advanceTimersByTime(500);

    expect(onLongPress).not.toHaveBeenCalled();
  });
});
