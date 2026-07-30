/* wandori.us — Config Panel
 * Panel para elegir fuentes, tamanos de letra e imagen de perfil.
 * Carga fuentes de Google Fonts dinamicamente. */

import { fontStore, profileImage, siteConfig, socialLinksStore, redesLayoutStore, type FontConfig, type SocialLink, type RedesLayout } from '../../store';

type SizeKey = 'tamanoTexto' | 'tamanoTitulo' | 'tamanoPequeno' | 'tamanoGrande' | 'tamanoTituloGrande' | 'menuSize' | 'menuSpacing' | 'menuLineHeight' | 'entradaTitleSize' | 'entradaSize' | 'navWidth' | 'profileWidth' | 'profileHeight' | 'sidebarSepHeight' | 'redesSize' | 'redesGap';
type NumberKey = SizeKey | 'menuOpacity' | 'entradaOpacity';
import { api } from '../../api/client';
import { showToast } from '../../components/ui/toast';

const GOOGLE_FONTS = [
  'Inter', 'Roboto', 'Open Sans', 'Lato', 'Montserrat',
  'Source Sans Pro', 'Nunito', 'Poppins', 'Raleway', 'Work Sans',
  'Merriweather', 'Playfair Display', 'Lora', 'PT Serif', 'Noto Serif',
  'Crimson Text', 'EB Garamond', 'Cormorant Garamond', 'Libre Baskerville',
  'DM Sans', 'Space Grotesk', 'JetBrains Mono', 'Fira Code',
];

let fontsLoaded = false;
let saveTimer: ReturnType<typeof setTimeout> | null = null;
let socialSaveTimer: ReturnType<typeof setTimeout> | null = null;

function loadGoogleFont(fontName: string): void {
  const id = `gf-${fontName.replace(/\s+/g, '-').toLowerCase()}`;
  if (document.getElementById(id)) return;

  const link = document.createElement('link');
  link.id = id;
  link.rel = 'stylesheet';
  link.href = `https://fonts.googleapis.com/css2?family=${fontName.replace(/\s+/g, '+')}:wght@400;500&display=swap`;
  document.head.appendChild(link);
}

function loadAllFonts(): void {
  if (fontsLoaded) return;
  fontsLoaded = true;
  for (const font of GOOGLE_FONTS) {
    loadGoogleFont(font);
  }
}

/* Helper: selector con flechas ← → para ciclar fuentes */
function createArrowSelect(
  etiqueta: string,
  currentValue: string,
  onChange: (value: string) => void,
): HTMLElement {
  const container = document.createElement('div');
  container.className = 'campo';

  const label = document.createElement('label');
  label.className = 'campo-etiqueta';
  label.textContent = etiqueta;

  const row = document.createElement('div');
  row.className = 'arrow-select';

  const btnPrev = document.createElement('button');
  btnPrev.className = 'arrow-select-btn';
  btnPrev.textContent = '←';
  btnPrev.setAttribute('aria-label', 'fuente anterior');

  const nombre = document.createElement('span');
  nombre.className = 'arrow-select-nombre';
  nombre.style.fontFamily = `'${currentValue}', system-ui`;
  nombre.textContent = currentValue;

  const btnNext = document.createElement('button');
  btnNext.className = 'arrow-select-btn';
  btnNext.textContent = '→';
  btnNext.setAttribute('aria-label', 'fuente siguiente');

  /* [297A-1] Los controles deben pertenecer a la fila visible; el dropdown
   * se monta aparte para poder posicionarse sobre el panel. */
  row.append(btnPrev, nombre, btnNext);

  let idx = GOOGLE_FONTS.indexOf(currentValue);
  if (idx === -1) idx = 0;

  function updateDisplay(): void {
    const font = GOOGLE_FONTS[idx];
    nombre.textContent = font;
    nombre.style.fontFamily = `'${font}', system-ui`;
    onChange(font);
    /* Sincronizar activo en dropdown */
    dropdown.querySelectorAll('.arrow-select-item').forEach((item, i) => {
      item.classList.toggle('activo', i === idx);
    });
  }

  btnPrev.addEventListener('click', () => {
    idx = (idx - 1 + GOOGLE_FONTS.length) % GOOGLE_FONTS.length;
    updateDisplay();
  });

  btnNext.addEventListener('click', () => {
    idx = (idx + 1) % GOOGLE_FONTS.length;
    updateDisplay();
  });

  /* Dropdown de fuentes — segunda forma de elegir */
  const dropdown = document.createElement('div');
  dropdown.className = 'arrow-select-dropdown';
  dropdown.style.display = 'none';

  for (const font of GOOGLE_FONTS) {
    const item = document.createElement('button');
    item.className = 'boton arrow-select-item';
    item.textContent = font;
    item.style.fontFamily = `'${font}', system-ui`;
    if (font === currentValue) item.classList.add('activo');
    item.addEventListener('click', () => {
      idx = GOOGLE_FONTS.indexOf(font);
      updateDisplay();
      dropdown.style.display = 'none';
      /* Marcar activo */
      dropdown.querySelectorAll('.arrow-select-item').forEach(i => i.classList.remove('activo'));
      item.classList.add('activo');
    });
    dropdown.appendChild(item);
  }

  /* Contenedor relativo para posicionar el dropdown */
  const selectWrapper = document.createElement('div');
  selectWrapper.className = 'arrow-select-wrapper';
  selectWrapper.append(row, dropdown);

  nombre.addEventListener('click', () => {
    const abierto = dropdown.style.display !== 'none';
    dropdown.style.display = abierto ? 'none' : 'flex';
  });

  /* Cerrar dropdown al hacer click fuera */
  const cerrarDropdown = (e: MouseEvent) => {
    if (!selectWrapper.contains(e.target as Node)) {
      dropdown.style.display = 'none';
    }
  };
  document.addEventListener('click', cerrarDropdown, true);
  /* Cleanup: observar solo el padre inmediato (barato vs document.body) */
  const observer = new MutationObserver(() => {
    if (!container.isConnected) {
      document.removeEventListener('click', cerrarDropdown, true);
      observer.disconnect();
    }
  });
  observer.observe(container.parentNode || document.body, { childList: true });

  container.append(label, selectWrapper);
  return container;
}

/* Helper: crear slider de tamano */
function createSizeSlider(
  label: string,
  min: number,
  max: number,
  value: number,
  onChange: (v: number) => void,
  suffix = 'px',
  step?: number,
): HTMLElement {
  const container = document.createElement('div');
  container.className = 'campo';

  const etiqueta = document.createElement('label');
  etiqueta.className = 'campo-etiqueta';
  etiqueta.textContent = label;

  const valor = document.createElement('span');
  valor.className = 'slider-valor';
  valor.textContent = `${value}${suffix}`;

  const header = document.createElement('div');
  header.className = 'slider-header';
  header.append(etiqueta, valor);

  const input = document.createElement('input');
  input.type = 'range';
  input.className = 'slider-input';
  input.min = String(min);
  input.max = String(max);
  input.value = String(value);
  if (step !== undefined) input.step = String(step);

  input.addEventListener('input', () => {
    const v = Number(input.value);
    valor.textContent = `${v}${suffix}`;
    onChange(v);
  });

  container.append(header, input);
  return container;
}

export function createFontPanel(): HTMLElement {
  loadAllFonts();

  const panel = document.createElement('div');
  panel.className = 'font-panel';

  function updateSize(key: NumberKey, value: number): void {
    fontStore.update(s => ({ ...s, [key]: value }));
    saveSettings();
  }

  function updateFont(key: keyof FontConfig, value: string): void {
    fontStore.update(s => ({ ...s, [key]: value }));
    saveSettings();
    showToast('fuente actualizada');
  }

  /* Guardar enlaces sociales */
  function saveSocialLinks(): void {
    const links = socialLinksStore.get();
    api.post('/api/settings', {
      settings: {
        social_links: JSON.stringify(links),
        redes_layout: redesLayoutStore.get(),
      },
    }).catch((err) => {
      console.error('Error guardando enlaces:', err);
    });
  }

  function debouncedSaveSocial(): void {
    if (socialSaveTimer) clearTimeout(socialSaveTimer);
    socialSaveTimer = setTimeout(saveSocialLinks, 500);
  }

  function saveSettings(): void {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      const c = fontStore.get();
      try {
        await api.post('/api/settings', {
          settings: {
            font_menu: c.menu,
            font_titulo: c.titulo,
            font_texto: c.texto,
            tamano_texto: String(c.tamanoTexto),
            tamano_titulo: String(c.tamanoTitulo),
            tamano_pequeno: String(c.tamanoPequeno),
            tamano_grande: String(c.tamanoGrande),
            tamano_titulo_grande: String(c.tamanoTituloGrande),
            menu_size: String(c.menuSize),
            menu_spacing: String(c.menuSpacing),
            menu_line_height: String(c.menuLineHeight),
            menu_opacity: String(c.menuOpacity),
            entrada_title_size: String(c.entradaTitleSize),
            entrada_size: String(c.entradaSize),
            entrada_opacity: String(c.entradaOpacity),
            nav_width: String(c.navWidth),
            profile_width: String(c.profileWidth),
            profile_height: String(c.profileHeight),
            profile_border: String(c.profileBorder),
            sidebar_sep_height: String(c.sidebarSepHeight),
            redes_size: String(c.redesSize),
            redes_gap: String(c.redesGap),
          },
        });
      } catch (err) {
        console.error('Error guardando settings:', err);
        showToast('error al guardar configuración');
      }
    }, 500);
  }

  /* === Tab: Perfil === */
  function renderTabPerfil(): HTMLElement {
    const tab = document.createElement('div');
    tab.className = 'config-tab-content';

    const imgPreview = document.createElement('img');
    imgPreview.className = 'config-imagen-preview';
    imgPreview.src = profileImage.get();
    imgPreview.onerror = () => { imgPreview.style.display = 'none'; };

    const imgInput = document.createElement('input');
    imgInput.type = 'file';
    imgInput.accept = 'image/*';
    imgInput.style.display = 'none';

    const btnImg = document.createElement('button');
    btnImg.className = 'boton';
    btnImg.textContent = 'cambiar imagen';
    btnImg.addEventListener('click', () => imgInput.click());

    imgInput.addEventListener('change', async () => {
      const file = imgInput.files?.[0];
      if (!file) return;
      const formData = new FormData();
      formData.append('file', file);
      formData.append('alt_text', 'profile');
      try {
        const media = await api.upload<{ file_path: string }>('/api/media', formData);
        profileImage.set(media.file_path);
        imgPreview.src = media.file_path;
        imgPreview.style.display = 'block';
        api.post('/api/settings', { settings: { profile_image: media.file_path } }).catch(() => {});
        showToast('imagen actualizada');
      } catch { showToast('error al subir imagen'); }
    });

    /* Entradas en home */
    const entradasLabel = document.createElement('label');
    entradasLabel.className = 'checkbox-personalizado';
    const entradasCheck = document.createElement('input');
    entradasCheck.type = 'checkbox';
    entradasCheck.checked = siteConfig.get().showEntriesOnHome;
    entradasCheck.addEventListener('change', () => {
      siteConfig.update(s => ({ ...s, showEntriesOnHome: entradasCheck.checked }));
      api.post('/api/settings', { settings: { show_entries_on_home: String(entradasCheck.checked) } }).catch(() => {});
    });
    const entradasTexto = document.createElement('span');
    entradasTexto.textContent = 'mostrar entradas en inicio';
    entradasLabel.append(entradasCheck, entradasTexto);

    /* Imagen: tamaño general */
    const cfg = fontStore.get();
    const imgSizeSlider = createSizeSlider('Tamaño', 40, 600, cfg.profileWidth, (v) => {
      updateSize('profileWidth', v);
      updateSize('profileHeight', v);
    });

    /* Imagen: ancho y alto independientes (recorte) */
    const imgWidthSlider = createSizeSlider('Ancho', 40, 600, cfg.profileWidth, (v) => updateSize('profileWidth', v));
    const imgHeightSlider = createSizeSlider('Alto', 40, 600, cfg.profileHeight, (v) => updateSize('profileHeight', v));

    /* Imagen: borde toggle */
    const borderLabel = document.createElement('label');
    borderLabel.className = 'checkbox-personalizado';
    const borderCheck = document.createElement('input');
    borderCheck.type = 'checkbox';
    borderCheck.checked = cfg.profileBorder;
    borderCheck.addEventListener('change', () => {
      fontStore.update(s => ({ ...s, profileBorder: borderCheck.checked }));
      saveSettings();
    });
    const borderTexto = document.createElement('span');
    borderTexto.textContent = 'borde';
    borderLabel.append(borderCheck, borderTexto);

    const imgLabel = document.createElement('label');
    imgLabel.className = 'campo-etiqueta';
    imgLabel.textContent = 'Imagen';

    /* === Enlaces sociales === */
    const enlacesLabel = document.createElement('label');
    enlacesLabel.className = 'campo-etiqueta';
    enlacesLabel.textContent = 'Enlaces';

    const redesSizeSlider = createSizeSlider('Tamaño enlaces', 8, 24, cfg.redesSize, (v) => updateSize('redesSize', v));
    const redesGapSlider = createSizeSlider('Separación enlaces', 0, 20, cfg.redesGap, (v) => updateSize('redesGap', v));

    const enlacesContainer = document.createElement('div');
    enlacesContainer.className = 'enlaces-editor';

    /* Toggle layout inline/stacked */
    const layoutLabel = document.createElement('label');
    layoutLabel.className = 'checkbox-personalizado';
    const layoutCheck = document.createElement('input');
    layoutCheck.type = 'checkbox';
    layoutCheck.checked = redesLayoutStore.get() === 'stacked';
    layoutCheck.addEventListener('change', () => {
      const layout: RedesLayout = layoutCheck.checked ? 'stacked' : 'inline';
      redesLayoutStore.set(layout);
      saveSocialLinks();
    });
    const layoutTexto = document.createElement('span');
    layoutTexto.textContent = 'uno por linea';
    layoutLabel.append(layoutCheck, layoutTexto);

    function renderEnlaces(): void {
      enlacesContainer.innerHTML = '';
      const links = socialLinksStore.get();

      for (let i = 0; i < links.length; i++) {
        const row = document.createElement('div');
        row.className = 'enlace-row';

        const nombreInput = document.createElement('input');
        nombreInput.className = 'campo-entrada enlace-nombre';
        nombreInput.value = links[i].nombre;
        nombreInput.placeholder = 'nombre';
        nombreInput.addEventListener('input', () => {
          socialLinksStore.update(arr => {
            const copy = [...arr];
            copy[i] = { ...copy[i], nombre: nombreInput.value };
            return copy;
          });
          debouncedSaveSocial();
        });

        const urlInput = document.createElement('input');
        urlInput.className = 'campo-entrada enlace-url';
        urlInput.value = links[i].url;
        urlInput.placeholder = 'https://...';
        urlInput.addEventListener('input', () => {
          socialLinksStore.update(arr => {
            const copy = [...arr];
            copy[i] = { ...copy[i], url: urlInput.value };
            return copy;
          });
          debouncedSaveSocial();
        });

        const btnQuitar = document.createElement('button');
        btnQuitar.className = 'boton enlace-quitar';
        btnQuitar.textContent = '×';
        btnQuitar.title = 'quitar enlace';
        btnQuitar.addEventListener('click', () => {
          socialLinksStore.update(arr => arr.filter((_, idx) => idx !== i));
          renderEnlaces();
          saveSocialLinks();
        });

        row.append(nombreInput, urlInput, btnQuitar);
        enlacesContainer.appendChild(row);
      }

      /* Boton agregar */
      const btnAgregar = document.createElement('button');
      btnAgregar.className = 'boton';
      btnAgregar.textContent = '+ agregar enlace';
      btnAgregar.style.marginTop = 'var(--espacio-sm)';
      btnAgregar.addEventListener('click', () => {
        socialLinksStore.update(arr => [...arr, { nombre: '', url: '' }]);
        renderEnlaces();
      });
      enlacesContainer.appendChild(btnAgregar);
    }

    renderEnlaces();

    const separador = document.createElement('div');
    separador.className = 'config-tab-separador';
    tab.append(imgLabel, imgPreview, btnImg, imgInput, imgSizeSlider, imgWidthSlider, imgHeightSlider, borderLabel, entradasLabel, separador, enlacesLabel, redesSizeSlider, redesGapSlider, layoutLabel, enlacesContainer);
    return tab;
  }

  /* === Tab: Fuentes === */
  function renderTabFuentes(): HTMLElement {
    const tab = document.createElement('div');
    tab.className = 'config-tab-content';
    const cfg = fontStore.get();

    const sep = (): HTMLElement => {
      const s = document.createElement('div');
      s.className = 'config-tab-separador';
      return s;
    };

    /* --- Menu --- */
    const menuArrow = createArrowSelect('Fuente del menú', cfg.menu, (v) => updateFont('menu', v));
    const menuSizeSlider = createSizeSlider('Tamaño', 10, 24, cfg.menuSize, (v) => updateSize('menuSize', v));
    const menuSpacing = createSizeSlider('Espaciado', 0, 10, cfg.menuSpacing, (v) => updateSize('menuSpacing', v));
    const menuLineHeight = createSizeSlider('Interlineado', 1, 4, cfg.menuLineHeight, (v) => updateSize('menuLineHeight', v), '', 0.1);
    const menuOpacity = createSizeSlider('Opacidad', 0, 1, cfg.menuOpacity, (v) => updateSize('menuOpacity', v), '', 0.05);

    /* --- Títulos --- */
    const tituloArrow = createArrowSelect('Fuente de títulos', cfg.titulo, (v) => updateFont('titulo', v));
    const tituloSize = createSizeSlider('Tamaño título', 18, 48, cfg.tamanoTitulo, (v) => updateSize('tamanoTitulo', v));
    const tituloGrandeSize = createSizeSlider('Tamaño título grande', 24, 56, cfg.tamanoTituloGrande, (v) => updateSize('tamanoTituloGrande', v));

    /* --- Texto --- */
    const textoArrow = createArrowSelect('Fuente del texto', cfg.texto, (v) => updateFont('texto', v));
    const textoSize = createSizeSlider('Tamaño texto', 12, 24, cfg.tamanoTexto, (v) => updateSize('tamanoTexto', v));
    const pequenoSize = createSizeSlider('Tamaño pequeño', 10, 18, cfg.tamanoPequeno, (v) => updateSize('tamanoPequeno', v));
    const grandeSize = createSizeSlider('Tamaño grande', 14, 28, cfg.tamanoGrande, (v) => updateSize('tamanoGrande', v));

    /* --- Títulos nav (títulos de entradas en el sidebar) --- */
    const navTitleLabel = document.createElement('label');
    navTitleLabel.className = 'campo-etiqueta';
    navTitleLabel.textContent = 'Títulos nav';
    const navTitleSizeSlider = createSizeSlider('Tamaño', 10, 20, cfg.entradaTitleSize, (v) => updateSize('entradaTitleSize', v));
    const navTitleOpacitySlider = createSizeSlider('Opacidad', 0, 1, cfg.entradaOpacity, (v) => updateSize('entradaOpacity', v), '', 0.05);

    /* --- Entradas (contenido de artículos) --- */
    const entradaLabel = document.createElement('label');
    entradaLabel.className = 'campo-etiqueta';
    entradaLabel.textContent = 'Entradas';
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

  /* === Tab: Tamanos (solo layout) === */
  function renderTabTamanos(): HTMLElement {
    const tab = document.createElement('div');
    tab.className = 'config-tab-content';
    const cfg = fontStore.get();

    tab.append(
      createSizeSlider('Ancho del nav', 200, 500, cfg.navWidth, (v) => updateSize('navWidth', v)),
      createSizeSlider('Separación nav', 0, 80, cfg.sidebarSepHeight, (v) => updateSize('sidebarSepHeight', v)),
    );
    return tab;
  }

  /* === Tabs UI === */
  const tabsNav = document.createElement('div');
  tabsNav.className = 'config-tabs-nav';

  const tabsContent = document.createElement('div');
  tabsContent.className = 'config-tabs-content';

  const tabDefs: Array<{ name: string; render: () => HTMLElement }> = [
    { name: 'Perfil', render: renderTabPerfil },
    { name: 'Fuentes', render: renderTabFuentes },
    { name: 'Tamaños', render: renderTabTamanos },
  ];

  function switchTab(name: string): void {
    tabsNav.querySelectorAll('.boton').forEach(b => {
      b.classList.toggle('activo', b.textContent === name);
    });
    tabsContent.innerHTML = '';
    const def = tabDefs.find(t => t.name === name);
    if (def) tabsContent.appendChild(def.render());
  }

  for (const def of tabDefs) {
    const btn = document.createElement('button');
    btn.className = 'boton';
    btn.textContent = def.name;
    btn.addEventListener('click', () => switchTab(def.name));
    tabsNav.appendChild(btn);
  }

  panel.append(tabsNav, tabsContent);    switchTab('Perfil');

  return panel;
}

/* Cargar fuentes y settings guardados al inicio */
export async function loadSavedFonts(): Promise<void> {
  loadAllFonts();

  let config: FontConfig = {
    menu: 'Space Grotesk',
    titulo: 'Playfair Display',
    texto: 'Inter',
    tamanoTexto: 15,
    tamanoTitulo: 24,
    tamanoPequeno: 13,
    tamanoGrande: 18,
    tamanoTituloGrande: 32,
    menuSize: 15,
    menuSpacing: 0,
    menuLineHeight: 2,
    menuOpacity: 1,
    entradaTitleSize: 13,
    entradaSize: 13,
    entradaOpacity: 1,
    navWidth: 320,
    profileWidth: 120,
    profileHeight: 120,
    profileBorder: true,
    sidebarSepHeight: 24,
    redesSize: 13,
    redesGap: 8,
  };

  try {
    const settings = await api.get<Record<string, string>>('/api/settings');
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
    /* Cargar imagen de perfil guardada */
    if (settings.profile_image) {
      profileImage.set(settings.profile_image);
    }
    if (settings.show_entries_on_home !== undefined) {
      siteConfig.update(s => ({ ...s, showEntriesOnHome: settings.show_entries_on_home === 'true' }));
    }
    /* Cargar enlaces sociales guardados */
    if (settings.social_links) {
      try {
        const links = JSON.parse(settings.social_links) as SocialLink[];
        if (Array.isArray(links)) socialLinksStore.set(links);
      } catch { /* JSON invalido, ignorar */ }
    }
    if (settings.redes_layout === 'stacked' || settings.redes_layout === 'inline') {
      redesLayoutStore.set(settings.redes_layout as RedesLayout);
    }
  } catch {
    /* Backend no disponible */
  }

  fontStore.set(config);
}
