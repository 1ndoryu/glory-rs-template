import { useCallback, useState } from 'react';

/* [09AA-27] Pestaña/filtro activo que sobrevive a recargas (localStorage).
 * Solo se restaura un valor que siga en `validas`: si se renombra o quita una
 * pestaña, el valor viejo guardado cae al `porDefecto` en vez de dejar la vista
 * sin ninguna activa. La escritura va en el manejador (no dentro del updater de
 * `setState`, que `StrictMode` ejecuta dos veces). Sin almacenamiento, el estado
 * en memoria sigue valiendo para la sesión. */
export function usePestanaPersistida<T extends string>(
  clave: string,
  validas: readonly T[],
  porDefecto: T,
): readonly [T, (siguiente: T) => void] {
  const [valor, setValor] = useState<T>(() => {
    try {
      const guardado = localStorage.getItem(clave);
      return guardado !== null && (validas as readonly string[]).includes(guardado) ? (guardado as T) : porDefecto;
    } catch {
      return porDefecto;
    }
  });

  const elegir = useCallback(
    (siguiente: T) => {
      setValor(siguiente);
      try {
        localStorage.setItem(clave, siguiente);
      } catch {
        // Sin almacenamiento la pestaña no persiste, pero la vista funciona.
      }
    },
    [clave],
  );

  return [valor, elegir] as const;
}
