// Lógica de tema pura (sin React): sirve igual en web y en el futuro
// wrapper móvil. `dark` en <html> activa las variantes `dark:` de Tailwind.

import { sistemaQuiereOscuro } from '../platform/ventana';

export type Tema = 'sistema' | 'claro' | 'oscuro';

const CLAVE_TEMA = 'inmobiliaria:tema:v1';

export function leerTemaGuardado(): Tema {
  try {
    const v = localStorage.getItem(CLAVE_TEMA);
    if (v === 'claro' || v === 'oscuro' || v === 'sistema') return v;
  } catch {
    // Sin almacenamiento: se usa el sistema.
  }
  return 'sistema';
}

export function guardarTema(t: Tema): void {
  try {
    localStorage.setItem(CLAVE_TEMA, t);
  } catch {
    // El tema en memoria sigue valiendo para la sesión.
  }
}

export function prefiereOscuro(): boolean {
  /* [08AA-23] Lectura vía plataforma (sin `window` directo aquí). */
  return sistemaQuiereOscuro();
}

export function resolverOscuro(t: Tema): boolean {
  return t === 'oscuro' || (t === 'sistema' && prefiereOscuro());
}

export function aplicarTema(t: Tema): void {
  document.documentElement.classList.toggle('dark', resolverOscuro(t));
}
