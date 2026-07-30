/* wandori.us — App Registry
 * Catálogo central de aplicaciones del OS.
 * Cada app se registra con id, título, icono, capacidades y función de render.
 * El shell consulta el registry para instanciar apps; las apps no crean chrome. */

import type { IconNode } from 'lucide';
import type { AppRenderFn, MountedView, RenderContext } from '../../core/lifecycle';

/** Referencia a un item del toolbar. Puede ser un command ID o un override. */
export type ToolbarItemRef =
  | string  /* Command ID (ej: 'workspace:trash') o '---' para separador */
  | { readonly id: string; readonly label?: string; readonly icon?: IconNode | null };

/** Grupo del toolbar de una app (equivalente a "Archivo", "Editar", etc.).
 * Cada item referencia un Command del CommandRegistry por ID. */
export interface AppToolbarGroup {
  /** Etiqueta del grupo (dropdown). */
  readonly label: string;
  /** IDs de comandos o overrides. '---' = separador. */
  readonly items: ToolbarItemRef[];
}

/** Capacidad que una app requiere para estar disponible. */
export type Capability =
  | 'public'        /* Visible para todos */
  | 'authenticated' /* Requiere sesión activa */
  | 'admin';        /* Requiere rol admin */

export interface AppDefinition {
  /** Identificador único de la app. */
  readonly id: string;
  /** Título visible en barra de título y taskbar. */
  readonly title: string;
  /** Icono Lucide para desktop y taskbar. */
  readonly icon: IconNode;
  /** Tipo de icono visual (folder, document, application). */
  readonly iconType?: 'folder' | 'document' | 'application';
  /** Si es singleton, no se pueden abrir múltiples instancias. */
  readonly singleton: boolean;
  /** Capacidad mínima requerida para abrir la app. */
  readonly requires: Capability;
  /** Patrones de ruta que esta app maneja. */
  readonly routePatterns?: string[];
  /** Layout del body de la ventana. Default: 'padded'. */
  readonly layout?: 'padded' | 'full-bleed';
  /** Grupos del toolbar de la app. Cada grupo es un dropdown con command IDs. */
  readonly toolbar?: AppToolbarGroup[];
  /** Función que devuelve el contenido de la app (sin chrome). */
  readonly render: AppRenderFn;
}

/**
 * Registry central de aplicaciones del OS.
 * Singleton local — se instancia una vez y se importa donde se necesite.
 */
/** Definición lazy de una app — se carga bajo demanda. */
export interface LazyAppDefinition extends Omit<AppDefinition, 'render'> {
  /** Dynamic import que devuelve el módulo con la función render. */
  readonly load: () => Promise<{ render: AppRenderFn }>;
}

class AppRegistryClass {
  private apps = new Map<string, AppDefinition>();
  private lazyApps = new Map<string, LazyAppDefinition>();
  private loadPromises = new Map<string, Promise<AppDefinition>>();

  /** Registrar una app en el catálogo. */
  register(app: AppDefinition): void {
    this.apps.set(app.id, app);
  }

  /** Registrar una app con lazy loading — el código se carga bajo demanda. */
  registerLazy(app: LazyAppDefinition): void {
    this.lazyApps.set(app.id, app);
    /* Crear wrapper en apps para que get() funcione sin await */
    this.apps.set(app.id, {
      ...app,
      render: async (ctx) => {
        const resolved = await this.resolveLazy(app.id);
        return resolved.render(ctx);
      },
    });
  }

  /** Resolver una app lazy: importar el módulo y reemplazar la definición. */
  private async resolveLazy(id: string): Promise<AppDefinition> {
    const existing = this.loadPromises.get(id);
    if (existing) return existing;

    const lazy = this.lazyApps.get(id);
    if (!lazy) throw new Error(`[AppRegistry] no lazy app: ${id}`);

    const promise = lazy.load().then((mod) => {
      const resolved: AppDefinition = { ...lazy, render: mod.render };
      this.apps.set(id, resolved);
      this.lazyApps.delete(id);
      return resolved;
    });
    this.loadPromises.set(id, promise);
    return promise;
  }

  /** Obtener definición de una app por ID. */
  get(id: string): AppDefinition | undefined {
    return this.apps.get(id);
  }

  /** Listar todas las apps registradas. */
  getAll(): readonly AppDefinition[] {
    return Array.from(this.apps.values());
  }

  /** Listar apps disponibles para una capacidad dada. */
  getAvailable(currentCapability: Capability): readonly AppDefinition[] {
    const hierarchy: Capability[] = ['public', 'authenticated', 'admin'];
    const level = hierarchy.indexOf(currentCapability);
    return this.getAll().filter(app => hierarchy.indexOf(app.requires) <= level);
  }

  /** Buscar app por patrón de ruta. */
  findByRoute(pathname: string): AppDefinition | undefined {
    for (const app of this.apps.values()) {
      if (app.routePatterns?.some(pattern => matchSimplePattern(pattern, pathname))) {
        return app;
      }
    }
    return undefined;
  }

  /** Instanciar el contenido de una app con un RenderContext. */
  async instantiate(appId: string, ctx: RenderContext): Promise<MountedView | null> {
    const app = this.apps.get(appId);
    if (!app) return null;
    return app.render(ctx);
  }
}

/** Coincidencia simple de patrones con :param. */
function matchSimplePattern(pattern: string, pathname: string): boolean {
  const patternParts = pattern.split('/').filter(Boolean);
  const pathParts = pathname.split('/').filter(Boolean);
  if (patternParts.length !== pathParts.length) return false;
  for (let i = 0; i < patternParts.length; i++) {
    if (!patternParts[i].startsWith(':') && patternParts[i] !== pathParts[i]) return false;
  }
  return true;
}

/** Instancia singleton del registry. */
export const AppRegistry = new AppRegistryClass();
