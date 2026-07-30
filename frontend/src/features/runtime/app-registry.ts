/* wandori.us — App Registry
 * Catálogo central de aplicaciones del OS.
 * Cada app se registra con id, título, icono, capacidades y función de render.
 * El shell consulta el registry para instanciar apps; las apps no crean chrome. */

import type { IconNode } from 'lucide';
import type { AppRenderFn, MountedView, RenderContext } from '../../core/lifecycle';

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
  /** Función que devuelve el contenido de la app (sin chrome). */
  readonly render: AppRenderFn;
}

/**
 * Registry central de aplicaciones del OS.
 * Singleton local — se instancia una vez y se importa donde se necesite.
 */
class AppRegistryClass {
  private apps = new Map<string, AppDefinition>();

  /** Registrar una app en el catálogo. */
  register(app: AppDefinition): void {
    this.apps.set(app.id, app);
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
