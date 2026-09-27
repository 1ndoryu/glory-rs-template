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
// - Respuestas fijas más allá de sí/no (amoblado…) → `tipo: 'opciones'`
//   con sus `opciones` (incluye `NO_SE`); se renderiza sola en /ask y admin.
// - «No lo sé» = `NO_SE`: respuesta válida que no repite la pregunta y
//   deja el dato vacío (la pública y la IA deben tratarlo como vacío).
// - Dato que es columna del inmueble (filtra, se publica, lo usa la IA) →
//   migración + backend + `ColumnasAsk` + entrada en COLUMNAS_POR_TIPO.
//   No lo metas en `extras`: los filtros y la pública no los leen.
// - `privada: false` = apto para mostrarse fuera del panel (la pública y
//   la IA aún no leen `extras`: pendiente cablearlas, ver roadmap).
// - `destino: 'precioMinimo'` = el valor va a su columna privada, no a
//   `extras`. Si mañana hay otro dato así (comisión, deuda…), añade una
//   variante de `destino`, no otro `clave === 'x'` regado.

import type { TipoInmueble } from './inmueble';

export type TipoPreguntaAsk = 'si_no' | 'texto_corto' | 'entero' | 'decimal' | 'opciones';

/* «No lo sé» como respuesta válida (279A-7): se guarda el centinela y la
 * pregunta no vuelve a salir, pero el dato queda vacío. Quien lea `extras`
 * fuera del panel (pública, IA) debe tratarlo como vacío, no como texto.
 * Nunca lo escribe el usuario a mano: solo el botón «No lo sé». */
export const NO_SE = 'no_se';

/* Opción de una pregunta `opciones` (ej. amoblado: sí / no / depende del
 * trato / no lo sé). `valor` booleano o cadena corta (la valida el backend
 * como cualquier `extra`); las etiquetas, cercanas y sin jerga. */
export interface OpcionAsk {
  valor: string | boolean;
  etiqueta: string;
}

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
  /** Respuestas fijas cuando `tipo` es `opciones` (incluye `NO_SE`). */
  opciones?: OpcionAsk[];
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
    tipo: 'opciones',
    privada: false,
    opciones: [
      { valor: true, etiqueta: 'Sí' },
      { valor: false, etiqueta: 'No' },
      { valor: 'a_veces', etiqueta: 'A veces' },
      { valor: NO_SE, etiqueta: 'No lo sé' },
    ],
  },
];

/* Amoblado (279A-7): compartida por los tipos de vivienda. Las respuestas
 * intermedias (`trato`, `NO_SE`) cuentan como respondida sin aportar dato:
 * la pregunta no vuelve a salir y el campo queda vacío. */
export const AMOBLADO: PreguntaAsk = {
  clave: 'amoblado',
  etiqueta: '¿Queda amoblado?',
  tipo: 'opciones',
  privada: false,
  opciones: [
    { valor: true, etiqueta: 'Sí' },
    { valor: false, etiqueta: 'No' },
    { valor: 'trato', etiqueta: 'Depende del trato' },
    { valor: NO_SE, etiqueta: 'No lo sé' },
  ],
};

/* Checklist fijo por tipo (degradado sin IA): 3 comunes + 2 servicios +
 * propias (amoblado en las viviendas). */
export const CHECKLIST_ASK: Record<TipoInmueble, PreguntaAsk[]> = {
  apartamento: [
    ...COMUNES,
    ...SERVICIOS,
    { clave: 'piso', etiqueta: '¿En qué piso queda?', tipo: 'entero', ayuda: '0 = planta baja.', privada: false },
    { clave: 'ascensor', etiqueta: '¿El edificio tiene ascensor?', tipo: 'si_no', privada: false },
    { clave: 'areas_comunes', etiqueta: '¿Qué áreas comunes tiene? (piscina, gimnasio, salón…)', tipo: 'texto_corto', privada: false },
    AMOBLADO,
  ],
  townhouse: [
    ...COMUNES,
    ...SERVICIOS,
    { clave: 'piso', etiqueta: '¿Cuántas plantas tiene?', tipo: 'entero', privada: false },
    { clave: 'conjunto_cerrado', etiqueta: '¿Está en conjunto cerrado con vigilancia?', tipo: 'si_no', privada: false },
    { clave: 'areas_comunes', etiqueta: '¿Qué áreas comunes tiene el conjunto?', tipo: 'texto_corto', privada: false },
    AMOBLADO,
  ],
  casa: [
    ...COMUNES,
    ...SERVICIOS,
    { clave: 'conjunto_cerrado', etiqueta: '¿Está en conjunto cerrado con vigilancia?', tipo: 'si_no', privada: false },
    { clave: 'ampliacion', etiqueta: '¿Tiene ampliación o posibilidad de ampliar?', tipo: 'si_no', privada: false },
    { clave: 'areas_externas', etiqueta: '¿Qué tiene afuera? (patio, terraza, parrillera…)', tipo: 'texto_corto', privada: false },
    AMOBLADO,
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

/* Clave de la marca «no sé» para preguntas con `destino` (hoy el mínimo):
 * su columna no puede guardar el desconocimiento (`null` = pendiente), así
 * que se guarda `extras['<clave>_nose'] = NO_SE`. Cuenta como respondida
 * con el dato vacío; al responder de verdad se borra la marca. */
export function claveNoSe(clave: string): string {
  return `${clave}_nose`;
}

/* Respondida = presente y no vacía. El `false` de un Sí/No cuenta (dijo
 * que no); `NO_SE` y las intermedias (`trato`) también: no vuelve a
 * preguntarse aunque el dato siga vacío. Las preguntas con `destino`
 * (mínimo) miran su columna, o la marca `claveNoSe` si se respondió
 * «No lo sé». */
export function preguntaRespondida(pregunta: PreguntaAsk, extras: ExtrasAsk, precioMinimo?: number | null): boolean {
  if (pregunta.destino === 'precioMinimo') {
    return (precioMinimo ?? 0) > 0 || extras[claveNoSe(pregunta.clave)] === NO_SE;
  }
  const v = extras[pregunta.clave];
  if (v === undefined || v === null) return false;
  if (typeof v === 'string') return v.trim().length > 0;
  return true;
}

/* % de ficha completa + claves que faltan (aviso visual, nunca bloquea).
 * Con `columnas` suma además el paso ubicación+residencia y los numéricos
 * (lógica en `./pasos-ask`; se reexporta aquí para no romper imports). */
export { calcularCompletitud } from './pasos-ask';
