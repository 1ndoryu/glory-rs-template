// Vista Mensajes (169A-5, 279A-2 F5, 289A-1 sesiones): bandeja del chat
// con IA + configuración + clientes de la dueña + uso/auditoría +
// vinculación WhatsApp. Pestañas internas para no ocupar más huecos del menú.
// [07AA-7] suma la pestaña Marketplace: borradores por chat del puente.

import { usePestanaPersistida } from '../../hooks/app/use-pestana-persistida';
import { BandejaMensajes } from './bandeja-mensajes';
import { ChatsMarketplace } from './chats-marketplace';
import { ClientesDuena } from './clientes-duena';
import { ConfigChat } from './config-chat';
import { SesionesWhatsapp } from './sesiones-whatsapp';
import { UsoAuditoria } from './uso-auditoria';
import { Button } from '@/components/ui/button';

type Pestana = 'bandeja' | 'clientes' | 'marketplace' | 'whatsapp' | 'uso' | 'config';

const PESTANAS: { clave: Pestana; titulo: string }[] = [
  { clave: 'bandeja', titulo: 'Bandeja' },
  { clave: 'clientes', titulo: 'Clientes' },
  { clave: 'marketplace', titulo: 'Marketplace' },
  { clave: 'whatsapp', titulo: 'WhatsApp' },
  { clave: 'uso', titulo: 'Uso y auditoría' },
  { clave: 'config', titulo: 'Configuración del chat' },
];

/* [09AA-27] La pestaña abierta sobrevive a recargas (admin:mensajes:pestana). */
const CLAVES_PESTANAS = PESTANAS.map((p) => p.clave);

export function VistaMensajes() {
  const [pestana, setPestana] = usePestanaPersistida<Pestana>('admin:mensajes:pestana', CLAVES_PESTANAS, 'bandeja');
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
      <div className={pestana === 'marketplace' ? '' : 'hidden'}>
        <ChatsMarketplace />
      </div>
      <div className={pestana === 'whatsapp' ? '' : 'hidden'}>
        <SesionesWhatsapp />
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
