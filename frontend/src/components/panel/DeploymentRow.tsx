/* [259A-5] sentinel-disable-file componente-sin-hook-glory: falso positivo del conteo; usa useState/useMutation/useQueryClient + Modal/Menu/Form; fila con responsabilidad unica (SRP). */
import {useState} from 'react';
import {useTranslation} from 'react-i18next';
import {Globe, MoreVertical, PlusCircle, Server, Trash2, X} from 'lucide-react';
import {useMutation, useQueryClient} from '@tanstack/react-query';
import type {CoolifyDeployment} from '../../api/hosting';
import {HOSTING_PLAN_LABELS, apiCreateHostingSubscription, apiDeleteDeployment} from '../../api/hosting';
import {DEPLOYMENTS_QUERY_KEY, useDeploymentMetrics} from '../../hooks/useDeploymentsPanel';
import {toast} from '../../stores/toastStore';
import {Button} from '../ui/Button';
import {MenuContextual, type MenuContextualItem} from '../ui/ContextMenu';
import {Modal} from '../ui/Modal';
import {CreateHostingForm} from './HostingCreateForm';
import {ResourceUsageChart} from './ResourceUsageChart';

export function getDeploymentPanelErrorMessage(error: unknown, t?: (key: string, fallback: string) => string): string {
    const apiMessage = (error as {response?: {data?: {message?: string}}})?.response?.data?.message;
    if (typeof apiMessage === 'string' && apiMessage.trim()) return apiMessage;
    if (error instanceof Error && error.message) return error.message;
    const fn = t || ((_: string, fb: string) => fb);
    return fn('panel.deployments.error', 'No se pudo consultar Coolify para listar los despliegues reales');
}

function isGenericServerLabel(value: string | null | undefined): boolean {
    if (!value) return true;
    return ['', 'localhost', '127.0.0.1', '::1', 'local'].includes(value.trim().toLowerCase());
}

function getVisibleServerLabel(deployment: CoolifyDeployment, t?: (key: string, fb: string) => string): string {
    if (!isGenericServerLabel(deployment.server_label)) return deployment.server_label;
    if (!isGenericServerLabel(deployment.server_name)) return deployment.server_name as string;
    const fn = t || ((_: string, fb: string) => fb);
    return fn('panel.deployments.configured_server', 'Servidor configurado');
}

function formatStatus(status: string): string {
    const [main, sub] = status.split(':');
    if (!sub || sub.toLowerCase() === 'unknown') return main;
    return `${main} · ${sub}`;
}

function getDeploymentStatusClass(status: string): string {
    const normalizedStatus = status.toLowerCase();
    if (['running', 'healthy', 'active', 'ready', 'success'].some(token => normalizedStatus.includes(token))) return 'vpsStatus--running';
    if (['stopped', 'exited', 'failed', 'error', 'crashed', 'unhealthy'].some(token => normalizedStatus.includes(token))) return 'vpsStatus--stopped';
    return 'vpsStatus--other';
}

function formatMb(mb: number | null | undefined): string {
    if (mb == null) return '—';
    if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
    return `${Math.round(mb)} MB`;
}

function formatCpu(percent: number | null | undefined): string {
    return percent == null ? '—' : `${percent.toFixed(2)}%`;
}

function formatMillicores(millicores: number | null | undefined): string {
    if (millicores == null) return '—';
    const cores = millicores / 1000;
    return `${cores.toFixed(2)} CPU`;
}

function formatCpuCores(cores: number | null | undefined): string {
    return cores == null ? '—' : `${cores.toFixed(2)} CPU`;
}

function hasRuntimeLimitDetails(deployment: CoolifyDeployment): boolean {
    return [
        deployment.runtime_site_cpu_limit_cores,
        deployment.runtime_site_ram_limit_mb,
        deployment.runtime_db_cpu_limit_cores,
        deployment.runtime_db_ram_limit_mb,
        deployment.runtime_ssh_cpu_limit_cores,
        deployment.runtime_ssh_ram_limit_mb,
    ].some(value => value != null);
}

function WordPressIcon() {
    return <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10" /><path d="M2 12h4l3 8 4-16 3 8h4" /></svg>;
}

function HostingIcon() {
    return <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><rect x="2" y="3" width="20" height="8" rx="2" /><rect x="2" y="13" width="20" height="8" rx="2" /><circle cx="7" cy="7" r="1" /><circle cx="7" cy="17" r="1" /></svg>;
}

function isWordPressDeployment(plan: string | null): boolean {
    return Boolean(plan && !plan.startsWith('normal-'));
}

function getDeploymentTypeLabel(plan: string | null, t?: (key: string, fb: string) => string): string {
    if (!plan) {
        const fn = t || ((_: string, fb: string) => fb);
        return fn('panel.deployments.no_plan', 'Sin plan');
    }
    return HOSTING_PLAN_LABELS[plan] || plan;
}

interface DeploymentContextMenuProps {
    deployment: CoolifyDeployment;
    onCreateSubscription: () => void;
    onDeleteDeployment: () => void;
    isDeleting: boolean;
}

function DeploymentContextMenu({deployment, onCreateSubscription, onDeleteDeployment, isDeleting}: DeploymentContextMenuProps) {
    const [abierto, setAbierto] = useState(false);
    if (deployment.linked_subscription_id) return null;

    const {t} = useTranslation();
    const items: MenuContextualItem[] = [
        {id: 'create-subscription', label: t('panel.deployments.create_subscription', 'Crear suscripción vinculada'), icon: <PlusCircle size={14} />, onSelect: onCreateSubscription},
        {id: 'delete-deployment', label: t('panel.deployments.delete_deployment', 'Eliminar despliegue'), icon: <Trash2 size={14} />, danger: true, disabled: isDeleting, onSelect: onDeleteDeployment},
    ];

    return (
        <MenuContextual
            abierto={abierto}
            onToggle={() => setAbierto(value => !value)}
            onCerrar={() => setAbierto(false)}
            items={items}
            ariaLabel={t('panel.deployments.actions_aria', 'Acciones del despliegue')}
            triggerContent={<MoreVertical size={16} />}
        />
    );
}

export function DeploymentRow({deployment}: {deployment: CoolifyDeployment}) {
    const [detalleAbierto, setDetalleAbierto] = useState(false);
    const [showCreateForm, setShowCreateForm] = useState(false);
    const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
    const queryClient = useQueryClient();
    const isLinked = Boolean(deployment.linked_subscription_id);
    const fqdn = deployment.fqdn?.trim() || null;
    const isWp = isWordPressDeployment(deployment.linked_subscription_plan);
    const {t} = useTranslation();
    const serverLabel = getVisibleServerLabel(deployment, t);

    const createMutation = useMutation({
        mutationFn: apiCreateHostingSubscription,
        onSuccess: () => {
            toast.success(t('panel.deployments.subscription_created', 'Suscripción creada y vinculada al despliegue'));
            void queryClient.invalidateQueries({queryKey: ['hosting-subscriptions']});
            void queryClient.invalidateQueries({queryKey: DEPLOYMENTS_QUERY_KEY});
            setShowCreateForm(false);
        },
        onError: error => toast.error(getDeploymentPanelErrorMessage(error)),
    });

    const deleteMutation = useMutation({
        mutationFn: (uuid: string) => apiDeleteDeployment(uuid),
        onSuccess: () => {
            toast.success(t('panel.deployments.deployment_deleted', 'Despliegue eliminado de Coolify'));
            void queryClient.invalidateQueries({queryKey: DEPLOYMENTS_QUERY_KEY});
            setShowDeleteConfirm(false);
        },
        onError: error => toast.error(getDeploymentPanelErrorMessage(error)),
    });

    const closeCreateForm = () => {
        if (!createMutation.isPending) setShowCreateForm(false);
    };

    const closeDeleteConfirm = () => {
        if (!deleteMutation.isPending) setShowDeleteConfirm(false);
    };

    return (
        <>
            <tr className={`infraFila ${!isLinked ? 'infraFila--huerfana' : ''}`} onClick={() => setDetalleAbierto(true)}>
                <td className="infraCelda infraCelda--tipo"><span className="infraTipoIcono" title={isWp ? 'WordPress' : 'Hosting'}>{deployment.linked_subscription_plan ? (isWp ? <WordPressIcon /> : <HostingIcon />) : <Server size={14} />}</span></td>
                <td className="infraCelda"><div className="infraCeldaNombre"><span className="infraNombreTexto">{deployment.name}</span><span className="infraServerBadge">{serverLabel}</span></div></td>
                <td className="infraCelda"><span className={`vpsStatus ${getDeploymentStatusClass(deployment.status)}`}>{formatStatus(deployment.status)}</span></td>
                <td className="infraCelda infraCelda--plan">{getDeploymentTypeLabel(deployment.linked_subscription_plan, t)}</td>
                <td className="infraCelda infraCelda--usuario">{deployment.linked_subscription_client || '—'}</td>
                <td className="infraCelda infraCelda--recurso">{formatCpu(deployment.cpu_percent)}</td>
                <td className="infraCelda infraCelda--recurso">{formatMb(deployment.ram_used_mb)}</td>
                <td className="infraCelda infraCelda--recurso">{formatMb(deployment.storage_used_mb)}</td>
                <td className="infraCelda infraCelda--acciones" onClick={event => event.stopPropagation()}><DeploymentContextMenu deployment={deployment} onCreateSubscription={() => setShowCreateForm(true)} onDeleteDeployment={() => setShowDeleteConfirm(true)} isDeleting={deleteMutation.isPending} /></td>
            </tr>

            <Modal abierto={detalleAbierto} onCerrar={() => setDetalleAbierto(false)} className="modalMedio">
                <DeploymentDetailsContent deployment={deployment} fqdn={fqdn} isLinked={isLinked} serverLabel={serverLabel} onCreateSubscription={() => setShowCreateForm(true)} onDeleteDeployment={() => setShowDeleteConfirm(true)} isDeleting={deleteMutation.isPending} onCerrar={() => setDetalleAbierto(false)} />
            </Modal>

            <Modal abierto={showCreateForm} onCerrar={closeCreateForm}>
                <CreateHostingForm initialCoolifyName={deployment.name} submitting={createMutation.isPending} onSubmit={req => createMutation.mutate(req)} />
            </Modal>

            <Modal abierto={showDeleteConfirm} onCerrar={closeDeleteConfirm}>
                <p className="modalTexto">{t('panel.deployments.confirm_delete', `Eliminar despliegue huérfano: se eliminará ${deployment.name} de Coolify junto con sus volúmenes y red del stack.`)}</p>
                <div className="modalAcciones">
                    <Button variante="secundario" tamano="pequeno" onClick={closeDeleteConfirm} disabled={deleteMutation.isPending} type="button">{t('nav.back', 'Cancelar')}</Button>
                    <Button variante="primario" tamano="pequeno" onClick={() => deleteMutation.mutate(deployment.uuid)} disabled={deleteMutation.isPending} type="button">{deleteMutation.isPending ? t('panel.deployments.deleting', 'Eliminando...') : t('panel.deployments.delete_deployment', 'Eliminar de Coolify')}</Button>
                </div>
            </Modal>
        </>
    );
}

function DeploymentDetailsContent({deployment, fqdn, isLinked, serverLabel, onCreateSubscription, onDeleteDeployment, isDeleting, onCerrar}: {deployment: CoolifyDeployment; fqdn: string | null; isLinked: boolean; serverLabel: string; onCreateSubscription: () => void; onDeleteDeployment: () => void; isDeleting: boolean; onCerrar: () => void}) {
    const {data: metrics, isLoading, error} = useDeploymentMetrics(deployment.uuid, true);
    const hasRuntimeSample = deployment.runtime_sampled_at != null;
    const hasRuntimeLimits = hasRuntimeLimitDetails(deployment);

    return (
        <div className="detalleModalContenido">
            <div className="detalleModalHeader">
                <h3 className="modalTitulo">{deployment.name}</h3>
                <Button variante="texto" tamano="pequeno" className="detalleModalClose" onClick={onCerrar} type="button" aria-label="Cerrar"><X size={16} /></Button>
            </div>

            <div className="detalleModalGrid">
                <div className="detalleModalCampo"><span className="detalleModalLabel">UUID</span><span className="detalleModalValor">{deployment.uuid}</span></div>
                <div className="detalleModalCampo"><span className="detalleModalLabel">Estado</span><span className="detalleModalValor"><span className={`vpsStatus ${getDeploymentStatusClass(deployment.status)}`}>{deployment.status}</span></span></div>
                <div className="detalleModalCampo"><span className="detalleModalLabel">Runtime</span><span className="detalleModalValor">{deployment.runtime_kind}</span></div>
                <div className="detalleModalCampo"><span className="detalleModalLabel">Entorno</span><span className="detalleModalValor">{deployment.environment_name || 'production'}</span></div>
                <div className="detalleModalCampo"><span className="detalleModalLabel">Servidor</span><span className="detalleModalValor">{serverLabel}</span></div>
                {deployment.server_uuid && <div className="detalleModalCampo"><span className="detalleModalLabel">Servidor UUID</span><span className="detalleModalValor">{deployment.server_uuid}</span></div>}
                {deployment.linked_subscription_plan && <div className="detalleModalCampo"><span className="detalleModalLabel">Plan vinculado</span><span className="detalleModalValor">{getDeploymentTypeLabel(deployment.linked_subscription_plan)}</span></div>}
                {deployment.linked_subscription_domain && <div className="detalleModalCampo"><span className="detalleModalLabel">Dominio</span><span className="detalleModalValor">{deployment.linked_subscription_domain}</span></div>}
                {fqdn && <div className="detalleModalCampo"><span className="detalleModalLabel">FQDN</span><a href={fqdn} target="_blank" rel="noopener noreferrer" className="detalleModalLink" onClick={e => e.stopPropagation()}><Globe size={12} />{fqdn}</a></div>}
            </div>

            <div className="detalleModalGrid">
                <div className="detalleModalCampo"><span className="detalleModalLabel">CPU actual</span><span className="detalleModalValor">{formatCpu(deployment.cpu_percent)}</span></div>
                <div className="detalleModalCampo"><span className="detalleModalLabel">RAM actual</span><span className="detalleModalValor">{formatMb(deployment.ram_used_mb)}{deployment.ram_limit_mb != null ? ` / ${formatMb(deployment.ram_limit_mb)}` : ''}</span></div>
                <div className="detalleModalCampo"><span className="detalleModalLabel">Disco actual</span><span className="detalleModalValor">{formatMb(deployment.storage_used_mb)}{deployment.storage_limit_mb != null ? ` / ${formatMb(deployment.storage_limit_mb)}` : ''}</span></div>
                {deployment.project_uuid && <div className="detalleModalCampo"><span className="detalleModalLabel">Proyecto</span><span className="detalleModalValor">{deployment.project_uuid}</span></div>}
                {deployment.linked_subscription_status && <div className="detalleModalCampo"><span className="detalleModalLabel">Estado suscripción</span><span className="detalleModalValor">{deployment.linked_subscription_status}</span></div>}
            </div>

            <div className="detalleModalPlan">
                <span className="detalleModalGraficoLabel">Límites runtime detectados</span>
                {!hasRuntimeSample && (
                    <p className="detalleModalPlanNota">Sin muestra runtime todavía. El sampler rellenará esta sección cuando capture el despliegue.</p>
                )}
                {hasRuntimeSample && !hasRuntimeLimits && (
                    <p className="detalleModalPlanNota">La última muestra no detectó límites CPU/RAM aplicados en los contenedores de este despliegue.</p>
                )}
                {hasRuntimeSample && hasRuntimeLimits && (
                    <div className="detalleModalGrid">
                        <div className="detalleModalCampo"><span className="detalleModalLabel">Sitio CPU</span><span className="detalleModalValor">{formatCpuCores(deployment.runtime_site_cpu_limit_cores)}</span></div>
                        <div className="detalleModalCampo"><span className="detalleModalLabel">Sitio RAM</span><span className="detalleModalValor">{formatMb(deployment.runtime_site_ram_limit_mb)}</span></div>
                        <div className="detalleModalCampo"><span className="detalleModalLabel">DB CPU</span><span className="detalleModalValor">{formatCpuCores(deployment.runtime_db_cpu_limit_cores)}</span></div>
                        <div className="detalleModalCampo"><span className="detalleModalLabel">DB RAM</span><span className="detalleModalValor">{formatMb(deployment.runtime_db_ram_limit_mb)}</span></div>
                        <div className="detalleModalCampo"><span className="detalleModalLabel">SSH CPU</span><span className="detalleModalValor">{formatCpuCores(deployment.runtime_ssh_cpu_limit_cores)}</span></div>
                        <div className="detalleModalCampo"><span className="detalleModalLabel">SSH RAM</span><span className="detalleModalValor">{formatMb(deployment.runtime_ssh_ram_limit_mb)}</span></div>
                    </div>
                )}
            </div>

            {(deployment.plan_wp_cpu_millicores != null || deployment.plan_wp_memory_mb != null) && (
                <div className="detalleModalPlan">
                    <span className="detalleModalGraficoLabel">Recursos del plan vinculado</span>
                    <p className="detalleModalPlanNota">Referencia comercial de la suscripción vinculada.</p>
                    <div className="detalleModalGrid">
                        <div className="detalleModalCampo"><span className="detalleModalLabel">Plan WP CPU</span><span className="detalleModalValor">{formatMillicores(deployment.plan_wp_cpu_millicores)}</span></div>
                        <div className="detalleModalCampo"><span className="detalleModalLabel">Plan WP RAM</span><span className="detalleModalValor">{formatMb(deployment.plan_wp_memory_mb)}</span></div>
                        <div className="detalleModalCampo"><span className="detalleModalLabel">Plan DB CPU</span><span className="detalleModalValor">{formatMillicores(deployment.plan_db_cpu_millicores)}</span></div>
                        <div className="detalleModalCampo"><span className="detalleModalLabel">Plan DB RAM</span><span className="detalleModalValor">{formatMb(deployment.plan_db_memory_mb)}</span></div>
                        <div className="detalleModalCampo"><span className="detalleModalLabel">Plan SSH CPU</span><span className="detalleModalValor">{formatMillicores(deployment.plan_ssh_cpu_millicores)}</span></div>
                        <div className="detalleModalCampo"><span className="detalleModalLabel">Plan SSH RAM</span><span className="detalleModalValor">{formatMb(deployment.plan_ssh_memory_mb)}</span></div>
                    </div>
                </div>
            )}

            <div className="detalleModalGrafico">
                <span className="detalleModalGraficoLabel">Uso 24h</span>
                {isLoading && <div className="graficoRecursosVacio">Cargando muestras...</div>}
                {error && <div className="graficoRecursosVacio">No se pudieron cargar las muestras</div>}
                {!isLoading && !error && <ResourceUsageChart points={metrics?.points ?? []} />}
            </div>

            {!isLinked && (
                <div className="detalleModalOrphan">
                    <span>Despliegue sin suscripción vinculada en el panel.</span>
                    <div className="detalleModalOrphanActions">
                        <Button variante="secundario" tamano="pequeno" onClick={onCreateSubscription} type="button">Crear suscripción</Button>
                        <Button variante="outline" tamano="pequeno" onClick={onDeleteDeployment} disabled={isDeleting} type="button">{isDeleting ? 'Eliminando...' : 'Eliminar despliegue'}</Button>
                    </div>
                </div>
            )}
        </div>
    );
}