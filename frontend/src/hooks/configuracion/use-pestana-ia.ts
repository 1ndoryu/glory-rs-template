import { useEffect, useState } from 'react';
import { useConfigIA } from '@/hooks/ia/use-config-ia';
import type { EstadoIA, ProveedorIA } from '@/domain/ia';

/* [08AA-32] Borrador de la pestaña IA (regla 8 / componente-sin-hook):
 * formulario local sobre el estado del servidor (activo + habilitados),
 * detección de cambios y guardado. El componente (`pestana-ia.tsx`) queda
 * con solo JSX + `fechaCorta` de presentación. */

export interface BorradorIA {
  activo: ProveedorIA;
  hab: Record<ProveedorIA, boolean>;
}

function desdeEstado(e: EstadoIA): BorradorIA {
  const hab = { gloryapi: true, 'opencode-go': true } as Record<ProveedorIA, boolean>;
  for (const p of e.proveedores) {
    if (p.id === 'gloryapi' || p.id === 'opencode-go') hab[p.id] = p.habilitado;
  }
  return { activo: e.activo, hab };
}

function sucio(form: BorradorIA, e: EstadoIA): boolean {
  const base = desdeEstado(e);
  return form.activo !== base.activo || form.hab.gloryapi !== base.hab.gloryapi || form.hab['opencode-go'] !== base.hab['opencode-go'];
}

export function usePestanaIA() {
  const { estado, cargando, guardando, probando, error, recargar, guardar, probar } = useConfigIA(true);
  const [form, setForm] = useState<BorradorIA | null>(null);

  useEffect(() => {
    if (estado && !form) setForm(desdeEstado(estado));
  }, [estado, form]);

  const borrador = form ?? (estado ? desdeEstado(estado) : null);
  const modificado = form !== null && estado !== null ? sucio(form, estado) : false;

  const poner = (parche: Partial<BorradorIA>) => {
    if (borrador) setForm({ ...borrador, ...parche });
  };
  const ponerHab = (id: ProveedorIA, v: boolean) => {
    if (borrador) setForm({ ...borrador, hab: { ...borrador.hab, [id]: v } });
  };

  async function guardarCambios(): Promise<void> {
    if (!borrador) return;
    await guardar(borrador.activo, borrador.hab.gloryapi, borrador.hab['opencode-go']);
    setForm(null);
  }

  return {
    estado,
    cargando,
    guardando,
    probando,
    error,
    recargar,
    probar,
    borrador,
    modificado,
    poner,
    ponerHab,
    guardarCambios,
  };
}
