/* [283A-20] Campana de notificaciones en el header.
 * Muestra badge con conteo de no leídas. Al hacer click, abre popover
 * con las últimas notificaciones. Permite marcar como leídas. */

import { Bell, CheckCheck } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { useNotificacionStore } from '@/stores/notificacionStore';
import axios from '@/api/axios-instance';
import { formatDistanceToNow } from 'date-fns';
import { es } from 'date-fns/locale';

export function NotificationBell() {
    const items = useNotificacionStore((s) => s.items);
    const noLeidas = useNotificacionStore((s) => s.noLeidas);
    const marcarLeida = useNotificacionStore((s) => s.marcarLeida);
    const marcarTodasLeidas = useNotificacionStore((s) => s.marcarTodasLeidas);

    async function handleMarcarLeida(id: string) {
        try {
            await axios.patch(`/api/notificaciones/${id}/leer`);
            marcarLeida(id);
        } catch { /* toast de error ya maneja el interceptor 401 */ }
    }

    async function handleMarcarTodas() {
        try {
            await axios.patch('/api/notificaciones/leer-todas');
            marcarTodasLeidas();
        } catch { /* idem */ }
    }

    return (
        <Popover>
            <PopoverTrigger asChild>
                <Button variant="ghost" size="icon" className="relative">
                    <Bell className="h-5 w-5" />
                    {noLeidas > 0 && (
                        <span className="absolute -top-1 -right-1 flex h-5 w-5 items-center justify-center rounded-full bg-destructive text-[10px] font-bold text-destructive-foreground">
                            {noLeidas > 99 ? '99+' : noLeidas}
                        </span>
                    )}
                </Button>
            </PopoverTrigger>
            <PopoverContent className="w-80 p-0" align="end">
                <div className="flex items-center justify-between border-b px-4 py-3">
                    <h4 className="text-sm font-semibold">Notificaciones</h4>
                    {noLeidas > 0 && (
                        <Button
                            variant="ghost"
                            className="h-auto px-2 py-1 text-xs"
                            onClick={handleMarcarTodas}
                        >
                            <CheckCheck className="mr-1 h-4 w-4" />
                            Marcar todas
                        </Button>
                    )}
                </div>
                <div className="max-h-80 overflow-y-auto">
                    {items.length === 0 ? (
                        <p className="px-4 py-6 text-center text-sm text-muted-foreground">
                            Sin notificaciones
                        </p>
                    ) : (
                        items.map((n) => (
                            <button
                                key={n.id}
                                type="button"
                                className={`flex w-full flex-col gap-0.5 border-b px-4 py-3 text-left transition-colors hover:bg-muted/50 ${
                                    !n.leida ? 'bg-muted/30' : ''
                                }`}
                                onClick={() => {
                                    if (!n.leida) handleMarcarLeida(n.id);
                                }}
                            >
                                <div className="flex items-center gap-2">
                                    {!n.leida && (
                                        <span className="h-2 w-2 shrink-0 rounded-full bg-primary" />
                                    )}
                                    <span className="text-sm font-medium truncate">{n.titulo}</span>
                                </div>
                                <p className="text-xs text-muted-foreground line-clamp-2">{n.mensaje}</p>
                                <span className="text-[10px] text-muted-foreground/70">
                                    {formatDistanceToNow(new Date(n.created_at), { addSuffix: true, locale: es })}
                                </span>
                            </button>
                        ))
                    )}
                </div>
            </PopoverContent>
        </Popover>
    );
}
