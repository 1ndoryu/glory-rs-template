// Tab de Logs del puente (09AA-5): tabla de eventos del flujo
// borrador/regenerar/IA con nivel y estado; click en la fila abre el modal
// con el detalle completo (campos estructurados, sin PII).

import { useState } from 'react';
import { useMarketplaceLogs, type FiltroNivel } from './use-marketplace-logs';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import type { LogEvento, NivelLog } from '../../data/chat/marketplace-logs';

function horaCorta(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString('es-VE', { dateStyle: 'short', timeStyle: 'medium' });
}

/* Prioridad pedida por ella: el nivel ES la prioridad (error=alta,
 * warn=media, info=baja). */
function prioridad(nivel: NivelLog): string {
  return nivel === 'error' ? 'alta' : nivel === 'warn' ? 'media' : 'baja';
}

function varianteNivel(nivel: NivelLog): 'default' | 'secondary' | 'destructive' {
  return nivel === 'error' ? 'destructive' : nivel === 'warn' ? 'default' : 'secondary';
}

const FILTROS: { valor: FiltroNivel; texto: string }[] = [
  { valor: 'todos', texto: 'Todos' },
  { valor: 'info', texto: 'Info' },
  { valor: 'warn', texto: 'Warn' },
  { valor: 'error', texto: 'Error' },
];

export function LogsMarketplace() {
  const { eventos, nivel, setNivel, pausado, setPausado, error, recargar } = useMarketplaceLogs();
  const [detalle, setDetalle] = useState<LogEvento | null>(null);

  return (
    <section>
      <div className="mb-2 flex flex-wrap items-center gap-2">
        {FILTROS.map((f) => (
          <Button
            key={f.valor}
            variant={nivel === f.valor ? 'default' : 'outline'}
            size="sm"
            onClick={() => setNivel(f.valor)}
          >
            {f.texto}
          </Button>
        ))}
        <span className="mx-1 h-4 w-px bg-border" />
        <Button variant="outline" size="sm" onClick={() => setPausado(!pausado)}>
          {pausado ? 'Reanudar' : 'Pausar'}
        </Button>
        <Button variant="outline" size="sm" onClick={() => recargar()}>
          Recargar
        </Button>
        <span className="text-xs text-muted-foreground">
          {pausado ? 'pausado' : 'auto-refresh 5s'} · {eventos.length} eventos
        </span>
      </div>
      {error && (
        <p className="mb-2 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Hora</TableHead>
            <TableHead>Nivel</TableHead>
            <TableHead>Estado</TableHead>
            <TableHead>Evento</TableHead>
            <TableHead>Mensaje</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {eventos.map((e) => (
            <TableRow key={e.id} className="cursor-pointer" onClick={() => setDetalle(e)}>
              <TableCell className="whitespace-nowrap">{horaCorta(e.ts)}</TableCell>
              <TableCell>
                <Badge variant={varianteNivel(e.nivel)} className="text-[10px]">
                  {e.nivel.toUpperCase()}
                </Badge>
              </TableCell>
              <TableCell>
                <Badge variant="outline" className="text-[10px]">
                  {e.estado}
                </Badge>
              </TableCell>
              <TableCell className="font-mono text-xs">{e.evento}</TableCell>
              <TableCell className="max-w-64 truncate">{e.mensaje}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {eventos.length === 0 && !error && (
        <p className="rounded-md border border-dashed px-4 py-8 text-center text-sm text-muted-foreground">
          Sin eventos todavía. Aparecen cuando el puente genera, cachea o regenera borradores.
        </p>
      )}
      <Dialog open={detalle !== null} onOpenChange={(abierto) => { if (!abierto) setDetalle(null); }}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="font-mono text-sm">{detalle?.evento}</DialogTitle>
            <DialogDescription>
              {detalle && horaCorta(detalle.ts)} · prioridad {detalle && prioridad(detalle.nivel)}
            </DialogDescription>
          </DialogHeader>
          {detalle && (
            <dl className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 text-xs">
              <dt className="text-muted-foreground">Nivel</dt>
              <dd>
                <Badge variant={varianteNivel(detalle.nivel)} className="text-[10px]">
                  {detalle.nivel.toUpperCase()}
                </Badge>
              </dd>
              <dt className="text-muted-foreground">Estado</dt>
              <dd>
                <Badge variant="outline" className="text-[10px]">
                  {detalle.estado}
                </Badge>
              </dd>
              <dt className="text-muted-foreground">Mensaje</dt>
              <dd className="break-words">{detalle.mensaje}</dd>
              <dt className="text-muted-foreground">Campos</dt>
              <dd>
                <pre className="max-h-64 overflow-auto rounded-md bg-muted/50 p-2 font-mono text-[11px] break-all whitespace-pre-wrap">
                  {JSON.stringify(detalle.campos, null, 2)}
                </pre>
              </dd>
            </dl>
          )}
        </DialogContent>
      </Dialog>
    </section>
  );
}
