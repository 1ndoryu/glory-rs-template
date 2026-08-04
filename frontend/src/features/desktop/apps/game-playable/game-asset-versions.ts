/* GAME-01 — Panel de versiones de Assets 3D (Assets 3D, 297A-73).
 * Lista las versiones GLB de un asset, importa una nueva (multipart), edita
 * metadata allowlisted (proxy/scale) de las inactivas, activa una versión y
 * muestra el preview 3D aislado del binario servido por el backend. SRP:
 * este módulo es la UI; el análisis del modelo vive en game-asset-preview. */

import { tryCatch } from '../../../../utils/result';
import { createEl } from '../../../../utils/dom';
import { createVacio } from '../../../../components/ui/empty-state';
import { createModal } from '../../../../components/ui/modal';
import { createInput } from '../../../../components/ui/input';
import { createSelect } from '../../../../components/ui/select';
import { showToast } from '../../../../components/ui/toast';
import { showConfirm } from '../../../../components/ui/confirm';
import {
  GameAssetAdminService,
  GAME_ASSET_GLB_MAX_BYTES,
  type GameAssetAdminEntry,
  type GameAssetVersionAdminEntry,
} from '../../../../services/game-asset-admin.service';
import { createGameAssetPreview, type GameAssetPreviewSummary } from './game-asset-preview';

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MiB`;
}

function formatFecha(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '';
  return `${date.getDate()}/${date.getMonth() + 1}/${date.getFullYear()}`;
}

function formatSummary(summary: GameAssetPreviewSummary): string {
  const parts = [
    `${summary.nodes} nodos`,
    `${summary.meshes} mallas`,
    `${summary.triangles.toLocaleString()} triángulos`,
    `${summary.materials} materiales`,
  ];
  if (summary.animations > 0) parts.push(`${summary.animations} animaciones`);
  if (summary.hasTextures) parts.push('con texturas');
  return parts.join(' · ');
}

/** Abre el panel de versiones de un asset (modal del OS). */
export function openAssetVersionsPanel(entry: GameAssetAdminEntry, onChanged: () => void): void {
  const lista = createEl('div', { className: 'admin-lista' });
  const hint = createEl('p', { className: 'modal-feedback', role: 'status' });

  const btnCerrar = createEl('button', { type: 'button', className: 'boton', textContent: 'cerrar' });
  const fileInput = createEl('input', {
    type: 'file',
    accept: '.glb,model/gltf-binary',
    className: 'file-input',
  });
  const btnImportar = createEl('button', {
    type: 'button',
    className: 'boton',
    textContent: '+ importar GLB',
  });
  const importarAccion = createEl('div', { className: 'admin-acciones' }, btnImportar, fileInput);

  const modal = createModal({
    titulo: `versiones de "${entry.displayName}"`,
    contenido: [hint, importarAccion, lista],
    ancho: '760px',
  });

  let generation = 0;

  const render = async (): Promise<void> => {
    const current = ++generation;
    lista.textContent = '';
    lista.appendChild(createEl('p', { className: 'cargando', textContent: 'cargando...' }));
    const result = await tryCatch(GameAssetAdminService.listVersions(entry.id));
    if (generation !== current) return;
    lista.textContent = '';
    if (!result.ok) {
      lista.appendChild(createVacio('error al cargar las versiones'));
      return;
    }
    if (result.value.length === 0) {
      lista.appendChild(createVacio('sin versiones todavía: importa un GLB'));
    }
    for (const version of result.value) {
      lista.appendChild(renderVersionItem(version, entry, () => {
        void render();
        onChanged();
      }));
    }
  };

  btnImportar.addEventListener('click', () => fileInput.click());
  fileInput.addEventListener('change', () => {
    const file = fileInput.files?.[0];
    if (!file) return;
    if (file.size > GAME_ASSET_GLB_MAX_BYTES) {
      hint.textContent = 'el GLB supera el tamaño máximo (16 MiB).';
      return;
    }
    btnImportar.disabled = true;
    hint.textContent = 'importando...';
    void tryCatch(GameAssetAdminService.importVersion(entry.id, file)).then((result) => {
      btnImportar.disabled = false;
      fileInput.value = '';
      if (!result.ok) {
        hint.textContent = 'no se pudo importar el GLB (¿archivo inválido?).';
        return;
      }
      hint.textContent = 'GLB importado como v' + String(result.value.version) + '.';
      showToast('versión importada');
      void render();
      onChanged();
    });
  });

  btnCerrar.addEventListener('click', () => modal.close());

  void render();

  /* El modal se cierra solo (backdrop/Escape); sin teardown extra porque la
   * vista de preview crea su propio handle y lo destruye al cerrar. */
}

function renderVersionItem(
  version: GameAssetVersionAdminEntry,
  entry: GameAssetAdminEntry,
  onChanged: () => void,
): HTMLElement {
  const tag = createEl('span', {
    className: 'tag-estado',
    textContent: version.isActive ? 'activa' : 'inactiva',
  });
  const info = createEl('div', { className: 'admin-item-info' },
    createEl('span', { textContent: `v${version.version} · ${formatBytes(version.byteSize)} · ${version.kind}` }),
    createEl('small', { className: 'ml-sm', textContent: formatFecha(version.createdAt) }),
  );

  const btnPreview = createEl('button', {
    type: 'button',
    className: 'boton boton-pequeno',
    textContent: 'preview 3D',
  });
  btnPreview.addEventListener('click', () => {
    openVersionPreview(version, entry);
  });

  const btnMetadata = createEl('button', {
    type: 'button',
    className: 'boton boton-pequeno',
    textContent: 'metadata',
  });
  btnMetadata.disabled = version.isActive;
  btnMetadata.addEventListener('click', () => {
    openVersionMetadataModal(version, onChanged);
  });

  const btnActivar = createEl('button', {
    type: 'button',
    className: 'boton boton-pequeno',
    textContent: 'activar',
  });
  btnActivar.disabled = version.isActive;
  btnActivar.addEventListener('click', () => {
    void (async () => {
      const confirmed = await showConfirm(`activar la versión v${version.version}? La anterior queda inactiva e inmutable.`);
      if (!confirmed) return;
      const result = await tryCatch(GameAssetAdminService.activateVersion(entry.id, version.version));
      if (!result.ok) {
        showToast('no se pudo activar la versión');
        return;
      }
      showToast('versión activada');
      onChanged();
    })();
  });

  const actions = createEl('div', { className: 'admin-acciones' }, tag, btnPreview, btnMetadata, btnActivar);
  return createEl('div', { className: 'admin-item' }, info, actions);
}

/** Preview 3D aislado de una versión: descarga el GLB y lo muestra. */
function openVersionPreview(
  version: GameAssetVersionAdminEntry,
  entry: GameAssetAdminEntry,
): void {
  const host = createEl('div', { className: 'asset-preview-host' });
  const resumen = createEl('p', { className: 'modal-feedback', role: 'status', textContent: 'cargando GLB...' });
  const btnCerrar = createEl('button', { type: 'button', className: 'boton', textContent: 'cerrar' });

  let previewDestroyed = false;
  const modal = createModal({
    titulo: `preview 3D · v${version.version} · ${entry.displayName}`,
    contenido: [host, resumen, btnCerrar],
    ancho: '720px',
    /* [297A-73] Cerrar por backdrop/Escape también destruye el handle WebGL. */
    onClose: () => {
      if (previewDestroyed) return;
      previewDestroyed = true;
      controller.abort();
      preview.destroy();
    },
  });

  const preview = createGameAssetPreview(host);
  const controller = new AbortController();
  const closePreview = (): void => modal.close();

  void (async () => {
    const blobResult = await tryCatch(
      GameAssetAdminService.readVersionFile(entry.id, version.version, { signal: controller.signal }),
    );
    if (controller.signal.aborted) return;
    if (!blobResult.ok) {
      resumen.textContent = 'no se pudo leer el GLB de esta versión.';
      return;
    }
    const loadResult = await tryCatch(preview.load(blobResult.value));
    if (controller.signal.aborted) return;
    if (!loadResult.ok) {
      resumen.textContent = 'no se pudo interpretar el GLB (¿archivo corrupto?).';
      return;
    }
    resumen.textContent = formatSummary(loadResult.value);
  })();

  btnCerrar.addEventListener('click', closePreview);
}

/** Modal de metadata de una versión NO activa (proxy/scale allowlisted). */
function openVersionMetadataModal(
  version: GameAssetVersionAdminEntry,
  onSaved: () => void,
): void {
  let kind: 'circle' | 'aabb' = version.proxy?.kind ?? 'circle';
  let radius = String(version.proxy?.kind === 'circle' ? (version.proxy.radius ?? 0.5) : 0.5);
  let halfWidth = String(version.proxy?.kind === 'aabb' ? (version.proxy.halfWidth ?? 1) : 1);
  let halfDepth = String(version.proxy?.kind === 'aabb' ? (version.proxy.halfDepth ?? 1) : 1);
  let scale = String(version.scale);
  let sinProxy = version.proxy === null;

  const sinProxyField = createSelect({
    label: 'proxy de colisión',
    options: [
      { value: 'false', label: 'con proxy' },
      { value: 'true', label: 'sin proxy' },
    ],
    value: String(sinProxy),
    onChange: (v) => { sinProxy = v === 'true'; refreshProxyFields(); },
  });
  const kindField = createSelect({
    label: 'tipo de proxy',
    options: [
      { value: 'circle', label: 'circle' },
      { value: 'aabb', label: 'aabb' },
    ],
    value: kind,
    onChange: (v) => { kind = v as 'circle' | 'aabb'; refreshProxyFields(); },
  });
  const radiusField = createInput({
    label: 'radio (circle)',
    value: radius,
    onInput: (v) => { radius = v; },
  });
  const halfWidthField = createInput({
    label: 'halfWidth (aabb)',
    value: halfWidth,
    onInput: (v) => { halfWidth = v; },
  });
  const halfDepthField = createInput({
    label: 'halfDepth (aabb)',
    value: halfDepth,
    onInput: (v) => { halfDepth = v; },
  });
  const scaleField = createInput({
    label: 'escala (0.1 – 4)',
    value: scale,
    onInput: (v) => { scale = v; },
  });
  const feedback = createEl('p', { className: 'modal-feedback', role: 'status' });

  const refreshProxyFields = (): void => {
    radiusField.hidden = sinProxy || kind !== 'circle';
    halfWidthField.hidden = sinProxy || kind !== 'aabb';
    halfDepthField.hidden = sinProxy || kind !== 'aabb';
    kindField.hidden = sinProxy;
  };
  refreshProxyFields();

  const btnCancelar = createEl('button', { type: 'button', className: 'boton', textContent: 'cancelar' });
  const btnGuardar = createEl('button', { type: 'button', className: 'boton', textContent: 'guardar' });
  const acciones = createEl('div', { className: 'modal-acciones' }, btnCancelar, btnGuardar);

  const modal = createModal({
    titulo: `metadata · v${version.version}`,
    contenido: [sinProxyField, kindField, radiusField, halfWidthField, halfDepthField, scaleField, feedback, acciones],
    ancho: '440px',
  });

  btnCancelar.addEventListener('click', () => modal.close());
  btnGuardar.addEventListener('click', () => {
    const scaleNumber = Number(scale);
    if (!Number.isFinite(scaleNumber) || scaleNumber < 0.1 || scaleNumber > 4) {
      feedback.textContent = 'escala fuera de rango (0.1 – 4).';
      return;
    }
    let proxy: GameAssetVersionAdminEntry['proxy'] = null;
    if (!sinProxy) {
      if (kind === 'circle') {
        const r = Number(radius);
        if (!Number.isFinite(r) || r <= 0) {
          feedback.textContent = 'radio inválido.';
          return;
        }
        proxy = { kind: 'circle', radius: r };
      } else {
        const w = Number(halfWidth);
        const d = Number(halfDepth);
        if (!Number.isFinite(w) || !Number.isFinite(d) || w <= 0 || d <= 0) {
          feedback.textContent = 'halfWidth/halfDepth inválidos.';
          return;
        }
        proxy = { kind: 'aabb', halfWidth: w, halfDepth: d };
      }
    }
    btnGuardar.disabled = true;
    feedback.textContent = 'guardando...';
    void tryCatch(
      GameAssetAdminService.updateVersionMetadata(version.assetId, version.version, {
        proxy,
        scale: scaleNumber,
      }),
    ).then((result) => {
      btnGuardar.disabled = false;
      if (!result.ok) {
        feedback.textContent = 'no se pudo guardar la metadata (¿versión ya activa?).';
        return;
      }
      showToast('metadata guardada');
      modal.close();
      onSaved();
    });
  });
}
