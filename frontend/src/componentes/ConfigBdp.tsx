/* [065A-2] Configuracion de BDP/WebLink REST API.
 * Mantiene credenciales fuera de respuestas publicas y ofrece diagnostico
 * Health + Login + GetVersion para la sesion remota con el PC del restaurante.
 * [147A-F5.6] Secciones de mapeo: tender, order_type, customer_code, poll_interval.
 * [197A-3] Distingue integración, lecturas y permiso puntual de escritura.
 * [237A-3] Desenterrado: catálogo, mapeos técnicos y polling ahora son visibles
 *          directamente. Selector de modo autorización integrado. */

import { useState } from 'react';
import { Activity, CheckCircle2, ChevronDown, ChevronRight, ClipboardCheck, Loader2, XCircle, BookOpen, Settings, Radio, Shield, ToggleLeft } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import axios from '@/api/axios-instance';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { toast } from 'sonner';
import type { EstadoConfiguracion } from '../hooks/useConfiguracion';
import type { BdpDiagnosticoResponse } from '../api/generated/gestionRestauranteAPI.schemas';
import ConfigBdpMapeos from '@/components/config-bdp-mapeos';

interface BdpSyncDryRunCheck {
  nombre: string;
  endpoint: string;
  ok: boolean;
  mensaje: string;
  cantidad?: number | null;
  muestra?: string | null;
}

interface BdpSyncDryRunResponse {
  configurado: boolean;
  sync_habilitado: boolean;
  escritura_real: boolean;
  listo_para_sincronizar: boolean;
  mensaje: string;
  checks: BdpSyncDryRunCheck[];
}

interface BdpUiState {
  diagnostico: BdpDiagnosticoResponse | null;
  dryRun: BdpSyncDryRunResponse | null;
  diagnosticando: boolean;
  probandoSync: boolean;
}

interface ConfigBdpProps {
  config: EstadoConfiguracion;
  cambiarCampo: <K extends keyof EstadoConfiguracion>(campo: K, valor: EstadoConfiguracion[K]) => void;
  guardar?: () => void;
  guardando?: boolean;
  mensaje?: string;
}

function ConfigBdp({ config, cambiarCampo, guardar, guardando, mensaje }: ConfigBdpProps) {
  const [estadoBdp, setEstadoBdp] = useState<BdpUiState>({
    diagnostico: null,
    dryRun: null,
    diagnosticando: false,
    probandoSync: false,
  });
  const [mostrarMapeos, setMostrarMapeos] = useState(false);
  const { diagnostico, dryRun, diagnosticando, probandoSync } = estadoBdp;
  const simuladorLocal = /^https?:\/\/(localhost|127\.0\.0\.1|\[::1\])(?::\d+)?\/?$/i.test(
    config.bdp_base_url.trim(),
  );

  async function diagnosticar() {
    setEstadoBdp((actual) => ({ ...actual, diagnosticando: true }));
    try {
      const resp = await axios.get<BdpDiagnosticoResponse>('/api/configuracion/bdp/diagnostico');
      setEstadoBdp((actual) => ({ ...actual, diagnostico: resp.data }));
      if (resp.data.health_ok && resp.data.login_ok) {
        toast.success('BDP conectado', { description: resp.data.mensaje });
      } else {
        toast.warning('BDP pendiente', { description: resp.data.mensaje });
      }
    } catch (err: unknown) {
      const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message ?? 'No se pudo diagnosticar BDP';
      toast.error('Error BDP', { description: msg });
    } finally {
      setEstadoBdp((actual) => ({ ...actual, diagnosticando: false }));
    }
  }

  async function probarSincronizacion() {
    setEstadoBdp((actual) => ({ ...actual, probandoSync: true }));
    try {
      const resp = await axios.get<BdpSyncDryRunResponse>('/api/configuracion/bdp/sync-dry-run');
      setEstadoBdp((actual) => ({ ...actual, dryRun: resp.data }));
      if (resp.data.listo_para_sincronizar) {
        toast.success('Sincronización validada', { description: resp.data.mensaje });
      } else {
        toast.warning('Sincronización pendiente', { description: resp.data.mensaje });
      }
    } catch (err: unknown) {
      const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message ?? 'No se pudo probar la sincronización BDP';
      toast.error('Error BDP', { description: msg });
    } finally {
      setEstadoBdp((actual) => ({ ...actual, probandoSync: false }));
    }
  }

  return (
    <>
    <Card>
      <CardHeader>
        <CardTitle>Conexión BDP</CardTitle>
        <CardDescription>Conexión al TPV/BDP del restaurante. La puesta en marcha debe dejar estos datos preparados; el cliente no debe inventar códigos ni identificadores.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-base-url">URL pública BDP</Label>
          <Input
            id="bdp-base-url"
            type="url"
            value={config.bdp_base_url}
            onChange={(e) => cambiarCampo('bdp_base_url', e.target.value)}
            placeholder="https://ip-o-dominio:8080"
          />
        </div>
        <div className="grid gap-4 md:grid-cols-3">
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-login">Login</Label>
            <Input
              id="bdp-login"
              value={config.bdp_login}
              onChange={(e) => cambiarCampo('bdp_login', e.target.value)}
              autoComplete="off"
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-password">Password</Label>
            <Input
              id="bdp-password"
              type="password"
              value={config.bdp_password}
              onChange={(e) => cambiarCampo('bdp_password', e.target.value)}
              autoComplete="new-password"
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-integrator-code">Código integrador</Label>
            <Input
              id="bdp-integrator-code"
              type="password"
              value={config.bdp_integrator_code}
              onChange={(e) => cambiarCampo('bdp_integrator_code', e.target.value)}
              autoComplete="off"
            />
          </div>
        </div>
        <div className="grid gap-4 md:grid-cols-3">
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-pos-id">Terminal POS</Label>
            <Input
              id="bdp-pos-id"
              type="number"
              min={1}
              value={config.bdp_pos_id}
              onChange={(e) => cambiarCampo('bdp_pos_id', Number(e.target.value))}
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-employee-id">Empleado</Label>
            <Input
              id="bdp-employee-id"
              type="number"
              min={1}
              value={config.bdp_employee_id}
              onChange={(e) => cambiarCampo('bdp_employee_id', Number(e.target.value))}
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-items-profile-id">Perfil artículos</Label>
            <Input
              id="bdp-items-profile-id"
              type="number"
              min={1}
              value={config.bdp_items_profile_id}
              onChange={(e) => cambiarCampo('bdp_items_profile_id', Number(e.target.value))}
            />
          </div>
        </div>
        <div className="flex items-center justify-between gap-4 rounded-md border p-3">
          <div>
            <Label htmlFor="bdp-sync-enabled">Integración BDP activa</Label>
            <p className="text-xs text-muted-foreground">Interruptor general. Activarlo no concede permiso para crear clientes, comandas, pagos ni facturas.</p>
          </div>
          <Switch
            id="bdp-sync-enabled"
            checked={config.bdp_sync_enabled}
            onCheckedChange={(checked: boolean) => cambiarCampo('bdp_sync_enabled', checked)}
          />
        </div>

        <div className="grid gap-3 md:grid-cols-3">
          <div className="rounded-md border p-3">
            <p className="text-sm font-medium">BDP → Glory</p>
            <p className="mt-1 text-xs text-muted-foreground">Catálogo, clientes, mesas y estados se consultan o importan sin modificar BDP.</p>
          </div>
          <div className="rounded-md border p-3">
            <p className="text-sm font-medium">Glory → BDP</p>
            <p className="mt-1 text-xs text-muted-foreground">Solo una operación concreta con permiso temporal. Después vuelve a Solo lectura.</p>
          </div>
          <div className="rounded-md border p-3">
            <p className="text-sm font-medium">Dos vías automáticas</p>
            <p className="mt-1 text-xs text-muted-foreground">No están habilitadas. Se separan las lecturas seguras de cada escritura autorizada.</p>
          </div>
        </div>

        {/* [237A-3] Catálogo BDP — sección visible de primer nivel */}
        <div className="border-t pt-4">
          <div className="flex items-center gap-2 mb-3">
            <BookOpen className="size-4 text-muted-foreground" />
            <span className="text-sm font-medium">Catálogo de artículos BDP</span>
          </div>
          <p className="mb-3 text-xs text-muted-foreground">
            Sincroniza el catálogo de artículos desde BDP a Glory. Crea mapeos automáticos por código y actualiza precios.
          </p>
          <ConfigBdpMapeos config={config} cambiarCampo={cambiarCampo} soloArticulos />
        </div>

        {/* [237A-3] Polling automático — visible en vista principal */}
        <div className="border-t pt-4">
          <div className="flex items-center gap-2 mb-3">
            <Radio className="size-4 text-muted-foreground" />
            <span className="text-sm font-medium">Actualización de estados</span>
          </div>
          <div className="grid gap-3 md:grid-cols-2">
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="bdp-poll-enabled-main">Actualizar estados automáticamente</Label>
                <p className="text-xs text-muted-foreground">Consulta periódicamente el estado de comandas BDP. No crea ni modifica registros.</p>
              </div>
              <Switch
                id="bdp-poll-enabled-main"
                checked={config.bdp_poll_enabled}
                onCheckedChange={(checked: boolean) => cambiarCampo('bdp_poll_enabled', checked)}
              />
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="bdp-poll-interval-main">Frecuencia de actualización (segundos)</Label>
              <Input
                id="bdp-poll-interval-main"
                type="number"
                min={10}
                max={600}
                value={config.bdp_poll_interval_secs}
                onChange={(e) => cambiarCampo('bdp_poll_interval_secs', Number(e.target.value))}
              />
              <p className="text-xs text-muted-foreground">Cada cuántos segundos Glory consulta el estado de comandas (10-600).</p>
            </div>
          </div>
        </div>

        {/* [237A-3] Configuración técnica — colapsable, contenido restante */}
        <div className="border-t pt-4">
          <button
            type="button"
            className="flex items-center gap-2 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
            onClick={() => setMostrarMapeos(!mostrarMapeos)}
          >
            {mostrarMapeos ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
            <Settings className="size-4" />
            Correspondencias Glory ↔ BDP (solo soporte)
          </button>
          {!mostrarMapeos && (
            <p className="mt-1 text-xs text-muted-foreground">
              Formas de pago, canales, artículo por defecto, cliente por defecto y exigir cliente confirmado. No debe modificarse sin verificar los valores con el restaurante.
            </p>
          )}
          {mostrarMapeos && (
            <div className="mt-4">
              <p className="mb-4 rounded-md border p-3 text-xs text-muted-foreground">
                Estos valores no son universales: formas de pago y canales pueden ser distintos en cada BDP.
              </p>
              <ConfigBdpMapeos config={config} cambiarCampo={cambiarCampo} soloMapeosTecnicos />
            </div>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <Button type="button" variant="outline" onClick={diagnosticar} disabled={diagnosticando}>
            {diagnosticando ? <Loader2 className="size-4 animate-spin" /> : <Activity className="size-4" />}
            Probar conexión
          </Button>
          <Button type="button" variant="secondary" onClick={probarSincronizacion} disabled={probandoSync || !simuladorLocal}>
            {probandoSync ? <Loader2 className="size-4 animate-spin" /> : <ClipboardCheck className="size-4" />}
            Validar con simulador local
          </Button>
          {diagnostico && (
            <span className={diagnostico.health_ok && diagnostico.login_ok ? 'text-sm text-green-600' : 'text-sm text-destructive'}>
              {diagnostico.mensaje}
            </span>
          )}
        </div>
        {!simuladorLocal && (
          <p className="text-xs text-muted-foreground">
            La validación de comandas está bloqueada contra el restaurante porque utiliza el mismo endpoint de creación. Solo puede ejecutarse con el simulador local.
          </p>
        )}
        {diagnostico?.version && (
          <div className="grid gap-2 rounded-md border p-3 text-sm md:grid-cols-2">
            <span>Versión: {diagnostico.version}.{diagnostico.sub_version ?? 0}</span>
            <span>Aplicación: {diagnostico.application_description || diagnostico.application}</span>
          </div>
        )}
        {dryRun && (
          <div className="flex flex-col gap-3 rounded-md border p-3 text-sm">
            <div className="flex items-start gap-2">
              {dryRun.listo_para_sincronizar ? (
                <CheckCircle2 className="mt-0.5 size-4 text-green-600" />
              ) : (
                <XCircle className="mt-0.5 size-4 text-destructive" />
              )}
              <div className="flex flex-col gap-1">
                <span className={dryRun.listo_para_sincronizar ? 'font-medium text-green-700' : 'font-medium text-destructive'}>
                  {dryRun.mensaje}
                </span>
                <span className="text-xs text-muted-foreground">
                  Escritura real: {dryRun.escritura_real ? 'sí' : 'no'} · Sync activo: {dryRun.sync_habilitado ? 'sí' : 'no'}
                </span>
              </div>
            </div>
            <div className="grid gap-2 md:grid-cols-2">
              {dryRun.checks.map((check) => (
                <div key={`${check.nombre}-${check.endpoint}`} className="rounded-md border p-2">
                  <div className="flex items-center gap-2">
                    {check.ok ? <CheckCircle2 className="size-4 text-green-600" /> : <XCircle className="size-4 text-destructive" />}
                    <span className="font-medium">{check.nombre}</span>
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">{check.mensaje}</p>
                  {(check.cantidad !== null && check.cantidad !== undefined) || check.muestra ? (
                    <p className="mt-1 text-xs text-muted-foreground">
                      {check.cantidad !== null && check.cantidad !== undefined ? `${check.cantidad} registros` : check.muestra}
                    </p>
                  ) : null}
                </div>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>

    {/* [237A-3] Información sobre modo de autorización — el selector real vive en PanelBdpBackup */}
    <Card className="mt-4">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Shield className="size-4" />
          Modo de operaciones BDP
        </CardTitle>
        <CardDescription>
          Controla cómo Glory interactúa con BDP para operaciones de escritura (crear comandas, pagar, facturar, crear clientes).
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="grid gap-3 md:grid-cols-2">
          <div className={`rounded-md border p-3 transition-colors ${(config.bdp_sync_mode || 'read_only') === 'read_only' ? 'border-primary bg-primary/5' : ''}`}>
            <div className="flex items-center gap-2 mb-1">
              {(config.bdp_sync_mode || 'read_only') === 'read_only' && (
                <Badge variant="default">Activo</Badge>
              )}
              <p className="text-sm font-medium">Solo lectura (BDP → Glory)</p>
            </div>
            <p className="text-xs text-muted-foreground">
              Permite consultas e importaciones. No se puede crear ni modificar nada en BDP. Es el modo seguro por defecto.
            </p>
          </div>
          <div className={`rounded-md border p-3 transition-colors ${config.bdp_sync_mode === 'unidirectional' ? 'border-amber-500 bg-amber-50 dark:bg-amber-950/20' : ''}`}>
            <div className="flex items-center gap-2 mb-1">
              {config.bdp_sync_mode === 'unidirectional' && (
                <Badge variant="default" className="bg-amber-600">Activo</Badge>
              )}
              <p className="text-sm font-medium">Autorización manual (Glory → BDP)</p>
            </div>
            <p className="text-xs text-muted-foreground">
              Para cada operación de escritura se requiere confirmación textual y un arming temporal. Después vuelve automáticamente a Solo lectura.
            </p>
          </div>
        </div>
        <div className="rounded-md border border-dashed p-3">
          <p className="text-sm text-muted-foreground">
            Para cambiar el modo de autorización, usa el selector <strong>"Permiso de operación"</strong> en el panel de <strong>Seguridad, respaldos e historial BDP</strong> más abajo en esta misma pestaña. Ese flujo incluye la confirmación de destino, alcance, motivo y duración requeridos para cada escritura.
          </p>
        </div>
      </CardContent>
    </Card>

    {/* [267A-4] Feature flags BDP — toggles para activar funcionalidades por restaurante */}
    <Card className="mt-4">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <ToggleLeft className="size-4" />
          Funcionalidades BDP
        </CardTitle>
        <CardDescription>
          Activa o desactiva funcionalidades avanzadas de la integración BDP. Los cambios se aplican inmediatamente sin necesidad de redeploy. Todos están desactivados por defecto por seguridad.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-4 rounded-md border p-3">
          <div>
            <Label htmlFor="ff-auto-arm">Auto-arming</Label>
            <p className="text-xs text-muted-foreground">Permite que las operaciones de escritura (comandas, pagos, facturas) activen automáticamente un permiso temporal sin ir a Configuración. Tras la operación vuelve a solo lectura.</p>
          </div>
          <Switch
            id="ff-auto-arm"
            checked={config.ff_bdp_auto_arm}
            onCheckedChange={(checked: boolean) => cambiarCampo('ff_bdp_auto_arm', checked)}
          />
        </div>
        <div className="flex items-center justify-between gap-4 rounded-md border p-3">
          <div>
            <Label htmlFor="ff-partial-payments">Pagos parciales</Label>
            <p className="text-xs text-muted-foreground">Permite pagar una comanda BDP en varios pagos parciales en vez de un único pago completo. Cada pago se registra en un ledger local con clave de idempotencia.</p>
          </div>
          <Switch
            id="ff-partial-payments"
            checked={config.ff_bdp_partial_payments}
            onCheckedChange={(checked: boolean) => cambiarCampo('ff_bdp_partial_payments', checked)}
          />
        </div>
        <div className="flex items-center justify-between gap-4 rounded-md border p-3">
          <div>
            <div className="flex items-center gap-2">
              <Label htmlFor="ff-cancel-order">Cancelar comandas</Label>
              <Badge variant="outline" className="text-xs bg-amber-50 text-amber-700 border-amber-300 dark:bg-amber-950/30 dark:text-amber-400 dark:border-amber-700">Bloqueado por BDP</Badge>
            </div>
            <p className="text-xs text-muted-foreground">Permitiría cancelar comandas directamente en BDP. El endpoint devuelve "Subscripción no activada" — activar solo cuando BDP habilite el módulo.</p>
          </div>
          <Switch
            id="ff-cancel-order"
            checked={config.ff_bdp_cancel_order}
            onCheckedChange={(checked: boolean) => cambiarCampo('ff_bdp_cancel_order', checked)}
          />
        </div>
        <div className="border-t pt-3">
          <p className="text-xs font-medium text-muted-foreground mb-3">Compras (albaranes de proveedores BDP)</p>
          <div className="grid gap-3 md:grid-cols-3">
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="ff-pn-read">Lectura</Label>
                <p className="text-xs text-muted-foreground">Sincronizar albaranes de compra desde BDP.</p>
              </div>
              <Switch
                id="ff-pn-read"
                checked={config.ff_bdp_purchase_notes_read}
                onCheckedChange={(checked: boolean) => cambiarCampo('ff_bdp_purchase_notes_read', checked)}
              />
            </div>
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="ff-pn-draft">Borradores</Label>
                <p className="text-xs text-muted-foreground">Crear borradores de compra locales (sin escribir en BDP).</p>
              </div>
              <Switch
                id="ff-pn-draft"
                checked={config.ff_bdp_purchase_notes_draft}
                onCheckedChange={(checked: boolean) => cambiarCampo('ff_bdp_purchase_notes_draft', checked)}
              />
            </div>
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="ff-pn-receive">Conciliación</Label>
                <p className="text-xs text-muted-foreground">Recepcionar y conciliar compras con gastos existentes o nuevos.</p>
              </div>
              <Switch
                id="ff-pn-receive"
                checked={config.ff_bdp_purchase_notes_receive}
                onCheckedChange={(checked: boolean) => cambiarCampo('ff_bdp_purchase_notes_receive', checked)}
              />
            </div>
          </div>
        </div>
      </CardContent>
    </Card>

    {/* [167A-1] Botón guardar propio de la pestaña BDP */}
    {guardar && (
      <div className="flex items-center gap-4 mt-4">
        <Button onClick={guardar} disabled={guardando}>
          {guardando ? 'Guardando...' : 'Guardar conexión BDP'}
        </Button>
        {mensaje && (
          <span className={`text-sm ${mensaje.includes('Error') ? 'text-destructive' : 'text-green-600'}`}>
            {mensaje}
          </span>
        )}
      </div>
    )}
    </>
  );
}

export default ConfigBdp;
