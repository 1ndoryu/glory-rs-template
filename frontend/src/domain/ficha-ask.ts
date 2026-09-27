// Ficha /ask (279A-3): cuestionario privado de la dueña por tipo.
// Dominio puro: sin DOM, sin fetch, sin React (reutilizable en móvil).
// Claves snake_case ≤64 (las valida el backend en `validar_extras`);
// `precio_minimo` NO vive aquí: es columna privada aparte (nunca viaja
// a la web pública ni a la IA con cifras).

import type { TipoInmueble } from './inmueble';

export type TipoPreguntaAsk = 'si_no' | 'texto_corto' | 'entero' | 'decimal';

export interface PreguntaAsk {
  /** Clave guardada en `extras` (`precio_minimo` usa su columna). */
  clave: string;
  /** Pregunta tal como la lee la dueña (cercana, sin jerga). */
  etiqueta: string;
  tipo: TipoPreguntaAsk;
  /** Ayuda corta bajo la pregunta (ejemplo de respuesta válida). */
  ayuda?: string;
  /** Unidad junto al campo numérico (`m²`, `€`…). */
  unidad?: string;
  /** Privado = nunca sale a la web pública (solo admin + IA sin cifras). */
  privada: boolean;
}

export type ExtrasAsk = Record<string, string | number | boolean>;

/* Comunes a todo tipo (3): referencia privada, negociable y mínimo.
 * (La zona/residencia dejó de ser pregunta de `extras`: es el paso
 * inteligente de columnas `ubicacion`+`residencia`, que rellena el
 * inmueble de verdad en vez de un texto suelto.)
 * Exportadas para la sección Ficha del modal admin (misma fuente que /ask). */
export const COMUNES: PreguntaAsk[] = [
  {
    clave: 'punto_referencia',
    etiqueta: '¿Qué punto de referencia hay cerca? (p. ej. frente al Orinokia)',
    tipo: 'texto_corto',
    privada: true,
  },
  {
    clave: 'negociable',
    etiqueta: '¿El precio es negociable?',
    tipo: 'si_no',
    privada: true,
  },
  {
    clave: 'precio_minimo',
    etiqueta: '¿Cuál es lo mínimo que aceptarías? (privado, nadie lo ve)',
    tipo: 'decimal',
    ayuda: 'Solo lo ves tú en el panel; la página nunca lo muestra.',
    unidad: '€',
    privada: true,
  },
];

/* Checklist fijo por tipo (degradado sin IA): 3 comunes + 3 propias. */
export const CHECKLIST_ASK: Record<TipoInmueble, PreguntaAsk[]> = {
  apartamento: [
    ...COMUNES,
    { clave: 'piso', etiqueta: '¿En qué piso queda?', tipo: 'entero', ayuda: '0 = planta baja.', privada: false },
    { clave: 'ascensor', etiqueta: '¿El edificio tiene ascensor?', tipo: 'si_no', privada: false },
    { clave: 'areas_comunes', etiqueta: '¿Qué áreas comunes tiene? (piscina, gimnasio, salón…)', tipo: 'texto_corto', privada: false },
  ],
  townhouse: [
    ...COMUNES,
    { clave: 'piso', etiqueta: '¿Cuántas plantas tiene?', tipo: 'entero', privada: false },
    { clave: 'conjunto_cerrado', etiqueta: '¿Está en conjunto cerrado con vigilancia?', tipo: 'si_no', privada: false },
    { clave: 'areas_comunes', etiqueta: '¿Qué áreas comunes tiene el conjunto?', tipo: 'texto_corto', privada: false },
  ],
  casa: [
    ...COMUNES,
    { clave: 'conjunto_cerrado', etiqueta: '¿Está en conjunto cerrado con vigilancia?', tipo: 'si_no', privada: false },
    { clave: 'ampliacion', etiqueta: '¿Tiene ampliación o posibilidad de ampliar?', tipo: 'si_no', privada: false },
    { clave: 'areas_externas', etiqueta: '¿Qué tiene afuera? (patio, terraza, parrillera…)', tipo: 'texto_corto', privada: false },
  ],
  local: [
    ...COMUNES,
    { clave: 'frente_metros', etiqueta: '¿Cuántos metros de frente (vidriera) tiene?', tipo: 'decimal', privada: false },
    { clave: 'bano', etiqueta: '¿Tiene baño propio?', tipo: 'si_no', privada: false },
    { clave: 'deposito', etiqueta: '¿Tiene depósito o trastienda?', tipo: 'si_no', privada: false },
  ],
  terreno: [
    ...COMUNES,
    { clave: 'frente_metros', etiqueta: '¿Cuántos metros de frente tiene?', tipo: 'decimal', privada: false },
    { clave: 'cercado', etiqueta: '¿Está cercado o amojonado?', tipo: 'si_no', privada: false },
    { clave: 'servicios', etiqueta: '¿Qué servicios llegan? (agua, luz, vialidad…)', tipo: 'texto_corto', privada: false },
  ],
};

/* Respondida = presente y no vacía (el `false` de un Sí/No cuenta: dijo que no). */
export function preguntaRespondida(pregunta: PreguntaAsk, extras: ExtrasAsk, precioMinimo?: number | null): boolean {
  if (pregunta.clave === 'precio_minimo') return (precioMinimo ?? 0) > 0;
  const v = extras[pregunta.clave];
  if (v === undefined || v === null) return false;
  if (typeof v === 'string') return v.trim().length > 0;
  return true;
}

/* % de ficha completa + claves que faltan (aviso visual, nunca bloquea).
 * Con `columnas` suma además el paso ubicación+residencia y los numéricos
 * (lógica en `./pasos-ask`; se reexporta aquí para no romper imports). */
export { calcularCompletitud } from './pasos-ask';
