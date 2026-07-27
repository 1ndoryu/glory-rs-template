/* [263A-16] Lista de ventas — reescrita con shadcn Table + Dialog + Button.
 * Iconos para eliminar en vez de texto (tarea 17).
 * [283A-22] Botón de edición con Dialog reutilizando FormularioVenta.
 * [283A-28] Filtros de fecha (desde/hasta) extraídos a useListaVentas hook.
 * [034A-5] Columna "Cliente" con nombre_cliente + botón para ver reserva asociada.
 * [044A-8+9] Buscador + cabeceras de columna clicables para ordenar (sort_by/sort_order).
 * [064A-3] Filtros por columna en Turno, Canal y Método de pago con ColumnFilterHeader.
 * [064A-8] Botón eliminar oculto cuando sync Haddock activo (petición cliente).
 * [064A-9] Badge estado sync Haddock + dialog confirmación al editar venta sincronizada. */

/* [147A-F5.2] Columna BDP + filtro + polling manual — equivalentes a los de Haddock. */
import useListaVentas from '../hooks/useListaVentas';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Input } from '@/components/ui/input';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Table, TableBody, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { AlertTriangle, RefreshCw } from 'lucide-react';
import VentaTableBody from '@/components/venta-table-body';
import FormularioVenta from './FormularioVenta';
import ReservaViewer from './ReservaViewer';
import ColumnFilterHeader from '@/components/column-filter-header';

/* [147A-F5.2] Opciones de filtro extraídas a nivel módulo para reducir line count del componente. */
const OPCIONES_TURNO = [
  { value: 'manana', label: 'Mañana' },
  { value: 'mediodia', label: 'Mediodía' },
  { value: 'noche', label: 'Noche' },
];

const OPCIONES_CANAL = [
  { value: 'comedor', label: 'Comedor' },
  { value: 'barra', label: 'Barra' },
  { value: 'terraza', label: 'Terraza' },
  { value: 'delivery', label: 'Delivery' },
  { value: 'just_eat', label: 'Just Eat' },
  { value: 'eventos', label: 'Eventos' },
];

const OPCIONES_METODO_PAGO = [
  { value: 'efectivo', label: 'Efectivo' },
  { value: 'tarjeta', label: 'Tarjeta' },
  { value: 'transferencia', label: 'Transferencia' },
];

const OPCIONES_ESTADO_HADDOCK = [
  { value: 'synced', label: 'Sincronizada' },
  { value: 'error', label: 'Con error' },
  { value: 'pending', label: 'Pendiente' },
];

const OPCIONES_ESTADO_BDP = [
  { value: 'synced', label: 'Sincronizada' },
  { value: 'accepted', label: 'Aceptada' },
  { value: 'invoiced', label: 'Facturada' },
  { value: 'error', label: 'Con error' },
  { value: 'pending', label: 'Pendiente' },
  { value: 'cancelled', label: 'Cancelada' },
];

function ListaVentas() {
  const {
    filtros,
    cambiarFiltro,
    toggleSort,
    cambiarFiltroColumna,
    modalAbierto,
    setModalAbierto,
    ventaEditando,
    setVentaEditando,
    ventaPendienteEdicion,
    iniciarEdicion,
    confirmarEdicion,
    cancelarEdicion,
    porPagina,
    ventas,
    isLoading,
    eliminarMutation,
    haddockSyncEnabled,
    retryHaddockMutation,
    bdpSyncEnabled,
    bdpPollMutation,
    retryBdpMutation,
    cerrarModalYRefrescar,
    cerrarEdicionYRefrescar,
    reservaIdViewer,
    setReservaIdViewer,
    reservaDetalle,
    reservaCargando,
  } = useListaVentas();

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-muted-foreground">{ventas ? `${ventas.total} registros` : ''}</p>
        </div>
        <div className="flex gap-2">
          {bdpSyncEnabled && (
            <TooltipButton
              variant="outline"
              tooltip="Sincronizar manualmente las ventas desde BDP"
              onClick={() => bdpPollMutation.mutate()}
              disabled={bdpPollMutation.isPending}
            >
              <RefreshCw className={`size-4 mr-1.5 ${bdpPollMutation.isPending ? 'animate-spin' : ''}`} />
              Polling BDP
            </TooltipButton>
          )}
          <Button onClick={() => setModalAbierto(true)}>+ Nueva Venta</Button>
        </div>
      </div>

      {/* [044A-9] Buscador + filtros de fecha */}
      <div className="flex flex-wrap gap-3 items-center">
        <Input
          type="search"
          placeholder="Buscar por descripción, cliente, turno, canal..."
          value={filtros.busqueda}
          onChange={(e) => cambiarFiltro('busqueda', e.target.value)}
          className="max-w-xs"
        />
        <label className="text-sm text-muted-foreground">Desde</label>
        <Input
          type="date"
          value={filtros.desde}
          onChange={(e) => cambiarFiltro('desde', e.target.value)}
          className="max-w-40"
        />
        <label className="text-sm text-muted-foreground">Hasta</label>
        <Input
          type="date"
          value={filtros.hasta}
          onChange={(e) => cambiarFiltro('hasta', e.target.value)}
          className="max-w-40"
        />
      </div>

      <Dialog open={modalAbierto} onOpenChange={setModalAbierto}>
        <DialogContent className="sm:max-w-3xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Nueva Venta</DialogTitle>
          </DialogHeader>
          <FormularioVenta onExito={cerrarModalYRefrescar} />
        </DialogContent>
      </Dialog>

      <Dialog open={!!ventaEditando} onOpenChange={(open) => { if (!open) setVentaEditando(null); }}>
        <DialogContent className="sm:max-w-3xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Editar Venta</DialogTitle>
          </DialogHeader>
          {ventaEditando && (
            <FormularioVenta onExito={cerrarEdicionYRefrescar} venta={ventaEditando} />
          )}
        </DialogContent>
      </Dialog>

      {/* [034A-5] Diálogo de detalle de la reserva asociada a una venta */}
      <ReservaViewer
        reservaId={reservaIdViewer}
        onClose={() => { setReservaIdViewer(null); }}
        reserva={reservaDetalle}
        cargando={reservaCargando}
      />

      {/* [064A-9] Diálogo de confirmación al editar venta sincronizada con Haddock */}
      <Dialog open={!!ventaPendienteEdicion} onOpenChange={(open) => { if (!open) cancelarEdicion(); }}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle><AlertTriangle className="inline size-4 text-amber-500 mr-1.5" />Venta sincronizada con Haddock</DialogTitle>
            <DialogDescription>
              Esta venta ya fue enviada a Haddock. Si guardas los cambios, se re-enviará con los datos actualizados.
              Los datos anteriores en Haddock serán reemplazados.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="gap-2 sm:gap-0">
            <Button variant="outline" onClick={cancelarEdicion}>Cancelar</Button>
            <Button onClick={confirmarEdicion}>Continuar y actualizar</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Cargando...</p>
      ) : ventas && ventas.items.length > 0 ? (
        <>
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="cursor-pointer select-none" onClick={() => toggleSort('fecha')}>
                    Fecha {filtros.sortBy === 'fecha' && (filtros.sortOrder === 'asc' ? '↑' : '↓')}
                  </TableHead>
                  <TableHead className="cursor-pointer select-none" onClick={() => toggleSort('nombre_cliente')}>
                    Cliente {filtros.sortBy === 'nombre_cliente' && (filtros.sortOrder === 'asc' ? '↑' : '↓')}
                  </TableHead>
                  <TableHead>
                    <ColumnFilterHeader
                      title="Turno"
                      sortKey="turno"
                      currentSortBy={filtros.sortBy}
                      currentSortOrder={filtros.sortOrder}
                      onSort={() => toggleSort('turno')}
                      options={OPCIONES_TURNO}
                      selectedValues={filtros.turno}
                      onFilterChange={(v) => cambiarFiltroColumna('turno', v)}
                    />
                  </TableHead>
                  <TableHead>
                    <ColumnFilterHeader
                      title="Canal"
                      sortKey="canal"
                      currentSortBy={filtros.sortBy}
                      currentSortOrder={filtros.sortOrder}
                      onSort={() => toggleSort('canal')}
                      options={OPCIONES_CANAL}
                      selectedValues={filtros.canal}
                      onFilterChange={(v) => cambiarFiltroColumna('canal', v)}
                    />
                  </TableHead>
                  <TableHead>
                    <ColumnFilterHeader
                      title="Método"
                      sortKey="metodo_pago"
                      currentSortBy={filtros.sortBy}
                      currentSortOrder={filtros.sortOrder}
                      onSort={() => toggleSort('metodo_pago')}
                      options={OPCIONES_METODO_PAGO}
                      selectedValues={filtros.metodoPago}
                      onFilterChange={(v) => cambiarFiltroColumna('metodoPago', v)}
                    />
                  </TableHead>
                  <TableHead className="text-right cursor-pointer select-none" onClick={() => toggleSort('importe_base')}>
                    Base {filtros.sortBy === 'importe_base' && (filtros.sortOrder === 'asc' ? '↑' : '↓')}
                  </TableHead>
                  <TableHead className="text-right">IVA</TableHead>
                  <TableHead className="text-right">Total</TableHead>
                  {/* [064A-9+12] Columna Haddock con filtro por estado (synced/error/pending) */}
                  {haddockSyncEnabled && (
                    <TableHead className="w-24">
                      <ColumnFilterHeader
                        title="Haddock"
                        sortKey="haddock"
                        currentSortBy={filtros.sortBy}
                        currentSortOrder={filtros.sortOrder}
                        onSort={() => {}}
                        options={OPCIONES_ESTADO_HADDOCK}
                        selectedValues={filtros.estadoHaddock}
                        onFilterChange={(v) => cambiarFiltroColumna('estadoHaddock', v)}
                      />
                    </TableHead>
                  )}
                  {bdpSyncEnabled && (
                    <TableHead className="w-24">
                      <ColumnFilterHeader
                        title="BDP"
                        sortKey="bdp"
                        currentSortBy={filtros.sortBy}
                        currentSortOrder={filtros.sortOrder}
                        onSort={() => {}}
                        options={OPCIONES_ESTADO_BDP}
                        selectedValues={filtros.estadoBdp}
                        onFilterChange={(v) => cambiarFiltroColumna('estadoBdp', v)}
                      />
                    </TableHead>
                  )}
                  <TableHead className="w-28"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                <VentaTableBody
                  ventas={ventas.items}
                  haddockSyncEnabled={haddockSyncEnabled}
                  bdpSyncEnabled={bdpSyncEnabled}
                  onVerReserva={(id) => setReservaIdViewer(id)}
                  onEditar={iniciarEdicion}
                  onEliminar={(id) => eliminarMutation.mutate({ id })}
                  onRetryHaddock={(id) => retryHaddockMutation.mutate({ id })}
                  onRetryBdp={(id) => retryBdpMutation.mutateAsync(id)}
                  eliminarPending={eliminarMutation.isPending}
                  retryHaddockPending={retryHaddockMutation.isPending}
                  retryBdpPending={retryBdpMutation.isPending}
                />
              </TableBody>
            </Table>
          </div>

          <div className="flex items-center justify-between">
            <Button variant="outline" size="sm" disabled={filtros.pagina <= 1} onClick={() => cambiarFiltro('pagina', filtros.pagina - 1)}>Anterior</Button>
            <span className="text-sm text-muted-foreground">Página {filtros.pagina} de {Math.ceil(ventas.total / porPagina)}</span>
            <Button variant="outline" size="sm" disabled={filtros.pagina * porPagina >= ventas.total} onClick={() => cambiarFiltro('pagina', filtros.pagina + 1)}>Siguiente</Button>
          </div>
        </>
      ) : (
        <p className="text-sm text-muted-foreground">No hay ventas registradas</p>
      )}
    </div>
  );
}

export default ListaVentas;
