import { unwrapGeneratedResponse } from '../api/client';
import {
  createAdmin,
  listAdmin,
  listMine,
  listPublic,
  markRead,
  updateStatusAdmin,
} from '../api/generated/notifications/notifications';

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
    return listPublic().then((response) => unwrapGeneratedResponse<NotificationsResponse>(response, [200]));
  },

  listMine(): Promise<NotificationsResponse> {
    return listMine().then((response) => unwrapGeneratedResponse<NotificationsResponse>(response, [200]));
  },

  async markRead(id: string): Promise<void> {
    const response = await markRead(encodeURIComponent(id));
    unwrapGeneratedResponse<void>(response, [204]);
  },

  async listAdmin(): Promise<NotificationsResponse> {
    const response = await listAdmin();
    return unwrapGeneratedResponse<NotificationsResponse>(response, [200]);
  },

  async createAdmin(data: { kind: string; title: string; body: string; status: 'draft' | 'published' }): Promise<ApiNotification> {
    const response = await createAdmin(data);
    return unwrapGeneratedResponse<ApiNotification>(response, [200]);
  },

  async updateStatus(id: string, status: 'draft' | 'published' | 'archived'): Promise<ApiNotification> {
    const response = await updateStatusAdmin(encodeURIComponent(id), { status });
    return unwrapGeneratedResponse<ApiNotification>(response, [200]);
  },
};
