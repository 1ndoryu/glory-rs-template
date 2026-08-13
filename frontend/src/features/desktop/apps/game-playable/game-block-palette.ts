/* GAME-01 — Paleta de bloques del Bosque (estilo Minecraft).
 * Colores base en hex; las mallas aplican jitter y AO por vértice. Ninguna
 * especificación visual vive en los componentes: aquí están los únicos
 * tokens de color del terreno. */

export const BLOCK_COLORS = {
  grass: 0x86c65c,
  dirt: 0x9b6b46,
  sand: 0xe8d8a0,
  sandSide: 0xd3bf86,
  trunk: 0x8a5a34,
  leaf: 0x63b543,
  leafDark: 0x4c9233,
  rock: 0x9d9d96,
  rockDark: 0x7d7d78,
  waterDeep: 0x36a79e,
  waterShallow: 0x63c9bb,
  foam: 0xeafbf5,
  sky: 0xaecfc4,
} as const;

/* AO falso en la base de las caras laterales de un bloque. */
export const BLOCK_SIDE_AO = 0.78;

/** Convierte un hex en [r, g, b] lineales (0..1) multiplicados por `mul`. */
export function tintRgb(hex: number, mul: number): [number, number, number] {
  const r = ((hex >> 16) & 0xff) / 255;
  const g = ((hex >> 8) & 0xff) / 255;
  const b = (hex & 0xff) / 255;
  return [r * mul, g * mul, b * mul];
}
