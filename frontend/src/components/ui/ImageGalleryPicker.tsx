/* [277A-18] Galería de imágenes para seleccionar OG image.
 * Muestra grid de miniaturas de imágenes subidas.
 * Click selecciona → abre ImageCropModal para recorte.
 * Botón "Subir nueva" → input file → directo al recorte. */
import React, {useState, useCallback, useRef} from 'react';
import {Upload, X} from 'lucide-react';
import {useQuery} from '@tanstack/react-query';
import {apiListUploads, apiUploadImage, type UploadEntry} from '../../api/uploads';
import {ImageCropModal} from './ImageCropModal';
import './ImageGalleryPicker.css';

interface Props {
    onSelect: (url: string) => void;
    onClose: () => void;
}

export const ImageGalleryPicker: React.FC<Props> = ({onSelect, onClose}) => {
    const [cropImage, setCropImage] = useState<string | null>(null);
    const [subiendo, setSubiendo] = useState(false);
    const inputRef = useRef<HTMLInputElement>(null);

    const {data: images, isLoading, error} = useQuery<UploadEntry[]>({
        queryKey: ['admin-uploads'],
        queryFn: apiListUploads,
        staleTime: 2 * 60 * 1000,
    });

    const handlePickFromGallery = (url: string) => {
        /* Las URLs de uploads son relativas (/uploads/content/...), convertir a absoluta si es necesario */
        const fullUrl = url.startsWith('http') ? url : `${window.location.origin}${url}`;
        setCropImage(fullUrl);
    };

    const handleUploadNew = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (!file) return;
        setSubiendo(true);
        try {
            const res = await apiUploadImage(file);
            const fullUrl = res.url.startsWith('http') ? res.url : `${window.location.origin}${res.url}`;
            setCropImage(fullUrl);
        } catch {
            /* error manejado por crop modal */
        } finally {
            setSubiendo(false);
            if (inputRef.current) inputRef.current.value = '';
        }
    }, []);

    const handleCropped = (url: string) => {
        setCropImage(null);
        onSelect(url);
    };

    return (
        <div className="galeriaOverlay" onClick={onClose}>
            <div className="galeriaContenedor" onClick={e => e.stopPropagation()}>
                <div className="galeriaHeader">
                    <h3 className="galeriaTitulo">Seleccionar imagen OG</h3>
                    <button type="button" className="galeriaCerrar" onClick={onClose} aria-label="Cerrar">
                        <X size={18} />
                    </button>
                </div>

                <div className="galeriaCuerpo">
                    {isLoading ? (
                        <div className="galeriaVacio">Cargando imágenes...</div>
                    ) : error ? (
                        <div className="galeriaVacio">Error al cargar imágenes: {(error as Error).message}</div>
                    ) : images && images.length > 0 ? (
                        <div className="galeriaGrid">
                            {images.map(img => (
                                <button
                                    key={img.url}
                                    type="button"
                                    className="galeriaItem"
                                    onClick={() => handlePickFromGallery(img.url)}
                                    title={img.file_name}
                                >
                                    <img
                                        src={img.url}
                                        alt={img.file_name}
                                        loading="lazy"
                                    />
                                </button>
                            ))}
                        </div>
                    ) : (
                        <div className="galeriaVacio">
                            No hay imágenes subidas. Sube la primera.
                        </div>
                    )}
                </div>

                <div className="galeriaFooter">
                    <input
                        ref={inputRef}
                        type="file"
                        accept="image/jpeg,image/png,image/webp"
                        className="galeriaInputFile"
                        onChange={handleUploadNew}
                    />
                    <button
                        type="button"
                        className="galeriaBtn galeriaBtn--subir"
                        onClick={() => inputRef.current?.click()}
                        disabled={subiendo}
                    >
                        <Upload size={14} />
                        {subiendo ? 'Subiendo...' : 'Subir nueva imagen'}
                    </button>
                </div>
            </div>

            {cropImage && (
                <ImageCropModal
                    imageUrl={cropImage}
                    onCropped={handleCropped}
                    onClose={() => setCropImage(null)}
                />
            )}
        </div>
    );
};
