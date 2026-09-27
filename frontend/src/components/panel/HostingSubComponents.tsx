/* [074A-63] HostingCard — sub-componente principal de SeccionHosting.
 * CreateHostingForm → HostingCreateForm.tsx, EventsPanel → HostingEventsPanel.tsx.
 * [074A-65] Incluye editar/eliminar en context menu.
 * [304A-3] Admin puede asignar hosting a cliente por email + generar link de pago.
 * [15A-SENT-1] Estado extraído a useHostingCard para cumplir limite de 3 useState. */

/* [259A-5] sentinel-disable-file html-nativo-en-vez-de-componente: migracion visual a componentes DS en fase 5b con verificacion en navegador. */
import {Server, ExternalLink, UserCheck, Link} from 'lucide-react';
import {
    HOSTING_PLAN_LABELS,
    HOSTING_STATUS_LABELS,
    HOSTING_STATUS_CLASS,
    getProvisionedHostingSiteUrl,
    type HostingSubscription,
    type UpdateHostingRequest,
} from '../../api/hosting';
import {Modal, ModalBody} from '../ui/Modal';
import {Input} from '../ui/Input';
import {Select} from '../ui/Select';
import {Button} from '../ui/Button';
import {MenuContextual, type MenuContextualItem} from '../ui/ContextMenu';
import {HOSTING_PLAN_OPTIONS} from './hostingPlanOptions';
import {useHostingCard} from '../../hooks/useHostingCard';

/* [074A-57] Card de hosting — layout similar a ordenCard de proyectosLista
 * [074A-63] Titulo = dominio o nombre del hosting (identidad unica), plan va debajo.
 * [074A-65] Context menu con editar/eliminar + edit modal inline.
 * [084A-4] Clientes ahora tienen context menu con Editar/Cancelar.
 * [304A-3] Admin puede asignar hosting a cliente y generar link de pago. */
export function HostingCard({
    sub,
    isAdmin,
    onStatusChange,
    onUpdate,
    onDelete,
    onCancel,
    onCheckout,
    onAssign,
    onSelect,
    checkoutLoading,
    assignLoading,
}: {
    sub: HostingSubscription;
    isAdmin: boolean;
    onStatusChange: (status: string) => void;
    onUpdate: (req: UpdateHostingRequest) => void;
    onDelete: () => void;
    onCancel: () => void;
    onCheckout: () => void;
    onAssign?: (email: string) => void;
    onSelect: () => void;
    checkoutLoading?: boolean;
    assignLoading?: boolean;
}) {
    const {
        menuOpen, setMenuOpen,
        editing, setEditing,
        showStatusModal, setShowStatusModal,
        showAssignModal, setShowAssignModal,
        assignEmail, setAssignEmail,
        editPlan, setEditPlan,
        editDomain, setEditDomain,
        handleEditSubmit,
        handleAssignSubmit,
    } = useHostingCard({ sub, onUpdate, onAssign });
    const siteUrl = getProvisionedHostingSiteUrl(sub);

    /* [084A-13] Context menu simplificado: "Cambiar estado" abre modal,
     * ya no lista cada status individualmente en el menú. */
    const menuItems: MenuContextualItem[] = [];

    if (isAdmin) {
        menuItems.push({
            id: 'change-status',
            label: 'Cambiar estado',
            onSelect: () => setShowStatusModal(true),
        });
        /* [304A-3] Asignar hosting a cliente registrado */
        if (onAssign) {
            menuItems.push({
                id: 'assign',
                label: sub.user_id ? 'Reasignar cliente' : 'Asignar a cliente',
                onSelect: () => {
                    setAssignEmail('');
                    setShowAssignModal(true);
                },
            });
        }
    }

    /* Editar: admin y cliente */
    menuItems.push({id: 'edit', label: 'Editar', onSelect: () => setEditing(true)});

    if (isAdmin) {
        menuItems.push({id: 'delete', label: 'Eliminar', onSelect: onDelete, danger: true});
    }

    /* Cancelar: disponible para cliente (y admin) si no está ya cancelada */
    if (sub.status !== 'cancelled') {
        menuItems.push({id: 'cancel', label: 'Cancelar suscripción', onSelect: onCancel, danger: true});
    }

    return (
        <>
            <div className="hostingCard" role="button" tabIndex={0} onClick={onSelect} onKeyDown={e => { if (e.key === 'Enter') onSelect(); }}>
                <div className="panelCardIcono">
                    {/* [195A-1] Screenshot automatico del dominio via thum.io para hostings activos. */}
                    {sub.domain && sub.status === 'active' ? (
                        <img
                            src={`https://image.thum.io/get/width/800/crop/500/https://${sub.domain}`}
                            alt={sub.domain}
                            className="panelCardIconoImg"
                            loading="lazy"
                            onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
                        />
                    ) : (
                        <Server size={28} strokeWidth={1.4} />
                    )}
                </div>
                <div className="hostingCardBody">
                    <div className="hostingCardHeader">
                        <h3 className="hostingCardTitulo">
                            {sub.domain || sub.client_name || HOSTING_PLAN_LABELS[sub.plan] || sub.plan}
                        </h3>
                        <span className={`hostingStatus ${HOSTING_STATUS_CLASS[sub.status] || ''}`}>
                            {HOSTING_STATUS_LABELS[sub.status] || sub.status}
                        </span>
                    </div>
                    <span className="hostingCardPlan">
                        {HOSTING_PLAN_LABELS[sub.plan] || sub.plan}
                    </span>
                    {isAdmin && (
                        <span className="hostingCardCliente">
                            {sub.client_name} · {sub.client_email}
                            {/* [304A-3] Indicador visual de asignación */}
                            {sub.user_id ? (
                                <span className="hostingAsignadoBadge" title="Asignado a cuenta">
                                    <UserCheck size={11} /> vinculado
                                </span>
                            ) : (
                                <span className="hostingNoAsignadoBadge" title="Sin cuenta asignada">
                                    sin cuenta
                                </span>
                            )}
                        </span>
                    )}
                    {/* [104A-24] Recursos movidos al footer, precio eliminado */}
                    <div className="hostingCardFooter">
                        <span className="hostingCardRecurso">
                            {(sub.storage_limit_mb / 1024).toFixed(0)} GB
                        </span>
                        {sub.domain && (
                            <span className="hostingCardRecurso">
                                {sub.domain}
                            </span>
                        )}
                        {/* [155A-13] Enlace rápido al sitio real — WordPress usa `wordpress`, hosting normal usa `site`. */}
                        {siteUrl && sub.status === 'active' && (
                            <a
                                href={siteUrl}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="hostingCardSiteLink"
                                onClick={e => e.stopPropagation()}
                                title="Abrir sitio"
                            >
                                <ExternalLink size={13} /> Ver sitio
                            </a>
                        )}
                        {/* [094A-2] stopPropagation evita navegar al detalle al usar acciones */}
                        <div className="hostingCardAcciones" onClick={e => e.stopPropagation()}>
                            {/* [084A-24] Botón de checkout Stripe para suscripciones pendientes (cliente) */}
                            {sub.status === 'pending' && !isAdmin && (
                                <Button
                                    type="button"
                                    variante="primario"
                                    tamano="pequeno"
                                    onClick={onCheckout}
                                    disabled={checkoutLoading}
                                >
                                    {checkoutLoading ? 'Redirigiendo…' : 'Pagar'}
                                </Button>
                            )}
                            {/* [304A-3] Admin genera link de pago para suscripciones pendientes sin cuenta */}
                            {sub.status === 'pending' && isAdmin && (
                                <Button
                                    type="button"
                                    variante="secundario"
                                    tamano="pequeno"
                                    onClick={onCheckout}
                                    disabled={checkoutLoading}
                                    title="Genera URL de Stripe para compartir con el cliente"
                                >
                                    <Link size={12} />
                                    {checkoutLoading ? 'Generando…' : 'Link de pago'}
                                </Button>
                            )}
                            {/* [084A-4] Menú visible para todos los roles, items varían por rol */}
                            <MenuContextual
                                abierto={menuOpen}
                                onToggle={() => setMenuOpen(prev => !prev)}
                                onCerrar={() => setMenuOpen(false)}
                                items={menuItems}
                                ariaLabel="Gestionar suscripción"
                            />
                        </div>
                    </div>
                </div>
            </div>

            {/* [074A-65] Modal de edición */}
            {editing && (
                <Modal abierto={editing} onCerrar={() => setEditing(false)}>
                    <ModalBody className="hostingCrearContenido">
                        <Select
                            className="hostingSelect"
                            value={editPlan}
                            onChange={e => setEditPlan(e.target.value)}
                        >
                            {HOSTING_PLAN_OPTIONS.map(option => (
                                <option key={option.value} value={option.value}>{option.label}</option>
                            ))}
                        </Select>
                        <Input
                            type="text"
                            placeholder="Dominio (opcional)"
                            value={editDomain}
                            onChange={e => setEditDomain(e.target.value)}
                        />
                        <div className="modalAcciones">
                            <Button type="button" tamano="pequeno" onClick={handleEditSubmit}>
                                Guardar cambios
                            </Button>
                        </div>
                    </ModalBody>
                </Modal>
            )}

            {/* [084A-13] Modal de cambio de estado — reemplaza los status sueltos del menú */}
            {showStatusModal && isAdmin && (
                <Modal abierto={showStatusModal} onCerrar={() => setShowStatusModal(false)}>
                    <ModalBody className="hostingCrearContenido">
                        <p className="hostingStatusModalSub">
                            Estado actual: <strong>{HOSTING_STATUS_LABELS[sub.status] || sub.status}</strong>
                        </p>
                        <div className="hostingStatusModalOpciones">
                            {(['pending', 'provisioning', 'active', 'suspended', 'cancelled'] as const)
                                .filter(s => s !== sub.status)
                                .map(s => (
                                    <Button
                                        key={s}
                                        variante={s === 'suspended' || s === 'cancelled' ? 'outline' : 'secundario'}
                                        type="button"
                                        onClick={() => {
                                            onStatusChange(s);
                                            setShowStatusModal(false);
                                        }}
                                    >
                                        {HOSTING_STATUS_LABELS[s] || s}
                                    </Button>
                                ))}
                        </div>
                    </ModalBody>
                </Modal>
            )}

            {/* [304A-3] Modal de asignación de hosting a cliente */}
            {showAssignModal && isAdmin && onAssign && (
                <Modal abierto={showAssignModal} onCerrar={() => setShowAssignModal(false)}>
                    <ModalBody as="form" className="hostingCrearContenido" onSubmit={handleAssignSubmit}>
                        <p className="hostingStatusModalSub">
                            Hosting: <strong>{sub.domain || sub.client_name}</strong>
                        </p>
                        {sub.user_id && (
                            <p className="hostingStatusModalSub">
                                Actualmente asignado a: <strong>{sub.client_email}</strong>
                            </p>
                        )}
                        <Input
                            type="email"
                            placeholder="Email del cliente registrado"
                            value={assignEmail}
                            onChange={e => setAssignEmail(e.target.value)}
                            required
                        />
                        <p className="hostingFormNota">
                            El cliente debe tener cuenta registrada en el sistema.
                        </p>
                        <div className="modalAcciones">
                            <Button type="submit" tamano="pequeno" disabled={assignLoading || !assignEmail.trim()}>
                                {assignLoading ? 'Asignando…' : 'Asignar'}
                            </Button>
                        </div>
                    </ModalBody>
                </Modal>
            )}
        </>
    );
}
