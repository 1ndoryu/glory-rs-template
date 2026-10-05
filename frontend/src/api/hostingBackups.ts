/* [01AA-4-f3s] Backups de hosting (split de api/hosting.ts). */

import axiosInstance from './axios-instance';

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
