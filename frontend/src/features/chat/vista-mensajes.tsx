// Vista Mensajes (169A-5, 279A-2 F5): bandeja del chat con IA +
// configuración + clientes de la dueña + uso/auditoría. Pestañas internas
// para no ocupar más huecos del menú.

import { useState } from 'react';
import { BandejaMensajes } from './bandeja-mensajes';
import { ClientesDuena } from './clientes-duena';
import { ConfigChat } from './config-chat';
import { UsoAuditoria } from './uso-auditoria';
import { Button } from '@/components/ui/button';

type Pestana = 'bandeja' | 'clientes' | 'uso' | 'config';

const PESTANAS: { clave: Pestana; titulo: string }[] = [
  { clave: 'bandeja', titulo: 'Bandeja' },
  { clave: 'clientes', titulo: 'Clientes' },
  { clave: 'uso', titulo: 'Uso y auditoría' },
  { clave: 'config', titulo: 'Configuración del chat' },
];

export function VistaMensajes() {
  const [pestana, setPestana] = useState<Pestana>('bandeja');
  return (
    <div>
      <div className="mb-4 flex flex-wrap gap-2">
        {PESTANAS.map((p) => (
          <Button
            key={p.clave}
            variant={pestana === p.clave ? 'default' : 'outline'}
            size="sm"
            onClick={() => setPestana(p.clave)}
          >
            {p.titulo}
          </Button>
        ))}
      </div>
      {/* Montadas siempre (ocultas): cambiar de pestaña no pierde el
       * cliente elegido ni el texto a medio escribir. */}
      <div className={pestana === 'bandeja' ? '' : 'hidden'}>
        <BandejaMensajes />
      </div>
      <div className={pestana === 'clientes' ? '' : 'hidden'}>
        <ClientesDuena />
      </div>
      <div className={pestana === 'uso' ? '' : 'hidden'}>
        <UsoAuditoria />
      </div>
      <div className={pestana === 'config' ? '' : 'hidden'}>
        <ConfigChat />
      </div>
    </div>
  );
}
