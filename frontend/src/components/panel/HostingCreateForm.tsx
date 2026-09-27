/* [074A-63+] UserSelector y CreateHostingForm extraidos de HostingSubComponents.tsx
 * para cumplir limite de 300 lineas. UserSelector es combobox con búsqueda inline. */

/* [259A-5] sentinel-disable-file html-nativo-en-vez-de-componente: migracion visual a componentes DS en fase 5b con verificacion en navegador. */
import React, {useState, useCallback, useRef, useEffect} from 'react';
import {useQuery} from '@tanstack/react-query';
import {ChevronDown} from 'lucide-react';
import {
    type CreateHostingRequest,
} from '../../api/hosting';
import {apiListUsers, type AdminUserItem} from '../../api/admin-users';
import {Input} from '../ui/Input';
import {Select} from '../ui/Select';
import {Button} from '../ui/Button';
import {ModalBody} from '../ui/Modal';
import {HOSTING_PLAN_OPTIONS} from './hostingPlanOptions';

/* Selector de usuario con búsqueda. Devuelve el usuario seleccionado via onSelect. */
function UserSelector({onSelect}: {onSelect: (user: AdminUserItem) => void}) {
    const [search, setSearch] = useState('');
    const [open, setOpen] = useState(false);
    const ref = useRef<HTMLDivElement>(null);

    const {data, isLoading} = useQuery({
        queryKey: ['admin-users-selector', search],
        queryFn: () => apiListUsers({search, per_page: 20, role: 'client'}),
        enabled: open,
        staleTime: 30_000,
    });

    /* Cerrar al hacer click fuera — UserSelector es un combobox con búsqueda inline;
     * MenuContextual solo soporta items estáticos, no input de búsqueda. */
    /* sentinel-disable-next-line componente-artesanal */
    useEffect(() => {
        if (!open) return;
        const handler = (e: MouseEvent) => {
            if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
        };
        /* sentinel-disable-next-line componente-artesanal */
        document.addEventListener('mousedown', handler);
        return () => document.removeEventListener('mousedown', handler);
    }, [open]);

    const users = data?.users ?? [];

    return (
        <div className="userSelectorContenedor" ref={ref}>
            <div className="userSelectorInput" onClick={() => setOpen(prev => !prev)}>
                <Input
                    type="text"
                    placeholder="Buscar cliente registrado…"
                    value={search}
                    onChange={e => {
                        setSearch(e.target.value);
                        setOpen(true);
                    }}
                    onFocus={() => setOpen(true)}
                />
                <ChevronDown size={14} className="userSelectorChevron" />
            </div>
            {open && (
                <div className="userSelectorDropdown">
                    {isLoading && <div className="userSelectorItem userSelectorItemMuted">Cargando…</div>}
                    {!isLoading && users.length === 0 && (
                        <div className="userSelectorItem userSelectorItemMuted">Sin resultados</div>
                    )}
                    {users.map(user => (
                        <div
                            key={user.id}
                            className="userSelectorItem"
                            onClick={() => {
                                onSelect(user);
                                setSearch(user.display_name || user.email);
                                setOpen(false);
                            }}
                        >
                            <span className="userSelectorName">{user.display_name || user.email}</span>
                            <span className="userSelectorEmail">{user.email}</span>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}

export function CreateHostingForm({
    onSubmit,
    submitting,
    initialCoolifyName,
}: {
    onSubmit: (req: CreateHostingRequest) => void;
    submitting: boolean;
    /* [304A-3] Pre-llena coolify_site_name cuando se crea desde un despliegue huérfano */
    initialCoolifyName?: string;
}) {
    const [selectedUser, setSelectedUser] = useState<AdminUserItem | null>(null);
    const [form, setForm] = useState({
        plan: 'basico',
        domain: '',
        coolify_site_name: initialCoolifyName || '',
    });

    const handleSubmit = useCallback(
        (e: React.FormEvent) => {
            e.preventDefault();
            if (!selectedUser) return;
            onSubmit({
                client_name: selectedUser.display_name || selectedUser.email,
                client_email: selectedUser.email,
                plan: form.plan,
                domain: form.domain.trim() || undefined,
                coolify_site_name: form.coolify_site_name.trim() || undefined,
            });
        },
        [form, onSubmit, selectedUser],
    );

    return (
        <ModalBody as="form" className="hostingCrearContenido" onSubmit={handleSubmit}>
            <UserSelector onSelect={setSelectedUser} />
            {selectedUser && (
                <p className="hostingFormNota">
                    Cliente: <strong>{selectedUser.display_name || selectedUser.email}</strong> · {selectedUser.email}
                </p>
            )}
            <Select
                className="hostingSelect"
                value={form.plan}
                onChange={e => setForm(prev => ({...prev, plan: e.target.value}))}
            >
                {HOSTING_PLAN_OPTIONS.map(option => (
                    <option key={option.value} value={option.value}>{option.label}</option>
                ))}
            </Select>
            <Input
                type="text"
                placeholder="Dominio (opcional)"
                value={form.domain}
                onChange={e => setForm(prev => ({...prev, domain: e.target.value}))}
            />
            {/* [304A-3] Campo visible solo cuando se vincula a un despliegue Coolify */}
            {initialCoolifyName !== undefined && (
                <Input
                    type="text"
                    placeholder="Nombre en Coolify"
                    value={form.coolify_site_name}
                    onChange={e => setForm(prev => ({...prev, coolify_site_name: e.target.value}))}
                />
            )}
            <div className="modalAcciones">
                <Button type="submit" variante="secundario" tamano="pequeno" disabled={submitting || !selectedUser}>
                    {submitting ? 'Creando...' : 'Crear suscripción WordPress'}
                </Button>
            </div>
        </ModalBody>
    );
}
