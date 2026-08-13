/* GAME-01 — API pública del toolkit procedural del Bosque (138A-1).
 * Paquete de datos puros: ruido determinista, heightfield de isla, mesh suave
 * y vegetación con presupuestos. No importa Three/DOM/red; los adaptadores
 * visuales viven en la capa app. */

export * from './noise';
export * from './heightmap';
export * from './heightfield-mesh';
export * from './vegetation';
export * from './vegetation-mesh';
