/* wandori.us — Account App View
 * Vista reactiva de Cuenta para el runtime del OS.
 * AuthService es la única frontera HTTP; authStore es la única fuente de
 * verdad de sesión. La vista no crea router ni estado paralelo.
 * [297A-13] */

import { createElement, LogIn, LogOut, ShieldCheck, UserRound, type IconNode } from 'lucide';
import { AuthService } from '../../services';
import { authStore, type AuthState } from '../../store';
import { createInput } from '../../components/ui/input';
import { showToast } from '../../components/ui/toast';
import { safeClick, safeRun } from '../../utils/safe-async';
import { createEl } from '../../utils/dom';
import { createPreferencesPanel } from './preferences-panel';
import type { MountedView, RenderContext } from '../../core/lifecycle';

function icon(iconNode: IconNode): HTMLElement {
  return createEl('span', { className: 'account-app__icon' }, createElement(iconNode));
}

function createActionButton(
  label: string,
  iconNode: IconNode,
  onClick: () => void,
  ariaLabel: string,
): HTMLButtonElement {
  const button = createEl('button', {
    type: 'button',
    className: 'boton account-app__action',
    ariaLabel,
  }, icon(iconNode), createEl('span', { textContent: label }));
  button.addEventListener('click', () => onClick());
  return button;
}

function renderGuest(container: HTMLElement): void {
  const title = createEl('h1', {
    className: 'account-app__title',
    textContent: 'cuenta',
  });
  const message = createEl('p', {
    className: 'account-app__message',
    textContent: 'inicia sesión para sincronizar tu organización y preferencias.',
  });
  const emailField = createInput({
    label: 'email',
    type: 'email',
    placeholder: 'email',
    required: true,
  });
  const passwordField = createInput({
    label: 'password',
    type: 'password',
    placeholder: 'password',
    required: true,
  });
  const emailInput = emailField.querySelector<HTMLInputElement>('input');
  const passwordInput = passwordField.querySelector<HTMLInputElement>('input');
  const error = createEl('p', {
    className: 'campo-mensaje-error account-app__error',
    textContent: '',
  });
  error.hidden = true;
  const submit = createEl('button', {
    type: 'button',
    className: 'boton boton-grande account-app__submit',
    ariaLabel: 'Iniciar sesión',
  }, icon(LogIn), createEl('span', { textContent: 'entrar' }));

  submit.addEventListener('click', safeClick(async () => {
    const email = emailInput?.value.trim() ?? '';
    const password = passwordInput?.value ?? '';
    if (!email || !password) {
      error.textContent = 'completa todos los campos';
      error.hidden = false;
      return;
    }

    submit.disabled = true;
    error.hidden = true;
    const label = submit.querySelector('span:last-child');
    if (label) label.textContent = 'entrando…';
    const result = await safeRun(AuthService.login(email, password), 'credenciales incorrectas');
    submit.disabled = false;
    if (label) label.textContent = 'entrar';
    if (!result.ok) {
      error.textContent = 'no se pudo iniciar sesión';
      error.hidden = false;
      return;
    }
    showToast('sesión iniciada');
  }));

  const form = createEl('div', {
    className: 'account-app__form',
    role: 'form',
    ariaLabel: 'Inicio de sesión',
  }, emailField, passwordField, error, submit);
  form.addEventListener('keydown', (event) => {
    if (event.key === 'Enter') submit.click();
  });

  const unavailable = createEl('p', {
    className: 'account-app__secondary',
    textContent: 'registro cerrado temporalmente.',
  });
  container.append(title, message, form, unavailable);
}

function renderAuthenticated(container: HTMLElement, state: AuthState): void {
  const title = createEl('h1', {
    className: 'account-app__title',
    textContent: 'cuenta',
  });
  const status = createEl('p', {
    className: 'account-app__status',
    role: 'status',
  }, icon(state.capability === 'admin' ? ShieldCheck : UserRound), createEl('span', {
    textContent: state.capability === 'admin' ? 'sesión activa · admin' : 'sesión activa',
  }));
  const message = createEl('p', {
    className: 'account-app__message',
    textContent: 'tus preferencias y organización se sincronizan con esta cuenta.',
  });
  const actions = createEl('div', { className: 'account-app__actions' });
  actions.appendChild(createActionButton('cerrar sesión', LogOut, () => {
    void safeRun(AuthService.logout(), 'no se pudo cerrar sesión');
  }, 'Cerrar sesión'));
  container.append(title, status, message, actions);
}

function renderView(container: HTMLElement, state: AuthState): void {
  container.replaceChildren();
  if (state.isAuthenticated) renderAuthenticated(container, state);
  else renderGuest(container);
}

interface AccountMount {
  readonly element: HTMLElement;
  readonly destroy: () => void;
}

interface PreferencesPanel {
  readonly element: HTMLElement;
  readonly destroy: () => void;
}

function mount(ctx: RenderContext): AccountMount {
  const container = createEl('section', {
    className: 'account-app',
    ariaLabel: 'Cuenta',
  });
  let stopped = false;
  let panel: PreferencesPanel | null = null;

  const render = (state: AuthState): void => {
    if (stopped) return;
    panel?.destroy();
    panel = null;
    renderView(container, state);
    /* [297A-26] Las preferencias (tema + resolución de conflicto) viven dentro
     * de la ventana Cuenta como panel embebido, no como modal global del
     * sistema. El panel siempre muestra la preferencia; si el conflicto sigue
     * pendiente al reabrir la ventana, reaparece. */
    if (state.isAuthenticated) {
      panel = createPreferencesPanel();
      container.append(panel.element);
    }
  };

  const stop = authStore.subscribe(render);
  const cleanup = (): void => {
    if (stopped) return;
    stopped = true;
    stop();
    panel?.destroy();
    panel = null;
  };
  ctx.signal.addEventListener('abort', cleanup, { once: true });
  return { element: container, destroy: cleanup };
}

/** Crear el elemento para el fallback de ruta legacy `/login`. */
export function createAccountView(ctx: RenderContext): HTMLElement {
  return mount(ctx).element;
}

/** Contrato MountedView usado por AppRegistry. */
export function mountAccountView(ctx: RenderContext): MountedView {
  return mount(ctx);
}
