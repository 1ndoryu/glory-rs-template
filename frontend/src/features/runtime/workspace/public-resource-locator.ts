/* wandori.us — Public Resource Locator
 * Traduce un locator público del workspace a una apertura de app allowlisted.
 * `refId` queda fuera del contrato: es interno y nunca se convierte en URL.
 */

import { AppRegistry } from '../app-registry';
import { hasCapability } from '../capability';
import { parseAppParams } from '../deep-links';
import type { ResolvedNode } from './types';

export interface PublicResourceTarget {
  readonly appId: string;
  readonly params: Record<string, string>;
}

/**
 * Resolver un locator público solo si apunta a una app pública registrada y
 * sus parámetros pasan el deep-link allowlist de esa app.
 */
export function resolvePublicResourceTarget(
  node: Pick<ResolvedNode, 'type' | 'requires' | 'publicLocator'>,
): PublicResourceTarget | null {
  const locator = node.publicLocator;
  if (!locator || !['resource', 'shortcut'].includes(node.type)) return null;
  if (node.requires && node.requires !== 'public') return null;

  const app = AppRegistry.get(locator.appId);
  if (!app || !hasCapability('public', app.requires) || !app.deepLink) return null;

  const params = parseAppParams(app, locator.params);
  if (!params) return null;

  return { appId: app.id, params };
}
