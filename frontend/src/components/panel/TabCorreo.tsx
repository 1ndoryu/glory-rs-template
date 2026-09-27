/* [265A-11] Tab de Correo del detalle de hosting.
 * Opcion A: Aliases/reenvios via Cloudflare Email Routing (activo).
 * Opcion B: Buzones IMAP preparados, NO activos aun (se activara cuando el usuario lo indique).
 * Muestra aliases creados, permite crear/eliminar, e indica estado del plan. */

/* [259A-5 5b-6] sentinel-disable-file html-nativo-en-vez-de-componente: inputs de texto del form de alias usan <input> nativo con clases locales (fuera de alcance 5b-6: solo botones/selects). */
import {useState, useCallback} from 'react';
import {useQuery, useMutation, useQueryClient} from '@tanstack/react-query';
import {
    Mail, Plus, Trash2, Loader2, AlertTriangle,
} from 'lucide-react';
import type {useHostingDetalle} from '../../hooks/useHostingDetalle';
import type {EmailAliasInfo} from '../../api/hosting';
import {
    apiGetHostingEmailInfo,
    apiCreateEmailAlias,
    apiDeleteEmailAlias,
} from '../../api/hosting';
import {Button} from '../ui/Button';

type Subscription = NonNullable<ReturnType<typeof useHostingDetalle>['subscription']>;

function getPlanAliasLimit(plan: string): number {
    const limits: Record<string, number> = {
        basico: 0,
        pro: 3,
        ecommerce: 5,
        'normal-basico': 0,
        'normal-pro': 3,
        'normal-ecommerce': 5,
    };
    return limits[plan] ?? 0;
}

export function TabCorreo({sub}: {sub: Subscription}) {
    const queryClient = useQueryClient();
    const [error, setError] = useState<string | null>(null);
    const [alias, setAlias] = useState('');
    const [domain, setDomain] = useState(sub.domain || '');
    const [destination, setDestination] = useState(sub.client_email || '');
    const [showForm, setShowForm] = useState(false);

    const aliasLimit = getPlanAliasLimit(sub.plan);

    const {data: emailInfo, isLoading} = useQuery({
        queryKey: ['hosting-email', sub.id],
        queryFn: () => apiGetHostingEmailInfo(sub.id),
        enabled: !!sub.id && aliasLimit > 0,
    });

    const createMutation = useMutation({
        mutationFn: (req: {alias: string; domain: string; destination: string}) =>
            apiCreateEmailAlias(sub.id, req),
        onMutate: () => { setError(null); },
        onSuccess: () => {
            setAlias('');
            setDestination(sub.client_email || '');
            setShowForm(false);
            queryClient.invalidateQueries({queryKey: ['hosting-email', sub.id]});
        },
        onError: (err: Error) => { setError(err.message); },
    });

    const deleteMutation = useMutation({
        mutationFn: (aliasId: string) => apiDeleteEmailAlias(sub.id, aliasId),
        onMutate: () => { setError(null); },
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: ['hosting-email', sub.id]});
        },
        onError: (err: Error) => { setError(err.message); },
    });

    const handleCreate = useCallback(() => {
        if (!alias.trim() || !domain.trim() || !destination.trim()) return;
        createMutation.mutate({
            alias: alias.trim().toLowerCase(),
            domain: domain.trim().toLowerCase(),
            destination: destination.trim(),
        });
    }, [alias, domain, destination, createMutation]);

    const handleDelete = useCallback((aliasId: string) => {
        if (window.confirm('¿Eliminar este alias de correo?')) {
            deleteMutation.mutate(aliasId);
        }
    }, [deleteMutation]);

    const aliases: EmailAliasInfo[] = emailInfo?.aliases ?? [];
    const used = aliases.length;
    const remaining = aliasLimit > 0 ? aliasLimit - used : 0;
    /* [259A-6] Cuota via --var (style prop solo inyecta --var, objeto nombrado). */
    const estiloCuota = {
        '--tabCorreo-cuota-uso': `${Math.min((used / aliasLimit) * 100, 100)}%`,
    } as React.CSSProperties;

    if (aliasLimit === 0) {
        return (
            <div className="hostingDetalleSection">
                <h3 className="hostingDetalleSectionTitle">
                    <Mail size={18} /> Correo
                </h3>
                <div className="tabCorreoEmpty">
                    <AlertTriangle size={24} />
                    <p>El plan <strong>{sub.plan}</strong> no incluye alias de correo.</p>
                    <p className="tabCorreoHint">
                        Actualiza a un plan Pro o Avanzado para crear alias profesionales
                        (info@, ventas@, soporte@) con reenvío gratuito.
                    </p>
                </div>
            </div>
        );
    }

    if (isLoading) {
        return (
            <div className="hostingDetalleSection">
                <h3 className="hostingDetalleSectionTitle">
                    <Mail size={18} /> Correo
                </h3>
                <div className="tabCorreoEmpty">
                    <Loader2 size={24} className="tabCorreoSpinner" />
                    <p>Cargando informacion de correo...</p>
                </div>
            </div>
        );
    }

    return (
        <div className="hostingDetalleSection">
            <div className="tabCorreoHeader">
                <h3 className="hostingDetalleSectionTitle hostingDetalleSectionTitle--sinBorde">
                    <Mail size={18} /> Correo
                </h3>
                {remaining > 0 && !showForm && (
                    <Button
                        onClick={() => setShowForm(true)}
                        variante="primario"
                        tamano="pequeno"
                    >
                        <Plus size={16} /> Agregar alias
                    </Button>
                )}
            </div>

            {/* Cuota */}
            <div className="tabCorreoQuota">
                <span className="tabCorreoQuotaLabel">
                    Aliases: {used}/{aliasLimit} usados
                    {remaining > 0 ? ` (${remaining} disponibles)` : ''}
                </span>
                {used > 0 && (
                    <div className="tabCorreoQuotaBar">
                        <div
                            className="tabCorreoQuotaFill"
                            style={estiloCuota}
                        />
                    </div>
                )}
            </div>

            {/* Error */}
            {error && (
                <div className="tabCorreoError">
                    <AlertTriangle size={16} />
                    <span>{error}</span>
                </div>
            )}

            {/* Formulario crear alias */}
            {showForm && (
                <div className="tabCorreoForm">
                    <div className="tabCorreoFormRow">
                        <input
                            type="text"
                            value={alias}
                            onChange={e => setAlias(e.target.value)}
                            placeholder="Ej: info"
                            className="tabCorreoInput tabCorreoInputAlias"
                            disabled={createMutation.isPending}
                        />
                        <span className="tabCorreoArroba">@</span>
                        <input
                            type="text"
                            value={domain}
                            onChange={e => setDomain(e.target.value)}
                            placeholder="tudominio.com"
                            className="tabCorreoInput tabCorreoInputDomain"
                            disabled={createMutation.isPending}
                        />
                    </div>
                    <div className="tabCorreoFormRow">
                        <input
                            type="email"
                            value={destination}
                            onChange={e => setDestination(e.target.value)}
                            placeholder="Correo destino (ej: cliente@gmail.com)"
                            className="tabCorreoInput tabCorreoInputDest"
                            disabled={createMutation.isPending}
                        />
                    </div>
                    <div className="tabCorreoFormActions">
                        <Button
                            onClick={() => setShowForm(false)}
                            variante="texto"
                            tamano="pequeno"
                            disabled={createMutation.isPending}
                        >
                            Cancelar
                        </Button>
                        <Button
                            onClick={handleCreate}
                            variante="primario"
                            tamano="pequeno"
                            disabled={!alias.trim() || !domain.trim() || !destination.trim() || createMutation.isPending}
                        >
                            {createMutation.isPending ? (
                                <Loader2 size={16} className="tabCorreoSpinner" />
                            ) : (
                                <Plus size={16} />
                            )}
                            Crear alias
                        </Button>
                    </div>
                </div>
            )}

            {/* Lista de aliases */}
            {aliases.length === 0 ? (
                <div className="tabCorreoEmpty">
                    <Mail size={24} />
                    <p>Aun no hay alias de correo configurados.</p>
                    <p className="tabCorreoHint">
                        Los alias reenvian correos a tu bandeja personal (Gmail, Outlook, etc.)
                        sin costo adicional.
                    </p>
                </div>
            ) : (
                <div className="tabCorreoList">
                    {aliases.map(a => (
                        <div key={a.id} className="tabCorreoItem">
                            <div className="tabCorreoItemInfo">
                                <span className="tabCorreoItemEmail">
                                    {a.full_email}
                                </span>
                                <span className="tabCorreoItemArrow">→</span>
                                <span className="tabCorreoItemDest">{a.destination}</span>
                                <span className={`tabCorreoItemStatus tabCorreoItemStatus--${a.status}`}>
                                    {a.status === 'active' ? 'Activo' : a.status}
                                </span>
                            </div>
                            <Button
                                onClick={() => handleDelete(a.id)}
                                variante="texto"
                                tamano="pequeno"
                                className="tabCorreoItemDelete"
                                disabled={deleteMutation.isPending}
                                title="Eliminar alias"
                            >
                                <Trash2 size={14} />
                            </Button>
                        </div>
                    ))}
                </div>
            )}

            {/* Opcion B — Preparado para buzones IMAP (Fase 2) */}
            {emailInfo && emailInfo.mailboxes_limit > 0 && (
                <div className="tabCorreoMailboxes">
                    <h4 className="tabCorreoMailboxesTitle">Buzones IMAP</h4>
                    <p className="tabCorreoHint">
                        Buzones de correo real con IMAP/SMTP/webmail disponibles proximamente.
                        Tu plan incluye {emailInfo.mailboxes_limit} buzón(es).
                    </p>
                </div>
            )}
        </div>
    );
}
