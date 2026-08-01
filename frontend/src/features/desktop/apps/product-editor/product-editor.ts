/* wandori.us — Product Editor App
 * Programa editorial de productos (Fase 3, vertical de comercio admin).
 * Solo devuelve contenido; el shell crea la ventana y el chrome.
 * [297A-14] Nace inactivo/private por defecto; editorial es independiente. */

import { ProductService } from '../../../../services';
import { createInput } from '../../../../components/ui/input';
import { createTextarea } from '../../../../components/ui/textarea';
import { createSelect } from '../../../../components/ui/select';
import { createEl } from '../../../../utils/dom';
import { createVacio } from '../../../../components/ui/empty-state';
import { safeClick, safeRun } from '../../../../utils/safe-async';
import { showToast } from '../../../../components/ui/toast';
import { tryCatch } from '../../../../utils/result';
import { publishProductEditorSaved } from '../../../runtime/product-editor-events';
import type { MountedView, RenderContext } from '../../../../core/lifecycle';
import type { Product } from '../../../../api/types';

function createLoadingView(): HTMLElement {
  return createEl('div', { className: 'product-editor flex-columna gap-lg' },
    createEl('p', { className: 'cargando', textContent: 'cargando editor...' }),
  );
}

async function loadProduct(ctx: RenderContext): Promise<Product | undefined> {
  const productId = ctx.params?.productId;
  if (!productId) return undefined;
  const result = await tryCatch(ProductService.getById(productId, { signal: ctx.signal }));
  if (!result.ok) throw new Error('No se pudo cargar el producto');
  return result.value;
}

/** Renderiza un editor de producto nuevo o existente como vista del OS. */
export function renderProductEditor(ctx: RenderContext): MountedView {
  const container = createLoadingView();
  let disposed = false;
  let currentProductId: string | undefined;

  const isActive = (): boolean => !disposed && !ctx.signal.aborted;

  const hydrate = async (): Promise<void> => {
    try {
      const product = await loadProduct(ctx);
      if (!isActive()) return;

      let name = product?.name || '';
      let description = product?.description || '';
      let priceCents = product?.price_cents ?? 0;
      let currency = product?.currency || 'USD';
      let isActiveState = product?.is_active ?? false;
      currentProductId = product?.id;

      const nameInput = createInput({
        label: 'nombre',
        placeholder: 'nombre del producto',
        value: name,
        onInput: value => { name = value; },
      });
      const descriptionInput = createTextarea({
        label: 'descripcion',
        placeholder: 'descripcion del producto',
        value: description,
        rows: 3,
        onInput: value => { description = value; },
      });
      const priceInput = createInput({
        label: 'precio (centavos)',
        type: 'number',
        value: String(priceCents),
        onInput: value => {
          const parsed = Number.parseInt(value, 10);
          priceCents = Number.isFinite(parsed) && parsed > 0 ? parsed : 0;
        },
      });
      const currencyInput = createInput({
        label: 'moneda (ISO 4217)',
        placeholder: 'USD',
        value: currency,
        onInput: value => { currency = value.toUpperCase(); },
      });
      const activeSelect = createSelect({
        label: 'estado',
        options: [
          { value: 'inactive', label: 'inactivo (oculto)' },
          { value: 'active', label: 'activo (a la venta)' },
        ],
        value: isActiveState ? 'active' : 'inactive',
        onChange: value => { isActiveState = value === 'active'; },
      });
      const saveButton = createEl('button', {
        type: 'button',
        className: 'boton boton-grande',
        textContent: currentProductId ? 'guardar' : 'crear',
      });

      saveButton.addEventListener('click', safeClick(async () => {
        if (!isActive()) return;
        if (!name.trim()) {
          showToast('el nombre es obligatorio');
          return;
        }
        if (priceCents <= 0) {
          showToast('el precio debe ser mayor que cero');
          return;
        }

        const productData = {
          name: name.trim(),
          description,
          price_cents: priceCents,
          currency: currency || 'USD',
          is_active: isActiveState,
        };
        const request = currentProductId
          ? ProductService.update(currentProductId, productData)
          : ProductService.create(productData);
        const result = await safeRun(request, 'error al guardar producto');
        if (!isActive() || !result.ok) return;

        const operation = currentProductId ? 'updated' : 'created';
        currentProductId = result.value.id;
        saveButton.textContent = 'guardar';
        publishProductEditorSaved({ productId: currentProductId, operation });
        showToast(operation === 'updated' ? 'producto actualizado' : 'producto creado');
      }));

      container.textContent = '';
      container.append(
        nameInput,
        descriptionInput,
        priceInput,
        currencyInput,
        activeSelect,
        saveButton,
      );
    } catch {
      if (!isActive()) return;
      container.textContent = '';
      container.appendChild(createVacio('error al cargar el editor de productos'));
    }
  };

  void hydrate();

  const abortHandler = (): void => {
    disposed = true;
  };
  ctx.signal.addEventListener('abort', abortHandler, { once: true });

  return {
    element: container,
    destroy: () => {
      disposed = true;
      ctx.signal.removeEventListener('abort', abortHandler);
    },
  };
}
