// Supresión de eco entre sesiones (289A-1): lo enviado por /send a un
// número vuelve como inbound en la sesión del otro número (los dos números
// se escriben entre sí en las pruebas y en producción). Sin esto, la
// respuesta IA se re-ingesta como mensaje `client` y la IA se responde a sí
// misma en bucle (caso real 2026-09-28: el texto IA de wa_b reapareció como
// `client` en wa_a 11s después del envío).
const ENVIADOS = new Map();
const TTL_MS = 180_000;
const MAX = 500;

function clave(numero, texto) {
  return `${(numero ?? "").replace(/\D/g, "")}|${(texto ?? "").trim()}`;
}

/* Se registra al enviar por /send (clave = destino + texto): el eco vuelve
 * con ese mismo número como remitente y el mismo texto. */
export function registrarEnviado(destino, texto) {
  const k = clave(destino, texto);
  if (k.length < 4) return;
  if (ENVIADOS.size >= MAX) {
    const ahora = Date.now();
    for (const [c, exp] of ENVIADOS) if (exp < ahora) ENVIADOS.delete(c);
  }
  ENVIADOS.set(k, Date.now() + TTL_MS);
}

/* Un solo uso: el eco llega una vez; si el usuario reenvía el mismo texto
 * después, pasa. Vencido también pasa. */
export function esEco(remitente, texto) {
  const exp = ENVIADOS.get(clave(remitente, texto));
  if (exp === undefined) return false;
  ENVIADOS.delete(clave(remitente, texto));
  return exp >= Date.now();
}
