/* wandori.us — API Client
 * Fetch wrapper con auth automática, JSON parsing y manejo de errores.
 * [297A-8] Auth vía cookie HttpOnly + CSRF token para mutaciones. */

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
  /** AbortSignal opcional para cancelar el fetch con el lifecycle de la app. */
  signal?: AbortSignal;
}

/* [297A-8] Leer cookie CSRF del browser */
function getCsrfToken(): string | null {
  const match = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]*)/);
  return match ? match[1] : null;
}

/* Request genérico con auth automática (cookies) y CSRF */
async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = 'GET', body, headers = {}, formData, signal } = options;

  const requestHeaders: Record<string, string> = { ...headers };

  /* [297A-8] Las cookies se envían automáticamente con credentials: 'include'.
   * Para mutaciones, añadir token CSRF desde la cookie. */
  const isMutation = method !== 'GET';
  if (isMutation) {
    const csrf = getCsrfToken();
    if (csrf) {
      requestHeaders['X-CSRF-Token'] = csrf;
    }
  }

  /* Solo setear Content-Type para JSON, no para FormData */
  if (!formData) {
    requestHeaders['Content-Type'] = 'application/json';
  }

  const response = await fetch(`${BASE_URL}${path}`, {
    method,
    headers: requestHeaders,
    credentials: 'include',
    body: formData ?? (body ? JSON.stringify(body) : undefined),
    signal,
  });

  if (!response.ok) {
    let errorBody: unknown;
    try {
      errorBody = await response.json();
    } catch {
      errorBody = await response.text();
    }

    /* [297A-8] Si 401, limpiar estado de auth */
    if (response.status === 401) {
      authStore.set({ isAuthenticated: false, userId: null, capability: 'public' });
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
  get: <T>(path: string, options?: { signal?: AbortSignal }) => request<T>(path, options),

  post: <T>(path: string, body: unknown, options?: { headers?: Record<string, string>; signal?: AbortSignal }) =>
    request<T>(path, { method: 'POST', body, ...options }),

  put: <T>(path: string, body: unknown) =>
    request<T>(path, { method: 'PUT', body }),

  patch: <T>(path: string, body: unknown) =>
    request<T>(path, { method: 'PATCH', body }),

  delete: <T>(path: string) =>
    request<T>(path, { method: 'DELETE' }),

  upload: <T>(path: string, formData: FormData) =>
    request<T>(path, { method: 'POST', formData }),
};
