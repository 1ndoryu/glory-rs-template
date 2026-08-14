/* 138A-8 — Panel de Assets del Constructor: inventario del manifiesto.
 * Recuento por categoría y por asset, quitar instancias, limpiar todo y
 * arrastrar un asset al mundo para colocarlo (el drop lo resuelve la escena
 * con raycast → pickTerrain → addInstance). Solo DOM + contrato puro:
 * `commitObjectEdits` valida cuotas y bounds fail-closed. */

import { createEl } from '../../../../utils/dom';
import {
  assetInstanceCounts,
  categoryInstanceCounts,
  type AssetCategory,
} from '../../../game-core';
import type { ConstructorPanelContext } from './game-world-constructor';

export const ASSET_DRAG_MIME = 'application/x-asset-version';

const CATEGORY_LABELS: Readonly<Record<AssetCategory, string>> = {
  terrain: 'Terreno',
  tree: 'Árboles',
  rock: 'Rocas',
  water: 'Agua',
  character: 'Personajes',
  generic: 'Genéricos',
};

const MAX_INSTANCE_ROWS = 60;

/** Panel Assets: inventario del mundo con drag para colocar y quitar. */
export function buildAssetsPanel(
  container: HTMLElement,
  ctx: ConstructorPanelContext,
): void {
  const status = createEl('p', { className: 'juegoPanelTerreno__statsLine', textContent: '' });
  const list = createEl('div', { className: 'juegoConstructor__assets' });
  container.appendChild(list);
  container.appendChild(status);

  const render = (): void => {
    list.textContent = '';
    const map = ctx.worldMap;
    if (!map) {
      list.appendChild(createEl('p', {
        className: 'juegoPanelTerreno__statsLine',
        textContent: 'Genera o importa un mundo para ver sus assets.',
      }));
      return;
    }
    const byAsset = assetInstanceCounts(map);
    const byCategory = categoryInstanceCounts(map);
    const total = map.instances.length;
    list.appendChild(createEl('p', {
      className: 'juegoPanelTerreno__statsLine',
      textContent: `${total} instancias · ${Object.keys(map.assetManifest).length} assets`,
    }));

    for (const category of Object.keys(CATEGORY_LABELS) as AssetCategory[]) {
      const count = byCategory[category];
      const row = createEl('div', { className: 'juegoConstructor__assetFila' });
      const label = createEl('span', {
        className: 'juegoConstructor__assetNombre',
        textContent: `${CATEGORY_LABELS[category]} · ${count}`,
      });
      row.appendChild(label);
      if (count > 0) {
        const quitar = createEl('button', {
          className: 'juegoPanelTerreno__boton juegoConstructor__assetBoton',
          type: 'button',
          textContent: 'Quitar',
        });
        quitar.addEventListener('click', () => {
          ctx.commitObjectEdits(map.instances
            .filter(instance => map.assetManifest[instance.assetVersionId]?.category === category)
            .map(instance => ({ kind: 'remove' as const, id: instance.id })));
        });
        row.appendChild(quitar);
      }
      list.appendChild(row);
    }

    for (const [assetId, asset] of Object.entries(map.assetManifest)) {
      const count = byAsset[assetId] ?? 0;
      const row = createEl('div', { className: 'juegoConstructor__assetFila' });
      const label = createEl('span', {
        className: 'juegoConstructor__assetNombre',
        textContent: `${assetId} · ${count} · ${asset.category}`,
      });
      row.appendChild(label);
      /* [138A-8] Arrastrar un asset al mundo coloca una instancia nueva
       * (el drop en el host lo resuelve la escena con raycast). */
      row.draggable = true;
      row.title = 'Arrastra al mundo para colocar';
      row.addEventListener('dragstart', (event) => {
        event.dataTransfer?.setData(ASSET_DRAG_MIME, assetId);
        event.dataTransfer!.effectAllowed = 'copy';
      });
      const quitar = createEl('button', {
        className: 'juegoPanelTerreno__boton juegoConstructor__assetBoton',
        type: 'button',
        textContent: count > 0 ? `Quitar ${count}` : '—',
      });
      quitar.disabled = count === 0;
      quitar.addEventListener('click', () => {
        ctx.commitObjectEdits(map.instances
          .filter(instance => instance.assetVersionId === assetId)
          .map(instance => ({ kind: 'remove' as const, id: instance.id })));
      });
      row.appendChild(quitar);
      list.appendChild(row);
    }

    /* Instancias individuales (primeras MAX_INSTANCE_ROWS) para quitar una. */
    const instancias = map.instances.slice(0, MAX_INSTANCE_ROWS);
    for (const instance of instancias) {
      const row = createEl('div', { className: 'juegoConstructor__assetFila' });
      const label = createEl('span', {
        className: 'juegoConstructor__assetNombre',
        title: `${instance.assetVersionId} · x ${instance.position.x} z ${instance.position.z}`,
        textContent: `${instance.id} · ${instance.assetVersionId}`,
      });
      const quitar = createEl('button', {
        className: 'juegoPanelTerreno__boton juegoConstructor__assetBoton',
        type: 'button',
        textContent: 'Quitar',
      });
      quitar.addEventListener('click', () => {
        ctx.commitObjectEdits([{ kind: 'remove', id: instance.id }]);
      });
      row.append(label, quitar);
      list.appendChild(row);
    }
    if (map.instances.length > MAX_INSTANCE_ROWS) {
      list.appendChild(createEl('p', {
        className: 'juegoPanelTerreno__statsLine',
        textContent: `… y ${map.instances.length - MAX_INSTANCE_ROWS} más`,
      }));
    }

    if (total > 0) {
      const limpiar = createEl('button', {
        className: 'juegoPanelTerreno__boton',
        type: 'button',
        textContent: 'Limpiar todo',
      });
      limpiar.addEventListener('click', () => {
        ctx.commitObjectEdits(map.instances.map(instance => ({ kind: 'remove' as const, id: instance.id })));
      });
      list.appendChild(limpiar);
    }
  };

  render();
  ctx.syncMap(render);
}
