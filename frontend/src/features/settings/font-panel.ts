/* wandori.us — Font Panel (legacy)
 * [297A-29 F1] Se retiró la configuración de fuentes/tamaños (estáticos en
 * variables.css).
 * [297A-29 F3] Los controles de Perfil viven en `profile-settings.ts`.
 * Este módulo solo delega para mantener operativa la app Configuración hasta
 * que la Fase 4 la elimine por completo. No agregar lógica nueva aquí. */

import { createProfileSettingsPanel } from './profile-settings';

export function createFontPanel(): HTMLElement {
  return createProfileSettingsPanel();
}
