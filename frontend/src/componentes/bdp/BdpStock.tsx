/* [BDP-STOCK-03] Página de stock BDP — solo lectura.
 * Estructura coherente con ListaVentas/ListaGastos/ListaReservas.
 * Añadido modo demo para visualizar datos de prueba sin conexión a BDP. */

import { useMemo } from 'react';
import {
  Search,
  RefreshCw,
  Loader2,
  Package,
  Download,
  AlertCircle,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useListarArticleMaps, useSyncCatalog } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { useBdpStockFilters } from '@/hooks/useBdpStockFilters';
import { useBdpDemoMode } from '@/hooks/useBdpDemoMode';
import { formatPrice, formatStock, formatDate, exportToCsv, PAGE_SIZES, type SortKey } from './bdp-stock-utils';
import { mockArticleMaps } from './bdp-mocks';
import { BdpDemoToggle } from './BdpDemoToggle';

function TableSkeleton() {
  return (
    <div className="space-y-2">
      {Array.from({ length: 5 }).map((_, i) => (
        <Skeleton key={i} className="h-10 w-full" />
      ))}
    </div>
  );
}

function ReadOnlyBanner() {
  return (
    <div className="rounded-lg border bg-muted/50 p-3 text-sm">
      <div className="flex items-start gap-3">
        <AlertCircle className="size-4 shrink-0 text-muted-foreground" />
        <div>
          <p className="text-muted-foreground">
            Vista solo lectura del stock BDP. Para ajustar inventario, usa el TPV/BDP.
          </p>
        </div>
      </div>
    </div>
  );
}

function BdpStock() {
  const queryClient = useQueryClient();
  const { demoMode, setDemoMode } = useBdpDemoMode();
  const { data, isLoading, error: listError } = useListarArticleMaps({
    query: { enabled: !demoMode },
  });
  const syncCatalogMutation = useSyncCatalog({
    mutation: {
      onSuccess: (resp) => {
        const d = resp as unknown as { creados?: number; actualizados?: number };
        toast.success(`Sync completado: ${d.creados ?? 0} nuevos, ${d.actualizados ?? 0} actualizados`);
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
      },
      onError: () => toast.error('Error al sincronizar catálogo BDP'),
    },
  });

  const apiRows = data?.status === 200 ? data.data : [];
  const mapeos = demoMode ? mockArticleMaps : apiRows;
  const lastSync = useMemo(() => {
    if (!mapeos.length) return null;
    const dates = mapeos.map((m) => m.ultima_sync_at).filter(Boolean) as string[];
    if (!dates.length) return null;
    dates.sort((a, b) => new Date(b).getTime() - new Date(a).getTime());
    return dates[0];
  }, [mapeos]);

  const {
    filtro,
    setFiltro,
    stockFilter,
    setStockFilter,
    activeFilter,
    setActiveFilter,
    sortKey,
    setSortKey,
    sortDir,
    setSortDir,
    page,
    setPage,
    pageSize,
    setPageSize,
    totalPages,
    paginated,
    sorted,
    filteredCount,
  } = useBdpStockFilters(mapeos);

  function handleSort(key: SortKey) {
    if (sortKey === key) {
      setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortKey(key);
      setSortDir('asc');
    }
    setPage(1);
  }

  function SortIcon({ column }: { column: SortKey }) {
    if (sortKey !== column) return <span className="inline-block w-3" />;
    return sortDir === 'asc' ? <ChevronUp className="size-3" /> : <ChevronDown className="size-3" />;
  }

  function handleExport() {
    exportToCsv(mapeos, sorted, {
      allRows: false,
      filterLabel: stockFilter !== 'all' ? stockFilter : undefined,
    });
  }

  const isLoadingEffective = !demoMode && isLoading;
  const hasError = !demoMode && !!listError;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          {`${filteredCount} artículos`} · Última sync: {lastSync ? formatDate(lastSync) : '—'}
        </p>
        <div className="flex items-center gap-2">
          <BdpDemoToggle demoMode={demoMode} onToggle={setDemoMode} />
          <Button
            variant="outline"
            onClick={handleExport}
            disabled={paginated.length === 0}
            title="Exportar a CSV con BOM para Excel"
          >
            <Download className="size-4 mr-1.5" />
            CSV
          </Button>
          <TooltipButton
            variant="outline"
            onClick={() => syncCatalogMutation.mutate()}
            disabled={syncCatalogMutation.isPending || demoMode}
            tooltip="Importa/actualiza artículos y stock desde BDP a Glory. No modifica BDP."
          >
            {syncCatalogMutation.isPending ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <RefreshCw className="size-3.5" />
            )}
            Sync catálogo
          </TooltipButton>
        </div>
      </div>

      <ReadOnlyBanner />

      <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
        <div className="flex flex-wrap gap-3 items-center">
          <div className="relative w-full sm:w-64">
            <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
            <Input
              placeholder="Código o nombre..."
              value={filtro}
              onChange={(e) => {
                setFiltro(e.target.value);
                setPage(1);
              }}
              className="pl-9 max-w-xs"
            />
          </div>
          <Select
            value={stockFilter}
            onValueChange={(v) => {
              setStockFilter(v as typeof stockFilter);
              setPage(1);
            }}
          >
            <SelectTrigger className="w-full sm:w-44">
              <SelectValue placeholder="Stock" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">Cualquier stock</SelectItem>
              <SelectItem value="with">Con stock</SelectItem>
              <SelectItem value="without">Sin stock</SelectItem>
            </SelectContent>
          </Select>
          <Select
            value={activeFilter}
            onValueChange={(v) => {
              setActiveFilter(v as typeof activeFilter);
              setPage(1);
            }}
          >
            <SelectTrigger className="w-full sm:w-44">
              <SelectValue placeholder="Estado" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">Cualquier estado</SelectItem>
              <SelectItem value="active">Activos</SelectItem>
              <SelectItem value="inactive">Inactivos</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      {hasError ? (
        <p className="text-sm text-destructive">
          Error al cargar el stock. Revisa que la sesión esté activa y vuelve a intentarlo.
        </p>
      ) : isLoadingEffective ? (
        <TableSkeleton />
      ) : filteredCount === 0 ? (
        <p className="text-sm text-muted-foreground">
          No hay artículos que coincidan con los filtros. Sincroniza el catálogo desde BDP o pulsa "Cargar demo".
        </p>
      ) : (
        <>
          <div className="rounded-md border overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="cursor-pointer" onClick={() => handleSort('articulo_glory_codigo')}>
                    <span className="flex items-center gap-1">Código Glory <SortIcon column="articulo_glory_codigo" /></span>
                  </TableHead>
                  <TableHead className="cursor-pointer" onClick={() => handleSort('articulo_bdp_codigo')}>
                    <span className="flex items-center gap-1">Código BDP <SortIcon column="articulo_bdp_codigo" /></span>
                  </TableHead>
                  <TableHead className="cursor-pointer" onClick={() => handleSort('articulo_bdp_nombre')}>
                    <span className="flex items-center gap-1">Nombre BDP <SortIcon column="articulo_bdp_nombre" /></span>
                  </TableHead>
                  <TableHead className="cursor-pointer" onClick={() => handleSort('precio_tarifa1')}>
                    <span className="flex items-center gap-1">Precio <SortIcon column="precio_tarifa1" /></span>
                  </TableHead>
                  <TableHead className="cursor-pointer" onClick={() => handleSort('stock_actual')}>
                    <span className="flex items-center gap-1">Stock <SortIcon column="stock_actual" /></span>
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {paginated.map((m) => {
                  const stock = formatStock(m.stock_actual);
                  return (
                    <TableRow key={m.id}>
                      <TableCell className="font-mono text-xs">{m.articulo_glory_codigo || '—'}</TableCell>
                      <TableCell className="font-mono text-xs">{m.articulo_bdp_codigo || '—'}</TableCell>
                      <TableCell className="text-xs">{m.articulo_bdp_nombre || '—'}</TableCell>
                      <TableCell className="text-xs tabular-nums">{formatPrice(m.precio_tarifa1)}</TableCell>
                      <TableCell>
                        {stock.hasStock ? (
                          <Badge variant="secondary" className="gap-1">
                            <Package className="size-3" />
                            {stock.text}
                          </Badge>
                        ) : (
                          <span className="text-xs text-muted-foreground">—</span>
                        )}
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>

          <div className="flex flex-col sm:flex-row items-center justify-between gap-3 text-sm">
            <span className="text-muted-foreground">
              Mostrando {paginated.length} de {filteredCount} artículos
            </span>
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" onClick={() => setPage((p) => Math.max(1, p - 1))} disabled={page <= 1}>
                Anterior
              </Button>
              <span className="text-muted-foreground">
                Página {page} de {totalPages}
              </span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                disabled={page >= totalPages}
              >
                Siguiente
              </Button>
            </div>
            <Select
              value={String(pageSize)}
              onValueChange={(v) => {
                setPageSize(Number(v) as 10 | 25 | 50);
                setPage(1);
              }}
            >
              <SelectTrigger className="w-20">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {PAGE_SIZES.map((s) => (
                  <SelectItem key={s} value={String(s)}>
                    {s}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </>
      )}
    </div>
  );
}

export default BdpStock;
