/* [074A-6] API client para upload de imágenes CMS.
 * Envía multipart/form-data al endpoint admin-only.
 * [277A-18] list_uploads para galería de imágenes SEO. */
import instance from './axios-instance';

export interface UploadResponse {
    url: string;
    file_name: string;
}

export interface UploadEntry {
    url: string;
    file_name: string;
    size_bytes: number;
}

export async function apiUploadImage(file: File): Promise<UploadResponse> {
    const formData = new FormData();
    formData.append('file', file);
    const { data } = await instance.post<UploadResponse>('/api/admin/uploads', formData, {
        headers: { 'Content-Type': 'multipart/form-data' },
    });
    return data;
}

export async function apiListUploads(): Promise<UploadEntry[]> {
    const { data } = await instance.get<UploadEntry[]>('/api/admin/uploads');
    return data;
}
