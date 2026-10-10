import { useState } from 'react';
import { Building2 } from 'lucide-react';

/* [09AA-28] Miniatura de la portada de un inmueble (antes `Foto` en
 * `tabla/celdas-tabla-inmuebles.tsx`): la comparten la tabla del admin y el
 * panel de chats de Marketplace. `respaldo` es la URL a la que se cae si la
 * primera falla (la `min160-` puede no existir en fotos viejas: entonces se
 * pide el original); sin ninguna que cargue se pinta el icono, no un hueco.
 * El llamador pone `key={src}` para reiniciar el estado al cambiar la foto. */
export function MiniaturaFoto({
  src,
  respaldo,
  titulo,
  tamano = 'h-14 w-20',
}: {
  src?: string;
  respaldo?: string;
  titulo: string;
  tamano?: string;
}) {
  const [fallos, setFallos] = useState(0);
  const actual = [src, respaldo].filter((u): u is string => !!u && u !== '')[fallos];
  if (!actual) {
    return (
      <div className={`flex shrink-0 items-center justify-center rounded-md bg-muted ${tamano}`}>
        <Building2 className="h-5 w-5 text-muted-foreground" />
      </div>
    );
  }
  return (
    <img
      src={actual}
      alt={titulo}
      className={`shrink-0 rounded-md object-cover ${tamano}`}
      onError={() => setFallos((n) => n + 1)}
    />
  );
}
