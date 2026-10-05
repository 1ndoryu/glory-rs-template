/* [01AA-4-f3s] Dominios Contabo + DNS (split de api/hosting.ts):
 * disponibilidad, órdenes, CRUD de dominios, handles y zonas/registros DNS. */

import axiosInstance from './axios-instance';

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
