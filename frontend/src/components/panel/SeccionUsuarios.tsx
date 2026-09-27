/* [259A-5] sentinel-disable-file menu-contextual-override-diseno: variantes por instancia via props API publica del DS; variantes canon al sistema en fase 5b. */
/* [054A-1] Sección admin: gestión de usuarios registrados.
 * Búsqueda por email/nombre, filtros por rol y status, paginación.
 * Acciones: cambiar rol, banear/reactivar.
 * [015A-1] Botón "Nuevo usuario" con modal de creación: email, contraseña, rol. */

import { Loader2, AlertCircle, Search, Users, ChevronDown, UserPlus } from 'lucide-react';
import { useUsersSection } from '../../hooks/useUsersSection';
import { ROLE_LABELS, STATUS_LABELS } from '../../api/admin-users';
import { Input } from '../ui/Input';
import { Modal } from '../ui/Modal';
import { Button } from '../ui/Button';
import { MenuContextual } from '../ui/ContextMenu';
import { UserRow } from './UsuariosFila';
import { ModalCrearUsuario } from './ModalCrearUsuario';
import './SeccionUsuarios.css';
import './UsuariosAcciones.css';

export function SeccionUsuarios() {
    const {
        users, total, page, totalPages, isLoading, error,
        search, roleFilter, statusFilter,
        setSearch, setRoleFilter, setStatusFilter, setPage,
        currentUserId,
        rolMenuAbierto,
        statusMenuAbierto,
        confirmAction,
        modalError,
        modalCrear,
        isProcessing,
        setRolMenuAbierto,
        setStatusMenuAbierto,
        openRoleConfirm,
        openStatusConfirm,
        openDeleteConfirm,
        closeConfirm,
        handleConfirm,
        openCreateModal,
        closeCreateModal,
        createUser,
        isCreatingUser,
    } = useUsersSection();

    if (isLoading) {
        return (
            <div className="usuariosVacio">
                <Loader2 className="usuariosSpinner" size={32} />
            </div>
        );
    }

    if (error) {
        return (
            <div className="usuariosError">
                <AlertCircle size={20} />
                <span>Error al cargar usuarios</span>
            </div>
        );
    }

    return (
        <div className="usuariosContenedor">
            {/* [064A-17] Filtros con MenuContextual personalizado */}
            <div className="usuariosFiltros">
                <div className="usuariosBusqueda">
                    <Search size={16} className="usuariosBusquedaIcono" />
                    <Input
                        type="text"
                        className="usuariosBusquedaInput"
                        placeholder="Buscar por email o nombre..."
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                    />
                </div>

                <MenuContextual
                    abierto={rolMenuAbierto}
                    onToggle={() => setRolMenuAbierto(prev => !prev)}
                    onCerrar={() => setRolMenuAbierto(false)}
                    ariaLabel="Filtrar por rol"
                    triggerClassName="usuariosFiltroBtn"
                    triggerVariante="outline"
                    triggerTamano="pequeno"
                    triggerContent={<>{roleFilter ? ROLE_LABELS[roleFilter] : 'Todos los roles'} <ChevronDown size={14} /></>}
                    items={[
                        {id: 'all', label: 'Todos los roles', onSelect: () => setRoleFilter('')},
                        {id: 'admin', label: 'Admin', onSelect: () => setRoleFilter('admin')},
                        {id: 'employee', label: 'Freelancer', onSelect: () => setRoleFilter('employee')},
                        {id: 'client', label: 'Cliente', onSelect: () => setRoleFilter('client')},
                    ]}
                />

                <MenuContextual
                    abierto={statusMenuAbierto}
                    onToggle={() => setStatusMenuAbierto(prev => !prev)}
                    onCerrar={() => setStatusMenuAbierto(false)}
                    ariaLabel="Filtrar por status"
                    triggerClassName="usuariosFiltroBtn"
                    triggerVariante="outline"
                    triggerTamano="pequeno"
                    triggerContent={<>{statusFilter ? STATUS_LABELS[statusFilter] : 'Todos los status'} <ChevronDown size={14} /></>}
                    items={[
                        {id: 'all', label: 'Todos los status', onSelect: () => setStatusFilter('')},
                        {id: 'active', label: 'Activo', onSelect: () => setStatusFilter('active')},
                        {id: 'banned', label: 'Baneado', onSelect: () => setStatusFilter('banned')},
                        {id: 'suspended', label: 'Suspendido', onSelect: () => setStatusFilter('suspended')},
                    ]}
                />

                <span className="usuariosTotal">
                    <Users size={14} />
                    {total} usuario{total !== 1 ? 's' : ''}
                </span>

                <Button
                    variante="primario"
                    tamano="pequeno"
                    onClick={openCreateModal}
                    type="button"
                    className="usuariosNuevoBtn"
                >
                    <UserPlus size={14} />
                    Nuevo usuario
                </Button>
            </div>

            {/* Tabla de usuarios */}
            {users.length === 0 ? (
                <div className="usuariosVacio">
                    <Users size={32} />
                    <p>No se encontraron usuarios</p>
                </div>
            ) : (
                <div className="usuariosTablaWrapper">
                    <table className="usuariosTabla">
                        <thead>
                            <tr>
                                <th>Usuario</th>
                                <th>Rol</th>
                                <th>Status</th>
                                <th>Registrado</th>
                                <th>Acciones</th>
                            </tr>
                        </thead>
                        <tbody>
                            {users.map((u) => (
                                <UserRow
                                    key={u.id}
                                    user={u}
                                    onChangeRole={(role) => openRoleConfirm(u.id, u.email, role)}
                                    onChangeStatus={(status) => openStatusConfirm(u.id, u.email, status)}
                                    onDelete={() => openDeleteConfirm(u.id, u.email)}
                                    canDelete={u.id !== currentUserId}
                                />
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            {/* Paginación */}
            {totalPages > 1 && (
                <div className="usuariosPaginacion">
                    <Button
                        className="usuariosPaginacionBtn"
                        variante="outline"
                        tamano="pequeno"
                        disabled={page <= 1}
                        onClick={() => setPage(page - 1)}
                        type="button"
                    >
                        ← Anterior
                    </Button>
                    <span className="usuariosPaginacionInfo">
                        Página {page} de {totalPages}
                    </span>
                    <Button
                        className="usuariosPaginacionBtn"
                        variante="outline"
                        tamano="pequeno"
                        disabled={page >= totalPages}
                        onClick={() => setPage(page + 1)}
                        type="button"
                    >
                        Siguiente →
                    </Button>
                </div>
            )}

            {/* [015A-1] Modal de confirmación de acción */}
            <Modal abierto={!!confirmAction} onCerrar={closeConfirm}>
                <p className="modalTexto usuariosModalMensaje">
                    {confirmAction?.type === 'role'
                        ? `¿Cambiar rol de ${confirmAction.email} a ${ROLE_LABELS[confirmAction.value] || confirmAction.value}?`
                        : confirmAction?.type === 'status'
                            ? `¿Cambiar status de ${confirmAction.email} a ${STATUS_LABELS[confirmAction.value] || confirmAction.value}?`
                            : `¿Eliminar permanentemente a ${confirmAction?.email}? Solo se podrá borrar si no tiene pedidos, hosting, chats u otras relaciones activas.`
                    }
                </p>
                {modalError && (
                    <div className="usuariosError usuariosError--modal">
                        <AlertCircle size={16} />
                        <span>{modalError}</span>
                    </div>
                )}
                <div className="modalAcciones">
                    <Button
                        variante="secundario"
                        tamano="pequeno"
                        onClick={closeConfirm}
                        disabled={isProcessing}
                        type="button"
                    >
                        Cancelar
                    </Button>
                    <Button
                        variante="primario"
                        tamano="pequeno"
                        onClick={() => void handleConfirm()}
                        disabled={isProcessing}
                        type="button"
                    >
                        {isProcessing ? 'Procesando...' : 'Confirmar'}
                    </Button>
                </div>
            </Modal>

            {/* [015A-1] Modal para crear nuevo usuario */}
            <Modal abierto={modalCrear} onCerrar={closeCreateModal}>
                <ModalCrearUsuario
                    onClose={closeCreateModal}
                    onSubmit={createUser}
                    isCreating={isCreatingUser}
                    onCreated={() => setPage(1)}
                />
            </Modal>
        </div>
    );
}

/* Fila individual de usuario → extraida a UsuariosFila.tsx
 * Modal de creación → extraida a ModalCrearUsuario.tsx */
