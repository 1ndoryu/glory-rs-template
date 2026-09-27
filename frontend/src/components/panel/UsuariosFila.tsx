/* [054A-1+] UserRow extraido de SeccionUsuarios.tsx para cumplir limite de 300 lineas.
 * Fila individual de usuario con menú de acciones (rol, status, eliminar). */

import { useState } from 'react';
import { Shield, Ban, UserCheck, Trash2 } from 'lucide-react';
import { MenuContextual, type MenuContextualItem } from '../ui/ContextMenu';
import { ROLE_LABELS, STATUS_LABELS, STATUS_CLASS } from '../../api/admin-users';
import type { AdminUserItem } from '../../api/admin-users';
import OptimizedImage from '../ui/OptimizedImage';
import './SeccionUsuarios.css';
import './UsuariosAcciones.css';

export function UserRow({ user, onChangeRole, onChangeStatus, onDelete, canDelete }: {
    user: AdminUserItem;
    onChangeRole: (role: string) => void;
    onChangeStatus: (status: string) => void;
    onDelete: () => void;
    canDelete: boolean;
}) {
    const [menuAbierto, setMenuAbierto] = useState(false);

    const rolesDisponibles = ['admin', 'employee', 'client'].filter(r => r !== user.role);
    const isBanned = user.status === 'banned';

    const menuItems: MenuContextualItem[] = [
        ...rolesDisponibles.map(r => ({
            id: `role-${r}`,
            label: `Cambiar a ${ROLE_LABELS[r]}`,
            icon: <Shield size={14} />,
            onSelect: () => onChangeRole(r),
        })),
        isBanned
            ? {
                id: 'reactivar',
                label: 'Reactivar',
                icon: <UserCheck size={14} />,
                onSelect: () => onChangeStatus('active'),
            }
            : {
                id: 'banear',
                label: 'Banear',
                icon: <Ban size={14} />,
                onSelect: () => onChangeStatus('banned'),
                danger: true,
            },
    ];

    if (canDelete) {
        menuItems.push({
            id: 'eliminar',
            label: 'Eliminar usuario',
            icon: <Trash2 size={14} />,
            onSelect: onDelete,
            danger: true,
        });
    }

    return (
        <tr className="usuariosFila">
            <td className="usuariosCeldaUsuario">
                <div className="usuariosAvatar">
                    {user.avatar_url
                        ? <OptimizedImage src={user.avatar_url} alt="" className="usuariosAvatarImg" loading="lazy" />
                        : <span className="usuariosAvatarPlaceholder">
                            {(user.display_name || user.email)[0].toUpperCase()}
                          </span>
                    }
                </div>
                <div className="usuariosInfo">
                    <span className="usuariosNombre">{user.display_name || '—'}</span>
                    <span className="usuariosEmail">{user.email}</span>
                </div>
            </td>
            <td>
                <span className={`usuariosRolBadge usuariosRol${user.role}`}>
                    {ROLE_LABELS[user.role] || user.role}
                </span>
            </td>
            <td>
                <span className={`usuariosStatusBadge ${STATUS_CLASS[user.status] || ''}`}>
                    {STATUS_LABELS[user.status] || user.status}
                </span>
            </td>
            <td className="usuariosFecha">
                {new Date(user.created_at).toLocaleDateString('es-ES', {
                    day: 'numeric', month: 'short', year: 'numeric',
                })}
            </td>
            <td className="usuariosAcciones">
                <MenuContextual
                    abierto={menuAbierto}
                    onToggle={() => setMenuAbierto(prev => !prev)}
                    onCerrar={() => setMenuAbierto(false)}
                    items={menuItems}
                ariaLabel="Acciones del usuario"
                variante="accion"
                />
            </td>
        </tr>
    );
}
