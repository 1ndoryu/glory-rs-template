/* wandori.us — Store (State Management)
 * Patrón pub/sub simple. Sin dependencias.
 * Cada store es un objeto reactivo que notifica suscriptores al cambiar.
 * [Auditoría v4 §3.1] FontConfig dividido en sub-interfaces (ISP). */

type Listener<T> = (value: T) => void;
type Unsubscribe = () => void;

/** Origen del cambio de estado. Permite distinguir causas en suscriptores. */
export type StoreSource = 'user' | 'api' | 'overlay' | 'init' | 'sync';

/** Listener que recibe el valor y el origen del cambio. */
export type TypedListener<T> = (value: T, source: StoreSource) => void;

export interface Store<T> {
  get(): T;
  set(value: T, source?: StoreSource): void;
  update(fn: (current: T) => T, source?: StoreSource): void;
  subscribe(listener: TypedListener<T>): Unsubscribe;
  /** Suscribirse ignorando el source (compatibilidad con listeners simples). */
  subscribeSimple(listener: Listener<T>): Unsubscribe;
}

/* Crear un store reactivo */
export function createStore<T>(initialValue: T): Store<T> {
  let value = initialValue;
  const listeners = new Set<TypedListener<T>>();
  const simpleListeners = new Set<Listener<T>>();

  return {
    get() { return value; },
    set(newValue: T, source: StoreSource = 'user') {
      value = newValue;
      for (const listener of listeners) listener(value, source);
      for (const listener of simpleListeners) listener(value);
    },
    update(fn: (current: T) => T, source: StoreSource = 'user') {
      value = fn(value);
      for (const listener of listeners) listener(value, source);
      for (const listener of simpleListeners) listener(value);
    },
    subscribe(listener: TypedListener<T>): Unsubscribe {
      listeners.add(listener);
      listener(value, 'init');
      return () => { listeners.delete(listener); };
    },
    subscribeSimple(listener: Listener<T>): Unsubscribe {
      simpleListeners.add(listener);
      listener(value);
      return () => { simpleListeners.delete(listener); };
    },
  };
}

/* === Stores globales de la aplicación === */

/* Estado de autenticación
 * [297A-8] Migrado de JWT localStorage a sesiones opacas en cookie HttpOnly. */
export interface AuthState {
  isAuthenticated: boolean;
  userId: string | null;
}

export const authStore = createStore<AuthState>({
  isAuthenticated: false,
  userId: null,
});

/* [Auditoría v4 §3.1] FontConfig dividido en sub-interfaces (ISP).
 * Cada consumidor puede importar solo el subtipo que necesita.
 * FontConfig sigue siendo la intersección completa para backward compatibility. */

/** Fuentes seleccionadas (nombres de Google Fonts). */
export interface FontTypography {
  menu: string;
  titulo: string;
  texto: string;
}

/** Tamaños de fuente en px. */
export interface FontSizes {
  tamanoTexto: number;
  tamanoTitulo: number;
  tamanoPequeno: number;
  tamanoGrande: number;
  tamanoTituloGrande: number;
  menuSize: number;
  entradaTitleSize: number;
  entradaSize: number;
}

/** Opacidades (0-1). */
export interface FontOpacity {
  menuOpacity: number;
  entradaOpacity: number;
}

/** Layout del sidebar, profile y entradas. */
export interface LayoutConfig {
  menuSpacing: number;
  menuLineHeight: number;
  navWidth: number;
  profileWidth: number;
  profileHeight: number;
  profileBorder: boolean;
  sidebarSepHeight: number;
  redesSize: number;
  redesGap: number;
}

/** Configuración completa de fuentes y layout (intersección de ISP). */
export interface FontConfig extends FontTypography, FontSizes, FontOpacity, LayoutConfig {}

export const fontStore = createStore<FontConfig>({
  menu: 'Inter', titulo: 'Inter', texto: 'Inter',
  tamanoTexto: 15, tamanoTitulo: 24, tamanoPequeno: 13, tamanoGrande: 18, tamanoTituloGrande: 32,
  menuSize: 15, menuSpacing: 0, menuLineHeight: 2, menuOpacity: 1,
  entradaTitleSize: 13, entradaSize: 13, entradaOpacity: 1,
  navWidth: 320, profileWidth: 120, profileHeight: 120, profileBorder: true,
  sidebarSepHeight: 24, redesSize: 13, redesGap: 8,
});

/* Aplicar fuentes y tamaños al cambiar */
fontStore.subscribe((config) => {
  const root = document.documentElement;
  root.style.setProperty('--fuente-menu', `'${config.menu}', system-ui, sans-serif`);
  root.style.setProperty('--fuente-titulo', `'${config.titulo}', system-ui, sans-serif`);
  root.style.setProperty('--fuente-texto', `'${config.texto}', system-ui, sans-serif`);
  root.style.setProperty('--tamano-texto', `${config.tamanoTexto}px`);
  root.style.setProperty('--tamano-titulo', `${config.tamanoTitulo}px`);
  root.style.setProperty('--tamano-pequeno', `${config.tamanoPequeno}px`);
  root.style.setProperty('--tamano-grande', `${config.tamanoGrande}px`);
  root.style.setProperty('--tamano-titulo-grande', `${config.tamanoTituloGrande}px`);
  root.style.setProperty('--menu-size', `${config.menuSize}px`);
  root.style.setProperty('--menu-spacing', `${config.menuSpacing}px`);
  root.style.setProperty('--menu-line-height', String(config.menuLineHeight));
  root.style.setProperty('--menu-opacity', String(config.menuOpacity));
  root.style.setProperty('--entrada-title-size', `${config.entradaTitleSize}px`);
  root.style.setProperty('--entrada-size', `${config.entradaSize}px`);
  root.style.setProperty('--entrada-opacity', String(config.entradaOpacity));
  root.style.setProperty('--nav-width', `${config.navWidth}px`);
  root.style.setProperty('--profile-width', `${config.profileWidth}px`);
  root.style.setProperty('--profile-height', `${config.profileHeight}px`);
  root.style.setProperty('--profile-border', config.profileBorder ? 'var(--borde)' : 'none');
  root.style.setProperty('--sidebar-sep-height', `${config.sidebarSepHeight}px`);
  root.style.setProperty('--redes-size', `${config.redesSize}px`);
  root.style.setProperty('--redes-gap', `${config.redesGap}px`);
});

/* Imagen de perfil */
export const profileImage = createStore<string>('/uploads/profile.jpg');

/* Configuración del sitio */
export interface SiteConfig {
  showEntriesOnHome: boolean;
}

export const siteConfig = createStore<SiteConfig>({
  showEntriesOnHome: false,
});

/* Redes sociales configurables */
export interface SocialLink {
  nombre: string;
  url: string;
}

export const socialLinksStore = createStore<SocialLink[]>([
  { nombre: 'instagram', url: 'https://instagram.com/wandorius' },
  { nombre: 'facebook', url: 'https://facebook.com/wandorius' },
  { nombre: 'threads', url: 'https://threads.net/@wandorius' },
  { nombre: 'youtube', url: 'https://youtube.com/@wandorius' },
  { nombre: 'spotify', url: 'https://open.spotify.com/artist/wandorius' },
  { nombre: 'github', url: 'https://github.com/wandorius' },
]);

/* Layout de redes: inline o stacked */
export type RedesLayout = 'inline' | 'stacked';
export const redesLayoutStore = createStore<RedesLayout>('inline');

/* Control de visibilidad del profile header */
export const showProfile = createStore<boolean>(true);

/* Control de visibilidad del sidebar */
export const showSidebar = createStore<boolean>(
  localStorage.getItem('wandorius:sidebar') !== 'hidden',
);
showSidebar.subscribe((visible) => {
  localStorage.setItem('wandorius:sidebar', visible ? 'visible' : 'hidden');
});
