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

export interface GeneratedResponse<T> {
  data: T;
  status: number;
  headers: Headers;
}

/* [297A-8] Leer cookie CSRF del browser */
function getCsrfToken(): string | null {
  const match = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]*)/);
  return match ? match[1] : null;
}

function withSession(options: RequestInit): RequestInit {
  const method = (options.method || 'GET').toUpperCase();
  const headers = new Headers(options.headers);
  const isMutation = !['GET', 'HEAD', 'OPTIONS'].includes(method);
  if (isMutation) {
    const csrf = getCsrfToken();
    if (csrf) headers.set('X-CSRF-Token', csrf);
  }
  return { ...options, credentials: 'include', headers };
}

async function parseResponseBody(response: Response): Promise<unknown> {
  const text = await response.text();
  if (!text || response.status === 204) return undefined;
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return text;
  }
}

/* [018A-32] Orval uses this single transport so generated clients inherit the
 * same cookie, CSRF, base URL and response-envelope rules as the manual API. */
export async function generatedFetcher<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const response = await fetch(`${BASE_URL}${path}`, withSession(options));
  const data = await parseResponseBody(response);
  return { data, status: response.status, headers: response.headers } as T;
}

export function unwrapGeneratedResponse<T>(
  response: GeneratedResponse<unknown>,
  successStatuses: readonly number[],
): T {
  if (successStatuses.includes(response.status)) return response.data as T;
  if (response.status === 401) {
    authStore.set({ isAuthenticated: false, userId: null, capability: 'public' });
  }
  throw new ApiError(response.status, response.data, `API Error: ${response.status}`);
}

/* Request genérico con auth automática (cookies) y CSRF */
async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = 'GET', body, headers = {}, formData, signal } = options;

  const requestHeaders: Record<string, string> = { ...headers };

  /* Solo setear Content-Type para JSON, no para FormData */
  if (!formData) {
    requestHeaders['Content-Type'] = 'application/json';
  }

  const response = await fetch(`${BASE_URL}${path}`, withSession({
    method,
    headers: requestHeaders,
    body: formData ?? (body ? JSON.stringify(body) : undefined),
    signal,
  }));

  if (!response.ok) {
    const errorBody = await parseResponseBody(response);

    /* [297A-8] Si 401, limpiar estado de auth */
    if (response.status === 401) {
      authStore.set({ isAuthenticated: false, userId: null, capability: 'public' });
    }

    throw new ApiError(response.status, errorBody, `API Error: ${response.status}`);
  }

  return (await parseResponseBody(response)) as T;
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
