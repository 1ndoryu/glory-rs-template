/* Cliente del chat del visitante (169A-1, F5): base de URLs, sesion
 * persistente y REST contra /api/agent/* del backend MN-Inmobiliaria.
 * El realtime va por WS en el hook; aqui solo REST + tipos.
 * [169A-6] Se suman `pedirInfo` (telefono/WhatsApp configurados) y
 * `enviarContacto` (el visitante deja nombre+telefono y staff lo llama). */

export interface MensajeServidor {
  id: string;
  session_id: string;
  sender: 'client' | 'ai' | 'staff' | 'system';
  body: string;
  sequence_num: number;
  created_at: string;
}

export interface RespuestaEnviar {
  ok: boolean;
  sequence_num: number;
  reply: string | null;
}

/* Mensaje listo para pintar: `propio` distingue visitante de IA/staff. */
export interface MensajeChat {
  id: string;
  propio: boolean;
  texto: string;
}

export function aChat(m: MensajeServidor): MensajeChat {
  return { id: m.id, propio: m.sender === 'client', texto: m.body };
}

const CLAVE_SESION = 'inmobiliaria:chat-sesion';

export function baseApi(): string {
  const env = import.meta.env as Record<string, string | undefined>;
  return (env['VITE_API_URL'] as string | undefined)?.trim().replace(/\/$/, '') || 'http://127.0.0.1:3110'; // [03AA-1] local en 3110: el 3000 lo ocupa glory-pulse.
}

export function urlWs(sesion: string): string {
  return `${baseApi().replace(/^http/, 'ws')}/api/agent/ws?session_id=${sesion}`;
}

/* La sesion sobrevive al cierre del navegador: el historial se recupera
 * del servidor al reconectar. Solo se rota si el id guardado es invalido. */
export function leerSesion(): string | null {
  try {
    const sesion = localStorage.getItem(CLAVE_SESION);
    return sesion && /^[0-9a-f-]{36}$/i.test(sesion) ? sesion : null;
  } catch {
    return null;
  }
}

export function crearSesion(): string {
  const sesion = crypto.randomUUID();
  try {
    localStorage.setItem(CLAVE_SESION, sesion);
  } catch {
    // Sin almacenamiento la sesion vive solo en memoria.
  }
  return sesion;
}

async function leerJson(respuesta: Response): Promise<unknown> {
  try {
    return (await respuesta.json()) as unknown;
  } catch {
    return null;
  }
}

export async function pedirHistorial(sesion: string): Promise<MensajeServidor[]> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${baseApi()}/api/agent/history?session_id=${sesion}&limit=50`);
  } catch {
    throw new Error('No se pudo contactar con la API: ¿arrancada?');
  }
  if (!respuesta.ok) throw new Error(`La API devolvió ${respuesta.status}.`);
  const cuerpo = await leerJson(respuesta);
  return Array.isArray(cuerpo) ? (cuerpo as MensajeServidor[]) : [];
}

export async function enviarRest(sesion: string, texto: string): Promise<RespuestaEnviar> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${baseApi()}/api/agent/messages`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_id: sesion, body: texto }),
    });
  } catch {
    throw new Error('No se pudo contactar con la API: ¿arrancada?');
  }
  if (!respuesta.ok) throw new Error(`La API devolvió ${respuesta.status}.`);
  const cuerpo = (await leerJson(respuesta)) as RespuestaEnviar | null;
  if (!cuerpo || typeof cuerpo.ok !== 'boolean') throw new Error('Respuesta inesperada de la API.');
  return cuerpo;
}

/* [169A-6] Datos publicos de contacto (`GET /api/agent/info`, sin auth).
 * Nombres en camelCase porque asi los devuelve el backend en esta ruta. */
export interface InfoAgente {
  contactoTelefono: string;
  whatsappUrl: string;
  iaHabilitada: boolean;
}

export async function pedirInfo(): Promise<InfoAgente | null> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${baseApi()}/api/agent/info`);
  } catch {
    return null;
  }
  if (!respuesta.ok) return null;
  const cuerpo = (await leerJson(respuesta)) as InfoAgente | null;
  if (!cuerpo || typeof cuerpo.contactoTelefono !== 'string') return null;
  return cuerpo;
}

/* [169A-6] El visitante deja su contacto (`POST
 * /api/agent/sesiones/:id/contacto`): staff lo ve en la bandeja y lo llama.
 * Devuelve `ok:true` o lanza con el motivo. */
export async function enviarContacto(
  sesion: string,
  nombre: string,
  telefono: string,
): Promise<void> {
  let respuesta: Response;
  try {
    respuesta = await fetch(`${baseApi()}/api/agent/sesiones/${sesion}/contacto`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ nombre, telefono, mensaje: 'Contacto dejado desde el chat.' }),
    });
  } catch {
    throw new Error('No se pudo contactar con la API: ¿arrancada?');
  }
  if (!respuesta.ok) throw new Error(`La API devolvió ${respuesta.status}.`);
}
