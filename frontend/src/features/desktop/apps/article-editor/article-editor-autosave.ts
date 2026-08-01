/* wandori.us — Article Editor Autosave
 * Autosave (borrador) con debounce y create→update idempotente.
 * [297A-14 F5] Paridad: el editor legacy guardaba manualmente; el programa
 * del OS guarda borradores automáticamente sin publicar (editorial sigue
 * siendo independiente y se publica explícitamente).
 *
 * Contrato: recibe un payload y decide crear o actualizar conservando el ID.
 * El teardown cancela timers pendientes; el abort del lifecycle evita I/O. */

import { safeRun } from '../../../../utils/safe-async';
import { showToast } from '../../../../components/ui/toast';
import { ArticleService } from '../../../../services';
import { publishArticleEditorSaved } from '../../../runtime/article-editor-events';
import type { CreateArticleRequest, UpdateArticleRequest } from '../../../../api/types';

/** Payload editorial del borrador (título, extracto, contenido, portada). */
export interface ArticleDraftPayload {
  title: string;
  excerpt: string;
  content: Record<string, unknown>;
  cover_image?: string;
}

interface AutosaveDeps {
  /** Devuelve el ID actual; undefined = aún no creado. */
  getArticleId: () => string | undefined;
  /** Actualizar el ID tras el primer create (idempotencia create→update). */
  setArticleId: (id: string) => void;
  /** Devuelve el payload actual del formulario. */
  getPayload: () => ArticleDraftPayload;
  /** Guarda true si el editor sigue activo (no abortado/desmontado). */
  isActive: () => boolean;
  /** Marcar como sucio cuando el usuario edita. */
  onDirty?: () => void;
}

export interface ArticleAutosave {
  /** Programar guardado tras debounce; se cancela al desmontar. */
  schedule: () => void;
  /** Cancelar el timer pendiente (manual save, close). */
  cancel: () => void;
  /** Destruir timers; idempotente. */
  destroy: () => void;
}

/** Debounce del autosave. Exportada para los tests (evita drift). */
export const AUTOSAVE_DELAY_MS = 2500;

/** Guardar el borrador (crear o actualizar) y publicar el evento de dominio. */
async function saveDraft(
  deps: AutosaveDeps,
): Promise<{ ok: boolean; operation?: 'created' | 'updated' }> {
  if (!deps.isActive()) return { ok: false };
  const payload = deps.getPayload();
  if (!payload.title.trim()) return { ok: false };

  const articleId = deps.getArticleId();
  const base: UpdateArticleRequest = {
    title: payload.title,
    excerpt: payload.excerpt,
    content: payload.content,
    cover_image: payload.cover_image,
  };

  /* Autosave nunca cambia el estado editorial: se conserva draft/private.
   * Solo create/update del contenido; publicar es explícito. */
  const request = articleId
    ? ArticleService.update(articleId, base)
    : ArticleService.create({ ...(base as CreateArticleRequest), status: 'draft' });

  const result = await safeRun(request, 'error al autoguardar');
  if (!deps.isActive() || !result.ok) return { ok: false };

  const operation = articleId ? 'updated' : 'created';
  deps.setArticleId(result.value.id);
  /* [297A-14 F5] El autosave solo anuncia CREATES: el listado del Admin debe
   * ver aparecer artículos nuevos, pero re-renderizar la lista completa en
   * cada guardado debounced (2.5s) es churn innecesario. El 'updated' lo
   * emite el guardado manual explícito. */
  if (operation === 'created') {
    publishArticleEditorSaved({ articleId: result.value.id, operation });
  }
  return { ok: true, operation };
}

/** Crear el autosave del editor de artículos con debounce y teardown. */
export function createArticleAutosave(deps: AutosaveDeps): ArticleAutosave {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let destroyed = false;
  let inFlight = false;
  let dirtyAgain = false;

  const run = async (): Promise<void> => {
    if (destroyed || inFlight) return;
    inFlight = true;
    try {
      const result = await saveDraft(deps);
      /* Si el usuario siguió editando durante el guardado, reprogramar. */
      if (dirtyAgain && !destroyed) {
        dirtyAgain = false;
        timer = setTimeout(() => { void run(); }, AUTOSAVE_DELAY_MS);
      } else if (result.ok && result.operation === 'created') {
        showToast('borrador creado');
      }
    } finally {
      inFlight = false;
    }
  };

  return {
    schedule: () => {
      if (destroyed) return;
      deps.onDirty?.();
      if (inFlight) {
        dirtyAgain = true;
        return;
      }
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        timer = undefined;
        void run();
      }, AUTOSAVE_DELAY_MS);
    },
    /* [297A-14 F5] cancel() no aborta un save ya inFlight: el guardado
     * manual y el autosave escriben el mismo payload (idempotente,
     * last-write-wins); el evento de dominio se emite una vez por save. */
    cancel: () => {
      if (timer) {
        clearTimeout(timer);
        timer = undefined;
      }
      dirtyAgain = false;
    },
    destroy: () => {
      destroyed = true;
      if (timer) {
        clearTimeout(timer);
        timer = undefined;
      }
      dirtyAgain = false;
    },
  };
}
