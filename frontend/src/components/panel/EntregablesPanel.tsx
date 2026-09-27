/* [044A-38 Fase 6] Panel de entregables dentro de cada fase.
 * [074A-53] Modal de entrega: notas opcionales + archivos opcionales.
 * [259A-5 5b-6] sentinel-disable-file html-nativo-en-vez-de-componente: solo el
 * input[type=file] oculto queda nativo (sin componente DS); boton quitar migrado a <Button>. */
import {Download, FileText, Package, Paperclip, X} from 'lucide-react';
import {useRef} from 'react';
import {formatFileSize} from '../../api/deliverables';
import {useEntregablesPanel} from '../../hooks/useEntregablesPanel';
import {Button} from '../ui/Button';
import {Modal} from '../ui/Modal';
import {Textarea} from '../ui/Textarea';
import './EntregablesPanel.css';

interface EntregablesPanelProps {
    orderId: string;
    phaseNumber: number;
    canDeliver: boolean;
}

export function EntregablesPanel({orderId, phaseNumber, canDeliver}: EntregablesPanelProps) {
    const {
        error, cargando, entregando,
        handleDeliver, handleDownload, sortedRevisions, revisionGroups,
        modalAbierto, abrirModal, cerrarModal,
        notas, setNotas, archivos, agregarArchivos, quitarArchivo,
    } = useEntregablesPanel(orderId, phaseNumber);
    const fileInputRef = useRef<HTMLInputElement>(null);

    return (
        <div className="entregablesPanel">
            {/* [074A-53] Botón abre modal de entrega */}
            {canDeliver && (
                <div className="entregablesEntregarZona">
                    {error && !modalAbierto && <p className="entregablesError">{error}</p>}
                    <Button
                        variante="primario"
                        tamano="pequeno"
                        className="faseBtn"
                        onClick={abrirModal}
                        type="button"
                    >
                        <Package size={14} /> Entregar
                    </Button>
                </div>
            )}

            {/* [074A-53] Modal de entrega con notas y adjuntos */}
            <Modal abierto={modalAbierto} onCerrar={cerrarModal}>
                <div className="entregablesModal">
                    <p className="entregablesModalDesc">
                        Describe lo que entregas. Adjuntar archivos es opcional.
                    </p>

                    <Textarea
                        className="entregablesModalNotas"
                        placeholder="Notas de la entrega (opcional)"
                        value={notas}
                        onChange={e => setNotas(e.target.value)}
                        rows={4}
                    />

                    <div className="entregablesModalArchivos">
                        <input
                            ref={fileInputRef}
                            type="file"
                            multiple
                            className="entregablesModalFileInput"
                            onChange={e => agregarArchivos(e.target.files)}
                        />
                        <Button
                            variante="secundario"
                            tamano="pequeno"
                            type="button"
                            onClick={() => fileInputRef.current?.click()}
                        >
                            <Paperclip size={14} /> Adjuntar archivos
                        </Button>

                        {archivos.length > 0 && (
                            <ul className="entregablesModalFileList">
                                {archivos.map((f, i) => (
                                    <li key={`${f.name}-${i}`} className="entregablesModalFileItem">
                                        <FileText size={12} />
                                        <span>{f.name}</span>
                                        <span className="entregablesFileSize">{formatFileSize(f.size)}</span>
                                        <Button
                                            type="button"
                                            variante="texto"
                                            className="entregablesModalFileRemove"
                                            onClick={() => quitarArchivo(i)}
                                            aria-label={`Quitar ${f.name}`}
                                        >
                                            <X size={12} />
                                        </Button>
                                    </li>
                                ))}
                            </ul>
                        )}
                    </div>

                    {error && <p className="entregablesError">{error}</p>}

                    <div className="modalAcciones">
                        <Button variante="texto" tamano="pequeno" onClick={cerrarModal} type="button">
                            Cancelar
                        </Button>
                        <Button
                            variante="primario"
                            tamano="pequeno"
                            onClick={handleDeliver}
                            disabled={entregando}
                            type="button"
                        >
                            <Package size={14} /> {entregando ? 'Entregando...' : 'Confirmar entrega'}
                        </Button>
                    </div>
                </div>
            </Modal>

            {/* Historial de entregables agrupado por revisión */}
            {cargando && <p className="entregablesCargando">Cargando entregables...</p>}
            {!cargando && sortedRevisions.length > 0 && (
                <div className="entregablesHistory">
                    {sortedRevisions.map(rev => (
                        <div key={rev} className="entregablesRevision">
                            <span className="entregablesRevisionLabel">Revisión {rev}</span>
                            <ul className="entregablesFileList">
                                {revisionGroups[rev].map(d => (
                                    <li key={d.id} className="entregablesFileItem">
                                        <FileText size={12} />
                                        <span className="entregablesFileName">{d.file_name}</span>
                                        <span className="entregablesFileSize">{formatFileSize(d.file_size_bytes)}</span>
                                        <Button
                                            variante="texto"
                                            className="entregablesDownloadBtn"
                                            onClick={() => handleDownload(d.id, d.file_name)}
                                            type="button"
                                        >
                                            <Download size={12} />
                                        </Button>
                                    </li>
                                ))}
                            </ul>
                            {revisionGroups[rev][0].notes && (
                                <p className="entregablesRevisionNotes">{revisionGroups[rev][0].notes}</p>
                            )}
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
