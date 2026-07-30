/* wandori.us — Login Page
 * Pagina de login minimalista. Solo email + password.
 * [Auditoría v4 §1.2] Migrado a createEl(). */

import { safeRun, safeClick } from '../utils/safe-async';
import { AuthService } from '../services';
import { navigate } from '../router';
import { showToast } from '../components/ui/toast';
import { createInput } from '../components/ui/input';
import { createEl } from '../utils/dom';

export function renderLogin(): HTMLElement {
  const page = createEl('div', { className: 'login-formulario' });
  const titulo = createEl('h1', { className: 'login-titulo', textContent: 'login' });

  let email = '';
  let password = '';

  const emailCampo = createInput({
    label: 'email',
    type: 'email',
    placeholder: 'email',
    onInput: (v) => { email = v; },
  });

  const passCampo = createInput({
    label: 'password',
    type: 'password',
    placeholder: 'password',
    onInput: (v) => { password = v; },
  });

  const btnLogin = createEl('button', { className: 'boton boton-grande', textContent: 'entrar' });
  btnLogin.style.alignSelf = 'flex-start';
  const errorMsg = createEl('p', { className: 'campo-mensaje-error' });
  errorMsg.style.display = 'none';

  btnLogin.addEventListener('click', safeClick(async () => {
    if (!email || !password) {
      showToast('completa todos los campos');
      return;
    }

    btnLogin.textContent = 'entrando...';
    errorMsg.style.display = 'none';

    const result = await safeRun(AuthService.login(email, password), 'credenciales incorrectas');
    btnLogin.textContent = 'entrar';  /* Restaurar texto siempre */

    if (result.ok) {
      showToast('sesion iniciada');
      navigate('/admin');
    }
  }));

  /* Enter para submit */
  page.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') btnLogin.click();
  });

  page.append(titulo, emailCampo, passCampo, errorMsg, btnLogin);
  return page;
}
