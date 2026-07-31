/* wandori.us — Commands Index
 * Barrel import que registra todos los comandos del OS como side-effects.
 * Reemplaza el antiguo command-registration.ts monolítico. */

import './window-commands';
import './geometry-commands';
import './app-commands';
import './workspace-commands';
import './workspace-reorder-commands';
import './toolbar-commands';
import './theme-commands';

export { initKeyboardShortcuts } from './keyboard-handler';
