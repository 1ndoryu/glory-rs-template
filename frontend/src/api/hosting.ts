/* sentinel-disable-file limite-lineas: módulo API legacy de hosting.
 * Reúne contratos y endpoints compartidos de suscripciones, stats y catálogo para no
 * romper imports existentes mientras el dominio se separa por archivos menores.
 */
/* [054A-2] API client de hosting: suscripciones y eventos.
 * Endpoints bajo /api/hosting/. Requiere JWT. */

import axiosInstance from './axios-instance';

/*    TIPOS */

export interface HostingSubscription {
    id: string;
    user_id: string | null;
    client_name: string;
    client_email: string;
    plan: string;
    domain: string | null;
    domain_verification_status: 'none' | 'pending_verification' | 'verified' | 'active';
    domain_verification_token: string | null;
    domain_verified_at: string | null;
    runtime_kind: string | null;
    deployment_id: string | null;
    coolify_site_name: string | null;
    status: string;
    monthly_price_cents: number;
    storage_limit_mb: number;
    /* [104A-42] Datos reales del servidor Coolify */
    server_uuid: string | null;
    server_ip: string | null;
    /* [104A-18] Credenciales SFTP generadas al provisionar */
    sftp_user: string | null;
    sftp_password: string | null;
    sftp_port: number | null;
    created_at: string;
    updated_at: string;
}

export interface HostingEvent {
    id: string;
    subscription_id: string;
    event_type: string;
    details: Record<string, unknown> | null;
    created_at: string;
}

export interface CreateHostingRequest {
    client_name: string;
    client_email: string;
    plan: string;
    domain?: string;
    /* [304A-3] Vincular a un despliegue Coolify existente (admin) */
    coolify_site_name?: string;
}

/*    REST API */

export async function apiListHostingSubscriptions(): Promise<HostingSubscription[]> {
    const {data} = await axiosInstance.get<HostingSubscription[]>('/api/hosting/subscriptions');
    return data;
}

export async function apiGetHostingSubscription(id: string): Promise<HostingSubscription> {
    const {data} = await axiosInstance.get<HostingSubscription>(`/api/hosting/subscriptions/${id}`);
    return data;
}

export async function apiCreateHostingSubscription(
    req: CreateHostingRequest,
): Promise<HostingSubscription> {
    const {data} = await axiosInstance.post<HostingSubscription>('/api/hosting/subscriptions', req);
    return data;
}

export async function apiUpdateHostingStatus(
    id: string,
    status: string,
    reason?: string,
): Promise<void> {
    await axiosInstance.patch(`/api/hosting/subscriptions/${id}/status`, {status, reason});
}

/* [074A-65] Actualizar suscripción (plan, dominio) — admin o dueño de la suscripción
 * [084A-4] Ya no es solo admin — clientes pueden editar sus propias suscripciones */
export interface UpdateHostingRequest {
    plan: string;
    domain?: string;
}

export async function apiUpdateHostingSubscription(
    id: string,
    req: UpdateHostingRequest,
): Promise<HostingSubscription> {
    const {data} = await axiosInstance.put<HostingSubscription>(`/api/hosting/subscriptions/${id}`, req);
    return data;
}

/* [074A-65] Eliminar suscripción — solo admin */
export async function apiDeleteHostingSubscription(id: string): Promise<void> {
    await axiosInstance.delete(`/api/hosting/subscriptions/${id}`);
}

/* [084A-4] Solicitar cancelación — cliente o admin.
 * Cambia status a 'cancelled' y registra evento de auditoría. */
export async function apiRequestCancelHosting(id: string): Promise<void> {
    await axiosInstance.post(`/api/hosting/subscriptions/${id}/cancel`);
}

/* [154A-11] Provisionar hosting: crea servicio Nginx real en Coolify (admin only). */
export async function apiProvisionHosting(id: string): Promise<HostingSubscription> {
    const {data} = await axiosInstance.post<HostingSubscription>(
        `/api/hosting/subscriptions/${id}/provision`,
    );
    return data;
}

/* [154A-16] Verificación DNS: comprueba si el dominio apunta al servidor correcto. */
export interface DnsCheckResult {
    configured: boolean;
    domain?: string;
    resolved?: boolean;
    resolved_ips?: string[];
    expected_ip?: string;
    points_to_server?: boolean;
    ssl_provider?: string;
    error?: string;
    message?: string;
}

export async function apiDnsCheck(id: string): Promise<DnsCheckResult> {
    const {data} = await axiosInstance.get<DnsCheckResult>(
        `/api/hosting/subscriptions/${id}/dns-check`,
    );
    return data;
}

export interface VerifyDomainResult {
    verified: boolean;
    applied: boolean;
    status: HostingSubscription['domain_verification_status'];
    domain: string;
    txt_host: string;
    txt_value: string | null;
    txt_records: string[];
    message: string;
}

export async function apiVerifyHostingDomain(id: string): Promise<VerifyDomainResult> {
    const {data} = await axiosInstance.post<VerifyDomainResult>(
        `/api/hosting/subscriptions/${id}/verify-domain`,
    );
    return data;
}

/* [084A-24] Iniciar checkout Stripe para suscripción de hosting.
 * Retorna la URL de Stripe Checkout a la que redirigir al cliente. */
export async function apiCreateHostingCheckout(id: string): Promise<string> {
    const { data } = await axiosInstance.post<{ checkout_url: string }>(
        `/api/hosting/subscriptions/${id}/checkout`,
    );
    return data.checkout_url;
}

/* [304A-3] Admin asigna una suscripción de hosting a un usuario registrado por email.
 * Vincula hostings creados manualmente a cuentas de clientes existentes. */
export async function apiAssignHostingToUser(
    id: string,
    userEmail: string,
): Promise<HostingSubscription> {
    const { data } = await axiosInstance.patch<HostingSubscription>(
        `/api/hosting/subscriptions/${id}/assign`,
        { user_email: userEmail },
    );
    return data;
}

export async function apiListHostingEvents(id: string): Promise<HostingEvent[]> {
    const {data} = await axiosInstance.get<HostingEvent[]>(`/api/hosting/subscriptions/${id}/events`);
    return data;
}

/* [094A-3] Self-service: cliente selecciona plan y recibe URL de Stripe Checkout */
export interface SelfSubscribeRequest {
    plan: string;
    domain?: string;
    billing_cycle_months?: 1 | 6 | 12;
    wp_admin_username?: string;
    wp_admin_password?: string;
    wp_language?: string;
    sftp_user?: string;
    sftp_password?: string;
}

export interface SelfSubscribeResponse {
    subscription: HostingSubscription;
    checkout_url: string;
}

export async function apiSelfSubscribe(req: SelfSubscribeRequest): Promise<SelfSubscribeResponse> {
    const {data} = await axiosInstance.post<SelfSubscribeResponse>('/api/hosting/subscribe', req);
    return data;
}

export interface PublicHostingPlan {
    plan_name: string;
    label: string;
    description: string;
    monthly_price_cents: number;
    wp_cpu_millicores: number;
    wp_memory_mb: number;
    db_cpu_millicores: number;
    db_memory_mb: number;
    ssh_cpu_millicores: number;
    ssh_memory_mb: number;
    storage_limit_mb: number;
    bandwidth_limit_gb: number;
    features: string[];
    recommended: boolean;
}

export async function apiListPublicHostingPlans(): Promise<PublicHostingPlan[]> {
    const {data} = await axiosInstance.get<PublicHostingPlan[]>('/api/hosting/public-plans');
    return data;
}

/*    CONSTANTES */

export const HOSTING_PLAN_LABELS: Record<string, string> = {
    basico: 'Básico',
    pro: 'Profesional',
    ecommerce: 'Avanzado',
    'normal-basico': 'Hosting Básico',
    'normal-pro': 'Hosting Profesional',
    'normal-ecommerce': 'Hosting Avanzado',
    custom: 'Custom',
};

export const HOSTING_STATUS_LABELS: Record<string, string> = {
    pending: 'Pendiente',
    provisioning: 'Provisionando',
    active: 'Activo',
    suspended: 'Suspendido',
    cancelled: 'Cancelado',
};

export const HOSTING_STATUS_COLORS: Record<string, string> = {
    pending: '#f59e0b',
    provisioning: '#3b82f6',
    active: '#22c55e',
    suspended: '#ef4444',
    cancelled: '#6b7280',
};

export const HOSTING_STATUS_CLASS: Record<string, string> = {
    pending: 'hostingStatus--pending',
    provisioning: 'hostingStatus--provisioning',
    active: 'hostingStatus--active',
    suspended: 'hostingStatus--suspended',
    cancelled: 'hostingStatus--cancelled',
};

/* [094A-3] Info de planes para el selector de contratación self-service
 * [114A-5] Especialización WordPress: features WP-CLI, WordPress pre-instalado */
export interface HostingPlanInfo {
    id: string;
    label: string;
    priceCents: number;
    storageMb: number;
    features: string[];
    description?: string;
    recommended?: boolean;
}

export function toHostingPlanInfo(plan: PublicHostingPlan): HostingPlanInfo {
    return {
        id: plan.plan_name,
        label: plan.label,
        priceCents: plan.monthly_price_cents,
        storageMb: plan.storage_limit_mb,
        features: plan.features,
        description: plan.description,
        recommended: plan.recommended,
    };
}

export const HOSTING_PLANS_FALLBACK: HostingPlanInfo[] = [
    {
        id: 'basico',
        label: 'Básico',
        priceCents: 248,
        storageMb: 5120,
        description: 'WordPress administrado para sitios livianos y landings con costo controlado.',
        features: ['WordPress pre-instalado', '5 GB almacenamiento', 'Tráfico ilimitado', 'Free temporary domain', 'Certificado SSL incluido', 'Free CDN', 'WP-CLI + SSH', 'Backups semanales'],
    },
    {
        id: 'pro',
        label: 'Profesional',
        priceCents: 413,
        storageMb: 20480,
        description: 'WordPress para negocios que necesitan más recursos y staging listo.',
        features: ['WordPress pre-instalado', '20 GB almacenamiento', 'Tráfico ilimitado', 'Free temporary domain', 'Certificado SSL incluido', 'Free CDN', 'WP-CLI + SSH', 'Backups diarios'],
        recommended: true,
    },
    {
        id: 'ecommerce',
        label: 'Avanzado',
        priceCents: 619,
        storageMb: 51200,
        description: 'WordPress administrado de mayor capacidad para sitios con más contenido, tráfico y caché avanzada.',
        features: ['WordPress pre-instalado', '50 GB almacenamiento', 'Tráfico ilimitado', 'Free temporary domain', 'Certificado SSL incluido', 'Free CDN', 'WP-CLI + SSH', 'Backups diarios + semanales', 'Caché avanzada'],
    },
    {
        id: 'normal-basico',
        label: 'Hosting Básico',
        priceCents: 323,
        storageMb: 5120,
        description: 'Hosting administrado con Nginx, SSL y SFTP para landings, sitios corporativos y proyectos sin WordPress.',
        features: ['Nginx administrado', '5 GB almacenamiento', 'Tráfico ilimitado', 'Free temporary domain', 'Certificado SSL incluido', 'SFTP seguro', 'Backups semanales'],
    },
    {
        id: 'normal-pro',
        label: 'Hosting Profesional',
        priceCents: 537,
        storageMb: 20480,
        description: 'Hosting administrado para sitios con más tráfico, frontends personalizados y despliegues con mayor exigencia operativa.',
        features: ['Nginx administrado', '20 GB almacenamiento', 'Tráfico ilimitado', 'Free temporary domain', 'Certificado SSL incluido', 'SFTP seguro', 'Backups diarios'],
        recommended: true,
    },
    {
        id: 'normal-ecommerce',
        label: 'Hosting Avanzado',
        priceCents: 805,
        storageMb: 51200,
        description: 'Hosting administrado de mayor capacidad para catálogos amplios, assets pesados y operaciones con más demanda.',
        features: ['Nginx administrado', '50 GB almacenamiento', 'Tráfico ilimitado', 'Free temporary domain', 'Certificado SSL incluido', 'SFTP seguro', 'Backups diarios + semanales', 'Recursos ampliados'],
    },
];

/* [165A-10] La URL bootstrap del hosting debe derivarse del nombre del servicio
 * persistido en la suscripción, no del UUID interno de Coolify. */
export function getProvisionedHostingSiteUrl(
    sub: Pick<HostingSubscription, 'plan' | 'coolify_site_name' | 'server_ip' | 'domain' | 'domain_verification_status'>,
): string | null {
    if (sub.domain && sub.domain_verification_status === 'active') {
        return `https://${sub.domain}`;
    }

    if (!sub.coolify_site_name || !sub.server_ip || !sub.coolify_site_name.startsWith('hosting-')) {
        return null;
    }

    const servicePrefix = sub.plan.startsWith('normal-') ? 'site' : 'wordpress';
    return `http://${servicePrefix}-${sub.coolify_site_name}.${sub.server_ip}.sslip.io`;
}

export function getProvisionedHostingAdminUrl(
    sub: Pick<HostingSubscription, 'plan' | 'coolify_site_name' | 'server_ip' | 'domain' | 'domain_verification_status'>,
): string | null {
    if (sub.plan.startsWith('normal-')) {
        return null;
    }

    const siteUrl = getProvisionedHostingSiteUrl(sub);
    return siteUrl ? `${siteUrl}/wp-admin` : null;
}

/* [154A-9] Control de servicio: restart / stop / start */
export async function apiRestartHosting(id: string): Promise<void> {
    await axiosInstance.post(`/api/hosting/subscriptions/${id}/restart`);
}

export async function apiStopHosting(id: string): Promise<void> {
    await axiosInstance.post(`/api/hosting/subscriptions/${id}/stop`);
}

export async function apiStartHosting(id: string): Promise<void> {
    await axiosInstance.post(`/api/hosting/subscriptions/${id}/start`);
}

/* [154A-14] Admin test subscribe: crea hosting sin Stripe */
export async function apiAdminTestSubscribe(req: SelfSubscribeRequest): Promise<{subscription: HostingSubscription; message: string}> {
    const {data} = await axiosInstance.post<{subscription: HostingSubscription; message: string}>('/api/hosting/admin-test-subscribe', req);
    return data;
}

/* [084A-24] VPS stats — proxy a Contabo API (admin only) */

/* [094A-8] Stats reales de una suscripción de hosting */
/* [114A-15+] Añadidos campos de CPU/RAM y stats por contenedor */
export interface ContainerStatsData {
    name: string;
    cpu_percent: number;
    mem_used_mb: number;
    mem_limit_mb: number;
    net_input_mb: number;
    net_output_mb: number;
}

export interface HostingStatsData {
    storage_limit_mb: number;
    storage_used_mb: number | null;
    bandwidth_limit_gb: number;
    bandwidth_used_gb: number | null;
    bandwidth_remaining_gb: number | null;
    bandwidth_reset_at: string;
    uptime_percent: number;
    active_since: string | null;
    total_events: number;
    last_event_at: string | null;
    monitoring_available: boolean;
    cpu_percent: number | null;
    ram_used_mb: number | null;
    ram_limit_mb: number | null;
    containers: ContainerStatsData[] | null;
}

export async function apiGetHostingStats(id: string): Promise<HostingStatsData> {
    const {data} = await axiosInstance.get<HostingStatsData>(`/api/hosting/subscriptions/${id}/stats`);
    return data;
}

/* [215A-14] Enriquecido con recursos reales y nombre del cliente dueño.
 * [259A-5] Segregado en facetas (ISP, regla large-interface-isp): la forma
 * plana se conserva via extends para no romper el contrato JSON del backend. */
export interface DeploymentIdentidad {
    uuid: string;
    runtime_kind: string;
    deployment_id: string;
    name: string;
    status: string;
    fqdn: string | null;
}

export interface DeploymentVinculo {
    server_uuid: string | null;
    server_name: string | null;
    server_label: string;
    project_uuid: string | null;
    environment_name: string | null;
    linked_subscription_id: string | null;
    linked_subscription_domain: string | null;
    linked_subscription_status: string | null;
    linked_subscription_plan: string | null;
    linked_subscription_client: string | null;
}

export interface DeploymentMuestra {
    runtime_sampled_at: string | null;
    cpu_percent: number | null;
    ram_used_mb: number | null;
    ram_limit_mb: number | null;
    storage_used_mb: number | null;
    storage_limit_mb: number | null;
}

export interface DeploymentLimitesRuntime {
    runtime_site_cpu_limit_cores: number | null;
    runtime_site_ram_limit_mb: number | null;
    runtime_db_cpu_limit_cores: number | null;
    runtime_db_ram_limit_mb: number | null;
    runtime_ssh_cpu_limit_cores: number | null;
    runtime_ssh_ram_limit_mb: number | null;
}

export interface DeploymentLimitesPlan {
    plan_wp_cpu_millicores: number | null;
    plan_db_cpu_millicores: number | null;
    plan_ssh_cpu_millicores: number | null;
    plan_wp_memory_mb: number | null;
    plan_db_memory_mb: number | null;
    plan_ssh_memory_mb: number | null;
}

export interface CoolifyDeployment extends DeploymentIdentidad, DeploymentVinculo, DeploymentMuestra, DeploymentLimitesRuntime, DeploymentLimitesPlan {
}

/* [164A-19] Despliegues reales de Coolify en todos los servidores configurados. */
export async function apiListDeployments(): Promise<CoolifyDeployment[]> {
    const {data} = await axiosInstance.get<CoolifyDeployment[]>('/api/hosting/deployments');
    return data;
}

/* [165A-4] Elimina un despliegue huérfano real desde Coolify para limpiar el panel. */
export async function apiDeleteDeployment(uuid: string): Promise<void> {
    await axiosInstance.delete(`/api/hosting/deployments/${encodeURIComponent(uuid)}`);
}

export interface ResourceMetricPoint {
    sampled_at: string;
    cpu_percent: number | null;
    ram_used_mb: number | null;
    ram_limit_mb: number | null;
    disk_used_mb: number | null;
    disk_limit_mb: number | null;
    site_cpu_limit_cores: number | null;
    site_ram_limit_mb: number | null;
    db_cpu_limit_cores: number | null;
    db_ram_limit_mb: number | null;
    ssh_cpu_limit_cores: number | null;
    ssh_ram_limit_mb: number | null;
}

export interface DeploymentMetricsResponse {
    deployment_uuid: string;
    range: string;
    points: ResourceMetricPoint[];
}

export async function apiGetDeploymentMetrics(uuid: string, range = '24h'): Promise<DeploymentMetricsResponse> {
    const {data} = await axiosInstance.get<DeploymentMetricsResponse>(`/api/hosting/deployments/${encodeURIComponent(uuid)}/metrics`, {params: {range}});
    return data;
}

export interface VpsSummary {
    inventory_id: string;
    instance_id: number | null;
    name: string;
    label: string;
    ip: string;
    status: string;
    region: string;
    cpu_cores: number | null;
    ram_mb: number | null;
    disk_mb: number | null;
    provider: string;
    source: string;
    is_configured: boolean;
    coolify_server_uuid: string | null;
    ssh_available: boolean;
    cpu_avg_1h: number | null;
    ram_used_mb: number | null;
    ram_limit_mb: number | null;
    disk_used_mb: number | null;
    disk_limit_mb: number | null;
    sampled_at: string | null;
}

export async function apiListVps(): Promise<VpsSummary[]> {
    const { data } = await axiosInstance.get<{ data: VpsSummary[] }>('/api/hosting/vps');
    return data.data;
}

export async function apiGetVps(instanceId: number): Promise<VpsSummary> {
    const { data } = await axiosInstance.get<{ data: VpsSummary }>(`/api/hosting/vps/${instanceId}`);
    return data.data;
}

export interface PublicVpsPlan {
    tier_name: string;
    display_name: string;
    description: string;
    monthly_price_cents: number;
    setup_fee_cents: number;
    cpu_cores: number;
    ram_mb: number;
    disk_mb: number;
    storage_type: string;
    storage_options: string[];
    port_speed_mbps: number;
    bandwidth_label: string;
    snapshot_count: number;
    region: string;
    /* [205A-1] Costos extra por región y opción de storage (centavos/mes) */
    region_extra_cents: Record<string, number>;
    storage_extra_cents: Record<string, number>;
    features: string[];
    approval_required: boolean;
    recommended: boolean;
}

export interface VpsSubscription {
    id: string;
    user_id: string | null;
    client_name: string;
    client_email: string;
    tier_name: string;
    requested_hostname: string | null;
    status: string;
    stripe_subscription_id: string | null;
    monthly_price_cents: number;
    contabo_instance_id: number | null;
    provisioning_ip: string | null;
    access_username: string | null;
    approved_by: string | null;
    approved_at: string | null;
    provisioned_at: string | null;
    rejected_reason: string | null;
    client_notes: string | null;
    created_at: string;
    updated_at: string;
}

export interface SelfSubscribeVpsRequest {
    tier: string;
    hostname?: string;
    storage_preference?: string;
    region_preference?: string;
    os_preference?: string;
    server_password?: string;
}

export interface SelfSubscribeVpsResponse {
    subscription: VpsSubscription;
    checkout_url: string;
}

export async function apiListPublicVpsPlans(): Promise<PublicVpsPlan[]> {
    const {data} = await axiosInstance.get<PublicVpsPlan[]>('/api/vps/public-plans');
    return data;
}

export async function apiListVpsSubscriptions(): Promise<VpsSubscription[]> {
    const {data} = await axiosInstance.get<VpsSubscription[]>('/api/vps/subscriptions');
    return data;
}

export async function apiGetVpsSubscription(id: string): Promise<VpsSubscription> {
    const {data} = await axiosInstance.get<VpsSubscription>(`/api/vps/subscriptions/${id}`);
    return data;
}

export async function apiSelfSubscribeVps(
    req: SelfSubscribeVpsRequest,
): Promise<SelfSubscribeVpsResponse> {
    const {data} = await axiosInstance.post<SelfSubscribeVpsResponse>('/api/vps/subscribe', req);
    return data;
}

export async function apiApproveVpsSubscription(id: string): Promise<VpsSubscription> {
    const {data} = await axiosInstance.post<VpsSubscription>(`/api/admin/vps/subscriptions/${id}/approve`);
    return data;
}

export async function apiRejectVpsSubscription(id: string, reason: string): Promise<void> {
    await axiosInstance.post(`/api/admin/vps/subscriptions/${id}/reject`, {reason});
}

export const VPS_STATUS_LABELS: Record<string, string> = {
    pending_payment: 'Pendiente de pago',
    pending_approval: 'Pendiente de aprobación',
    provisioning: 'Provisionando',
    active: 'Activo',
    suspended: 'Suspendido',
    rejected: 'Rechazado',
    cancelled: 'Cancelado',
};

/* [154A-1] Contabo Domain Management API */

export interface ContaboDomain {
    sld: string | null;
    tld: string | null;
    status: string | null;
    handles: DomainHandles | null;
    nameservers: DomainNameserver[] | null;
    createdDate: string | null;
    paidUntil: string | null;
}

export interface DomainHandles {
    owner: string | null;
    admin: string | null;
    tech: string | null;
    zone: string | null;
}

export interface DomainNameserver {
    hostname: string | null;
    ipv4: string | null;
    ipv6: string | null;
}

export interface DomainAvailability {
    domain: string;
    available: boolean;
    tld: string;
    base_cost_cents: number;
    price_cents: number;
}

export interface DomainOrder {
    id: string;
    user_id: string;
    domain: string;
    tld: string;
    status: string;
    base_cost_cents: number;
    price_cents: number;
    stripe_session_id: string | null;
    created_at: string;
    updated_at: string;
}

export interface DnsZone {
    zoneName: string | null;
}

export interface DnsRecord {
    recordId: number | null;
    name: string | null;
    type: string | null;
    ttl: number | null;
    prio: number | null;
    data: string | null;
    port: number | null;
    weight: number | null;
    flag: number | null;
    tag: string | null;
}

export interface ContaboHandle {
    handleId: string | null;
    handleType: string | null;
    firstName: string | null;
    lastName: string | null;
    organization: string | null;
    email: string | null;
    gender: string | null;
    address: HandleAddress | null;
    phone: HandlePhone | null;
}

export interface HandleAddress {
    street: string | null;
    streetNumber: string | null;
    city: string | null;
    country: string | null;
    zipCode: string | null;
}

export interface HandlePhone {
    prefix: string | null;
    number: string | null;
}

/* Domains */

export async function apiCheckDomainAvailability(domain: string): Promise<DomainAvailability> {
    const { data } = await axiosInstance.get<DomainAvailability>(
        `/api/hosting/domains/check/${encodeURIComponent(domain)}`,
    );
    return data;
}

export async function apiListDomainOrders(): Promise<DomainOrder[]> {
    const { data } = await axiosInstance.get<DomainOrder[]>('/api/hosting/domain-orders');
    return data;
}

export async function apiCreateDomainCheckout(domain: string): Promise<{order: DomainOrder; checkout_url: string}> {
    const { data } = await axiosInstance.post<{order: DomainOrder; checkout_url: string}>(
        '/api/hosting/domains/checkout',
        { domain },
    );
    return data;
}

export async function apiListDomains(): Promise<ContaboDomain[]> {
    const { data } = await axiosInstance.get<ContaboDomain[]>('/api/hosting/domains');
    return data;
}

export async function apiGetDomain(domain: string): Promise<ContaboDomain> {
    const { data } = await axiosInstance.get<ContaboDomain>(
        `/api/hosting/domains/${encodeURIComponent(domain)}`,
    );
    return data;
}

export interface OrderDomainRequest {
    domain: string;
    auth_code?: string;
    handles: DomainHandles;
    nameservers: DomainNameserver[];
}

export async function apiOrderDomain(req: OrderDomainRequest): Promise<ContaboDomain> {
    const { data } = await axiosInstance.post<ContaboDomain>('/api/hosting/domains', req);
    return data;
}

export async function apiUpdateDomain(
    domain: string,
    body: { nameservers?: DomainNameserver[]; handles?: DomainHandles },
): Promise<ContaboDomain> {
    const { data } = await axiosInstance.patch<ContaboDomain>(
        `/api/hosting/domains/${encodeURIComponent(domain)}`,
        body,
    );
    return data;
}

export async function apiCancelDomain(domain: string, reason?: string): Promise<void> {
    await axiosInstance.post(`/api/hosting/domains/${encodeURIComponent(domain)}/cancel`, { reason });
}

export async function apiGetDomainAuthCode(domain: string): Promise<{ domain: string; auth_code: string }> {
    const { data } = await axiosInstance.post<{ domain: string; auth_code: string }>(
        `/api/hosting/domains/${encodeURIComponent(domain)}/auth-code`,
    );
    return data;
}

/* Handles */

export async function apiListHandles(): Promise<ContaboHandle[]> {
    const { data } = await axiosInstance.get<ContaboHandle[]>('/api/hosting/handles');
    return data;
}

export async function apiCreateHandle(handle: Omit<ContaboHandle, 'handleId'>): Promise<ContaboHandle> {
    const { data } = await axiosInstance.post<ContaboHandle>('/api/hosting/handles', handle);
    return data;
}

/* DNS Zones */

export async function apiListDnsZones(): Promise<DnsZone[]> {
    const { data } = await axiosInstance.get<DnsZone[]>('/api/hosting/dns/zones');
    return data;
}

export async function apiCreateDnsZone(zoneName: string): Promise<DnsZone> {
    const { data } = await axiosInstance.post<DnsZone>('/api/hosting/dns/zones', { zone_name: zoneName });
    return data;
}

export async function apiDeleteDnsZone(zoneName: string): Promise<void> {
    await axiosInstance.delete(`/api/hosting/dns/zones/${encodeURIComponent(zoneName)}`);
}

/* DNS Records */

export async function apiListDnsRecords(zoneName: string): Promise<DnsRecord[]> {
    const { data } = await axiosInstance.get<DnsRecord[]>(
        `/api/hosting/dns/zones/${encodeURIComponent(zoneName)}/records`,
    );
    return data;
}

export interface CreateDnsRecordRequest {
    name?: string;
    type: string;
    ttl: number;
    prio: number;
    data: string;
    port?: number;
    weight?: number;
    flag?: number;
    tag?: string;
}

export async function apiCreateDnsRecord(zoneName: string, record: CreateDnsRecordRequest): Promise<DnsRecord> {
    const { data } = await axiosInstance.post<DnsRecord>(
        `/api/hosting/dns/zones/${encodeURIComponent(zoneName)}/records`,
        record,
    );
    return data;
}

export async function apiUpdateDnsRecord(
    zoneName: string,
    recordId: number,
    record: Omit<CreateDnsRecordRequest, 'name'>,
): Promise<DnsRecord> {
    const { data } = await axiosInstance.patch<DnsRecord>(
        `/api/hosting/dns/zones/${encodeURIComponent(zoneName)}/records/${recordId}`,
        record,
    );
    return data;
}

export async function apiDeleteDnsRecord(zoneName: string, recordId: number): Promise<void> {
    await axiosInstance.delete(
        `/api/hosting/dns/zones/${encodeURIComponent(zoneName)}/records/${recordId}`,
    );
}

/* ── Client DNS (por suscripción) ─────── */

export async function apiClientListDnsRecords(subId: string): Promise<DnsRecord[]> {
    const {data} = await axiosInstance.get<DnsRecord[]>(
        `/api/hosting/subscriptions/${subId}/dns`,
    );
    return data;
}

export async function apiClientCreateDnsRecord(
    subId: string,
    req: CreateDnsRecordRequest,
): Promise<DnsRecord> {
    const {data} = await axiosInstance.post<DnsRecord>(
        `/api/hosting/subscriptions/${subId}/dns`,
        req,
    );
    return data;
}

export async function apiClientUpdateDnsRecord(
    subId: string,
    recordId: number,
    req: {type: string; ttl: number; prio: number; data: string},
): Promise<DnsRecord> {
    const {data} = await axiosInstance.patch<DnsRecord>(
        `/api/hosting/subscriptions/${subId}/dns/${recordId}`,
        req,
    );
    return data;
}

export async function apiClientDeleteDnsRecord(subId: string, recordId: number): Promise<void> {
    await axiosInstance.delete(`/api/hosting/subscriptions/${subId}/dns/${recordId}`);
}

/* [265A-6] Tipos y funciones para gestión de backups de hosting */

export interface HostingBackupEntry {
    backup_id: string;
    tier: string;
    file_id: string;
    file_name: string;
    file_size_bytes: number | null;
    created_at: string | null;
}

export interface HostingBackupReport {
    backup_id: string;
    tier: string;
    status: string;
    notes: string[];
}

export interface HostingRestoreReport {
    backup_id: string;
    status: string;
    fqdn: string | null;
    access_user: string | null;
    access_password: string | null;
    notes: string[];
}

export interface HostingBackupListResponse {
    runtime_kind: string;
    deployment_id: string;
    data: HostingBackupEntry[];
}

export async function apiListBackups(id: string): Promise<HostingBackupListResponse> {
    const { data } = await axiosInstance.get<HostingBackupListResponse>(
        `/api/hosting/subscriptions/${id}/backups`,
    );
    return data;
}

export async function apiCreateBackup(
    id: string,
    tier: string = 'manual',
    label?: string,
): Promise<HostingBackupReport> {
    const { data } = await axiosInstance.post<{ data: HostingBackupReport }>(
        `/api/hosting/subscriptions/${id}/backups`,
        { tier, label },
    );
    return data.data;
}

export async function apiRestoreBackup(
    id: string,
    backupId: string,
    accessPassword?: string,
    skipSafetySnapshot?: boolean,
): Promise<HostingRestoreReport> {
    const { data } = await axiosInstance.post<{ data: HostingRestoreReport }>(
        `/api/hosting/subscriptions/${id}/restore`,
        { backup_id: backupId, access_password: accessPassword, skip_safety_snapshot: skipSafetySnapshot },
    );
    return data.data;
}

export async function apiDeleteBackup(id: string, fileName: string): Promise<void> {
    await axiosInstance.delete(`/api/hosting/subscriptions/${id}/backups`, {
        data: { file_name: fileName },
    });
}

/* [265A-11] Alias de correo electronico (Cloudflare Email Routing) */

export interface EmailAliasInfo {
    id: string;
    alias: string;
    domain: string;
    destination: string;
    full_email: string;
    status: string;
    created_at: string;
}

export interface EmailMailboxInfo {
    id: string;
    email: string;
    provider: string;
    status: string;
    storage_used_mb: number;
    created_at: string;
}

export interface HostingEmailInfo {
    aliases: EmailAliasInfo[];
    aliases_limit: number;
    aliases_used: number;
    mailboxes: EmailMailboxInfo[];
    mailboxes_limit: number;
    mailboxes_used: number;
}

export interface CreateEmailAliasRequest {
    alias: string;
    domain: string;
    destination: string;
}

export async function apiGetHostingEmailInfo(
    subscriptionId: string,
): Promise<HostingEmailInfo> {
    const {data} = await axiosInstance.get<HostingEmailInfo>(
        `/api/hosting/subscriptions/${subscriptionId}/email`,
    );
    return data;
}

export async function apiCreateEmailAlias(
    subscriptionId: string,
    req: CreateEmailAliasRequest,
): Promise<EmailAliasInfo> {
    const {data} = await axiosInstance.post<EmailAliasInfo>(
        `/api/hosting/subscriptions/${subscriptionId}/email/aliases`,
        req,
    );
    return data;
}

export async function apiDeleteEmailAlias(
    subscriptionId: string,
    aliasId: string,
): Promise<void> {
    await axiosInstance.delete(
        `/api/hosting/subscriptions/${subscriptionId}/email/aliases/${aliasId}`,
    );
}
