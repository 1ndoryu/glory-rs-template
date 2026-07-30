import {
  FileText,
  FileUser,
  Folder,
  FolderCode,
  Gamepad2,
  Settings,
  ShieldUser,
} from 'lucide';
import { createDesktopIcon } from './components/desktop-icon';
import { createDesktopMenuBar } from './components/desktop-menu-bar';
import { createDesktopTaskbar } from './components/desktop-taskbar';
import { createDesktopWindow } from './components/desktop-window';
import { createFinderPreview } from './apps/finder/finder-preview';
import { createReaderPreview } from './apps/reader/reader-preview';
import { createFontPanel } from '../settings/font-panel';

export interface DesktopConcept {
  element: HTMLElement;
  profileWindow: HTMLElement;
  contentWindow: HTMLElement;
  navControl: HTMLElement;
}

export interface DesktopConceptOptions {
  showAdminTools: boolean;
  onOpenAdmin: () => void;
}

type DesktopPreviewAction =
  | { type: 'finder' }
  | { type: 'reader'; title: string };

const publicDesktopItems = [
  { id: 'gallery', label: 'Galería', type: 'folder' as const, selected: true, lucideIcon: Folder, action: { type: 'finder' } as const },
  { id: 'projects', label: 'Proyectos', type: 'folder' as const, lucideIcon: FolderCode },
  { id: 'about', label: 'About', type: 'document' as const, lucideIcon: FileUser },
  { id: 'silence', label: 'el silencio…', type: 'document' as const, lucideIcon: FileText, action: { type: 'reader', title: 'El silencio de las máquinas' } as const },
  { id: 'fragments', label: 'fragmentos…', type: 'document' as const, lucideIcon: FileText, action: { type: 'reader', title: 'Fragmentos de código' } as const },
  { id: 'snake', label: 'Snake', type: 'application' as const, lucideIcon: Gamepad2 },
];

function createIconGrid(
  showAdminTools: boolean,
  onOpenGallery: () => void,
  onOpenArticle: (title: string) => void,
  onOpenSettings: () => void,
  onOpenAdmin: () => void,
): HTMLElement {
  const grid = document.createElement('div');
  grid.className = 'desktop-icon-grid';
  grid.setAttribute('aria-label', 'Objetos del escritorio');

  for (const item of publicDesktopItems) {
    const action = 'action' in item
      ? item.action as DesktopPreviewAction
      : undefined;
    const onActivate = action?.type === 'finder'
      ? onOpenGallery
      : action?.type === 'reader'
        ? () => onOpenArticle(action.title)
        : undefined;
    grid.appendChild(createDesktopIcon({
      label: item.label,
      type: item.type,
      selected: 'selected' in item ? item.selected : undefined,
      lucideIcon: item.lucideIcon,
      onActivate,
    }));
  }

  if (showAdminTools) {
    grid.append(
      createDesktopIcon({
        label: 'Configuración',
        type: 'application',
        lucideIcon: Settings,
        onActivate: onOpenSettings,
      }),
      createDesktopIcon({
        label: 'Admin',
        type: 'application',
        lucideIcon: ShieldUser,
        onActivate: onOpenAdmin,
      }),
    );
  }

  return grid;
}

/* [297A-2] Concepto visual con una navegación mínima Finder -> Reader para evaluación.
 * No añade store, gestor de ventanas, rutas ni sincronización de taskbar. */
export function createDesktopConcept(
  profile: HTMLElement,
  content: HTMLElement,
  options: DesktopConceptOptions,
): DesktopConcept {
  const shell = document.createElement('section');
  shell.className = 'desktop-shell';
  shell.setAttribute('aria-label', 'Escritorio');

  const workspace = document.createElement('div');
  workspace.className = 'desktop-workspace';

  const profileWindow = createDesktopWindow({
    title: 'Perfil',
    content: profile,
    className: 'desktop-profile-window',
    active: true,
    resizable: true,
  });

  const contentWindow = createDesktopWindow({
    title: 'Documento',
    content,
    className: 'desktop-content-window',
    resizable: true,
  });
  contentWindow.style.display = 'none';

  let settingsWindow: HTMLElement | null = null;
  let finderWindow: HTMLElement | null = null;
  let readerWindow: HTMLElement | null = null;

  function openArticle(title: string): void {
    readerWindow?.remove();
    readerWindow = createDesktopWindow({
      title,
      content: createReaderPreview({ title }),
      className: 'desktop-reader-window',
      active: true,
      resizable: true,
      onClose: () => {
        readerWindow?.remove();
        readerWindow = null;
      },
    });
    workspace.appendChild(readerWindow);
  }

  function openGallery(): void {
    if (finderWindow?.isConnected) return;

    finderWindow = createDesktopWindow({
      title: 'Galería',
      content: createFinderPreview({
        folderId: 'desktop',
        onOpenApp: (appId: string, params?: Record<string, string>) => {
          if (appId === 'finder' && params?.folderId) {
            /* Navegación entre carpetas: re-crear Finder con nuevo folderId */
            finderWindow?.remove();
            finderWindow = null;
            openGallery();
          }
        },
      }),
      className: 'desktop-finder-window',
      active: true,
      resizable: true,
      onClose: () => {
        finderWindow?.remove();
        finderWindow = null;
      },
    });
    workspace.appendChild(finderWindow);
  }

  function openSettings(): void {
    if (settingsWindow?.isConnected) return;

    settingsWindow = createDesktopWindow({
      title: 'Configuración',
      content: createFontPanel(),
      className: 'desktop-settings-window',
      active: true,
      resizable: true,
      onClose: () => {
        settingsWindow?.remove();
        settingsWindow = null;
      },
    });
    workspace.appendChild(settingsWindow);
  }

  workspace.append(
    createIconGrid(
      options.showAdminTools,
      openGallery,
      openArticle,
      openSettings,
      options.onOpenAdmin,
    ),
    profileWindow,
    contentWindow,
  );
  const taskbar = createDesktopTaskbar([
    { label: 'Perfil', icon: FileUser, state: 'active' },
    { label: 'Configuración', icon: Settings, state: 'minimized' },
  ]);

  shell.append(createDesktopMenuBar(), workspace, taskbar.element);

  return {
    element: shell,
    profileWindow,
    contentWindow,
    navControl: taskbar.navControl,
  };
}
