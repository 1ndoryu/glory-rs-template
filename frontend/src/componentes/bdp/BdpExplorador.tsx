/* [BDP-EXPLORER-02] Explorador de menús, packs y fastfoods BDP.
 * Estructura coherente con ListaVentas/ListaGastos/ListaReservas.
 * En modo real permite consultar por código numérico a BDP.
 * Modo demo muestra una tabla con datos de ejemplo. */

import { useMemo, useState } from 'react';
import { Search, Loader2, Eye } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { toast } from 'sonner';
import { ErrorResponse } from '@/api/generated/gestionRestauranteAPI.schemas';
import {
  useGetMenuDefinition,
  useGetFastfoodDefinition,
  useGetPackDefinition,
} from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { useBdpDemoMode } from '@/hooks/useBdpDemoMode';
import { mockExplorerItems, type BdpExplorerItem } from './bdp-mocks';
import { BdpDemoToggle } from './BdpDemoToggle';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

type ExploreType = 'all' | 'menu' | 'fastfood' | 'pack';

const TIPOS: { value: ExploreType; label: string }[] = [
  { value: 'all', label: 'Todos' },
  { value: 'menu', label: 'Menú' },
  { value: 'fastfood', label: 'Fastfood' },
  { value: 'pack', label: 'Pack' },
];

function tipoLabel(tipo: string) {
  return TIPOS.find((t) => t.value === tipo)?.label ?? tipo;
}

interface BdpDefLine {
  article_code?: string | number;
  article_name?: string;
  quantity?: number;
}

interface BdpDefinition {
  code?: string | number;
  name?: string;
  description?: string;
  lines?: BdpDefLine[];
}

function DefinitionDetail({ data }: { data: BdpExplorerItem }) {
  return (
    <div className="space-y-3 text-sm">
      <div className="grid grid-cols-2 gap-2">
        <div>
          <p className="text-xs text-muted-foreground">Código</p>
          <p className="font-medium">{data.code}</p>
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Tipo</p>
          <Badge variant="outline">{tipoLabel(data.type)}</Badge>
        </div>
      </div>
      <div>
        <p className="text-xs text-muted-foreground">Descripción</p>
        <p>{data.description}</p>
      </div>
      <div>
        <p className="text-xs text-muted-foreground">Artículos</p>
        <div className="rounded-md border overflow-hidden mt-1">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="text-xs">Artículo</TableHead>
                <TableHead className="text-xs">Código</TableHead>
                <TableHead className="text-xs text-right">Cantidad</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.lines.map((line, i) => (
                <TableRow key={i}>
                  <TableCell className="text-xs">{line.articleName}</TableCell>
                  <TableCell className="font-mono text-xs">{line.articleCode}</TableCell>
                  <TableCell className="text-xs text-right">{line.quantity}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </div>
    </div>
  );
}

function BdpExplorador() {
  const { demoMode, setDemoMode } = useBdpDemoMode();
  const [tipo, setTipo] = useState<ExploreType>('all');
  const [busqueda, setBusqueda] = useState('');
  const [identificador, setIdentificador] = useState('');
  const [buscado, setBuscado] = useState(false);
  const [seleccionado, setSeleccionado] = useState<BdpExplorerItem | null>(null);

  const idNum = Number(identificador);
  const idValido = Number.isInteger(idNum) && idNum > 0;

  const menuQuery = useGetMenuDefinition(idNum, {
    query: { enabled: buscado && tipo !== 'all' && !demoMode },
  });
  const fastfoodQuery = useGetFastfoodDefinition(idNum, {
    query: { enabled: buscado && tipo === 'fastfood' && !demoMode },
  });
  const packQuery = useGetPackDefinition(idNum, {
    query: { enabled: buscado && tipo === 'pack' && !demoMode },
  });

  const isLoading = menuQuery.isLoading || fastfoodQuery.isLoading || packQuery.isLoading;
  const error = menuQuery.error || fastfoodQuery.error || packQuery.error;

  function extraerMensajeError(err: unknown): string {
    if (typeof err === 'string') return err;
    if (err instanceof Error) return err.message;
    const er = err as ErrorResponse | undefined;
    if (er?.message) return er.message;
    return 'Error al consultar. Verifica que el código existe en BDP y que la integración está activa.';
  }

  const items = useMemo(() => {
    if (!demoMode) return [];
    return mockExplorerItems.filter((item) => {
      const matchesTipo = tipo === 'all' || item.type === tipo;
      const q = busqueda.trim().toLowerCase();
      const matchesText =
        !q ||
        item.name.toLowerCase().includes(q) ||
        item.code.toLowerCase().includes(q) ||
        item.description.toLowerCase().includes(q);
      return matchesTipo && matchesText;
    });
  }, [demoMode, tipo, busqueda]);

  const resultadoReal = buscado
    ? (menuQuery.data as { data?: BdpDefinition })?.data ??
      (fastfoodQuery.data as { data?: BdpDefinition })?.data ??
      (packQuery.data as { data?: BdpDefinition })?.data
    : null;

  function buscar() {
    if (!idValido) {
      toast.warning('Introduce un código numérico válido');
      return;
    }
    if (tipo === 'all') {
      toast.warning('Selecciona un tipo (Menú, Fastfood o Pack)');
      return;
    }
    setBuscado(true);
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          {demoMode ? `${items.length} definiciones` : 'Consulta una definición de BDP por código'}
        </p>
        <BdpDemoToggle demoMode={demoMode} onToggle={setDemoMode} />
      </div>

      <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
        {demoMode ? (
          <>
            <div className="flex flex-wrap gap-3 items-center">
              <Input
                type="search"
                placeholder="Buscar nombre, código o descripción..."
                value={busqueda}
                onChange={(e) => setBusqueda(e.target.value)}
                className="max-w-xs"
              />
              <Select
                value={tipo}
                onValueChange={(v) => setTipo(v as ExploreType)}
              >
                <SelectTrigger className="w-full sm:w-44">
                  <SelectValue placeholder="Tipo" />
                </SelectTrigger>
                <SelectContent>
                  {TIPOS.map((t) => (
                    <SelectItem key={t.value} value={t.value}>
                      {t.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </>
        ) : (
          <div className="flex flex-wrap gap-3 items-end">
            <Select
              value={tipo}
              onValueChange={(v) => {
                setTipo(v as ExploreType);
                setBuscado(false);
              }}
            >
              <SelectTrigger className="w-full sm:w-44">
                <SelectValue placeholder="Tipo" />
              </SelectTrigger>
              <SelectContent>
                {TIPOS.filter((t) => t.value !== 'all').map((t) => (
                  <SelectItem key={t.value} value={t.value}>
                    {t.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Input
              type="number"
              min={1}
              className="w-36"
              placeholder="Código numérico..."
              value={identificador}
              onChange={(e) => {
                setIdentificador(e.target.value);
                setBuscado(false);
              }}
            />
            <Button onClick={buscar} disabled={isLoading}>
              {isLoading ? (
                <Loader2 className="size-3.5 animate-spin mr-1" />
              ) : (
                <Search className="size-3.5 mr-1" />
              )}
              Consultar
            </Button>
          </div>
        )}
      </div>

      {!demoMode && !buscado && (
        <p className="text-sm text-muted-foreground">
          Selecciona un tipo e introduce un código numérico para consultar BDP, o pulsa Cargar demo para ver datos de
          ejemplo.
        </p>
      )}

      {error && !demoMode && <p className="text-sm text-destructive">{extraerMensajeError(error)}</p>}

      {!demoMode && buscado && !isLoading && !resultadoReal && (
        <p className="text-sm text-muted-foreground">
          No se encontró un {tipo} con código {identificador}.
        </p>
      )}

      {!demoMode && resultadoReal && (
        <div className="rounded-md border p-3 text-sm space-y-3">
          <div className="flex items-center gap-2">
            <span className="font-medium">{resultadoReal.name ?? 'Sin nombre'}</span>
            <Badge variant="outline">{tipoLabel(tipo)}</Badge>
          </div>
          {resultadoReal.description && (
            <p className="text-xs text-muted-foreground">{resultadoReal.description}</p>
          )}
          <div className="grid grid-cols-2 gap-2 text-xs">
            <span>
              Código: <Badge variant="outline">{resultadoReal.code ?? '—'}</Badge>
            </span>
            <span>
              Artículos: <Badge variant="secondary">{resultadoReal.lines?.length ?? 0}</Badge>
            </span>
          </div>
          {resultadoReal.lines && resultadoReal.lines.length > 0 && (
            <div className="rounded-md border overflow-hidden">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="text-xs">Artículo</TableHead>
                    <TableHead className="text-xs">Código</TableHead>
                    <TableHead className="text-xs text-right">Cantidad</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {resultadoReal.lines.map((line, i) => (
                    <TableRow key={i}>
                      <TableCell className="text-xs">{line.article_name ?? '—'}</TableCell>
                      <TableCell className="font-mono text-xs">{line.article_code ?? '—'}</TableCell>
                      <TableCell className="text-xs text-right">{line.quantity ?? '—'}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </div>
      )}

      {demoMode && (
        <>
          <div className="rounded-md border overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Código</TableHead>
                  <TableHead>Nombre</TableHead>
                  <TableHead>Tipo</TableHead>
                  <TableHead>Descripción</TableHead>
                  <TableHead className="w-10"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {items.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={5} className="text-center text-sm text-muted-foreground">
                      No hay definiciones que coincidan.
                    </TableCell>
                  </TableRow>
                ) : (
                  items.map((item) => (
                    <TableRow key={item.id}>
                      <TableCell className="font-mono text-xs">{item.code}</TableCell>
                      <TableCell className="text-sm font-medium">{item.name}</TableCell>
                      <TableCell>
                        <Badge variant="outline">{tipoLabel(item.type)}</Badge>
                      </TableCell>
                      <TableCell className="text-xs text-muted-foreground max-w-xs truncate">
                        {item.description}
                      </TableCell>
                      <TableCell>
                        <Button variant="ghost" size="icon" onClick={() => setSeleccionado(item)}>
                          <Eye className="size-4" />
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </div>
        </>
      )}

      <Dialog open={!!seleccionado} onOpenChange={(open) => !open && setSeleccionado(null)}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Detalle de definición BDP</DialogTitle>
            <DialogDescription>{seleccionado?.name}</DialogDescription>
          </DialogHeader>
          {seleccionado && <DefinitionDetail data={seleccionado} />}
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default BdpExplorador;
