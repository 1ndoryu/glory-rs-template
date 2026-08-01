import { createEl } from '../../utils/dom';
import { createVacio } from '../../components/ui/empty-state';
import { NotificationsService, type ApiNotification } from '../../services/notifications.service';

export function createNotificationsAdminView(signal?: AbortSignal): { element: HTMLElement; destroy: () => void } {
  const root = createEl('section', { className: 'notificacionesAdmin', ariaLabel: 'Administrar novedades' });
  const title = createEl('h3', { textContent: 'Administrar novedades' });
  const form = createEl('form', { className: 'notificacionesAdmin__form' });
  const kind = createEl('input', { name: 'kind', value: 'manual', ariaLabel: 'Tipo' });
  const noticeTitle = createEl('input', { name: 'title', placeholder: 'Título', ariaLabel: 'Título' });
  const body = createEl('textarea', { name: 'body', placeholder: 'Mensaje', ariaLabel: 'Mensaje' });
  const status = createEl('select', { name: 'status', ariaLabel: 'Estado' });
  for (const value of ['draft', 'published']) status.appendChild(createEl('option', { value, textContent: value }));
  const submit = createEl('button', { type: 'submit', className: 'boton', textContent: 'Crear aviso' });
  const feedback = createEl('p', { className: 'notificacionesAdmin__feedback', role: 'status' });
  form.append(kind, noticeTitle, body, status, submit, feedback);
  const list = createEl('div', { className: 'notificacionesAdmin__lista', role: 'list' });
  root.append(title, form, list);

  let disposed = false;
  async function load(): Promise<void> {
    try {
      const response = await NotificationsService.listAdmin();
      if (disposed || signal?.aborted) return;
      list.replaceChildren(...response.items.map(renderItem));
      if (response.items.length === 0) list.appendChild(createVacio('No hay avisos administrables.'));
    } catch {
      if (!disposed && !signal?.aborted) list.replaceChildren(createVacio('No se pudieron cargar los avisos.'));
    }
  }

  function renderItem(item: ApiNotification): HTMLElement {
    const state = createEl('select', { ariaLabel: `Estado de ${item.title}` });
    for (const value of ['draft', 'published', 'archived']) {
      const option = createEl('option', { value, textContent: value });
      option.selected = item.status === value;
      state.appendChild(option);
    }
    state.addEventListener('change', () => {
      void NotificationsService.updateStatus(item.id, state.value as 'draft' | 'published' | 'archived')
        .catch(() => { state.value = item.status; });
    });
    return createEl('article', { className: 'notificacionesAdmin__item', role: 'listitem' },
      createEl('strong', { textContent: item.title }),
      createEl('p', { textContent: item.body }), state);
  }

  form.addEventListener('submit', event => {
    event.preventDefault();
    if (!kind.value.trim() || !noticeTitle.value.trim() || !body.value.trim()) {
      feedback.textContent = 'Completa tipo, título y mensaje.';
      return;
    }
    submit.disabled = true;
    feedback.textContent = 'Guardando…';
    void NotificationsService.createAdmin({
      kind: kind.value.trim(), title: noticeTitle.value.trim(), body: body.value.trim(),
      status: status.value as 'draft' | 'published',
    }).then(() => {
      feedback.textContent = 'Aviso guardado.';
      noticeTitle.value = '';
      body.value = '';
      void load();
    }).catch(() => {
      feedback.textContent = 'No se pudo guardar el aviso.';
    }).finally(() => { submit.disabled = false; });
  });

  void load();
  return { element: root, destroy: () => { disposed = true; } };
}
