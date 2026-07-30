/* wandori.us — Font Panel
 * Panel principal de configuración. Orquesta tabs y delega a módulos.
 * La persistencia vive en settings-repo.ts.
 * [Auditoría v4 §1.2] Migrado a createEl(). */

import { safeRun } from '../../utils/safe-async';
import { fontStore, profileImage, siteConfig, type FontConfig } from '../../store';
import { MediaService, SettingsService } from '../../services';
import { showToast } from '../../components/ui/toast';
import { createArrowSelect, createSizeSlider } from './font-helpers';
import { renderSocialLinksSection } from './social-links';
import { loadAllFonts, saveSettings } from './settings-repo';
import { createEl } from '../../utils/dom';

type SizeKey = 'tamanoTexto' | 'tamanoTitulo' | 'tamanoPequeno' | 'tamanoGrande' | 'tamanoTituloGrande' | 'menuSize' | 'menuSpacing' | 'menuLineHeight' | 'entradaTitleSize' | 'entradaSize' | 'navWidth' | 'profileWidth' | 'profileHeight' | 'sidebarSepHeight' | 'redesSize' | 'redesGap';
type NumberKey = SizeKey | 'menuOpacity' | 'entradaOpacity';

export function createFontPanel(): HTMLElement {
  loadAllFonts();
  const panel = createEl('div', { className: 'font-panel' });

  function updateSize(key: NumberKey, value: number): void {
    fontStore.update(s => ({ ...s, [key]: value }));
    saveSettings();
  }

  function updateFont(key: keyof FontConfig, value: string): void {
    fontStore.update(s => ({ ...s, [key]: value }));
    saveSettings();
    showToast('fuente actualizada');
  }

  /* === Tab: Perfil === */
  function renderTabPerfil(): HTMLElement {
    const tab = createEl('div', { className: 'config-tab-content' });
    const cfg = fontStore.get();

    const imgPreview = createEl('img', { className: 'config-imagen-preview', src: profileImage.get() });
    imgPreview.onerror = () => { imgPreview.classList.add('oculto'); };

    const imgInput = createEl('input', { type: 'file' });
    imgInput.accept = 'image/*';
    imgInput.classList.add('oculto');

    const btnImg = createEl('button', { className: 'boton', textContent: 'cambiar imagen' });
    btnImg.addEventListener('click', () => imgInput.click());

    imgInput.addEventListener('change', async () => {
      const file = imgInput.files?.[0];
      if (!file) return;
      const result = await safeRun(MediaService.upload(file), 'error al subir imagen');
      if (result.ok) {
        profileImage.set(result.value.file_path);
        imgPreview.src = result.value.file_path;
        imgPreview.classList.remove('oculto');
        SettingsService.save({ profile_image: result.value.file_path }).catch(() => {});
        showToast('imagen actualizada');
      }
    });

    const entradasCheck = createEl('input', { type: 'checkbox' });
    entradasCheck.checked = siteConfig.get().showEntriesOnHome;
    entradasCheck.addEventListener('change', () => {
      siteConfig.update(s => ({ ...s, showEntriesOnHome: entradasCheck.checked }));
      SettingsService.save({ show_entries_on_home: String(entradasCheck.checked) }).catch(() => {});
    });
    const entradasLabel = createEl('label', { className: 'checkbox-personalizado' }, entradasCheck, 'mostrar entradas en inicio');

    const imgSizeSlider = createSizeSlider('Tamaño', 40, 600, cfg.profileWidth, (v) => {
      updateSize('profileWidth', v);
      updateSize('profileHeight', v);
    });
    const imgWidthSlider = createSizeSlider('Ancho', 40, 600, cfg.profileWidth, (v) => updateSize('profileWidth', v));
    const imgHeightSlider = createSizeSlider('Alto', 40, 600, cfg.profileHeight, (v) => updateSize('profileHeight', v));

    const borderCheck = createEl('input', { type: 'checkbox' });
    borderCheck.checked = cfg.profileBorder;
    borderCheck.addEventListener('change', () => {
      fontStore.update(s => ({ ...s, profileBorder: borderCheck.checked }));
      saveSettings();
    });
    const borderLabel = createEl('label', { className: 'checkbox-personalizado' }, borderCheck, 'borde');

    const imgLabel = createEl('label', { className: 'campo-etiqueta', textContent: 'Imagen' });
    const separador = createEl('div', { className: 'config-tab-separador' });
    const socialElements = renderSocialLinksSection(cfg, updateSize as (k: string, v: number) => void);

    tab.append(imgLabel, imgPreview, btnImg, imgInput, imgSizeSlider, imgWidthSlider, imgHeightSlider, borderLabel, entradasLabel, separador, ...socialElements);
    return tab;
  }

  /* === Tab: Fuentes === */
  function renderTabFuentes(): HTMLElement {
    const tab = createEl('div', { className: 'config-tab-content' });
    const cfg = fontStore.get();
    const sep = (): HTMLElement => createEl('div', { className: 'config-tab-separador' });

    const menuArrow = createArrowSelect('Fuente del menú', cfg.menu, (v) => updateFont('menu', v));
    const menuSizeSlider = createSizeSlider('Tamaño', 10, 24, cfg.menuSize, (v) => updateSize('menuSize', v));
    const menuSpacing = createSizeSlider('Espaciado', 0, 10, cfg.menuSpacing, (v) => updateSize('menuSpacing', v));
    const menuLineHeight = createSizeSlider('Interlineado', 1, 4, cfg.menuLineHeight, (v) => updateSize('menuLineHeight', v), '', 0.1);
    const menuOpacity = createSizeSlider('Opacidad', 0, 1, cfg.menuOpacity, (v) => updateSize('menuOpacity', v), '', 0.05);

    const tituloArrow = createArrowSelect('Fuente de títulos', cfg.titulo, (v) => updateFont('titulo', v));
    const tituloSize = createSizeSlider('Tamaño título', 18, 48, cfg.tamanoTitulo, (v) => updateSize('tamanoTitulo', v));
    const tituloGrandeSize = createSizeSlider('Tamaño título grande', 24, 56, cfg.tamanoTituloGrande, (v) => updateSize('tamanoTituloGrande', v));

    const textoArrow = createArrowSelect('Fuente del texto', cfg.texto, (v) => updateFont('texto', v));
    const textoSize = createSizeSlider('Tamaño texto', 12, 24, cfg.tamanoTexto, (v) => updateSize('tamanoTexto', v));
    const pequenoSize = createSizeSlider('Tamaño pequeño', 10, 18, cfg.tamanoPequeno, (v) => updateSize('tamanoPequeno', v));
    const grandeSize = createSizeSlider('Tamaño grande', 14, 28, cfg.tamanoGrande, (v) => updateSize('tamanoGrande', v));

    const navTitleLabel = createEl('label', { className: 'campo-etiqueta', textContent: 'Títulos nav' });
    const navTitleSizeSlider = createSizeSlider('Tamaño', 10, 20, cfg.entradaTitleSize, (v) => updateSize('entradaTitleSize', v));
    const navTitleOpacitySlider = createSizeSlider('Opacidad', 0, 1, cfg.entradaOpacity, (v) => updateSize('entradaOpacity', v), '', 0.05);

    const entradaLabel = createEl('label', { className: 'campo-etiqueta', textContent: 'Entradas' });
    const entradaSizeSlider = createSizeSlider('Tamaño texto', 10, 24, cfg.entradaSize, (v) => updateSize('entradaSize', v));

    tab.append(
      menuArrow, menuSizeSlider, menuSpacing, menuLineHeight, menuOpacity, sep(),
      navTitleLabel, navTitleSizeSlider, navTitleOpacitySlider, sep(),
      entradaLabel, entradaSizeSlider, sep(),
      tituloArrow, tituloSize, tituloGrandeSize, sep(),
      textoArrow, textoSize, pequenoSize, grandeSize,
    );
    return tab;
  }

  /* === Tab: Tamaños === */
  function renderTabTamanos(): HTMLElement {
    const tab = createEl('div', { className: 'config-tab-content' });
    const cfg = fontStore.get();
    tab.append(
      createSizeSlider('Ancho del nav', 200, 500, cfg.navWidth, (v) => updateSize('navWidth', v)),
      createSizeSlider('Separación nav', 0, 80, cfg.sidebarSepHeight, (v) => updateSize('sidebarSepHeight', v)),
    );
    return tab;
  }

  /* === Tabs UI === */
  const tabsNav = createEl('div', { className: 'config-tabs-nav' });
  const tabsContent = createEl('div', { className: 'config-tabs-content' });

  const tabDefs: Array<{ name: string; render: () => HTMLElement }> = [
    { name: 'Perfil', render: renderTabPerfil },
    { name: 'Fuentes', render: renderTabFuentes },
    { name: 'Tamaños', render: renderTabTamanos },
  ];

  function switchTab(name: string): void {
    tabsNav.querySelectorAll('.boton').forEach(b => b.classList.toggle('activo', b.textContent === name));
    tabsContent.textContent = '';
    const def = tabDefs.find(t => t.name === name);
    if (def) tabsContent.appendChild(def.render());
  }

  for (const def of tabDefs) {
    const btn = createEl('button', { className: 'boton', textContent: def.name });
    btn.addEventListener('click', () => switchTab(def.name));
    tabsNav.appendChild(btn);
  }

  panel.append(tabsNav, tabsContent);
  switchTab('Perfil');
  return panel;
}

/* Re-export loadSavedFonts para backward compatibility con main.ts */
export { loadSavedFonts } from './settings-repo';
