/* [265A-6] Tab de Respaldos del detalle de hosting.
 * Lista, crea, restaura y elimina backups del hosting.
 * Soporta Coolify (SSH a volumen Docker) y Lightweight (coolify-manager-rs). */

import {useState, useCallback} from 'react';
import {useQuery, useMutation, useQueryClient} from '@tanstack/react-query';
import {HardDrive, RotateCcw, Trash2, Plus, AlertTriangle, Loader2, Archive} from 'lucide-react';
import type {useHostingDetalle} from '../../hooks/useHostingDetalle';
import type {HostingBackupEntry} from '../../api/hosting';
import {apiListBackups, apiCreateBackup, apiRestoreBackup, apiDeleteBackup} from '../../api/hosting';
import {Button} from '../ui/Button';

type Subscription = NonNullable<ReturnType<typeof useHostingDetalle>['subscription']>;

function formatFileSize(bytes: number | null): string {
    if (bytes == null) return '—';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function formatDate(dateStr: string | null): string {
    if (!dateStr) return '—';
    try {
        return new Date(dateStr).toLocaleString('es', {
            year: 'numeric', month: 'short', day: 'numeric',
            hour: '2-digit', minute: '2-digit',
        });
    } catch {
        return dateStr;
    }
}

function getTierLabel(tier: string): string {
    const labels: Record<string, string> = {
        daily: 'Diario',
        weekly: 'Semanal',
        monthly: 'Mensual',
        manual: 'Manual',
        automatic: 'Automático',
    };
    return labels[tier] ?? tier;
}

interface BackupActionState {
    type: 'restore' | 'delete' | null;
    fileName: string;
    backupId: string;
}

export function TabBackups({sub}: {sub: Subscription}) {
    const queryClient = useQueryClient();
    const [createLoading, setCreateLoading] = useState(false);
    const [actionState, setActionState] = useState<BackupActionState>({type: null, fileName: '', backupId: ''});
    const [error, setError] = useState<string | null>(null);

    const {data: backupData, isLoading, isError, refetch} = useQuery({
        queryKey: ['hosting-backups', sub.id],
        queryFn: () => apiListBackups(sub.id),
        enabled: !!sub.id,
    });

    const entries: HostingBackupEntry[] = backupData?.data ?? [];

    const createMutation = useMutation({
        mutationFn: () => apiCreateBackup(sub.id, 'manual'),
        onMutate: () => { setCreateLoading(true); setError(null); },
        onSuccess: () => { queryClient.invalidateQueries({queryKey: ['hosting-backups', sub.id]}); },
        onError: (err: Error) => { setError(err.message); },
        onSettled: () => { setCreateLoading(false); },
    });

    const restoreMutation = useMutation({
        mutationFn: (backupId: string) => apiRestoreBackup(sub.id, backupId),
        onMutate: () => { setError(null); },
        onSuccess: (_data, _backupId) => {
            setActionState({type: null, fileName: '', backupId: ''});
            queryClient.invalidateQueries({queryKey: ['hosting-backups', sub.id]});
        },
        onError: (err: Error) => { setError(err.message); setActionState({type: null, fileName: '', backupId: ''}); },
    });

    const deleteMutation = useMutation({
        mutationFn: (fileName: string) => apiDeleteBackup(sub.id, fileName),
        onMutate: () => { setError(null); },
        onSuccess: () => {
            setActionState({type: null, fileName: '', backupId: ''});
            queryClient.invalidateQueries({queryKey: ['hosting-backups', sub.id]});
        },
        onError: (err: Error) => { setError(err.message); setActionState({type: null, fileName: '', backupId: ''}); },
    });

    const handleCreate = useCallback(() => createMutation.mutate(), [createMutation]);
    const handleRestore = useCallback((entry: HostingBackupEntry) => {
        setActionState({type: 'restore', fileName: entry.file_name, backupId: entry.backup_id});
    }, []);
    const handleDelete = useCallback((entry: HostingBackupEntry) => {
        setActionState({type: 'delete', fileName: entry.file_name, backupId: entry.backup_id});
    }, []);
    const confirmAction = useCallback(() => {
        if (actionState.type === 'restore') {
            restoreMutation.mutate(actionState.backupId);
        } else if (actionState.type === 'delete') {
            deleteMutation.mutate(actionState.fileName);
        }
    }, [actionState, restoreMutation, deleteMutation]);
    const cancelAction = useCallback(() => {
        setActionState({type: null, fileName: '', backupId: ''});
        setError(null);
    }, []);

    const isCoolify = backupData?.runtime_kind === 'coolify';

    if (isLoading) {
        return (
            <div className="hostingDetalleSection">
                <h3 className="hostingDetalleSectionTitle">
                    <HardDrive size={18} /> Respaldos
                </h3>
                <div className="tabBackupsEmpty">
                    <Loader2 size={24} className="tabBackupsSpinner" />
                    <p>Cargando respaldos...</p>
                </div>
            </div>
        );
    }

    return (
        <div className="hostingDetalleSection">
            <div className="tabBackupsHeader">
                <h3 className="hostingDetalleSectionTitle hostingDetalleSectionTitle--sinBorde">
                    <HardDrive size={18} /> Respaldos
                </h3>
                <Button
                    onClick={handleCreate}
                    disabled={createLoading}
                    variante="primario"
                    tamano="pequeno"
                >
                    {createLoading ? (
                        <Loader2 size={16} className="tabBackupsSpinner" />
                    ) : (
                        <Plus size={16} />
                    )}
                    Crear backup
                </Button>
            </div>

            <p className="hostingDetalleSectionDesc">
                Backups del hosting. Los backups automáticos se crean según el plan contratado.
                Los backups manuales se pueden crear bajo demanda.
            </p>

            {error && (
                <div className="tabBackupsError">
                    <AlertTriangle size={16} />
                    <span>{error}</span>
                </div>
            )}

            {/* Confirmación de restauración */}
            {actionState.type === 'restore' && (
                <div className="tabBackupsConfirm">
                    <AlertTriangle size={20} className="tabBackupsConfirmIcon" />
                    <div className="tabBackupsConfirmText">
                        <strong>¿Restaurar "{actionState.fileName}"?</strong>
                        <p>La restauración reemplazará los datos actuales del hosting por los del backup seleccionado. Este proceso puede causar downtime. ¿Deseas continuar?</p>
                    </div>
                    <div className="tabBackupsConfirmActions">
                        <Button variante="texto" tamano="pequeno" onClick={cancelAction}>Cancelar</Button>
                        <Button
                            variante="secundario"
                            tamano="pequeno"
                            onClick={confirmAction}
                            disabled={restoreMutation.isPending}
                        >
                            {restoreMutation.isPending ? <Loader2 size={14} className="tabBackupsSpinner" /> : <RotateCcw size={14} />}
                            Restaurar
                        </Button>
                    </div>
                </div>
            )}

            {/* Confirmación de eliminación */}
            {actionState.type === 'delete' && (
                <div className="tabBackupsConfirm">
                    <AlertTriangle size={20} className="tabBackupsConfirmIcon" />
                    <div className="tabBackupsConfirmText">
                        <strong>¿Eliminar "{actionState.fileName}"?</strong>
                        <p>Esta acción es irreversible. El archivo de backup se eliminará definitivamente.</p>
                    </div>
                    <div className="tabBackupsConfirmActions">
                        <Button variante="texto" tamano="pequeno" onClick={cancelAction}>Cancelar</Button>
                        <Button
                            variante="secundario"
                            tamano="pequeno"
                            onClick={confirmAction}
                            disabled={deleteMutation.isPending}
                        >
                            {deleteMutation.isPending ? <Loader2 size={14} className="tabBackupsSpinner" /> : <Trash2 size={14} />}
                            Eliminar
                        </Button>
                    </div>
                </div>
            )}

            {isError ? (
                <div className="tabBackupsEmpty">
                    <AlertTriangle size={32} className="tabBackupsEmptyIcon" />
                    <p>Error al cargar los respaldos</p>
                    <Button variante="texto" tamano="pequeno" onClick={() => refetch()}>Reintentar</Button>
                </div>
            ) : entries.length === 0 ? (
                <div className="tabBackupsEmpty">
                    <Archive size={32} className="tabBackupsEmptyIcon" />
                    <p>No hay respaldos disponibles</p>
                    <p className="hostingDetalleSectionDesc">
                        {isCoolify
                            ? 'Los backups se almacenan en el volumen Docker del hosting. Crea tu primer backup manual.'
                            : 'Crea tu primer backup manual o espera al backup automático programado.'}
                    </p>
                </div>
            ) : (
                <div className="tabBackupsTable">
                    <div className="tabBackupsTableHeader">
                        <span className="tabBackupsColName">Archivo</span>
                        <span className="tabBackupsColTier">Tipo</span>
                        <span className="tabBackupsColSize">Tamaño</span>
                        <span className="tabBackupsColDate">Fecha</span>
                        <span className="tabBackupsColActions">Acciones</span>
                    </div>
                    {entries.map((entry) => (
                        <div key={entry.backup_id} className="tabBackupsRow">
                            <span className="tabBackupsColName">
                                <Archive size={14} className="tabBackupsFileIcon" />
                                <span className="tabBackupsFileName" title={entry.file_name}>{entry.file_name}</span>
                            </span>
                            <span className="tabBackupsColTier">
                                <span className={`tabBackupsTierBadge tabBackupsTier--${entry.tier}`}>
                                    {getTierLabel(entry.tier)}
                                </span>
                            </span>
                            <span className="tabBackupsColSize">{formatFileSize(entry.file_size_bytes)}</span>
                            <span className="tabBackupsColDate">{formatDate(entry.created_at)}</span>
                            <span className="tabBackupsColActions">
                                <Button
                                    variante="texto"
                                    tamano="pequeno"
                                    onClick={() => handleRestore(entry)}
                                    title="Restaurar este backup"
                                >
                                    <RotateCcw size={14} />
                                </Button>
                                <Button
                                    variante="texto"
                                    tamano="pequeno"
                                    onClick={() => handleDelete(entry)}
                                    title="Eliminar este backup"
                                >
                                    <Trash2 size={14} />
                                </Button>
                            </span>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}