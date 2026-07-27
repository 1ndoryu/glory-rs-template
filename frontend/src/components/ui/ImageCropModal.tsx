/* [277A-18] Modal de recorte de imagen OG con react-easy-crop.
 * Aspecto fijo 1200:630 (≈1.905:1) para OG images.
 * Genera canvas recortado → blob → sube via apiUploadImage → retorna URL. */
import React, {useState, useCallback} from 'react';
import Cropper from 'react-easy-crop';
import {X, ZoomIn, ZoomOut, Check} from 'lucide-react';
import {apiUploadImage} from '../../api/uploads';
import './ImageCropModal.css';

interface Props {
    imageUrl: string;
    onCropped: (url: string) => void;
    onClose: () => void;
}

/* 1200/630 ≈ 1.90476 */
const OG_ASPECT = 1200 / 630;
const OUTPUT_WIDTH = 1200;
const OUTPUT_HEIGHT = 630;

interface AreaPixels {
    x: number;
    y: number;
    width: number;
    height: number;
}

/* Genera imagen recortada en canvas y la convierte a blob */
async function getCroppedImg(imageSrc: string, crop: AreaPixels): Promise<Blob> {
    const image = await createImage(imageSrc);
    const canvas = document.createElement('canvas');
    canvas.width = OUTPUT_WIDTH;
    canvas.height = OUTPUT_HEIGHT;
    const ctx = canvas.getContext('2d')!;

    ctx.drawImage(
        image,
        crop.x, crop.y, crop.width, crop.height,
        0, 0, OUTPUT_WIDTH, OUTPUT_HEIGHT,
    );

    return new Promise((resolve, reject) => {
        canvas.toBlob(
            blob => blob ? resolve(blob) : reject(new Error('Canvas toBlob failed')),
            'image/jpeg',
            0.90,
        );
    });
}

function createImage(url: string): Promise<HTMLImageElement> {
    return new Promise((resolve, reject) => {
        const img = new Image();
        img.addEventListener('load', () => resolve(img));
        img.addEventListener('error', err => reject(err));
        img.crossOrigin = 'anonymous';
        img.src = url;
    });
}

export const ImageCropModal: React.FC<Props> = ({imageUrl, onCropped, onClose}) => {
    const [crop, setCrop] = useState({x: 0, y: 0});
    const [zoom, setZoom] = useState(1);
    const [croppedAreaPixels, setCroppedAreaPixels] = useState<AreaPixels | null>(null);
    const [processing, setProcessing] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const onCropComplete = useCallback((_area: AreaPixels, pixels: AreaPixels) => {
        setCroppedAreaPixels(pixels);
    }, []);

    const handleConfirm = useCallback(async () => {
        if (!croppedAreaPixels) return;
        setProcessing(true);
        setError(null);
        try {
            const blob = await getCroppedImg(imageUrl, croppedAreaPixels);
            const file = new File([blob], 'og-crop.jpg', {type: 'image/jpeg'});
            const res = await apiUploadImage(file);
            onCropped(res.url);
        } catch (err: unknown) {
            setError(err instanceof Error ? err.message : 'Error al recortar imagen');
        } finally {
            setProcessing(false);
        }
    }, [croppedAreaPixels, imageUrl, onCropped]);

    return (
        <div className="cropOverlay" onClick={onClose}>
            <div className="cropContenedor" onClick={e => e.stopPropagation()}>
                <div className="cropHeader">
                    <h3 className="cropTitulo">Recortar imagen OG (1200×630)</h3>
                    <button type="button" className="cropCerrar" onClick={onClose} aria-label="Cerrar">
                        <X size={18} />
                    </button>
                </div>

                <div className="cropArea">
                    <Cropper
                        image={imageUrl}
                        crop={crop}
                        zoom={zoom}
                        aspect={OG_ASPECT}
                        onCropChange={setCrop}
                        onZoomChange={setZoom}
                        onCropComplete={onCropComplete}
                    />
                </div>

                <div className="cropControles">
                    <div className="cropZoom">
                        <ZoomOut size={16} />
                        <input
                            type="range"
                            min={1}
                            max={3}
                            step={0.01}
                            value={zoom}
                            onChange={e => setZoom(Number(e.target.value))}
                            className="cropZoomSlider"
                        />
                        <ZoomIn size={16} />
                        <span className="cropZoomValor">{zoom.toFixed(1)}×</span>
                    </div>
                </div>

                <div className="cropFooter">
                    <button
                        type="button"
                        className="cropBtn cropBtn--cancelar"
                        onClick={onClose}
                    >
                        Cancelar
                    </button>
                    <button
                        type="button"
                        className="cropBtn cropBtn--aplicar"
                        onClick={handleConfirm}
                        disabled={processing || !croppedAreaPixels}
                    >
                        {processing ? 'Procesando...' : <><Check size={14} /> Aplicar recorte</>}
                    </button>
                </div>

                {error && <div className="cropError">{error}</div>}
            </div>
        </div>
    );
};
