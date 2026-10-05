/* [01AA-4-f3s] Despliegues Coolify + métricas (split de api/hosting.ts). */

import axiosInstance from './axios-instance';

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
