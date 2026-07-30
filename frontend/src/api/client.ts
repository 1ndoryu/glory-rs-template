/* wandori.us — API Client
 * Fetch wrapper con auth automática, JSON parsing y manejo de errores.
 * Reemplaza axios por Fetch API nativa (más ligero). */

import { authStore } from '../store';

const BASE_URL = import.meta.env.VITE_API_URL || '';

export class ApiError extends Error {
  constructor(
    public status: number,
    public body: unknown,
    message: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

interface RequestOptions {
  method?: string;
  body?: unknown;
  headers?: Record<string, string>;
  formData?: FormData;
}

/* Request genérico con auth automática */
async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = 'GET', body, headers = {}, formData } = options;

  const requestHeaders: Record<string, string> = { ...headers };

  /* Agregar token JWT si existe */
  const { token } = authStore.get();
  if (token) {
    requestHeaders['Authorization'] = `Bearer ${token}`;
  }

  /* Solo setear Content-Type para JSON, no para FormData */
  if (!formData) {
    requestHeaders['Content-Type'] = 'application/json';
  }

  const response = await fetch(`${BASE_URL}${path}`, {
    method,
    headers: requestHeaders,
    body: formData ?? (body ? JSON.stringify(body) : undefined),
  });

  if (!response.ok) {
    let errorBody: unknown;
    try {
      errorBody = await response.json();
    } catch {
      errorBody = await response.text();
    }
    throw new ApiError(response.status, errorBody, `API Error: ${response.status}`);
  }

  /* 204 No Content o body vacío */
  const text = await response.text();
  if (!text || response.status === 204) {
    return undefined as T;
  }

  return JSON.parse(text) as T;
}

/* Métodos de conveniencia */
export const api = {
  get: <T>(path: string) => request<T>(path),

  post: <T>(path: string, body: unknown) =>
    request<T>(path, { method: 'POST', body }),

  put: <T>(path: string, body: unknown) =>
    request<T>(path, { method: 'PUT', body }),

  patch: <T>(path: string, body: unknown) =>
    request<T>(path, { method: 'PATCH', body }),

  delete: <T>(path: string) =>
    request<T>(path, { method: 'DELETE' }),

  upload: <T>(path: string, formData: FormData) =>
    request<T>(path, { method: 'POST', formData }),
};
