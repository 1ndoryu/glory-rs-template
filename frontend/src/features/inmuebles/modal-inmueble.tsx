import { Trash2, Sparkles } from 'lucide-react';
import {
  ESTADOS,
  ETIQUETAS_TIPO,
  TIPOS,
  draftTieneContenido,
  type Inmueble,
  type InmuebleDraft,
} from '@/domain/inmueble';
import { BannerBorrador, CLASE_SELECT, Etiqueta } from './campos-formulario';
import { FichaFormulario } from './ficha-formulario';
import { useFichaInmueble } from '@/hooks/inmuebles/use-ficha-inmueble';
import { FotosFormulario } from './fotos-formulario';
import { ModalIA } from './modal-ia';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/lib/utils';
import { useModalInmueble } from '@/hooks/inmuebles/use-modal-inmueble';
import { confirmar } from '@/platform/ventana';

/* Borrador persistente (solo modo "nuevo"; vive en el padre para
 * sobrevivir al modal). */
interface BorradorExterno {
  borrador: InmuebleDraft;
  alCambiar: (d: InmuebleDraft) => void;
  hayGuardado: boolean;
  alRestaurar: () => void;
  alEmpezarDeCero: () => void;
  alLimpiarTrasGuardar: () => void;
}

interface Props {
  abierto: boolean;
  alCambiarAbierto: (abierto: boolean) => void;
  editando: Inmueble | null;
  borrador: BorradorExterno;
  alGuardarNuevo: (inmueble: Inmueble) => Promise<boolean>;
  alGuardarEdicion: (inmueble: Inmueble) => Promise<boolean>;
}

export function ModalInmueble(props: Props) {
  const { abierto, alCambiarAbierto } = props;
  /* Ficha /ask editable: se fusiona al guardar (pisa lo guardado). */
  const ficha = useFichaInmueble(abierto, props.editando);
  const {
    esEdicion,
    form,
    errores,
    avisoFotos,
    iaAbierto,
    setIaAbierto,
    inputFotos,
    cambiar,
    anadirFotos,
    quitarFoto,
    hacerPrincipal,
    guardar,
    aplicarFichaIA,
  } = useModalInmueble({
    abierto,
    editando: props.editando,
    borrador: props.borrador.borrador,
    setBorrador: props.borrador.alCambiar,
    alCambiarAbierto,
    limpiarTrasGuardar: props.borrador.alLimpiarTrasGuardar,
    onGuardarNuevo: (inm) => props.alGuardarNuevo(ficha.conFicha(inm)),
    onGuardarEdicion: (inm) => props.alGuardarEdicion(ficha.conFicha(inm)),
  });

  const mostrarBannerRecuperar =
    !esEdicion && props.borrador.hayGuardado && !draftTieneContenido(props.borrador.borrador);

  const descartar = () => {
    if (confirmar('Descartar el borrador y empezar de cero. Esta acción no se puede deshacer.')) {
      void props.borrador.alEmpezarDeCero();
    }
  };

  return (
    <>
    <Dialog open={abierto} onOpenChange={alCambiarAbierto}>
      <DialogContent className="max-h-[92dvh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{esEdicion ? 'Editar inmueble' : 'Añadir inmueble'}</DialogTitle>
          <DialogDescription>
            {esEdicion
              ? 'Modifica la ficha. Cerrar sin guardar conserva la ficha original.'
              : 'Sin campos obligatorios: guarda con la información que tengas. El borrador se autoguard: puedes cerrar sin miedo, no se pierde nada.'}
          </DialogDescription>
          <Button variant="outline" className="mt-2 w-full sm:w-auto" onClick={() => setIaAbierto(true)}>
            <Sparkles className="h-4 w-4" /> Redactar con IA
          </Button>
        </DialogHeader>

        {mostrarBannerRecuperar && (
          <BannerBorrador alRestaurar={props.borrador.alRestaurar} alEmpezarDeCero={props.borrador.alEmpezarDeCero} />
        )}

        <div className="grid gap-4">
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1 sm:col-span-2">
              <Etiqueta error={errores.titulo}>Título</Etiqueta>
              <Input
                value={form.titulo}
                onChange={(e) => cambiar('titulo', e.target.value)}
                placeholder="Piso luminoso en el centro"
                aria-invalid={Boolean(errores.titulo)}
                className={cn(errores.titulo && 'border-destructive')}
              />
              {errores.titulo && <p className="text-xs text-destructive">{errores.titulo}</p>}
            </div>
            <div className="space-y-1 sm:col-span-2">
              <Etiqueta error={errores.ubicacion}>Ubicación</Etiqueta>
              <Input
                value={form.ubicacion}
                onChange={(e) => cambiar('ubicacion', e.target.value)}
                placeholder="Calle Mayor 12, Madrid"
                aria-invalid={Boolean(errores.ubicacion)}
                className={cn(errores.ubicacion && 'border-destructive')}
              />
              {errores.ubicacion && <p className="text-xs text-destructive">{errores.ubicacion}</p>}
            </div>
            <div className="space-y-1 sm:col-span-2">
              <Etiqueta>Residencia / conjunto</Etiqueta>
              <Input
                value={form.residencia}
                onChange={(e) => cambiar('residencia', e.target.value)}
                placeholder="Residencias Los Naranjos"
              />
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.precio}>Precio ($)</Etiqueta>
              <Input
                value={form.precio}
                onChange={(e) => cambiar('precio', e.target.value)}
                placeholder="250000"
                inputMode="numeric"
                aria-invalid={Boolean(errores.precio)}
                className={cn(errores.precio && 'border-destructive')}
              />
              {errores.precio && <p className="text-xs text-destructive">{errores.precio}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta>Estado</Etiqueta>
              <select
                value={form.estado}
                onChange={(e) => cambiar('estado', e.target.value as InmuebleDraft['estado'])}
                className={CLASE_SELECT}
              >
                {ESTADOS.map((s) => (
                  <option key={s} value={s}>
                    {s}
                  </option>
                ))}
              </select>
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.tipo}>Tipo</Etiqueta>
              <select
                value={form.tipo}
                onChange={(e) => cambiar('tipo', e.target.value as InmuebleDraft['tipo'])}
                className={cn(CLASE_SELECT, errores.tipo && 'border-destructive')}
              >
                <option value="">Seleccionar…</option>
                {TIPOS.map((t) => (
                  <option key={t} value={t}>
                    {ETIQUETAS_TIPO[t]}
                  </option>
                ))}
              </select>
              {errores.tipo && <p className="text-xs text-destructive">{errores.tipo}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.operacion}>Operación</Etiqueta>
              <select
                value={form.operacion}
                onChange={(e) => cambiar('operacion', e.target.value as InmuebleDraft['operacion'])}
                className={cn(CLASE_SELECT, errores.operacion && 'border-destructive')}
              >
                <option value="">Seleccionar…</option>
                <option value="venta">venta</option>
                <option value="alquiler">alquiler</option>
              </select>
              {errores.operacion && <p className="text-xs text-destructive">{errores.operacion}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.habitaciones}>Habitaciones</Etiqueta>
                <Input
                  value={form.habitaciones}
                  onChange={(e) => cambiar('habitaciones', e.target.value)}
                  placeholder="3"
                  inputMode="numeric"
                />
              {errores.habitaciones && <p className="text-xs text-destructive">{errores.habitaciones}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.banos}>Baños</Etiqueta>
              <Input value={form.banos} onChange={(e) => cambiar('banos', e.target.value)} placeholder="2" inputMode="numeric" />
              {errores.banos && <p className="text-xs text-destructive">{errores.banos}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.metros}>M² propiedad (construidos)</Etiqueta>
              <Input
                value={form.metros}
                onChange={(e) => cambiar('metros', e.target.value)}
                placeholder="90"
                inputMode="numeric"
                className={cn(errores.metros && 'border-destructive')}
              />
              {errores.metros && <p className="text-xs text-destructive">{errores.metros}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta error={errores.metrosTerreno}>M² terreno</Etiqueta>
              <Input
                value={form.metrosTerreno}
                onChange={(e) => cambiar('metrosTerreno', e.target.value)}
                placeholder="500"
                inputMode="numeric"
                className={cn(errores.metrosTerreno && 'border-destructive')}
              />
              {errores.metrosTerreno && <p className="text-xs text-destructive">{errores.metrosTerreno}</p>}
            </div>
            <div className="space-y-1">
              <Etiqueta>Puestos de estacionamiento</Etiqueta>
              <Input
                value={form.puestos}
                onChange={(e) => cambiar('puestos', e.target.value)}
                placeholder="1"
                inputMode="numeric"
              />
            </div>
            <div className="space-y-1 sm:col-span-2">
              <Etiqueta>Descripción</Etiqueta>
              <Textarea
                value={form.descripcion}
                onChange={(e) => cambiar('descripcion', e.target.value)}
                placeholder="Orientación, plantas, extras, estado de conservación…"
              />
            </div>
            <div className="sm:col-span-2">
              <FichaFormulario
                tipo={form.tipo}
                extras={ficha.extras}
                minimo={ficha.minimo}
                alCambiarExtra={ficha.cambiarExtra}
                alCambiarMinimo={ficha.setMinimo}
              />
            </div>
            <FotosFormulario
              fotos={form.fotos}
              aviso={avisoFotos}
              alElegir={() => inputFotos.current?.click()}
              alQuitar={quitarFoto}
              alHacerPrincipal={hacerPrincipal}
              inputRef={inputFotos}
              alSeleccionar={(files) => void anadirFotos(files)}
            />
          </div>
        </div>

        <DialogFooter>
          {!esEdicion && draftTieneContenido(form) && (
            <Button type="button" variant="ghost" onClick={descartar} className="sm:mr-auto">
              <Trash2 /> Descartar borrador
            </Button>
          )}
          <Button type="button" variant="outline" onClick={() => alCambiarAbierto(false)}>
            {esEdicion ? 'Cancelar' : 'Cerrar (conserva borrador)'}
          </Button>
          <Button type="button" onClick={guardar}>
            {esEdicion ? 'Guardar cambios' : 'Guardar inmueble'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
    <ModalIA
      abierto={iaAbierto}
      alCambiarAbierto={setIaAbierto}
      textoInicial={[form.titulo, form.descripcion].filter(Boolean).join('\n')}
      fotosIniciales={form.fotos}
      onAplicar={aplicarFichaIA}
    />
    </>
  );
}
