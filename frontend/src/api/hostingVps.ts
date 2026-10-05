/* [01AA-4-f3s] VPS: inventario, planes, suscripciones y aprobación (split de api/hosting.ts). */

import axiosInstance from './axios-instance';

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
