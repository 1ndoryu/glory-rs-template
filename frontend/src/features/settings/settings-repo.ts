/* wandori.us — Settings Repository
 * Persistencia de configuración del OS: carga/guardado de settings y Google Fonts.
 * Extraído de font-panel.ts para reducir tamaño bajo límite de 300 líneas.
 * [Auditoría v3 §2.3] */

import { safeRun } from '../../utils/safe-async';
import { createEl } from '../../utils/dom';
import { fontStore, profileImage, siteConfig, socialLinksStore, redesLayoutStore, type FontConfig } from '../../store';
import { SettingsService } from '../../services';
import { GOOGLE_FONTS } from './font-constants';

let fontsLoaded = false;
let saveTimer: ReturnType<typeof setTimeout> | null = null;

function loadGoogleFont(fontName: string): void {
  const id = `gf-${fontName.replace(/\s+/g, '-').toLowerCase()}`;
  if (document.getElementById(id)) return;
  const link = createEl('link', { id, rel: 'stylesheet' });
  link.href = `https://fonts.googleapis.com/css2?family=${fontName.replace(/\s+/g, '+')}:wght@400;500&display=swap`;
  document.head.appendChild(link);
}

export function loadAllFonts(): void {
  if (fontsLoaded) return;
  fontsLoaded = true;
  for (const font of GOOGLE_FONTS) loadGoogleFont(font);
}

export function saveSettings(): void {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    const c = fontStore.get();
    await safeRun(SettingsService.save({
      font_menu: c.menu, font_titulo: c.titulo, font_texto: c.texto,
        tamano_texto: String(c.tamanoTexto), tamano_titulo: String(c.tamanoTitulo),
        tamano_pequeno: String(c.tamanoPequeno), tamano_grande: String(c.tamanoGrande),
        tamano_titulo_grande: String(c.tamanoTituloGrande),
        menu_size: String(c.menuSize), menu_spacing: String(c.menuSpacing),
        menu_line_height: String(c.menuLineHeight), menu_opacity: String(c.menuOpacity),
        entrada_title_size: String(c.entradaTitleSize), entrada_size: String(c.entradaSize),
        entrada_opacity: String(c.entradaOpacity), nav_width: String(c.navWidth),
        profile_width: String(c.profileWidth), profile_height: String(c.profileHeight),
        profile_border: String(c.profileBorder), sidebar_sep_height: String(c.sidebarSepHeight),
        redes_size: String(c.redesSize), redes_gap: String(c.redesGap),
      }), 'error al guardar configuración');
  }, 500);
}

export async function loadSavedFonts(): Promise<void> {
  loadAllFonts();

  let config: FontConfig = {
    menu: 'Space Grotesk', titulo: 'Playfair Display', texto: 'Inter',
    tamanoTexto: 15, tamanoTitulo: 24, tamanoPequeno: 13, tamanoGrande: 18,
    tamanoTituloGrande: 32, menuSize: 15, menuSpacing: 0, menuLineHeight: 2,
    menuOpacity: 1, entradaTitleSize: 13, entradaSize: 13, entradaOpacity: 1,
    navWidth: 320, profileWidth: 120, profileHeight: 120, profileBorder: true,
    sidebarSepHeight: 24, redesSize: 13, redesGap: 8,
  };

  try {
    const settings = await SettingsService.getAll();
    if (settings && Object.keys(settings).length > 0) {
      config = {
        menu: settings.font_menu || config.menu,
        titulo: settings.font_titulo || config.titulo,
        texto: settings.font_texto || config.texto,
        tamanoTexto: Number(settings.tamano_texto) || config.tamanoTexto,
        tamanoTitulo: Number(settings.tamano_titulo) || config.tamanoTitulo,
        tamanoPequeno: Number(settings.tamano_pequeno) || config.tamanoPequeno,
        tamanoGrande: Number(settings.tamano_grande) || config.tamanoGrande,
        tamanoTituloGrande: Number(settings.tamano_titulo_grande) || config.tamanoTituloGrande,
        menuSize: settings.menu_size !== undefined ? Number(settings.menu_size) : config.menuSize,
        menuSpacing: settings.menu_spacing !== undefined ? Number(settings.menu_spacing) : config.menuSpacing,
        menuLineHeight: settings.menu_line_height !== undefined ? Number(settings.menu_line_height) : config.menuLineHeight,
        menuOpacity: settings.menu_opacity !== undefined ? Number(settings.menu_opacity) : config.menuOpacity,
        entradaTitleSize: settings.entrada_title_size !== undefined ? Number(settings.entrada_title_size) : config.entradaTitleSize,
        entradaSize: settings.entrada_size !== undefined ? Number(settings.entrada_size) : config.entradaSize,
        entradaOpacity: settings.entrada_opacity !== undefined ? Number(settings.entrada_opacity) : config.entradaOpacity,
        navWidth: settings.nav_width !== undefined ? Number(settings.nav_width) : config.navWidth,
        profileWidth: settings.profile_width !== undefined ? Number(settings.profile_width) : config.profileWidth,
        profileHeight: settings.profile_height !== undefined ? Number(settings.profile_height) : config.profileHeight,
        profileBorder: settings.profile_border !== undefined ? settings.profile_border === 'true' : config.profileBorder,
        sidebarSepHeight: settings.sidebar_sep_height !== undefined ? Number(settings.sidebar_sep_height) : config.sidebarSepHeight,
        redesSize: settings.redes_size !== undefined ? Number(settings.redes_size) : config.redesSize,
        redesGap: settings.redes_gap !== undefined ? Number(settings.redes_gap) : config.redesGap,
      };
    }
    if (settings.profile_image) profileImage.set(settings.profile_image);
    if (settings.show_entries_on_home !== undefined) {
      siteConfig.update(s => ({ ...s, showEntriesOnHome: settings.show_entries_on_home === 'true' }));
    }
    if (settings.social_links) {
      try {
        const links = JSON.parse(settings.social_links) as Array<{ nombre: string; url: string }>;
        if (Array.isArray(links)) socialLinksStore.set(links);
      } catch { /* JSON invalido */ }
    }
    if (settings.redes_layout === 'stacked' || settings.redes_layout === 'inline') {
      redesLayoutStore.set(settings.redes_layout as 'inline' | 'stacked');
    }
  } catch { /* Backend no disponible */ }

  fontStore.set(config);
}
