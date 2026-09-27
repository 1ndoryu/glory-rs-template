/* [311A-1][311A-INV] Sección admin con dos sub-vistas:
 * - "Enviados": trazabilidad de correos enviados (logs) con filtro y paginación.
 * - "Vista previa": galería de plantillas renderizadas con datos de muestra.
 * Sigue el patrón visual de SeccionReembolsos. */

/* [259A-5 5b-6] sentinel-disable-file html-nativo-en-vez-de-componente: solo las pestanias usan <button> nativo (sin componente Tabs en el DS); paginacion ya en <Button>. */
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Loader2, AlertCircle, Mail, ChevronLeft, ChevronRight, Filter, Eye } from 'lucide-react';
import { apiListEmailLogs, TEMPLATE_OPTIONS, type EmailLogItem } from '../../api/admin-email';
import { VistaPreviaCorreos } from './VistaPreviaCorreos';
import { Button } from '../ui/Button';
import {SelectDropdown} from '../ui/SelectDropdown';
import './SeccionCorreo.css';

type Pestaña = 'enviados' | 'preview';

const PAGE_SIZE = 50;

function PestañaEnviados() {
    const [filtroTemplate, setFiltroTemplate] = useState<string>('');
    const [offset, setOffset] = useState(0);

    const { data, isLoading, error } = useQuery({
        queryKey: ['admin-email-logs', filtroTemplate, offset],
        queryFn: () => apiListEmailLogs({
            template: filtroTemplate || undefined,
            limit: PAGE_SIZE,
            offset,
        }),
    });

    const logs = data?.logs ?? [];
    const total = data?.total ?? 0;
    const paginaActual = Math.floor(offset / PAGE_SIZE) + 1;
    const totalPaginas = Math.max(1, Math.ceil(total / PAGE_SIZE));

    const handleFilterChange = (value: string) => {
        setFiltroTemplate(value);
        setOffset(0);
    };

    if (isLoading) {
        return (
            <div className="correosVacio">
                <Loader2 className="correosSpinner" size={32} />
            </div>
        );
    }

    if (error) {
        return (
            <div className="correosError">
                <AlertCircle size={20} />
                <span>Error al cargar correos: {(error as Error).message}</span>
            </div>
        );
    }

    return (
        <>
            {/* Filtro por plantilla */}
            <div className="correosFiltro">
                <Filter size={18} />
                <SelectDropdown
                    value={filtroTemplate}
                    opciones={[{value: '', label: 'Todas las plantillas'}, ...TEMPLATE_OPTIONS]}
                    onChange={handleFilterChange}
                    ariaLabel="Filtrar por plantilla"
                />
                <span className="correosTotal">{total} correos</span>
            </div>

            {/* Tabla */}
            {logs.length === 0 ? (
                <div className="correosVacio">
                    <Mail size={40} />
                    <p>No hay correos enviados</p>
                </div>
            ) : (
                <div className="correosTablaWrapper">
                    <table className="correosTabla">
                        <thead>
                            <tr>
                                <th>Plantilla</th>
                                <th>Destinatario</th>
                                <th>Asunto</th>
                                <th>Estado</th>
                                <th>Fecha</th>
                            </tr>
                        </thead>
                        <tbody>
                            {logs.map((log: EmailLogItem) => (
                                <tr key={log.id} className="correosFila">
                                    <td>
                                        <span className="correosTag">{log.template_label}</span>
                                    </td>
                                    <td className="correosDestinatario">{log.to_email}</td>
                                    <td className="correosAsunto">{log.subject}</td>
                                    <td>
                                        <span className={`correosBadge ${log.status_color}`}>
                                            {log.status === 'sent' ? 'Enviado' : log.status === 'failed' ? 'Fallido' : log.status}
                                        </span>
                                    </td>
                                    <td className="correosFecha">
                                        {new Date(log.created_at).toLocaleDateString('es-ES', {
                                            year: 'numeric',
                                            month: 'short',
                                            day: 'numeric',
                                            hour: '2-digit',
                                            minute: '2-digit',
                                        })}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            {/* Paginación */}
            {total > PAGE_SIZE && (
                <div className="correosPaginacion">
                    <Button
                        variante="texto"
                        tamano="pequeno"
                        type="button"
                        disabled={offset === 0}
                        onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
                    >
                        <ChevronLeft size={16} /> Anterior
                    </Button>
                    <span className="correosPagInfo">
                        Página {paginaActual} de {totalPaginas} ({(paginaActual - 1) * PAGE_SIZE + 1}–{Math.min(paginaActual * PAGE_SIZE, total)} de {total})
                    </span>
                    <Button
                        variante="texto"
                        tamano="pequeno"
                        type="button"
                        disabled={offset + PAGE_SIZE >= total}
                        onClick={() => setOffset(offset + PAGE_SIZE)}
                    >
                        Siguiente <ChevronRight size={16} />
                    </Button>
                </div>
            )}
        </>
    );
}

export function SeccionCorreo() {
    const [pestaña, setPestaña] = useState<Pestaña>('preview');

    return (
        <div className="correosContenedor">
            {/* Pestañas */}
            <div className="correosPestanias">
                <button
                    className={`correosPestaniaBtn ${pestaña === 'preview' ? 'correosPestaniaActiva' : ''}`}
                    onClick={() => setPestaña('preview')}
                >
                    <Eye size={16} />
                    Vista previa
                </button>
                <button
                    className={`correosPestaniaBtn ${pestaña === 'enviados' ? 'correosPestaniaActiva' : ''}`}
                    onClick={() => setPestaña('enviados')}
                >
                    <Mail size={16} />
                    Enviados
                </button>
            </div>

            {/* Contenido según pestaña */}
            {pestaña === 'preview' ? <VistaPreviaCorreos /> : <PestañaEnviados />}
        </div>
    );
}
