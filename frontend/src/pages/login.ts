/* wandori.us — Login Page
 * Pagina de login minimalista. Solo email + password. */

import { AuthService } from '../services';
import { navigate } from '../router';
import { showToast } from '../components/ui/toast';
import { createInput } from '../components/ui/input';

export function renderLogin(): HTMLElement {
  const page = document.createElement('div');
  page.className = 'login-formulario';

  const titulo = document.createElement('h1');
  titulo.className = 'login-titulo';
  titulo.textContent = 'login';

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

  const btnLogin = document.createElement('button');
  btnLogin.className = 'boton boton-grande';
  btnLogin.textContent = 'entrar';
  btnLogin.style.alignSelf = 'flex-start';

  const errorMsg = document.createElement('p');
  errorMsg.className = 'campo-mensaje-error';
  errorMsg.style.display = 'none';

  btnLogin.addEventListener('click', async () => {
    if (!email || !password) {
      showToast('completa todos los campos');
      return;
    }

    btnLogin.textContent = 'entrando...';
    errorMsg.style.display = 'none';

    try {
      await AuthService.login(email, password);
      showToast('sesion iniciada');
      navigate('/admin');
    } catch {
      errorMsg.textContent = 'credenciales incorrectas';
      errorMsg.style.display = 'block';
      btnLogin.textContent = 'entrar';
    }
  });

  /* Enter para submit */
  page.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') btnLogin.click();
  });

  page.append(titulo, emailCampo, passCampo, errorMsg, btnLogin);
  return page;
}
