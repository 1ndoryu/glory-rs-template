/* [247A-11] Página de albaranes de compra BDP — Fase 1 (solo lectura).
 * Estructura coherente con ListaVentas/ListaGastos/ListaReservas.
 * Modo demo incluido para visualizar datos de prueba. */

import { useMemo, useState } from 'react';
import { Search, RefreshCw, Loader2, FilePen, CheckCircle } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { format } from 'date-fns';
import { useBdpDemoMode } from '@/hooks/useBdpDemoMode';
import {
  useBdpPurchaseNotes,
  useDraftBdpPurchaseNote,
  useReconcileBdpPurchaseNote,
  useSyncBdpPurchaseNotes,
} from '@/api/bdp';
import type { BdpPurchaseNote, BdpPurchaseNoteReconcileRequest } from '@/api/bdp';
import { mockPurchaseNotes } from './bdp-mocks';
import { BdpDemoToggle } from './BdpDemoToggle';
import { BdpComprasReconcileModal } from './BdpComprasReconcileModal';

function formatDate(value: string | null) {
  if (!value) return '—';
  try {
    return format(new Date(`${value}T00:00:00`), 'dd/MM/yyyy');
  } catch {
    return value;
  }
}

function formatCurrency(value: string | null) {
  if (!value) return '—';
  const n = Number(value);
  if (Number.isNaN(n)) return '—';
  return new Intl.NumberFormat('es-ES', { style: 'currency', currency: 'EUR' }).format(n);
}

function BdpCompras() {
  const queryClient = useQueryClient();
  const { demoMode, setDemoMode } = useBdpDemoMode();
  const [proveedor, setProveedor] = useState('');
  const [fechaDesde, setFechaDesde] = useState('');
  const [fechaHasta, setFechaHasta] = useState('');
  const [profileCode, setProfileCode] = useState('1');
  const [reconcileNote, setReconcileNote] = useState<BdpPurchaseNote | null>(null);

  const filters = useMemo(
    () => ({
      proveedor: proveedor || undefined,
      fecha_desde: fechaDesde || undefined,
      fecha_hasta: fechaHasta || undefined,
    }),
    [proveedor, fechaDesde, fechaHasta],
  );

  const { data, isLoading, error } = useBdpPurchaseNotes(filters, !demoMode);
  const syncMutation = useSyncBdpPurchaseNotes(queryClient);
  const draftMutation = useDraftBdpPurchaseNote(queryClient);
  const reconcileMutation = useReconcileBdpPurchaseNote(queryClient);

  const filteredDemoNotes = useMemo(() => {
    return mockPurchaseNotes.filter((n) => {
      const q = proveedor.trim().toLowerCase();
      const matchesProveedor =
        !q ||
        (n.nombre_proveedor?.toLowerCase().includes(q) ?? false) ||
        (n.codigo_proveedor?.toLowerCase().includes(q) ?? false);
      const matchesFecha =
        (!fechaDesde || (n.fecha && n.fecha >= fechaDesde)) &&
        (!fechaHasta || (n.fecha && n.fecha <= fechaHasta));
      return matchesProveedor && matchesFecha;
    });
  }, [proveedor, fechaDesde, fechaHasta]);

  const notes = demoMode ? filteredDemoNotes : (data ?? []);

  function handleSync() {
    const code = Number(profileCode);
    if (Number.isNaN(code) || code <= 0) {
      toast.error('El perfil de exportación debe ser un número mayor que 0');
      return;
    }
    if (!fechaDesde || !fechaHasta) {
      toast.error('Indica fecha_desde y fecha_hasta para el sync');
      return;
    }
    syncMutation.mutate(
      {
        export_profile_code: code,
        fecha_desde: fechaDesde,
        fecha_hasta: fechaHasta,
      },
      {
        onSuccess: (res) => {
          toast.success(`Sync completado: ${res.procesados} albaranes procesados de ${res.total_bdp}`);
        },
        onError: () => {
          toast.error('Error al sincronizar albaranes BDP');
        },
      },
    );
  }

  function handleDraft(note: BdpPurchaseNote) {
    if (demoMode) {
      toast.info('En modo demo no se guardan cambios reales');
      return;
    }
    draftMutation.mutate(note.id, {
      onSuccess: () => {
        toast.success('Albarán marcado como borrador');
      },
      onError: () => {
        toast.error('No se pudo marcar como borrador');
      },
    });
  }

  function handleReconcile(note: BdpPurchaseNote) {
    if (demoMode) {
      toast.info('En modo demo no se guardan cambios reales');
      return;
    }
    setReconcileNote(note);
  }

  function submitReconcile(req: BdpPurchaseNoteReconcileRequest) {
    if (!reconcileNote) return;
    reconcileMutation.mutate(
      { id: reconcileNote.id, req },
      {
        onSuccess: (res) => {
          toast.success(`Albarán conciliado con gasto ${res.gasto_id}`);
          setReconcileNote(null);
        },
        onError: () => {
          toast.error('No se pudo conciliar el albarán');
        },
      },
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">{`${notes.length} albaranes`}</p>
        <div className="flex items-center gap-2">
          <BdpDemoToggle demoMode={demoMode} onToggle={setDemoMode} />
          <div className="flex items-center gap-2">
            <Input
              type="number"
              min={1}
              value={profileCode}
              onChange={(e) => setProfileCode(e.target.value)}
              className="w-24"
              placeholder="Perfil"
              disabled={demoMode}
            />
            <TooltipButton
              variant="outline"
              onClick={handleSync}
              disabled={syncMutation.isPending || demoMode}
              tooltip="Importa/actualiza albaranes desde BDP para el rango de fechas indicado. No modifica BDP."
            >
              {syncMutation.isPending ? (
                <Loader2 className="size-3.5 animate-spin" />
              ) : (
                <RefreshCw className="size-3.5" />
              )}
              Sync albaranes
            </TooltipButton>
          </div>
        </div>
      </div>

      <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
        <div className="flex flex-wrap gap-3 items-center">
          <div className="relative w-full sm:w-64">
            <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
            <Input
              type="search"
              placeholder="Proveedor..."
              value={proveedor}
              onChange={(e) => setProveedor(e.target.value)}
              className="pl-9 max-w-xs"
            />
          </div>
          <div className="flex items-center gap-1.5">
            <span className="text-sm text-muted-foreground shrink-0">Desde:</span>
            <Input
              type="date"
              value={fechaDesde}
              onChange={(e) => setFechaDesde(e.target.value)}
              className="max-w-40"
              aria-label="Fecha desde"
            />
          </div>
          <div className="flex items-center gap-1.5">
            <span className="text-sm text-muted-foreground shrink-0">Hasta:</span>
            <Input
              type="date"
              value={fechaHasta}
              onChange={(e) => setFechaHasta(e.target.value)}
              className="max-w-40"
              aria-label="Fecha hasta"
            />
          </div>
        </div>
      </div>

      {isLoading && !demoMode ? (
        <div className="space-y-2">
          {Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="h-10 w-full animate-pulse rounded bg-muted" />
          ))}
        </div>
      ) : error && !demoMode ? (
        <p className="text-sm text-destructive">
          Error al cargar los albaranes. Revisa que la sesión esté activa y vuelve a intentarlo.
        </p>
      ) : notes.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          No hay albaranes importados. Selecciona un rango de fechas y pulsa Sync albaranes, o pulsa Cargar demo.
        </p>
      ) : (
        <div className="rounded-md border overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Fecha</TableHead>
                <TableHead>Serie</TableHead>
                <TableHead>Número</TableHead>
                <TableHead>Proveedor</TableHead>
                <TableHead className="text-right">Total</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead className="text-right">Acciones</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {notes.map((note) => (
                <TableRow key={note.id}>
                  <TableCell className="text-xs">{formatDate(note.fecha)}</TableCell>
                  <TableCell className="font-mono text-xs">{note.serie || '—'}</TableCell>
                  <TableCell className="font-mono text-xs">{note.numero || '—'}</TableCell>
                  <TableCell className="text-xs">
                    {note.nombre_proveedor || note.codigo_proveedor || '—'}
                  </TableCell>
                  <TableCell className="text-right text-xs tabular-nums">
                    {formatCurrency(note.total)}
                  </TableCell>
                  <TableCell className="text-xs">{formatEstado(note.estado)}</TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-2">
                      {note.estado === 'pendiente' && (
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleDraft(note)}
                          disabled={draftMutation.isPending}
                        >
                          <FilePen className="mr-1 size-3.5" />
                          Borrador
                        </Button>
                      )}
                      {note.estado === 'borrador' && (
                        <Button
                          variant="default"
                          size="sm"
                          onClick={() => handleReconcile(note)}
                          disabled={reconcileMutation.isPending}
                        >
                          <CheckCircle className="mr-1 size-3.5" />
                          Conciliar
                        </Button>
                      )}
                      {note.estado === 'conciliado' && (
                        <Button variant="outline" size="sm" disabled>
                          <CheckCircle className="mr-1 size-3.5" />
                          Conciliado
                        </Button>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      <BdpComprasReconcileModal
        open={!!reconcileNote}
        note={reconcileNote}
        onClose={() => setReconcileNote(null)}
        onSubmit={submitReconcile}
      />
    </div>
  );
}

function formatEstado(estado: BdpPurchaseNote['estado']) {
  switch (estado) {
    case 'pendiente':
      return <span className="text-xs text-muted-foreground">Pendiente</span>;
    case 'borrador':
      return <span className="text-xs text-blue-600">Borrador</span>;
    case 'conciliado':
      return (
        <Button variant="outline" size="sm" disabled className="h-7 text-xs pointer-events-none">
          <CheckCircle className="mr-1 size-3" />
          Conciliado
        </Button>
      );
    default:
      return <span className="text-xs text-muted-foreground">—</span>;
  }
}

export default BdpCompras;
