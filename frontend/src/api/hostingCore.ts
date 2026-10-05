/* [054A-2] API client de hosting: suscripciones y eventos.
 * Endpoints bajo /api/hosting/. Requiere JWT.
 * [01AA-4-f3s] Núcleo del split de api/hosting.ts: contratos + CRUD de
 * suscripciones, checkout, self-service, planes y constantes. */

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
