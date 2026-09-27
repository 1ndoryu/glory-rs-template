import axios from 'axios';
import type { AxiosRequestConfig } from 'axios';
/* [259A-5] Acceso window via platform/navigation (boundary sentinel). */
import {obtenerHost, obtenerOrigen} from '../platform/navigation';

/* [104A-1] En producción, API_BASE_URL vacío = URLs relativas al mismo origen (nakomi.studio/api/...).
 * En desarrollo, VITE_API_URL=http://localhost:3000 viene de .env.development. */
export const API_BASE_URL = import.meta.env.VITE_API_URL || '';

const instance = axios.create({
  baseURL: API_BASE_URL,
});

export function getApiHost(): string {
  if (!API_BASE_URL) return obtenerHost();
  return new URL(API_BASE_URL, obtenerOrigen()).host;
}

/* Interceptor: agrega el token JWT a cada request si existe */
instance.interceptors.request.use((config) => {
  const token = localStorage.getItem('token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

/**
 * Mutator para Orval — envuelve axios para que los hooks generados
 * funcionen con React Query y cancellation.
 */
export const customInstance = <T>(config: AxiosRequestConfig): Promise<T> => {
  const source = axios.CancelToken.source();
  const promise = instance({
    ...config,
    cancelToken: source.token,
  }).then(({ data }) => data);

  // @ts-expect-error -- propiedad cancel para React Query
  promise.cancel = () => source.cancel('Query cancelado');

  return promise;
};

export default instance;
