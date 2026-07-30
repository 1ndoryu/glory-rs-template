import { createElement, PanelLeftOpen, X, type IconNode } from 'lucide';

export type DesktopTaskState = 'active' | 'minimized';

export interface DesktopTask {
  label: string;
  icon: IconNode;
  state: DesktopTaskState;
}

export interface DesktopTaskbar {
  element: HTMLElement;
  navControl: HTMLButtonElement;
}

/* [297A-2] La barra representa el destino visual de las ventanas minimizadas.
 * En fase 1 sus tareas son estados estáticos; el gestor de ventanas las conectará después. */
export function createDesktopTaskbar(tasks: DesktopTask[]): DesktopTaskbar {
  const taskbar = document.createElement('footer');
  taskbar.className = 'desktop-taskbar';
  taskbar.setAttribute('aria-label', 'Ventanas abiertas');

  const navControl = document.createElement('button');
  navControl.type = 'button';
  navControl.className = 'desktop-taskbar__nav-control';
  navControl.disabled = true;
  navControl.setAttribute('aria-label', 'Mostrar navegación; disponible en la fase interactiva');

  const navIcon = createElement(PanelLeftOpen);
  navIcon.classList.add('desktop-taskbar__icon');

  const navLabel = document.createElement('span');
  navLabel.textContent = 'Nav';
  navControl.append(navIcon, navLabel);

  const taskList = document.createElement('div');
  taskList.className = 'desktop-taskbar__tasks';

  for (const task of tasks) {
    const item = document.createElement('div');
    item.className = `desktop-taskbar__task desktop-taskbar__task--${task.state}`;
    item.setAttribute('aria-label', `${task.label}: ${task.state === 'active' ? 'activa' : 'minimizada'}`);

    const icon = createElement(task.icon);
    icon.classList.add('desktop-taskbar__icon');

    const label = document.createElement('span');
    label.className = 'desktop-taskbar__label';
    label.textContent = task.label;

    const closeControl = document.createElement('button');
    closeControl.type = 'button';
    closeControl.className = 'desktop-taskbar__close';
    closeControl.disabled = true;
    closeControl.setAttribute('aria-label', `Cerrar ${task.label}; disponible en la fase interactiva`);
    closeControl.appendChild(createElement(X));

    item.append(icon, label, closeControl);
    taskList.appendChild(item);
  }

  taskbar.append(navControl, taskList);
  return { element: taskbar, navControl };
}
