import { miniaturaDe } from '@/domain/inmueble';
import { urlAbsoluta } from '@/data/inmuebles/api';
import type { ChatResumen } from '@/data/chat/marketplace-chats';
import { Badge } from '@/components/ui/badge';
import { MiniaturaFoto } from '@/components/ui/miniatura-foto';

/* [09AA-28] Vínculo del hilo con su inmueble: miniatura de la portada +
 * título (sin «Vinculado:», la miniatura ya lo dice), o «Sin ficha» si el
 * aviso no empareja. El título se parte en 2 líneas como mucho y el bloque
 * `min-w-0` deja que se encoja en vez de desbordar la fila (el Badge viejo
 * era `whitespace-nowrap`). Sin dato (backend viejo) no pinta nada. */
export function VinculoInmueble({ chat }: { chat: ChatResumen | undefined }) {
  if (!chat) return null;
  if (chat.inmueble_vinculado) {
    const original = chat.inmueble_foto ? urlAbsoluta(chat.inmueble_foto) : undefined;
    return (
      <span className="flex min-w-0 items-center gap-2" title={chat.inmueble_vinculado}>
        <MiniaturaFoto
          key={original}
          src={original ? miniaturaDe(original) : undefined}
          respaldo={original}
          titulo={chat.inmueble_vinculado}
          tamano="h-9 w-9"
        />
        <span className="line-clamp-2 min-w-0 break-words text-xs text-muted-foreground">{chat.inmueble_vinculado}</span>
      </span>
    );
  }
  if (chat.aviso_conocido === false) {
    return (
      <Badge variant="destructive" className="text-[10px]">
        Sin ficha
      </Badge>
    );
  }
  return null;
}
