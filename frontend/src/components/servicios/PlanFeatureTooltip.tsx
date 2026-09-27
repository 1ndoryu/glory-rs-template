/* [259A-5] sentinel-disable-file componente-sin-hook-glory: logica pura sin estado/efectos (lookup de tooltip); nada que extraer a hook. */
import {Info} from 'lucide-react';

export type PlanFeatureTooltipContext = 'hosting' | 'wordpress' | 'vps';

interface PlanFeatureTooltipProps {
    feature: string;
    context: PlanFeatureTooltipContext;
}

function normalizeFeature(feature: string): string {
    return feature
        .normalize('NFD')
        .replace(/[\u0300-\u036f]/g, '')
        .toLocaleLowerCase();
}

function isObviousResourceLabel(normalized: string): boolean {
    return /\b\d+(?:[.,]\d+)?\s*(gb|tb|mb)\b/.test(normalized)
        && (normalized.includes('almacenamiento') || normalized.includes('storage') || normalized.includes('ram'));
}

function hostingTooltip(normalized: string, context: PlanFeatureTooltipContext): string | null {
    if (normalized.includes('wordpress pre')) {
        return 'El sitio se entrega con WordPress instalado y listo para entrar al panel inicial.';
    }
    if (normalized.includes('nginx administrado')) {
        return 'Servidor web gestionado para sitios estaticos, landings y frontends sin mantener Nginx por tu cuenta.';
    }
    if (normalized.includes('trafico ilimitado') || normalized.includes('unlimited traffic')) {
        return 'No hay cuota mensual fija de transferencia; se vigila uso abusivo para proteger la estabilidad del servidor.';
    }
    if (normalized.includes('free temporary domain')) {
        return 'Dominio temporal incluido para publicar y revisar el sitio antes de conectar tu dominio definitivo.';
    }
    if (normalized.includes('ssl') || normalized.includes('certificado')) {
        return 'Certificado HTTPS gestionado para que el sitio cargue cifrado cuando el dominio apunte correctamente.';
    }
    if (normalized.includes('free cdn') || normalized.includes('cdn')) {
        return 'Capa de cache y distribucion para servir archivos estaticos con menor latencia.';
    }
    if (normalized.includes('wp-cli') || normalized.includes('ssh')) {
        return context === 'wordpress'
            ? 'Acceso SSH con WP-CLI para tareas tecnicas de WordPress como cache, usuarios, plugins y mantenimiento.'
            : 'Acceso seguro para subir y administrar archivos del hosting sin exponer el panel del servidor.';
    }
    if (normalized.includes('sftp')) {
        return 'Acceso de archivos por SFTP con credenciales aisladas del resto de clientes.';
    }
    if (normalized.includes('backup')) {
        return normalized.includes('semanal') || normalized.includes('weekly')
            ? 'Se crea una copia automatica semanal de los archivos del sitio y, en WordPress, tambien de la base de datos.'
            : 'Se crean copias automaticas diarias y se conserva una copia semanal de referencia.';
    }
    if (normalized.includes('staging')) {
        return 'Entorno separado para probar cambios antes de llevarlos al sitio publico.';
    }
    if (normalized.includes('cache')) {
        return 'Configuracion orientada a reducir consultas repetidas y acelerar paginas de WordPress.';
    }
    if (normalized.includes('soporte prioritario') || normalized.includes('priority')) {
        return 'Tus incidencias operativas se atienden por delante de solicitudes no urgentes.';
    }
    if (normalized.includes('recursos aislados') || normalized.includes('recursos ampliados')) {
        return 'El sitio corre en su propio stack con limites de CPU y memoria definidos para evitar interferencias.';
    }
    return null;
}

function vpsTooltip(normalized: string): string | null {
    if (normalized.includes('vcpu')) {
        return 'Capacidad de CPU asignada al VPS para procesos, servidores web, bases de datos y workers.';
    }
    if (normalized.includes('puerto') || normalized.includes('mbit') || normalized.includes('gbit')) {
        return 'Velocidad maxima del enlace de red del servidor; afecta picos de descarga, backups y trafico concurrente.';
    }
    if (normalized.includes('trafico ilimitado') || normalized.includes('unlimited traffic')) {
        return 'Transferencia mensual sin cuota fija, con politica de uso razonable para mantener la red estable.';
    }
    if (normalized.includes('root') || normalized.includes('ssh')) {
        return 'Acceso administrativo completo para instalar paquetes, configurar servicios y desplegar tus aplicaciones.';
    }
    return null;
}

export function getPlanFeatureTooltip(feature: string, context: PlanFeatureTooltipContext): string | null {
    const normalized = normalizeFeature(feature);
    if (isObviousResourceLabel(normalized)) {
        return null;
    }
    return context === 'vps' ? vpsTooltip(normalized) : hostingTooltip(normalized, context);
}

export function PlanFeatureTooltip({feature, context}: PlanFeatureTooltipProps): JSX.Element {
    const tooltip = getPlanFeatureTooltip(feature, context);

    return (
        <span className="tarjetaPlanItemTexto planFeatureTooltipText">
            {feature}
            {tooltip && (
                <span className="planFeatureTooltipTrigger" tabIndex={0} aria-label={tooltip}>
                    <Info size={13} strokeWidth={2} aria-hidden="true" />
                    <span className="planFeatureTooltipBubble" role="tooltip">{tooltip}</span>
                </span>
            )}
        </span>
    );
}