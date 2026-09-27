import { ImagePlus, Loader2, Sparkles, X } from 'lucide-react';
import { ETIQUETAS_TIPO } from '@/domain/inmueble';
import type { FichaIA } from '@/data/ia/ia';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Textarea } from '@/components/ui/textarea';
import { useModalIA } from '@/hooks/inmuebles/use-modal-ia';

interface Props {
  abierto: boolean;
  alCambiarAbierto: (abierto: boolean) => void;
  textoInicial: string;
  fotosIniciales: string[];
  onAplicar: (ficha: FichaIA, fotosNuevas: string[]) => void;
}

function fila(etiqueta: string, valor: string) {
  if (!valor) return null;
  return (
    <div className="grid grid-cols-[110px_1fr] gap-2 text-sm">
      <dt className="text-muted-foreground">{etiqueta}</dt>
      <dd className="font-medium">{valor}</dd>
    </div>
  );
}

export function ModalIA(props: Props) {
  const { abierto, alCambiarAbierto } = props;
  const {
    texto,
    fotos,
    fase,
    resultado,
    arrastrando,
    setArrastrando,
    setTexto,
    setFase,
    quitarFoto,
    inputFotos,
    cerrar,
    anadirFotos,
    organizar,
  } = useModalIA(abierto, props.textoInicial, props.fotosIniciales, alCambiarAbierto);

  const ficha = resultado && resultado.ok ? resultado.ficha : null;
  const modelo = resultado && resultado.ok ? resultado.modelo : '';

  const aplicar = () => {
    if (!ficha) return;
    props.onAplicar(ficha, fotos);
    cerrar(false);
  };

  return (
    <Dialog open={abierto} onOpenChange={cerrar}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Sparkles className="h-4 w-4" /> Redactar con IA
          </DialogTitle>
          <DialogDescription>
            Describe el inmueble como quieras y suelta fotos. La IA lo ordena en una ficha lista para revisar.
          </DialogDescription>
        </DialogHeader>

        {fase !== 'previa' ? (
          <div className="grid gap-4">
            <div className="space-y-1">
              <span className="block text-sm font-medium">Descripción libre</span>
              <Textarea
                value={texto}
                onChange={(e) => setTexto(e.target.value)}
                rows={6}
                placeholder="apartamento 3 hab centro 180000 dos baños 90m2 para vender, cocina nueva, fotos del salón…"
                disabled={fase === 'pensando'}
              />
            </div>

            <div className="space-y-1">
              <span className="block text-sm font-medium">Fotos ({fotos.length})</span>
              <div
                role="button"
                tabIndex={0}
                aria-label="Soltar fotos para la IA"
                onClick={() => inputFotos.current?.click()}
                onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') inputFotos.current?.click(); }}
                onDragOver={(e) => { e.preventDefault(); setArrastrando(true); }}
                onDragLeave={() => setArrastrando(false)}
                onDrop={(e) => { e.preventDefault(); setArrastrando(false); void anadirFotos(e.dataTransfer.files); }}
                className={
                  'flex min-h-24 cursor-pointer flex-col items-center justify-center gap-1 rounded-md border border-dashed p-4 text-sm text-muted-foreground ' +
                  (arrastrando ? 'border-primary bg-accent' : 'border-input')
                }
              >
                <ImagePlus className="h-5 w-5" />
                Suelta fotos aquí o pulsa para elegirlas
              </div>
              <input
                ref={inputFotos}
                type="file"
                accept="image/*"
                multiple
                className="hidden"
                onChange={(e) => void anadirFotos(e.target.files)}
              />
              {fotos.length > 0 && (
                <div className="grid grid-cols-4 gap-2 sm:grid-cols-6">
                  {fotos.map((f) => (
                    <div key={f.slice(-32)} className="group relative aspect-square overflow-hidden rounded-md border">
                      <img src={f} alt="Foto para la IA" className="h-full w-full object-cover" />
                      <button
                        type="button"
                        aria-label="Quitar foto"
                        onClick={() => quitarFoto(f)}
                        className="absolute right-1 top-1 rounded bg-black/60 p-1 text-white opacity-0 transition group-hover:opacity-100"
                      >
                        <X className="h-3 w-3" />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {resultado && !resultado.ok && (
              <p role="alert" className="rounded-md border border-destructive/50 bg-destructive/10 p-3 text-sm">
                {resultado.motivo}
              </p>
            )}
          </div>
        ) : (
          ficha && (
            <div className="grid gap-3">
              <p className="text-xs text-muted-foreground">
                Propuesta de la IA{modelo ? ` (modelo: ${modelo})` : ''}. Revísala y aplícala a la ficha.
              </p>
              <dl className="grid gap-1.5 rounded-md border p-3">
                {fila('Título', ficha.titulo)}
                {fila('Descripción', ficha.descripcion)}
                {fila('Ubicación', ficha.ubicacion)}

                {fila('Residencia', ficha.residencia)}
                {fila('Precio', ficha.precio !== null ? `${ficha.precio.toLocaleString('es-ES')} $` : '')}
                {fila('Tipo', ficha.tipo ? ETIQUETAS_TIPO[ficha.tipo] : '')}
                {fila('Operación', ficha.operacion)}
                {fila('Habitaciones', ficha.habitaciones !== null ? String(ficha.habitaciones) : '')}
                {fila('Baños', ficha.banos !== null ? String(ficha.banos) : '')}
                {fila('M² propiedad', ficha.metros !== null ? `${ficha.metros} m²` : '')}
                {fila('M² terreno', ficha.metrosTerreno !== null ? `${ficha.metrosTerreno} m²` : '')}

                {fila('Puestos', ficha.puestos !== null ? String(ficha.puestos) : '')}
              </dl>
            </div>
          )
        )}

        <DialogFooter>
          {fase === 'previa' && ficha ? (
            <>
              <Button variant="outline" onClick={() => setFase('editar')} className="mr-auto">
                Seguir editando
              </Button>
              <Button variant="ghost" onClick={() => cerrar(false)}>
                Cerrar
              </Button>
              <Button onClick={aplicar}>Aplicar a la ficha</Button>
            </>
          ) : (
            <>
              <Button variant="ghost" onClick={() => cerrar(false)} disabled={fase === 'pensando'}>
                {fase === 'pensando' ? 'Cancelar' : 'Cerrar'}
              </Button>
              <Button onClick={() => void organizar()} disabled={fase === 'pensando' || (!texto.trim() && fotos.length === 0)}>
                {fase === 'pensando' ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" /> Ordenando…
                  </>
                ) : (
                  <>
                    <Sparkles className="h-4 w-4" /> Ordenar con IA
                  </>
                )}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
