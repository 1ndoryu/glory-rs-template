import { useEffect, useRef, useState } from 'react';
import { EVENTO_SESION_EXPIRADA, borrarSesion, entrar, leerEmailSesion, leerToken } from '../../data/inmuebles/api';
import { suscribirEvento } from '../../platform/ventana';

// Sesión del admin contra la API: el token vive en localStorage (dura 1
// año y sobrevive al cierre del navegador). `email === null` = hay que entrar.
// [03AA-2] Auto-entrada solo-local: con `npm run dev` (`import.meta.env.DEV`
// es falso en el build de prod por construcción, así que ese bundle jamás
// la intenta) y hostname local, si no hay token se entra UNA vez con las
// credenciales de `frontend/.env.local` (gitignored: VITE_DEV_EMAIL +
// VITE_DEV_PASSWORD). Si falla (API caída, clave cambiada) se muestra el
// login normal sin reintentos; "Salir" no se sabotea porque el intento solo
// ocurre al montar sin token.
const HOSTS_LOCALES = new Set(['localhost', '127.0.0.1', 'inmobiliaria.localhost']);

function credencialesDev(): { email: string; clave: string } | null {
  if (!import.meta.env.DEV) return null;
  let host = '';
  try {
    host = window.location.hostname.toLowerCase();
  } catch {
    return null;
  }
  if (!HOSTS_LOCALES.has(host) && !host.endsWith('.localhost')) return null;
  const email = (import.meta.env.VITE_DEV_EMAIL as string | undefined)?.trim();
  const clave = import.meta.env.VITE_DEV_PASSWORD as string | undefined;
  if (!email || !clave) return null;
  return { email, clave };
}

export function useSesion() {
  const [email, setEmail] = useState<string | null>(() =>
    leerToken() ? (leerEmailSesion() ?? 'admin') : null,
  );
  const [autoEntrando, setAutoEntrando] = useState(false);
  const intentado = useRef(false);

  useEffect(() => {
    const alExpirar = () => setEmail(null);
    return suscribirEvento(EVENTO_SESION_EXPIRADA, alExpirar);
  }, []);

  useEffect(() => {
    if (leerToken() || intentado.current) return;
    const cred = credencialesDev();
    if (!cred) return;
    intentado.current = true;
    setAutoEntrando(true);
    /* Sin AbortController: `entrar` no acepta señal; el flag `vivo` evita
     * fijar estado si el componente se desmontó antes de responder. */
    let vivo = true;
    entrar(cred.email, cred.clave)
      .then((quien) => {
        if (vivo) setEmail(quien);
      })
      .catch(() => {
        // Falla en silencio: el formulario de login ya explica cómo entrar.
      })
      .finally(() => {
        if (vivo) setAutoEntrando(false);
      });
    return () => {
      vivo = false;
    };
  }, []);

  function salir() {
    borrarSesion();
    setEmail(null);
  }

  return { email, alEntrar: setEmail, salir, autoEntrando };
}
