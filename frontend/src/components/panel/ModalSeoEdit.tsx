/* [277A-13] Modal de edición SEO para páginas estáticas.
 * Permite editar title, description y og_image_url.
 * Validación en vivo: char count, límites recomendados. */
import React, {useState, useEffect} from 'react';
import {X} from 'lucide-react';
import {useMutation, useQueryClient} from '@tanstack/react-query';
import {apiUpdateSeoSetting, type SeoSetting} from '../../api/admin-seo';
import './ModalSeoEdit.css';

interface Props {
    setting: SeoSetting | null;
    onClose: () => void;
}

const TITLE_MIN = 30;
const TITLE_MAX = 60;
const DESC_MIN = 70;
const DESC_MAX = 160;

export const ModalSeoEdit: React.FC<Props> = ({setting, onClose}) => {
    const queryClient = useQueryClient();
    const [title, setTitle] = useState('');
    const [description, setDescription] = useState('');
    const [ogImageUrl, setOgImageUrl] = useState('');

    useEffect(() => {
        if (setting) {
            setTitle(setting.title);
            setDescription(setting.description);
            setOgImageUrl(setting.og_image_url || '');
        }
    }, [setting]);

    const mutation = useMutation({
        mutationFn: () =>
            apiUpdateSeoSetting(setting!.path, {
                title: title.trim(),
                description: description.trim(),
                og_image_url: ogImageUrl.trim() || null,
            }),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: ['admin-seo-audit']});
            queryClient.invalidateQueries({queryKey: ['admin-seo-settings']});
            onClose();
        },
    });

    if (!setting) return null;

    const titleLen = title.length;
    const descLen = description.length;
    const titleStatus =
        titleLen === 0 ? 'error' : titleLen < TITLE_MIN || titleLen > TITLE_MAX ? 'warning' : 'ok';
    const descStatus =
        descLen === 0 ? 'error' : descLen < DESC_MIN || descLen > DESC_MAX ? 'warning' : 'ok';
    const isValid = titleLen > 0 && descLen > 0;

    return (
        <div className="modalSeoOverlay" onClick={onClose}>
            <div className="modalSeoContenedor" onClick={e => e.stopPropagation()}>
                <div className="modalSeoHeader">
                    <h2 className="modalSeoTitulo">Editar SEO: {setting.label}</h2>
                    <button type="button" className="modalSeoCerrar" onClick={onClose} aria-label="Cerrar">
                        <X size={18} />
                    </button>
                </div>

                <div className="modalSeoCuerpo">
                    <div className="modalSeoCampo">
                        <label className="modalSeoLabel">
                            Título
                            <span className={`modalSeoContador modalSeoContador--${titleStatus}`}>
                                {titleLen}/{TITLE_MAX}
                            </span>
                        </label>
                        <input
                            type="text"
                            className="modalSeoInput"
                            value={title}
                            onChange={e => setTitle(e.target.value)}
                            maxLength={255}
                            placeholder="Título de la página para Google"
                        />
                        <span className="modalSeoAyuda">
                            {titleLen < TITLE_MIN && titleLen > 0 && `⚠ Mínimo recomendado: ${TITLE_MIN} caracteres`}
                            {titleLen > TITLE_MAX && `⚠ Máximo recomendado: ${TITLE_MAX} caracteres`}
                            {titleLen >= TITLE_MIN && titleLen <= TITLE_MAX && '✓ Longitud ideal'}
                        </span>
                    </div>

                    <div className="modalSeoCampo">
                        <label className="modalSeoLabel">
                            Descripción
                            <span className={`modalSeoContador modalSeoContador--${descStatus}`}>
                                {descLen}/{DESC_MAX}
                            </span>
                        </label>
                        <textarea
                            className="modalSeoTextarea"
                            value={description}
                            onChange={e => setDescription(e.target.value)}
                            maxLength={500}
                            rows={3}
                            placeholder="Descripción para los resultados de búsqueda"
                        />
                        <span className="modalSeoAyuda">
                            {descLen < DESC_MIN && descLen > 0 && `⚠ Mínimo recomendado: ${DESC_MIN} caracteres`}
                            {descLen > DESC_MAX && `⚠ Máximo recomendado: ${DESC_MAX} caracteres`}
                            {descLen >= DESC_MIN && descLen <= DESC_MAX && '✓ Longitud ideal'}
                        </span>
                    </div>

                    <div className="modalSeoCampo">
                        <label className="modalSeoLabel">Imagen OG (URL)</label>
                        <input
                            type="text"
                            className="modalSeoInput"
                            value={ogImageUrl}
                            onChange={e => setOgImageUrl(e.target.value)}
                            placeholder="https://nakomi.studio/assets/og-image.jpg"
                        />
                        <span className="modalSeoAyuda">Vacío = imagen por defecto. Recomendado: 1200×630px</span>
                    </div>

                    <div className="modalSeoCampo modalSeoCampo--readonly">
                        <label className="modalSeoLabel">Tipo JSON-LD</label>
                        <input
                            type="text"
                            className="modalSeoInput modalSeoInput--disabled"
                            value={setting.json_ld_type || '—'}
                            disabled
                        />
                        <span className="modalSeoAyuda">Se gestiona automáticamente por el frontend</span>
                    </div>

                    <div className="modalSeoCampo modalSeoCampo--readonly">
                        <label className="modalSeoLabel">Ruta</label>
                        <input
                            type="text"
                            className="modalSeoInput modalSeoInput--disabled"
                            value={setting.path}
                            disabled
                        />
                    </div>
                </div>

                <div className="modalSeoFooter">
                    <button
                        type="button"
                        className="modalSeoBtn modalSeoBtn--cancelar"
                        onClick={onClose}
                    >
                        Cancelar
                    </button>
                    <button
                        type="button"
                        className="modalSeoBtn modalSeoBtn--guardar"
                        onClick={() => mutation.mutate()}
                        disabled={!isValid || mutation.isPending}
                    >
                        {mutation.isPending ? 'Guardando...' : 'Guardar'}
                    </button>
                </div>

                {mutation.isError && (
                    <div className="modalSeoError">
                        Error al guardar: {(mutation.error as Error).message}
                    </div>
                )}
            </div>
        </div>
    );
};
