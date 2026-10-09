// Formulario de vínculo manual hilo→inmueble (09AA-19 F7c): elige el
// inmueble destino y confirma el ID de aviso (sugerido desde el hilo).
// Se monta con `key={hilo}` para reiniciar el form al cambiar de hilo.

import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { esMarketplaceIdValido, type Inmueble } from '@/domain/inmueble';
import { CLASE_SELECT, Etiqueta } from '@/features/inmuebles/campos-formulario';

export function VincularHilo({
  inmuebles,
  sugerido,
  vinculando,
  alVincular,
}: {
  inmuebles: Inmueble[];
  /* Dígitos del aviso si el hilo los trae; si no, cadena vacía. */
  sugerido: string;
  vinculando: boolean;
  alVincular: (inmuebleId: string, marketplaceId: string) => void;
}) {
  const [inmuebleId, setInmuebleId] = useState('');
  const [marketplaceId, setMarketplaceId] = useState(sugerido);
  const valido = esMarketplaceIdValido(marketplaceId);

  return (
    <div className="space-y-3 rounded-md border p-3">
      <div className="space-y-1">
        <Etiqueta>Inmueble destino</Etiqueta>
        <select className={CLASE_SELECT} value={inmuebleId} onChange={(e) => setInmuebleId(e.target.value)}>
          <option value="">Elige el inmueble…</option>
          {inmuebles.map((i) => (
            <option key={i.id} value={i.id}>
              {i.titulo || 'Sin título'}
              {i.marketplaceId ? ` (aviso ${i.marketplaceId})` : ''}
            </option>
          ))}
        </select>
      </div>
      <div className="space-y-1">
        <Etiqueta>ID aviso Marketplace</Etiqueta>
        <Input
          value={marketplaceId}
          onChange={(e) => setMarketplaceId(e.target.value)}
          placeholder="1234567890 (/marketplace/item/<id>)"
          inputMode="numeric"
        />
        {!valido && marketplaceId.trim() !== '' && (
          <p className="text-xs text-destructive">Solo dígitos (o pega la URL del aviso).</p>
        )}
      </div>
      <Button disabled={!inmuebleId || !valido || vinculando} onClick={() => alVincular(inmuebleId, marketplaceId)}>
        {vinculando ? 'Vinculando…' : 'Vincular hilo'}
      </Button>
    </div>
  );
}
