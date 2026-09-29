// Hilo abierto (169A-5): historial + responder como humano + tomar/soltar
// la IA + cerrar. Responder toma el hilo (lo hace el backend).

import { useEffect, useRef, useState } from 'react';
import { Bot, BotOff, CheckCheck } from 'lucide-react';
import type { MensajeServidor } from '../../data/chat/cliente-chat';
import type { ResumenSesion } from '../../data/chat/cliente-admin';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/lib/utils';

function etiquetaRemitente(remitente: string): string {
  if (remitente === 'staff') return 'Tú';
  if (remitente === 'ai') return 'IA';
  if (remitente === 'system') return 'Sistema';
  return 'Visitante';
}

/* [289A-8] Lado por remitente: el Visitante a la izquierda, la IA y lo
 * propio (staff) a la derecha (la IA en gris, lo propio destacado),
 * sistema centrado. */
function claseLado(remitente: string): string {
  if (remitente === 'system') return 'mx-auto max-w-[90%] bg-transparent text-center text-xs text-muted-foreground';
  if (remitente === 'ai') return 'ml-auto bg-muted';
  if (remitente === 'staff') return 'ml-auto bg-primary text-primary-foreground';
  return 'mr-auto border-primary/40 bg-primary/10';
}

/* [289A-2] Fecha corta del mensaje (created_at ISO del servidor). */
function formatoFecha(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return d.toLocaleString('es-VE', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' });
}

export function HiloMensajes({
  sesion,
  hilo,
  respondiendo,
  alResponder,
  alCambiar,
}: {
  sesion: ResumenSesion;
  hilo: MensajeServidor[];
  respondiendo: boolean;
  alResponder: (texto: string) => Promise<boolean>;
  alCambiar: (cambio: { aiEnabled?: boolean; status?: 'open' | 'escalated' | 'closed' }) => Promise<void>;
}) {
  const [texto, setTexto] = useState('');
  const cajaRef = useRef<HTMLDivElement>(null);

  /* [289A-9] Orden normal: el hilo crece hacia abajo; acompaña al fondo
   * solo si ya estabas cerca de él (no roba el scroll al releer arriba). */
  useEffect(() => {
    const caja = cajaRef.current;
    if (!caja) return;
    const cercaFondo = caja.scrollHeight - caja.scrollTop - caja.clientHeight < 160;
    if (cercaFondo) caja.scrollTop = caja.scrollHeight;
  }, [hilo]);

  async function enviar(): Promise<void> {
    if (await alResponder(texto)) setTexto('');
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex flex-wrap items-center gap-2 border-b px-4 py-2">
        <Badge variant={sesion.status === 'open' ? 'default' : sesion.status === 'escalated' ? 'destructive' : 'secondary'}>
          {sesion.status}
        </Badge>
        <Badge variant={sesion.ai_enabled ? 'outline' : 'secondary'}>
          {sesion.ai_enabled ? 'IA activa' : 'IA apagada'}
        </Badge>
        {sesion.contact && <span className="text-xs text-muted-foreground">{sesion.contact}</span>}
        {/* [289A-2] Número del cliente en la cabecera del hilo. */}
        {sesion.telefono && <span className="text-xs font-medium">{sesion.telefono}</span>}
        <span className="ml-auto flex gap-1">
          {sesion.ai_enabled ? (
            <Button variant="outline" size="sm" title="Tomar el hilo a mano" onClick={() => void alCambiar({ aiEnabled: false })}>
              <BotOff className="h-3.5 w-3.5" /> Tomar
            </Button>
          ) : (
            <Button variant="outline" size="sm" title="Devolver el hilo a la IA" onClick={() => void alCambiar({ aiEnabled: true })}>
              <Bot className="h-3.5 w-3.5" /> Soltar IA
            </Button>
          )}
          {sesion.status !== 'closed' && (
            <Button variant="ghost" size="sm" title="Cerrar el hilo" onClick={() => void alCambiar({ status: 'closed' })}>
              <CheckCheck className="h-3.5 w-3.5" /> Cerrar
            </Button>
          )}
        </span>
      </div>
      <div ref={cajaRef} className="min-h-0 flex-1 space-y-2 overflow-y-auto px-4 py-3">
        {hilo.length === 0 && <p className="text-sm text-muted-foreground">Sin mensajes todavía.</p>}
        {hilo.map((m) => (
          <div key={m.id} className={cn('max-w-[85%] rounded-md border px-3 py-2 text-sm', claseLado(m.sender))}>
            <p className="mb-0.5 text-[11px] opacity-70" title={m.created_at}>
              {etiquetaRemitente(m.sender)}
              {formatoFecha(m.created_at) && ` · ${formatoFecha(m.created_at)}`}
            </p>
            <p className="whitespace-pre-wrap">{m.body}</p>
          </div>
        ))}
      </div>
      <div className="flex gap-2 border-t px-4 py-3">
        <Textarea
          value={texto}
          onChange={(e) => setTexto(e.target.value)}
          placeholder="Responder como humano… (toma el hilo)"
          rows={2}
          className="resize-none"
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault();
              void enviar();
            }
          }}
        />
        <Button onClick={() => void enviar()} disabled={respondiendo || !texto.trim()}>
          {respondiendo ? '…' : 'Enviar'}
        </Button>
      </div>
    </div>
  );
}
