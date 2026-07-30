import { ChevronRight, createElement } from 'lucide';

const julyArticles = [
  'El silencio de las máquinas',
  'Fragmentos de código y otras nostalgias',
  'Sobre diseño y otros actos de fe',
];

function createMenuLabel(label: string, expanded?: boolean): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'desktop-menu-bar__item';
  button.disabled = true;
  button.textContent = label;
  button.setAttribute('aria-haspopup', 'menu');
  if (expanded !== undefined) button.setAttribute('aria-expanded', String(expanded));
  return button;
}

function createArchiveMenu(): HTMLElement {
  const menu = document.createElement('div');
  menu.className = 'desktop-context-menu';
  menu.setAttribute('role', 'menu');
  menu.setAttribute('aria-label', 'Archivo por mes');

  for (const [index, month] of ['Julio 2026', 'Junio 2026', 'Mayo 2026'].entries()) {
    const monthItem = document.createElement('div');
    monthItem.className = 'desktop-context-menu__branch';

    const monthLabel = document.createElement('div');
    monthLabel.className = index === 0
      ? 'desktop-context-menu__item desktop-context-menu__item--selected'
      : 'desktop-context-menu__item';
    monthLabel.setAttribute('role', 'menuitem');
    monthLabel.setAttribute('aria-haspopup', 'menu');
    monthLabel.setAttribute('aria-expanded', String(index === 0));

    const text = document.createElement('span');
    text.textContent = month;
    const chevron = createElement(ChevronRight);
    chevron.classList.add('desktop-context-menu__chevron');
    monthLabel.append(text, chevron);
    monthItem.appendChild(monthLabel);

    if (index === 0) {
      const submenu = document.createElement('div');
      submenu.className = 'desktop-context-menu desktop-context-menu--nested';
      submenu.setAttribute('role', 'menu');
      submenu.setAttribute('aria-label', `Artículos de ${month}`);

      for (const article of julyArticles) {
        const articleItem = document.createElement('div');
        articleItem.className = 'desktop-context-menu__item';
        articleItem.setAttribute('role', 'menuitem');
        articleItem.textContent = article;
        submenu.appendChild(articleItem);
      }

      monthItem.appendChild(submenu);
    }

    menu.appendChild(monthItem);
  }

  return menu;
}

/* [297A-2] Muestra abierta la jerarquía Archivo > mes > artículos para validar
 * su densidad. Aplicaciones y Configuración se conectarán al registro en fase 2. */
export function createDesktopMenuBar(): HTMLElement {
  const bar = document.createElement('header');
  bar.className = 'desktop-menu-bar';

  const menus = document.createElement('div');
  menus.className = 'desktop-menu-bar__menus';

  const brand = document.createElement('span');
  brand.className = 'desktop-menu-bar__brand';
  brand.setAttribute('aria-label', 'Menú del sistema');

  const archiveEntry = document.createElement('div');
  archiveEntry.className = 'desktop-menu-bar__entry';
  const archiveMenu = createArchiveMenu();
  archiveMenu.hidden = true;
  archiveEntry.append(createMenuLabel('Archivo', false), archiveMenu);

  const applicationsEntry = document.createElement('div');
  applicationsEntry.className = 'desktop-menu-bar__entry';
  applicationsEntry.appendChild(createMenuLabel('Aplicaciones', false));

  const settingsEntry = document.createElement('div');
  settingsEntry.className = 'desktop-menu-bar__entry';
  settingsEntry.appendChild(createMenuLabel('Configuración', false));

  menus.append(brand, archiveEntry, applicationsEntry, settingsEntry);

  const clock = document.createElement('time');
  clock.className = 'desktop-menu-bar__clock';
  clock.textContent = '11:42';

  bar.append(menus, clock);
  return bar;
}
