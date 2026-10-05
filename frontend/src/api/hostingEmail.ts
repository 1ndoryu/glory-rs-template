/* [01AA-4-f3s] Email de hosting: alias y buzones (split de api/hosting.ts). */

import axiosInstance from './axios-instance';

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
