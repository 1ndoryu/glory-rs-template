/* [263A-16] Header del sitio — SidebarTrigger + título dinámico por ruta
 * [283A-20] Añadida campana de notificaciones en tiempo real.
 * [237A-3] Indicador rápido de estado BDP en la barra superior. */

import { useLocation, useNavigate } from "react-router-dom"
import { Separator } from "@/components/ui/separator"
import { SidebarTrigger } from "@/components/ui/sidebar"
import { NotificationBell } from "@/componentes/NotificationBell"
import { useNotificaciones } from "@/hooks/useNotificaciones"
import { useObtenerConfiguracion } from "@/api/generated/configuracion/configuracion"
import { Badge } from "@/components/ui/badge"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { useSetSyncMode } from "@/api/bdp-backup"
import { toast } from "sonner"
import axios from "@/api/axios-instance"
import { useQueryClient } from "@tanstack/react-query"

const titulos: Record<string, string> = {
  "/": "Dashboard",
  "/ventas": "Ventas",
  "/gastos": "Gastos",
  "/reservas": "Reservas",
  "/reservas/calendario": "Calendario",
  "/clientes": "Clientes",
  "/canales": "Canales de Reserva",
  "/reservas/no-shows": "No-Shows",
  "/plano-sala": "Plano de Sala",
  "/configuracion": "Configuración",
  "/marketing/campanas": "Campañas de Marketing",
  "/marketing/campanas/nueva": "Nueva Campaña",
  "/marketing/plantillas": "Plantillas WhatsApp",
  "/marketing/plantillas/nueva": "Nueva Plantilla",
  "/marketing/recordatorios": "Recordatorios",
  "/bdp/stock": "Stock BDP",
  "/bdp/explorador": "Explorador BDP",
  "/bdp/historial": "Historial BDP",
  "/bdp/compras": "Compras BDP",
}

function BdpStatusIndicator() {
  const { data: config } = useObtenerConfiguracion()
  const { mutate: setSyncMode, isPending: isChangingMode } = useSetSyncMode()
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const cfg = config?.status === 200 ? (config.data as unknown as Record<string, unknown>) : null
  if (!cfg) return null

  const syncEnabled = Boolean(cfg.bdp_sync_enabled)
  const syncMode = String(cfg.bdp_sync_mode ?? 'read_only')

  const credencialesOk =
    Boolean(cfg.bdp_base_url) &&
    Boolean(cfg.bdp_login) &&
    Boolean(cfg.bdp_password) &&
    Boolean(cfg.bdp_integrator_code)

  if (!syncEnabled) {
    return (
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button type="button" className="focus:outline-none">
            <Badge variant="outline" className="text-xs gap-1 cursor-pointer hover:bg-muted">
              BDP: off
            </Badge>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-64">
          <div className="px-2 py-1.5 text-sm font-medium">
            Integración BDP desactivada
          </div>
          <DropdownMenuSeparator />
          {credencialesOk ? (
            <DropdownMenuItem onClick={async () => {
              try {
                await axios.patch('/api/configuracion', { bdp_sync_enabled: true })
                await queryClient.invalidateQueries({ queryKey: ['configuracion'] })
                toast.success('BDP activado', { description: 'La integración BDP está ahora en modo lectura.' })
              } catch {
                toast.error('No se pudo activar BDP')
              }
            }}>
              Activar BDP
            </DropdownMenuItem>
          ) : (
            <DropdownMenuItem disabled>
              Sin credenciales — configura BDP primero
            </DropdownMenuItem>
          )}
          <DropdownMenuItem onClick={() => navigate('/configuracion', { state: { bdpSection: 'bdp' } })}>
            {credencialesOk ? 'Configuración BDP' : 'Configurar credenciales BDP'}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    )
  }

  const isWrite = syncMode === 'unidirectional'
  const bdpBaseUrl = String(cfg?.bdp_base_url ?? '')

  function desactivarEscritura() {
    if (isChangingMode) return
    const baseUrl = bdpBaseUrl
    setSyncMode(
      {
        modo: 'read_only',
        confirmarDestino: baseUrl,
        alcances: [],
        duracionMinutos: 0,
        maxOperaciones: 0,
        motivo: '',
        targetEntityType: '',
        targetEntityId: '',
      },
      {
        onSuccess: () => toast.success('BDP vuelve a modo solo lectura'),
        onError: (err: unknown) =>
          toast.error('No se pudo cambiar el modo BDP', {
            description: String((err as { message?: string })?.message ?? 'Error desconocido'),
          }),
      }
    )
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button type="button" className="focus:outline-none" disabled={isChangingMode}>
          {isWrite ? (
            <Badge variant="default" className="text-xs gap-1 bg-amber-600 cursor-pointer hover:bg-amber-700">
              BDP: escritura
            </Badge>
          ) : (
            <Badge variant="secondary" className="text-xs gap-1 cursor-pointer hover:bg-secondary/80">
              BDP: lectura
            </Badge>
          )}
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <div className="px-2 py-1.5 text-sm font-medium">
          Estado BDP: {isWrite ? 'Escritura temporal' : 'Solo lectura'}
        </div>
        <DropdownMenuSeparator />
        {isWrite ? (
          <DropdownMenuItem onClick={desactivarEscritura} disabled={isChangingMode}>
            {isChangingMode ? 'Cambiando...' : 'Desactivar escritura'}
          </DropdownMenuItem>
        ) : (
          /* [C2-3] TODO: restringir a admin/owner cuando el auth store exponga rol. */
          <DropdownMenuItem onClick={() => navigate('/configuracion', { state: { bdpArming: true } })}>
            Activar escritura temporal
          </DropdownMenuItem>
        )}
        <DropdownMenuItem onClick={() => navigate('/configuracion/bdp-backup')}>
          Ver historial BDP
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => navigate('/configuracion', { state: { bdpSection: 'bdp' } })}>
          Configuración BDP
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

export function SiteHeader() {
  const location = useLocation()
  const titulo = titulos[location.pathname] || "Restaurante"

  /* [283A-20] Conectar SSE de notificaciones al montar el header */
  useNotificaciones()

  return (
    <header className="flex h-(--header-height) shrink-0 items-center gap-2 border-b transition-[width,height] ease-linear group-has-data-[collapsible=icon]/sidebar-wrapper:h-(--header-height)">
      <div className="flex w-full items-center gap-1 px-4 lg:gap-2 lg:px-6">
        <SidebarTrigger className="-ml-1" />
        <Separator
          orientation="vertical"
          className="mx-2 data-[orientation=vertical]:h-4"
        />
        <h1 className="text-base font-medium">{titulo}</h1>
        <div className="ml-auto flex items-center gap-2">
          <BdpStatusIndicator />
          <NotificationBell />
        </div>
      </div>
    </header>
  )
}
