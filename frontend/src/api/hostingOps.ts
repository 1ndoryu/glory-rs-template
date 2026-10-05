/* [01AA-4-f3s] Operaciones sobre suscripciones de hosting (split de api/hosting.ts):
 * URLs derivadas, control de servicio, test-subscribe y stats reales. */

import axiosInstance from './axios-instance';
import type {HostingSubscription, SelfSubscribeRequest} from './hostingCore';

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
