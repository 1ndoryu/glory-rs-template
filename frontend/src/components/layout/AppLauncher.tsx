/* [259A-5] sentinel-disable-file menu-contextual-override-diseno: variantes por instancia via props API publica del DS; variantes canon al sistema en fase 5b. */
import {useState} from 'react';
import {Music2, Server} from 'lucide-react';
import {MenuContextual} from '../ui/ContextMenu';

const APPS_NAKOMI = [
    {id: 'catask', nombre: 'Catask', url: 'https://catask.nakomi.studio', icono: <img src="/assets/icons/catask.svg" alt="" className="menuContextualAppsLogo" />},
    {id: 'vps', nombre: 'VPS', url: 'https://vps.nakomi.studio', icono: <Server size={18} />},
    {id: 'kamples', nombre: 'Kamples', url: 'https://kamples.com', icono: <Music2 size={18} />},
];

export function AppLauncher() {
    const [abierto, setAbierto] = useState(false);

    return (
        <MenuContextual
            abierto={abierto}
            onToggle={() => setAbierto(prev => !prev)}
            onCerrar={() => setAbierto(false)}
            ariaLabel="Aplicaciones Nakomi"
            tipo="apps"
            triggerContent={<img src="/assets/icons/apps.svg" alt="" className="menuContextualAppsTriggerLogo" aria-hidden="true" />}
        >
            {/* [105A-38] Launcher de apps junto al avatar: mantiene el patrón MenuContextual
             * y evita reintroducir el botón de chat en la navegación principal. */}
            {/* [155A-4] Catask usa SVG propio negro y el trigger usa apps.svg. */}
            <div className="menuContextualAppsGrid">
                {APPS_NAKOMI.map(app => (
                    <a
                        key={app.id}
                        className="menuContextualAppsItem"
                        href={app.url}
                        onClick={() => setAbierto(false)}
                    >
                        <span className="menuContextualAppsIcono">{app.icono}</span>
                        <span className="menuContextualAppsNombre">{app.nombre}</span>
                    </a>
                ))}
            </div>
        </MenuContextual>
    );
}