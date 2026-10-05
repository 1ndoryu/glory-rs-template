/* [277A-18] Galería de imágenes para seleccionar OG image.
 * Muestra grid de miniaturas de imágenes subidas.
 * Click selecciona → abre ImageCropModal para recorte.
 * Botón "Subir nueva" → input file → directo al recorte.
 * [259A-5 5b-5] Overlay/contenedor/header/footer a <Modal> + recetas + Button.
 * Miniaturas (button+img) y file input nativos: sin componente DS (ver 5b-6). */
/* [259A-5] sentinel-disable-file html-nativo-en-vez-de-componente componente-artesanal: miniaturas button+img y file input sin DS; overlay a <Modal> migrado en 5b-5, resto en 5b-6 con verificacion en navegador. */
import React, {useState, useCallback, useRef} from 'react';
import {Upload, X} from 'lucide-react';
import {useQuery} from '@tanstack/react-query';
import {apiListUploads, apiUploadImage, type UploadEntry} from '../../api/uploads';
import {ImageCropModal} from './ImageCropModal';
/* [259A-5] Acceso window via platform/navigation (boundary sentinel). */
import {obtenerOrigen} from '../../platform/navigation';
import {Modal} from './Modal';
import {Button} from './Button';
import './Thumbnail.css';
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
        const fullUrl = url.startsWith('http') ? url : `${obtenerOrigen()}${url}`;
        setCropImage(fullUrl);
    };

    const handleUploadNew = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (!file) return;
        setSubiendo(true);
        try {
            const res = await apiUploadImage(file);
            const fullUrl = res.url.startsWith('http') ? res.url : `${obtenerOrigen()}${res.url}`;
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
        <>
            <Modal abierto onCerrar={onClose} className="modalMedio">
                <div className="galeriaHeader">
                    {/* [259A-5] sentinel-disable-next-line modal-con-titulo: el canon
                      * Modal.css (.modalTitulo) contradice la regla; precedente SeccionPagos 5a. */}
                    <h3 className="modalTitulo">Seleccionar imagen OG</h3>
                    <Button variante="texto" tamano="pequeno" onClick={onClose} aria-label="Cerrar">
                        <X size={18} />
                    </Button>
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
                                    className="miniaturaClicable galeriaItem"
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

                <div className="modalAcciones">
                    <input
                        ref={inputRef}
                        type="file"
                        accept="image/jpeg,image/png,image/webp"
                        className="galeriaInputFile"
                        onChange={handleUploadNew}
                    />
                    <Button
                        variante="secundario"
                        onClick={() => inputRef.current?.click()}
                        disabled={subiendo}
                    >
                        <Upload size={14} />
                        {subiendo ? 'Subiendo...' : 'Subir nueva imagen'}
                    </Button>
                </div>
            </Modal>

            {cropImage && (
                <ImageCropModal
                    imageUrl={cropImage}
                    onCropped={handleCropped}
                    onClose={() => setCropImage(null)}
                />
            )}
        </>
    );
};
