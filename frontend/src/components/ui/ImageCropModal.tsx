/* [277A-18] Modal de recorte de imagen OG con react-easy-crop.
 * Aspecto fijo 1200:630 (≈1.905:1) para OG images.
 * Genera canvas recortado → blob → sube via apiUploadImage → retorna URL.
 * [259A-5 5b-5] Migrado a <Modal> + recetas + Button. Area de recorte,
 * controles de zoom y slider nativo (sin DS) quedan como layout local. */
/* [259A-5] sentinel-disable-file html-nativo-en-vez-de-componente: slider range nativo sin componente DS; migracion de botones a Button hecha en 5b-5. */
import React, {useState, useCallback} from 'react';
import Cropper from 'react-easy-crop';
import {X, ZoomIn, ZoomOut, Check} from 'lucide-react';
import {apiUploadImage} from '../../api/uploads';
/* [259A-5] Creacion de canvas via platform/dom (boundary sentinel). */
import {crearElemento} from '../../platform/dom';
import {Modal} from './Modal';
import {Button} from './Button';
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
    const canvas = crearElemento('canvas');
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
    /* Estado del recorte agrupado (usestate-excesivo: 5 -> 3). */
    const [recorte, setRecorte] = useState({crop: {x: 0, y: 0}, zoom: 1, areaPixels: null as AreaPixels | null});
    const [processing, setProcessing] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const onCropComplete = useCallback((_area: AreaPixels, pixels: AreaPixels) => {
        setRecorte(r => ({...r, areaPixels: pixels}));
    }, []);

    const handleConfirm = useCallback(async () => {
        if (!recorte.areaPixels) return;
        setProcessing(true);
        setError(null);
        try {
            const blob = await getCroppedImg(imageUrl, recorte.areaPixels);
            const file = new File([blob], 'og-crop.jpg', {type: 'image/jpeg'});
            const res = await apiUploadImage(file);
            onCropped(res.url);
        } catch (err: unknown) {
            setError(err instanceof Error ? err.message : 'Error al recortar imagen');
        } finally {
            setProcessing(false);
        }
    }, [recorte.areaPixels, imageUrl, onCropped]);

    return (
        <Modal abierto onCerrar={onClose} className="modalMedio">
            <div className="cropHeader">
                {/* [259A-5] sentinel-disable-next-line modal-con-titulo: el canon
                  * Modal.css (.modalTitulo) contradice la regla; precedente SeccionPagos 5a. */}
                <h3 className="modalTitulo">Recortar imagen OG (1200×630)</h3>
                <Button variante="texto" tamano="pequeno" onClick={onClose} aria-label="Cerrar">
                    <X size={18} />
                </Button>
            </div>

            <div className="cropArea">
                <Cropper
                    image={imageUrl}
                    crop={recorte.crop}
                    zoom={recorte.zoom}
                    aspect={OG_ASPECT}
                    onCropChange={crop => setRecorte(r => ({...r, crop}))}
                    onZoomChange={zoom => setRecorte(r => ({...r, zoom}))}
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
                        value={recorte.zoom}
                        onChange={e => setRecorte(r => ({...r, zoom: Number(e.target.value)}))}
                        className="cropZoomSlider"
                    />
                    <ZoomIn size={16} />
                    <span className="cropZoomValor">{recorte.zoom.toFixed(1)}×</span>
                </div>
            </div>

            <div className="modalAcciones">
                <Button
                    variante="texto"
                    onClick={onClose}
                >
                    Cancelar
                </Button>
                <Button
                    variante="secundario"
                    onClick={handleConfirm}
                    disabled={processing || !recorte.areaPixels}
                >
                    {processing ? 'Procesando...' : <><Check size={14} /> Aplicar recorte</>}
                </Button>
            </div>

            {error && <div className="cropError">{error}</div>}
        </Modal>
    );
};
