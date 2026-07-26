/* [SEO-A] API para el dashboard SEO admin. */
import instance from './axios-instance';

export interface SeoAuditSummary {
    total_pages: number;
    ok: number;
    warnings: number;
    errors: number;
    blog_published: number;
    services_active: number;
    projects_published: number;
}

export interface SeoPageEntry {
    path: string;
    label: string;
    page_type: string;
    entity_id: string | null;
    title: string | null;
    title_len: number;
    description: string | null;
    description_len: number;
    og_image_is_default: boolean;
    json_ld_type: string | null;
    status: string;
    issues: string[];
}

export interface SeoBlogEntry {
    id: string;
    title: string;
    slug: string;
    meta_title: string | null;
    meta_description: string | null;
    status: string;
    seo_status: string;
    issues: string[];
}

export interface GeoCheck {
    id: string;
    label: string;
    passed: boolean;
    detail: string | null;
}

export interface SeoAuditResponse {
    summary: SeoAuditSummary;
    pages: SeoPageEntry[];
    blog_posts: SeoBlogEntry[];
    geo_checks: GeoCheck[];
}

export async function apiGetSeoAudit(): Promise<SeoAuditResponse> {
    const {data} = await instance.get<SeoAuditResponse>('/api/admin/seo/audit');
    return data;
}
