import type { ExtrasAsk, PreguntaAsk } from './ficha-ask';
import type { TipoInmueble } from './inmueble';
import { CHECKLIST_ASK, NO_SE, claveNoSe, preguntaRespondida } from './ficha-ask';

/* Columnas del inmueble que /ask puede rellenar (el resto llega del import
 * o del formulario admin; habitaciones/baños ya se piden al publicar). */
export interface ColumnasAsk {
  ubicacion: string;
  residencia: string;
  metros: number;
  metrosTerreno: number;
  puestos: number;
}

export type ColumnaNumericaAsk = 'metros' | 'metrosTerreno' | 'puestos';

/* Respuesta ubicación+residencia en un solo paso (dos campos). */
export interface ValorUbicacion {
  ubicacion: string;
  residencia: string;
}

/* Un paso del cuestionario: pregunta de ficha (`extras`), paso inteligente
 * de ubicación+residencia (rellena columnas del inmueble) o numérico
 * condicional (solo aparece cuando la columna falta). */
export type PasoAsk =
  | { kind: 'ficha'; pregunta: PreguntaAsk }
  | { kind: 'ubicacion' }
  | { kind: 'columna'; columna: ColumnaNumericaAsk; etiqueta: string; ayuda: string; entero: boolean; unidad: string };

/* Guardia de exhaustividad (279A-6): si mañana se añade un `kind` de paso,
 * el compilador obliga a atenderlo donde se use; en runtime nunca se llama. */
export function assertNunca(valor: never): never {
  throw new Error(`Paso /ask no soportado: ${JSON.stringify(valor)}`);
}

/* Numéricos por tipo (Record, no if-chain: un tipo nuevo falla en
 * compilación en vez de heredar numéricos ajenos en silencio).
 * Construcción salvo terreno, parcela en casa y terreno, puestos salvo
 * terreno (no hay dónde estacionar). */
const COLUMNAS_POR_TIPO: Record<TipoInmueble, ColumnaNumericaAsk[]> = {
  apartamento: ['metros', 'puestos'],
  townhouse: ['metros', 'puestos'],
  casa: ['metros', 'metrosTerreno', 'puestos'],
  local: ['metros', 'puestos'],
  terreno: ['metrosTerreno'],
};

export function columnasAplicables(tipo: TipoInmueble): ColumnaNumericaAsk[] {
  return COLUMNAS_POR_TIPO[tipo];
}

const ETIQUETAS_COLUMNA: Record<ColumnaNumericaAsk, { etiqueta: string; ayuda: string; entero: boolean; unidad: string }> = {
  metros: { etiqueta: '¿Cuántos m² construidos tiene?', ayuda: 'Solo el número.', entero: false, unidad: 'm²' },
  metrosTerreno: { etiqueta: '¿Cuántos m² de terreno tiene?', ayuda: 'Solo el número.', entero: false, unidad: 'm²' },
  puestos: { etiqueta: '¿Cuántos puestos de estacionamiento tiene?', ayuda: '0 = ninguno.', entero: true, unidad: 'puestos' },
};

/* Nombre con artículo para preguntar en concreto (279A-5): "¿Cuántos m²
 * tiene el apartamento?" en vez del genérico "construidos". */
const NOMBRE_TIPO: Record<TipoInmueble, string> = {
  apartamento: 'el apartamento',
  townhouse: 'el townhouse',
  casa: 'la casa',
  local: 'el local',
  terreno: 'el terreno',
};

function etiquetaColumna(tipo: TipoInmueble, columna: ColumnaNumericaAsk): string {
  const nombre = NOMBRE_TIPO[tipo];
  if (columna === 'metrosTerreno' && tipo === 'terreno') return `¿Cuántos m² tiene ${nombre}?`;
  if (columna === 'metrosTerreno') return `¿Cuántos m² de terreno tiene ${nombre}?`;
  if (columna === 'puestos') return `¿Cuántos puestos de estacionamiento tiene ${nombre}?`;
  return `¿Cuántos m² tiene ${nombre}?`;
}

function ubicacionCompleta(columnas: ColumnasAsk): boolean {
  return columnas.ubicacion.trim() !== '' && columnas.residencia.trim() !== '';
}

/* Marcas «no sé» para pasos que no guardan en `extras` (279A-7, todas las
 * preguntas llevan No lo sé): la columna no puede guardar el
 * desconocimiento, así que vive en `extras` con clave snake. El backend
 * (`update`/`set_ficha`) borra la marca al llegar el dato real; el front
 * la borra en local al responder de verdad. */
export const CLAVE_NO_SE_UBICACION = 'ubicacion_nose';

const CLAVE_NO_SE_COLUMNA: Record<ColumnaNumericaAsk, string> = {
  metros: 'metros_nose',
  metrosTerreno: 'metros_terreno_nose',
  puestos: 'puestos_nose',
};

/* Marca «no sé» de un paso, o `null` si el No lo sé se guarda como valor
 * (preguntas de ficha sin `destino`). */
export function marcaNoSePaso(paso: PasoAsk): string | null {
  if (paso.kind === 'ubicacion') return CLAVE_NO_SE_UBICACION;
  if (paso.kind === 'columna') return CLAVE_NO_SE_COLUMNA[paso.columna];
  return paso.pregunta.destino ? claveNoSe(paso.pregunta.clave) : null;
}

/* Pregunta inteligente ubicación vs residencia: confirma lo que hay y pide
 * lo que falta, nombrando los valores para reconocerlos al responder. */
export function etiquetaUbicacion(columnas: ColumnasAsk): string {
  const u = columnas.ubicacion.trim();
  const r = columnas.residencia.trim();
  if (u && r) return `La ubicación dice «${u}» y la residencia «${r}», ¿está bien? Corrige lo que haga falta.`;
  if (u) return `La ubicación dice «${u}», ¿es correcta? Y dime la residencia o conjunto.`;
  if (r) return `La residencia dice «${r}», ¿es correcta? Y dime la ubicación (zona).`;
  return '¿Dónde queda? Dime la ubicación (zona) y la residencia o conjunto.';
}

/* Foto de pasos al elegir: ubicación primero, luego la ficha fija y al
 * final solo los numéricos que faltan (los que ya tienen valor no estorban). */
export function pasosPara(tipo: TipoInmueble, columnas: ColumnasAsk): PasoAsk[] {
  const pasos: PasoAsk[] = [{ kind: 'ubicacion' }];
  for (const pregunta of CHECKLIST_ASK[tipo] ?? []) pasos.push({ kind: 'ficha', pregunta });
  for (const columna of columnasAplicables(tipo)) {
    if (!(columnas[columna] > 0)) {
      const { ayuda, entero, unidad } = ETIQUETAS_COLUMNA[columna];
      pasos.push({ kind: 'columna', columna, etiqueta: etiquetaColumna(tipo, columna), ayuda, entero, unidad });
    }
  }
  return pasos;
}

/* Orden aleatorio por sesión (Fisher-Yates puro): /ask no es una lista
 * fija, cada vez sale en distinto orden. Se mezcla una vez al elegir. */
export function mazclarPasos(pasos: PasoAsk[]): PasoAsk[] {
  const m = [...pasos];
  for (let i = m.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [m[i], m[j]] = [m[j], m[i]];
  }
  return m;
}

export function pasoRespondido(
  paso: PasoAsk,
  columnas: ColumnasAsk,
  extras: ExtrasAsk,
  precioMinimo?: number | null,
): boolean {
  if (paso.kind === 'ubicacion') {
    return ubicacionCompleta(columnas) || extras[CLAVE_NO_SE_UBICACION] === NO_SE;
  }
  if (paso.kind === 'columna') {
    return columnas[paso.columna] > 0 || extras[CLAVE_NO_SE_COLUMNA[paso.columna]] === NO_SE;
  }
  if (paso.kind === 'ficha') return preguntaRespondida(paso.pregunta, extras, precioMinimo);
  return assertNunca(paso);
}

/* Progreso sobre la foto de pasos (denominador estable en la sesión). */
export function progresoPasos(
  pasos: PasoAsk[],
  columnas: ColumnasAsk,
  extras: ExtrasAsk,
  precioMinimo?: number | null,
): { porcentaje: number; faltan: number } {
  if (pasos.length === 0) return { porcentaje: 100, faltan: 0 };
  const faltan = pasos.filter((p) => !pasoRespondido(p, columnas, extras, precioMinimo)).length;
  return { porcentaje: Math.round(((pasos.length - faltan) / pasos.length) * 100), faltan };
}

/* Respuesta pura sobre la ficha (sin fetch): `null` = no aplica y borra la
 * clave para que no cuente como respondida. `NO_SE` en una pregunta con
 * `destino` guarda la marca `claveNoSe` (la columna no puede guardar el
 * desconocimiento); una respuesta real borra la marca. */
export function extrasTrasRespuesta(
  ficha: { extras: ExtrasAsk; precioMinimo: number | null },
  pregunta: PreguntaAsk,
  valor: string | number | boolean | null,
): { extras: ExtrasAsk; precioMinimo: number | null } {
  const extras: ExtrasAsk = { ...ficha.extras };
  let precioMinimo = ficha.precioMinimo;
  if (valor === null) {
    if (pregunta.destino === 'precioMinimo') {
      precioMinimo = null;
      delete extras[claveNoSe(pregunta.clave)];
    } else delete extras[pregunta.clave];
  } else if (pregunta.destino === 'precioMinimo') {
    if (valor === NO_SE) extras[claveNoSe(pregunta.clave)] = NO_SE;
    else {
      precioMinimo = typeof valor === 'number' ? valor : Number(valor);
      delete extras[claveNoSe(pregunta.clave)];
    }
  } else {
    extras[pregunta.clave] = typeof valor === 'string' ? valor.trim() : valor;
  }
  return { extras, precioMinimo };
}

/* % de ficha completa + claves que faltan (aviso visual, nunca bloquea).
 * Con `columnas` suma además el paso ubicación+residencia y los numéricos
 * condicionales (m², terreno, puestos): el semáforo del panel y la lista
 * /ask pasan el inmueble para el % completo; sin él solo cuenta `extras`. */
export function calcularCompletitud(
  tipo: TipoInmueble,
  extras: ExtrasAsk,
  precioMinimo?: number | null,
  columnas?: ColumnasAsk | null,
): { porcentaje: number; faltan: string[] } {
  const lista = CHECKLIST_ASK[tipo] ?? [];
  const faltan = lista.filter((p) => !preguntaRespondida(p, extras, precioMinimo)).map((p) => p.clave);
  if (columnas) {
    if (!ubicacionCompleta(columnas) && extras[CLAVE_NO_SE_UBICACION] !== NO_SE) faltan.push('ubicacion_residencia');
    for (const c of columnasAplicables(tipo)) {
      if (!(columnas[c] > 0) && extras[CLAVE_NO_SE_COLUMNA[c]] !== NO_SE) faltan.push(c);
    }
  }
  const total = lista.length + (columnas ? 1 + columnasAplicables(tipo).length : 0);
  if (total === 0) return { porcentaje: 100, faltan: [] };
  const porcentaje = Math.round(((total - faltan.length) / total) * 100);
  return { porcentaje, faltan };
}
