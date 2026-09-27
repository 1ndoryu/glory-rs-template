/* [154A-6] Gestión de registros DNS para clientes.
 * Permite al cliente ver, crear, editar y eliminar registros DNS de su dominio.
 * Se muestra dentro de TabDominio cuando el dominio está configurado. */

/* [259A-5 5b-6] Nativos migrados a DS; disable html retirado por obsoleto. */
import {useState} from 'react';
import {useMutation, useQuery, useQueryClient} from '@tanstack/react-query';
import {Loader, Plus, Trash2, Edit2, Save, X} from 'lucide-react';
import {
    apiClientListDnsRecords,
    apiClientCreateDnsRecord,
    apiClientDeleteDnsRecord,
    apiClientUpdateDnsRecord,
    type DnsRecord,
    type CreateDnsRecordRequest,
} from '../../api/hosting';
import {Button} from '../ui/Button';
import {Input} from '../ui/Input';
import {SelectDropdown} from '../ui/SelectDropdown';
import {toast} from '../../stores/toastStore';

const DNS_TYPES = ['A', 'AAAA', 'CNAME', 'MX', 'TXT', 'SRV', 'CAA'] as const;

interface Props {
    subscriptionId: string;
}

export function DnsManager({subscriptionId}: Props) {
    const queryClient = useQueryClient();
    const dnsKey = ['dns-records', subscriptionId];

    const {data: records, isLoading, error} = useQuery({
        queryKey: dnsKey,
        queryFn: () => apiClientListDnsRecords(subscriptionId),
    });

    const [showForm, setShowForm] = useState(false);
    const [editingId, setEditingId] = useState<number | null>(null);
    const [form, setForm] = useState<CreateDnsRecordRequest>({
        name: '',
        type: 'A',
        data: '',
        ttl: 3600,
        prio: 0,
    });

    const resetForm = () => {
        setForm({name: '', type: 'A', data: '', ttl: 3600, prio: 0});
        setShowForm(false);
        setEditingId(null);
    };

    const createMutation = useMutation({
        mutationFn: (req: CreateDnsRecordRequest) =>
            apiClientCreateDnsRecord(subscriptionId, req),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: dnsKey});
            toast.success('Registro DNS creado');
            resetForm();
        },
        onError: () => toast.error('Error al crear registro DNS'),
    });

    const updateMutation = useMutation({
        mutationFn: ({id, req}: {id: number; req: {type: string; ttl: number; prio: number; data: string}}) =>
            apiClientUpdateDnsRecord(subscriptionId, id, req),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: dnsKey});
            toast.success('Registro DNS actualizado');
            resetForm();
        },
        onError: () => toast.error('Error al actualizar registro DNS'),
    });

    const deleteMutation = useMutation({
        mutationFn: (id: number) => apiClientDeleteDnsRecord(subscriptionId, id),
        onSuccess: () => {
            queryClient.invalidateQueries({queryKey: dnsKey});
            toast.success('Registro DNS eliminado');
        },
        onError: () => toast.error('Error al eliminar registro DNS'),
    });

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        if (!form.name || !form.data) return;
        if (editingId !== null) {
            updateMutation.mutate({id: editingId, req: {type: form.type, ttl: form.ttl, prio: form.prio, data: form.data}});
        } else {
            createMutation.mutate(form);
        }
    };

    const startEdit = (record: DnsRecord) => {
        setEditingId(record.recordId);
        setForm({
            name: record.name ?? '',
            type: record.type ?? 'A',
            data: record.data ?? '',
            ttl: record.ttl ?? 3600,
            prio: record.prio ?? 0,
        });
        setShowForm(true);
    };

    if (error) {
        return (
            <p className="hostingDetalleSectionDesc dnsErrorMessage">
                No se pudieron cargar los registros DNS. Es posible que el dominio no esté gestionado en nuestros servidores.
            </p>
        );
    }

    return (
        <div className="dnsManager">
            <div className="dnsManagerHeader">
                <h4 className="hostingDetalleSubTitle">Registros DNS</h4>
                {!showForm && (
                    <Button
                        type="button"
                        variante="outline"
                        tamano="pequeno"
                        onClick={() => { resetForm(); setShowForm(true); }}
                    >
                        <Plus size={14} /> Agregar registro
                    </Button>
                )}
            </div>

            {showForm && (
                <form className="dnsForm" onSubmit={handleSubmit}>
                    <div className="dnsFormRow">
                        <label className="dnsFormField">
                            <span>Tipo</span>
                            <SelectDropdown
                                value={form.type}
                                opciones={DNS_TYPES.map(t => ({value: t, label: t}))}
                                onChange={v => setForm(f => ({...f, type: v}))}
                                ariaLabel="Tipo de registro DNS"
                            />
                        </label>
                        <label className="dnsFormField dnsFormField--name">
                            <span>Nombre</span>
                            <Input
                                type="text"
                                value={form.name}
                                onChange={e => setForm(f => ({...f, name: e.target.value}))}
                                placeholder="@ o subdominio"
                                required
                            />
                        </label>
                        <label className="dnsFormField dnsFormField--value">
                            <span>Valor</span>
                            <Input
                                type="text"
                                value={form.data}
                                onChange={e => setForm(f => ({...f, data: e.target.value}))}
                                placeholder="IP o dominio destino"
                                required
                            />
                        </label>
                        <label className="dnsFormField">
                            <span>TTL</span>
                            <Input
                                type="number"
                                value={form.ttl}
                                onChange={e => setForm(f => ({...f, ttl: Number(e.target.value)}))}
                                min={60}
                            />
                        </label>
                        {(form.type === 'MX' || form.type === 'SRV') && (
                            <label className="dnsFormField">
                                <span>Prioridad</span>
                                <Input
                                    type="number"
                                    value={form.prio}
                                    onChange={e => setForm(f => ({
                                        ...f,
                                        prio: Number(e.target.value) || 0,
                                    }))}
                                    min={0}
                                />
                            </label>
                        )}
                    </div>
                    <div className="dnsFormActions">
                        <Button
                            type="submit"
                            variante="primario"
                            tamano="pequeno"
                            disabled={createMutation.isPending || updateMutation.isPending}
                        >
                            {createMutation.isPending || updateMutation.isPending ? (
                                <Loader size={14} className="hostingSpinner" />
                            ) : editingId !== null ? (
                                <><Save size={14} /> Guardar</>
                            ) : (
                                <><Plus size={14} /> Crear</>
                            )}
                        </Button>
                        <Button type="button" variante="outline" tamano="pequeno" onClick={resetForm}>
                            <X size={14} /> Cancelar
                        </Button>
                    </div>
                </form>
            )}

            {isLoading ? (
                <div className="dnsLoading"><Loader size={18} className="hostingSpinner" /> Cargando registros…</div>
            ) : records && records.length > 0 ? (
                <div className="dnsTable">
                    <div className="dnsTableHeader">
                        <span>Tipo</span>
                        <span>Nombre</span>
                        <span>Valor</span>
                        <span>TTL</span>
                        <span></span>
                    </div>
                    {records.map(record => (
                        <div key={record.recordId} className="dnsTableRow">
                            <span className="dnsType">{record.type}</span>
                            <span className="dnsName">{record.name}</span>
                            <span className="dnsValue" title={record.data ?? ''}>{record.data}</span>
                            <span className="dnsTtl">{record.ttl}</span>
                            <span className="dnsActions">
                                <Button
                                    type="button"
                                    variante="texto"
                                    tamano="pequeno"
                                    title="Editar"
                                    onClick={() => startEdit(record)}
                                >
                                    <Edit2 size={14} />
                                </Button>
                                <Button
                                    type="button"
                                    variante="texto"
                                    tamano="pequeno"
                                    title="Eliminar"
                                    onClick={() => { if (record.recordId != null) deleteMutation.mutate(record.recordId); }}
                                    disabled={deleteMutation.isPending}
                                >
                                    <Trash2 size={14} />
                                </Button>
                            </span>
                        </div>
                    ))}
                </div>
            ) : (
                <p className="hostingDetalleSectionDesc">
                    No hay registros DNS configurados todavía.
                </p>
            )}
        </div>
    );
}
