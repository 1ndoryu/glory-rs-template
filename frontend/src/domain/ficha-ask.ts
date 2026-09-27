// Ficha /ask (279A-3): cuestionario privado por tipo para completar fichas.
// Dominio puro: sin DOM, sin fetch, sin React (reutilizable en móvil).
// Claves snake_case ≤64 (las valida el backend en `validar_extras`);
// `precio_minimo` NO vive aquí: es columna privada aparte (nunca viaja
// a la web pública ni a la IA con cifras).
//
// Cómo agregar un hueco mañana (279A-6):
// - Dato suelto (¿tiene X?, texto corto, número privado) → 1 entrada en
//   COMUNES, SERVICIOS o la lista del tipo. Sin tocar backend ni UI: /ask,
//   el formulario admin (FichaFormulario), el % de completitud y
//   buscarPendiente la toman sola. El backend acepta cualquier clave
//   snake_case ≤64 (objeto plano, valores simples).
// - Dato que es columna del inmueble (filtra, se publica, lo usa la IA) →
//   migración + backend + `ColumnasAsk` + entrada en COLUMNAS_POR_TIPO.
//   No lo metas en `extras`: los filtros y la pública no los leen.
// - `privada: false` = apto para mostrarse fuera del panel (la pública y
//   la IA aún no leen `extras`: pendiente cablearlas, ver roadmap).
// - `destino: 'precioMinimo'` = el valor va a su columna privada, no a
//   `extras`. Si mañana hay otro dato así (comisión, deuda…), añade una
//   variante de `destino`, no otro `clave === 'x'` regado.

import type { TipoInmueble } from './inmueble';

export type TipoPreguntaAsk = 'si_no' | 'texto_corto' | 'entero' | 'decimal';

/* A dónde va el valor al responder: `extras` (defecto) o la columna
 * privada `precio_minimo`. Existe para no hardcodear la clave en cada
 * rama que lee la ficha (preguntaRespondida, extrasTrasRespuesta…). */
export type DestinoAsk = 'precioMinimo';

export interface PreguntaAsk {
  /** Clave guardada en `extras` (salvo `destino`, que usa su columna). */
  clave: string;
  /** Pregunta tal como se lee al responder (cercana, sin jerga). */
  etiqueta: string;
  tipo: TipoPreguntaAsk;
  /** Ayuda corta bajo la pregunta (ejemplo de respuesta válida). */
  ayuda?: string;
  /** Unidad junto al campo numérico (`m²`, `$`…). */
  unidad?: string;
  /** Destino distinto de `extras` (hoy solo el mínimo privado). */
  destino?: DestinoAsk;
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
    etiqueta: '¿Cuál es el precio mínimo aceptado? (privado, no se publica)',
    tipo: 'decimal',
    ayuda: 'Solo visible en el panel; la página nunca lo muestra.',
    unidad: '$',
    destino: 'precioMinimo',
    privada: true,
  },
];

/* Servicios básicos (279A-6): aplican a todo tipo. `privada: false` los
 * marca aptos para la ficha visible y la IA, pero hoy nadie los lee fuera
 * del panel (pendiente: cablear pública + IA, ver roadmap). Huecos que
 * faltaban: internet y agua. */
export const SERVICIOS: PreguntaAsk[] = [
  {
    clave: 'internet',
    etiqueta: '¿Posee internet?',
    tipo: 'si_no',
    privada: false,
  },
  {
    clave: 'agua',
    etiqueta: '¿Llega el agua?',
    tipo: 'si_no',
    privada: false,
  },
];

/* Checklist fijo por tipo (degradado sin IA): 3 comunes + 2 servicios + 3 propias. */
export const CHECKLIST_ASK: Record<TipoInmueble, PreguntaAsk[]> = {
  apartamento: [
    ...COMUNES,
    ...SERVICIOS,
    { clave: 'piso', etiqueta: '¿En qué piso queda?', tipo: 'entero', ayuda: '0 = planta baja.', privada: false },
    { clave: 'ascensor', etiqueta: '¿El edificio tiene ascensor?', tipo: 'si_no', privada: false },
    { clave: 'areas_comunes', etiqueta: '¿Qué áreas comunes tiene? (piscina, gimnasio, salón…)', tipo: 'texto_corto', privada: false },
  ],
  townhouse: [
    ...COMUNES,
    ...SERVICIOS,
    { clave: 'piso', etiqueta: '¿Cuántas plantas tiene?', tipo: 'entero', privada: false },
    { clave: 'conjunto_cerrado', etiqueta: '¿Está en conjunto cerrado con vigilancia?', tipo: 'si_no', privada: false },
    { clave: 'areas_comunes', etiqueta: '¿Qué áreas comunes tiene el conjunto?', tipo: 'texto_corto', privada: false },
  ],
  casa: [
    ...COMUNES,
    ...SERVICIOS,
    { clave: 'conjunto_cerrado', etiqueta: '¿Está en conjunto cerrado con vigilancia?', tipo: 'si_no', privada: false },
    { clave: 'ampliacion', etiqueta: '¿Tiene ampliación o posibilidad de ampliar?', tipo: 'si_no', privada: false },
    { clave: 'areas_externas', etiqueta: '¿Qué tiene afuera? (patio, terraza, parrillera…)', tipo: 'texto_corto', privada: false },
  ],
  local: [
    ...COMUNES,
    ...SERVICIOS,
    { clave: 'frente_metros', etiqueta: '¿Cuántos metros de frente (vidriera) tiene?', tipo: 'decimal', privada: false },
    { clave: 'bano', etiqueta: '¿Tiene baño propio?', tipo: 'si_no', privada: false },
    { clave: 'deposito', etiqueta: '¿Tiene depósito o trastienda?', tipo: 'si_no', privada: false },
  ],
  terreno: [
    ...COMUNES,
    ...SERVICIOS,
    { clave: 'frente_metros', etiqueta: '¿Cuántos metros de frente tiene?', tipo: 'decimal', privada: false },
    { clave: 'cercado', etiqueta: '¿Está cercado o amojonado?', tipo: 'si_no', privada: false },
    { clave: 'servicios', etiqueta: '¿Qué servicios llegan? (agua, luz, vialidad…)', tipo: 'texto_corto', privada: false },
  ],
};

/* Respondida = presente y no vacía (el `false` de un Sí/No cuenta: dijo que no). */
export function preguntaRespondida(pregunta: PreguntaAsk, extras: ExtrasAsk, precioMinimo?: number | null): boolean {
  if (pregunta.destino === 'precioMinimo') return (precioMinimo ?? 0) > 0;
  const v = extras[pregunta.clave];
  if (v === undefined || v === null) return false;
  if (typeof v === 'string') return v.trim().length > 0;
  return true;
}

/* % de ficha completa + claves que faltan (aviso visual, nunca bloquea).
 * Con `columnas` suma además el paso ubicación+residencia y los numéricos
 * (lógica en `./pasos-ask`; se reexporta aquí para no romper imports). */
export { calcularCompletitud } from './pasos-ask';
