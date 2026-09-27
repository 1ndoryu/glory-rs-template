import type { ExtrasAsk, PreguntaAsk, TipoInmueble } from './ficha-ask';
import { CHECKLIST_ASK, preguntaRespondida } from './ficha-ask';

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

/* Numéricos que aplican por tipo: construcción salvo terreno, parcela en
 * casa y terreno, puestos salvo terreno (no hay dónde estacionar). */
export function columnasAplicables(tipo: TipoInmueble): ColumnaNumericaAsk[] {
  if (tipo === 'terreno') return ['metrosTerreno'];
  if (tipo === 'casa') return ['metros', 'metrosTerreno', 'puestos'];
  return ['metros', 'puestos'];
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
  if (paso.kind === 'ubicacion') return ubicacionCompleta(columnas);
  if (paso.kind === 'columna') return columnas[paso.columna] > 0;
  return preguntaRespondida(paso.pregunta, extras, precioMinimo);
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
 * clave para que no cuente como respondida. */
export function extrasTrasRespuesta(
  ficha: { extras: ExtrasAsk; precioMinimo: number | null },
  pregunta: PreguntaAsk,
  valor: string | number | boolean | null,
): { extras: ExtrasAsk; precioMinimo: number | null } {
  const extras: ExtrasAsk = { ...ficha.extras };
  let precioMinimo = ficha.precioMinimo;
  if (valor === null) {
    if (pregunta.clave === 'precio_minimo') precioMinimo = null;
    else delete extras[pregunta.clave];
  } else if (pregunta.clave === 'precio_minimo') {
    precioMinimo = typeof valor === 'number' ? valor : Number(valor);
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
    if (!ubicacionCompleta(columnas)) faltan.push('ubicacion_residencia');
    for (const c of columnasAplicables(tipo)) {
      if (!(columnas[c] > 0)) faltan.push(c);
    }
  }
  const total = lista.length + (columnas ? 1 + columnasAplicables(tipo).length : 0);
  if (total === 0) return { porcentaje: 100, faltan: [] };
  const porcentaje = Math.round(((total - faltan.length) / total) * 100);
  return { porcentaje, faltan };
}
