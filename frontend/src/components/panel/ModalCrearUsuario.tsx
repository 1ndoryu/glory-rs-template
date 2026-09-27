/* [259A-5 5b-7] sentinel-disable-file menu-contextual-override-diseno: trigger de rol
 * a ancho completo del formulario (layout one-off width:100%); no encaja en la
 * variante canon 'filtro' del DS. */
/* [015A-1+] ModalCrearUsuario extraido de SeccionUsuarios.tsx para cumplir limite de 300 lineas.
 * Form state local (email, password, role). Solo cierra en éxito; muestra error inline.
 * Gotcha: no usar estado global para el form — evita contaminar useUsersSection.
 * [15A-SENT-1] Estado extraído a useModalCrearUsuario para cumplir limite de 3 useState.
 * [035A-23] <form> cambiado a <div> para evitar inconsistencia visual con otros modales. */

import { AlertCircle, ChevronDown } from 'lucide-react';
import { MenuContextual } from '../ui/ContextMenu';
import { ROLE_LABELS } from '../../api/admin-users';
import { Input } from '../ui/Input';
import { Button } from '../ui/Button';
import { ModalBody, ModalField, ModalLabel } from '../ui/Modal';
import { useModalCrearUsuario } from '../../hooks/useModalCrearUsuario';
import './SeccionUsuarios.css';

export function ModalCrearUsuario({ onClose, onSubmit, isCreating, onCreated }: {
    onClose: () => void;
    onSubmit: (payload: { email: string; password: string; role?: 'admin' | 'employee' | 'client' }) => Promise<unknown>;
    isCreating: boolean;
    onCreated: () => void;
}) {
    const { email, setEmail, password, setPassword, role, setRole, error, rolMenuAbierto, setRolMenuAbierto, handleSubmit } =
        useModalCrearUsuario({ onSubmit, onClose, onCreated });

    return (
        <ModalBody>
            <ModalField>
                <ModalLabel htmlFor="crear-email">Email</ModalLabel>
                <Input
                    id="crear-email"
                    type="email"
                    value={email}
                    onChange={e => setEmail(e.target.value)}
                    placeholder="usuario@ejemplo.com"
                    required
                    disabled={isCreating}
                />
            </ModalField>

            <ModalField>
                <ModalLabel htmlFor="crear-password">Contraseña</ModalLabel>
                <Input
                    id="crear-password"
                    type="password"
                    value={password}
                    onChange={e => setPassword(e.target.value)}
                    placeholder="Mínimo 8 caracteres"
                    minLength={8}
                    required
                    disabled={isCreating}
                />
            </ModalField>

            <ModalField>
                <ModalLabel>Rol</ModalLabel>
                <MenuContextual
                    abierto={rolMenuAbierto}
                    onToggle={() => setRolMenuAbierto(prev => !prev)}
                    onCerrar={() => setRolMenuAbierto(false)}
                    ariaLabel="Seleccionar rol"
                    triggerVariante="outline"
                    triggerTamano="pequeno"
                    triggerClassName="usuariosCrearRolBtn"
                    triggerContent={<>{ROLE_LABELS[role]} <ChevronDown size={14} /></>}
                    items={[
                        { id: 'client', label: 'Cliente', onSelect: () => setRole('client') },
                        { id: 'employee', label: 'Freelancer', onSelect: () => setRole('employee') },
                        { id: 'admin', label: 'Admin', onSelect: () => setRole('admin') },
                    ]}
                />
            </ModalField>

            {error && (
                <div className="usuariosError usuariosError--modal">
                    <AlertCircle size={16} />
                    <span>{error}</span>
                </div>
            )}

            <div className="modalAcciones">
                <Button
                    variante="secundario"
                    tamano="pequeno"
                    onClick={onClose}
                    disabled={isCreating}
                    type="button"
                >
                    Cancelar
                </Button>
                <Button
                    variante="primario"
                    tamano="pequeno"
                    disabled={isCreating}
                    type="button"
                    onClick={() => void handleSubmit()}
                >
                    {isCreating ? 'Creando...' : 'Crear usuario'}
                </Button>
            </div>
        </ModalBody>
    );
}
