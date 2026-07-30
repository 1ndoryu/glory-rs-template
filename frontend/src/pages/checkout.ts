/* wandori.us — Checkout Pages
 * Páginas de success/cancel para el flujo de Stripe checkout.
 * Success: confirma compra y muestra mensaje.
 * Cancel: informa que el pago fue cancelado. */

import { showProfile } from '../store';
import { updateMeta } from '../features/seo/meta';

/* === Checkout Success === */
export function renderCheckoutSuccess(): HTMLElement {
  showProfile.set(true);
  updateMeta({ title: 'compra exitosa', description: 'gracias por tu compra' });

  const page = document.createElement('div');
  page.className = 'checkout-resultado';

  const icono = document.createElement('div');
  icono.className = 'checkout-icono';
  icono.textContent = '✓';

  const titulo = document.createElement('h1');
  titulo.className = 'checkout-titulo';
  titulo.textContent = 'gracias por tu compra';

  const mensaje = document.createElement('p');
  mensaje.className = 'checkout-mensaje';
  mensaje.textContent = 'recibiras un correo con el enlace de descarga. si no lo ves en unos minutos, revisa tu carpeta de spam.';

  const btnInicio = document.createElement('a');
  btnInicio.className = 'boton';
  btnInicio.href = '/';
  btnInicio.textContent = 'volver al inicio';

  page.append(icono, titulo, mensaje, btnInicio);
  return page;
}

/* === Checkout Cancel === */
export function renderCheckoutCancel(): HTMLElement {
  showProfile.set(true);
  updateMeta({ title: 'pago cancelado', description: 'el pago fue cancelado' });

  const page = document.createElement('div');
  page.className = 'checkout-resultado';

  const icono = document.createElement('div');
  icono.className = 'checkout-icono';
  icono.textContent = '×';

  const titulo = document.createElement('h1');
  titulo.className = 'checkout-titulo';
  titulo.textContent = 'pago cancelado';

  const mensaje = document.createElement('p');
  mensaje.className = 'checkout-mensaje';
  mensaje.textContent = 'el pago no se completo. puedes intentar de nuevo cuando quieras.';

  const btnInicio = document.createElement('a');
  btnInicio.className = 'boton';
  btnInicio.href = '/';
  btnInicio.textContent = 'volver al inicio';

  page.append(icono, titulo, mensaje, btnInicio);
  return page;
}
