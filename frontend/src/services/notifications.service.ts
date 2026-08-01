import { api } from '../api/client';

export interface ApiNotification {
  id: string;
  kind: string;
  title: string;
  body: string;
  release_version: number | null;
  status: string;
  created_by: string | null;
  published_at: string | null;
  created_at: string;
  read: boolean;
}

export interface NotificationsResponse {
  items: ApiNotification[];
  unread_count: number;
}

export const NotificationsService = {
  listPublic(): Promise<NotificationsResponse> {
    return api.get<NotificationsResponse>('/api/notifications');
  },

  listMine(): Promise<NotificationsResponse> {
    return api.get<NotificationsResponse>('/api/me/notifications');
  },

  markRead(id: string): Promise<void> {
    return api.post<void>(`/api/notifications/${encodeURIComponent(id)}/read`, {});
  },

  listAdmin(): Promise<NotificationsResponse> {
    return api.get<NotificationsResponse>('/api/admin/notifications');
  },

  createAdmin(data: { kind: string; title: string; body: string; status: 'draft' | 'published' }): Promise<ApiNotification> {
    return api.post<ApiNotification>('/api/admin/notifications', data);
  },

  updateStatus(id: string, status: 'draft' | 'published' | 'archived'): Promise<ApiNotification> {
    return api.patch<ApiNotification>(`/api/admin/notifications/${encodeURIComponent(id)}/status`, { status });
  },
};
