/* [016A-3] Modal para que el admin asigne una orden a un empleado.
 * [20CA-4] Campo de búsqueda para filtrar freelancers por email o especialidad.
 * Carga lista de empleados desde GET /api/admin/employees.
 * Al confirmar hace PUT /api/orders/:id/assign/:employeeId. */

import { useState, useMemo } from 'react';
import { Loader2, Search } from 'lucide-react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiListEmployees, apiAssignOrder, type EmployeeListItem } from '../../api/assignment';
import { Modal, ModalBody, ModalField, ModalLabel } from '../ui/Modal';
import { Input } from '../ui/Input';
import { Button } from '../ui/Button';
import './ModalAsignar.css';

interface ModalAsignarProps {
    orderId: string;
    orderNumber: number;
    abierto: boolean;
    onCerrar: () => void;
    onAsignado: () => void;
}

export function ModalAsignar({ orderId, orderNumber, abierto, onCerrar, onAsignado }: ModalAsignarProps) {
    const [seleccionado, setSeleccionado] = useState<string | null>(null);
    const [busqueda, setBusqueda] = useState('');
    const queryClient = useQueryClient();

    const { data: empleados = [], isLoading } = useQuery<EmployeeListItem[]>({
        queryKey: ['admin-employees'],
        queryFn: apiListEmployees,
        enabled: abierto,
    });

    /* [20CA-4] Filtrar por email o especialidades */
    const filtrados = useMemo(() => {
        if (!busqueda.trim()) return empleados;
        const q = busqueda.toLowerCase();
        return empleados.filter(e =>
            e.email.toLowerCase().includes(q)
            || e.specialties.some(s => s.toLowerCase().includes(q))
        );
    }, [empleados, busqueda]);

    const asignar = useMutation({
        mutationFn: () => apiAssignOrder(orderId, seleccionado!),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['ordenes'] });
            queryClient.invalidateQueries({ queryKey: ['orden-detalle', orderId] });
            onAsignado();
        },
    });

    return (
        <Modal abierto={abierto} onCerrar={onCerrar} className="modalMedio">

            <ModalBody>
                <div className="modalAsignarHeader">
                    <span className="modalAsignarOrden">Orden #{orderNumber}</span>
                </div>
                {isLoading ? (
                    <div className="modalAsignarCargando">
                        <Loader2 className="modalAsignarSpinner" size={24} />
                    </div>
                ) : empleados.length === 0 ? (
                    <p className="modalAsignarVacio">No hay freelancers disponibles</p>
                ) : (
                    <ModalField>
                        <ModalLabel>Seleccionar freelancer</ModalLabel>
                        <div className="modalAsignarBusqueda">
                            <Search size={16} className="modalAsignarBusquedaIcono" />
                            <Input
                                variante="outline"
                                placeholder="Buscar por email o especialidad..."
                                value={busqueda}
                                onChange={e => setBusqueda(e.target.value)}
                                className="modalAsignarBusquedaInput"
                            />
                        </div>
                        {filtrados.length === 0 ? (
                            <p className="modalAsignarVacio">Sin resultados para "{busqueda}"</p>
                        ) : (
                            <ul className="modalAsignarLista" role="listbox">
                                {filtrados.map(emp => (
                                <li key={emp.user_id} role="option" aria-selected={seleccionado === emp.user_id}>
                                    <Button
                                        type="button"
                                        variante={seleccionado === emp.user_id ? 'secundario' : 'outline'}
                                        tamano="pequeno"
                                        className="modalAsignarItem"
                                        onClick={() => setSeleccionado(emp.user_id)}
                                    >
                                        <div className="modalAsignarItemInfo">
                                            <span className="modalAsignarNombre">{emp.email}</span>
                                            {emp.specialties.length > 0 && (
                                                <span className="modalAsignarEspecialidades">
                                                    {emp.specialties.join(', ')}
                                                </span>
                                            )}
                                        </div>
                                        <span className="modalAsignarCarga">
                                            {emp.current_orders}/{emp.max_concurrent_orders} órdenes
                                        </span>
                                    </Button>
                                </li>
                            ))}
                        </ul>
                        )}
                    </ModalField>
                )}
            </ModalBody>

            <div className="modalAcciones">
                <Button
                    variante="primario"
                    tamano="pequeno"
                    disabled={!seleccionado || asignar.isPending}
                    onClick={() => asignar.mutate()}
                >
                    {asignar.isPending ? <Loader2 className="modalAsignarSpinner" size={14} /> : 'Confirmar asignación'}
                </Button>
                <Button variante="texto" tamano="pequeno" onClick={onCerrar}>
                    Cancelar
                </Button>
                {asignar.isError && (
                    <span className="modalAsignarError">Error al asignar</span>
                )}
            </div>
        </Modal>
    );
}
