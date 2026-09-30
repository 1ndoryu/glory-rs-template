/* [154A-10] Selector de iconos para enlaces de proyecto.
 * Popover con buscador y grid de iconos curados. Al seleccionar, setea el tipo de enlace.
 * Iconos soportados: social, plataformas de código, sitios comunes.
 * Usa lucide-react para los iconos — el mismo set que ProyectoIndividualIsland renderiza.
 * sentinel-disable-file html-nativo-en-vez-de-componente: Popover especializado con items de selección
 * y search input — <Button>/<Input> estándar no aplican a este patrón de icon picker.
 * sentinel-disable-file menu-contextual-artesanal: Es un icon picker popover, no un menú contextual. */
import React, { useState, useCallback, useRef, useEffect } from 'react';
import { useClickOutside } from '../../hooks/useClickOutside';
import {
    Globe, ExternalLink, Package, GitBranch,
    AtSign, Briefcase, PlayCircle, Camera, Share2,
    PenTool, CircleDot, Mail, FileText, Code, Play,
    Smartphone, MonitorPlay, Music, BookOpen, Search,
    ShoppingBag, Palette, Pen, MessageCircle, Hash
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import './IconPickerEnlace.css';

export interface TipoEnlace {
    id: string;
    label: string;
    icon: LucideIcon;
    keywords: string[];
}

/* [124A-SENT-R10] as const previene mutación accidental del catálogo de tipos de enlace.
 * También mejora el narrowing de tipos en TypeScript. */
export const TIPOS_ENLACE = [
    { id: 'web', label: 'Web', icon: Globe, keywords: ['sitio', 'website', 'pagina', 'url'] },
    { id: 'github', label: 'GitHub', icon: GitBranch, keywords: ['git', 'repo', 'repositorio', 'codigo'] },
    { id: 'npm', label: 'npm', icon: Package, keywords: ['paquete', 'libreria', 'node'] },
    { id: 'demo', label: 'Demo', icon: ExternalLink, keywords: ['preview', 'live', 'ver'] },
    { id: 'figma', label: 'Figma', icon: PenTool, keywords: ['diseño', 'design', 'ui', 'prototipo'] },
    { id: 'dribbble', label: 'Dribbble', icon: CircleDot, keywords: ['portfolio', 'shot'] },
    { id: 'behance', label: 'Behance', icon: Palette, keywords: ['portfolio', 'adobe'] },
    { id: 'twitter', label: 'Twitter / X', icon: AtSign, keywords: ['x', 'tweet', 'red social'] },
    { id: 'linkedin', label: 'LinkedIn', icon: Briefcase, keywords: ['profesional', 'empleo'] },
    { id: 'youtube', label: 'YouTube', icon: PlayCircle, keywords: ['video', 'canal', 'tutorial'] },
    { id: 'instagram', label: 'Instagram', icon: Camera, keywords: ['foto', 'reel', 'social'] },
    { id: 'facebook', label: 'Facebook', icon: Share2, keywords: ['meta', 'social', 'pagina'] },
    { id: 'discord', label: 'Discord', icon: MessageCircle, keywords: ['chat', 'servidor', 'comunidad'] },
    { id: 'slack', label: 'Slack', icon: Hash, keywords: ['chat', 'workspace'] },
    { id: 'email', label: 'Email', icon: Mail, keywords: ['correo', 'contacto', 'mail'] },
    { id: 'docs', label: 'Docs', icon: FileText, keywords: ['documentacion', 'guia', 'manual'] },
    { id: 'api', label: 'API', icon: Code, keywords: ['endpoint', 'swagger', 'rest'] },
    { id: 'playstore', label: 'Play Store', icon: Play, keywords: ['android', 'google', 'app'] },
    { id: 'appstore', label: 'App Store', icon: Smartphone, keywords: ['ios', 'apple', 'iphone'] },
    { id: 'video', label: 'Video', icon: MonitorPlay, keywords: ['screencast', 'grabacion'] },
    { id: 'podcast', label: 'Podcast', icon: Music, keywords: ['audio', 'episodio'] },
    { id: 'blog', label: 'Blog', icon: BookOpen, keywords: ['articulo', 'post'] },
    { id: 'tienda', label: 'Tienda', icon: ShoppingBag, keywords: ['shop', 'ecommerce', 'store'] },
    { id: 'git', label: 'Git', icon: GitBranch, keywords: ['branch', 'repositorio', 'version'] },
    { id: 'otro', label: 'Otro', icon: Pen, keywords: ['custom', 'personalizado'] },
] as const satisfies TipoEnlace[];

/* Mapa para lookup rápido por id. Typed con string key para aceptar valores dinámicos.
 * [higiene] Sin `export`: solo se usa en este módulo (.get en el render) y el
 * estado mutable exportado a nivel de módulo lo marcaba singleton-mutable-state. */
const TIPO_ENLACE_MAP = new Map<string, TipoEnlace>(TIPOS_ENLACE.map(t => [t.id, t]));

interface IconPickerEnlaceProps {
    value: string;
    onChange: (tipo: string) => void;
}

export const IconPickerEnlace: React.FC<IconPickerEnlaceProps> = ({ value, onChange }) => {
    const [abierto, setAbierto] = useState(false);
    const [busqueda, setBusqueda] = useState('');
    const contenedorRef = useRef<HTMLDivElement>(null);
    const inputRef = useRef<HTMLInputElement>(null);

    /* [124A-SENT-R7] Cerrar al hacer click fuera */
    useClickOutside(contenedorRef, () => setAbierto(false), abierto);

    /* Focus en el input de búsqueda al abrir */
    useEffect(() => {
        if (abierto && inputRef.current) {
            inputRef.current.focus();
        }
    }, [abierto]);

    const filtrados = busqueda.trim()
        ? TIPOS_ENLACE.filter(t => {
            const q = busqueda.toLowerCase();
            return t.id.includes(q)
                || t.label.toLowerCase().includes(q)
                || t.keywords.some(k => k.includes(q));
        })
        : TIPOS_ENLACE;

    const seleccionado = TIPO_ENLACE_MAP.get(value);
    const IconoActual = seleccionado?.icon || ExternalLink;

    const handleSelect = useCallback((tipo: string) => {
        onChange(tipo);
        setAbierto(false);
        setBusqueda('');
    }, [onChange]);

    return (
        <div className="iconPickerEnlace" ref={contenedorRef}>
            <button
                type="button"
                className="iconPickerEnlaceBoton"
                onClick={() => setAbierto(!abierto)}
                title={seleccionado?.label || value}
            >
                <IconoActual size={16} />
                <span className="iconPickerEnlaceTexto">{seleccionado?.label || value || 'Tipo'}</span>
            </button>

            {abierto && (
                <div className="iconPickerEnlacePopover">
                    <div className="iconPickerEnlaceBusqueda">
                        <Search size={14} />
                        <input
                            ref={inputRef}
                            type="text"
                            value={busqueda}
                            onChange={e => setBusqueda(e.target.value)}
                            placeholder="Buscar..."
                            className="iconPickerEnlaceInput"
                        />
                    </div>
                    <div className="iconPickerEnlaceGrid">
                        {filtrados.map(tipo => {
                            const Icono = tipo.icon;
                            return (
                                <button
                                    key={tipo.id}
                                    type="button"
                                    className={`iconPickerEnlaceFila ${value === tipo.id ? 'iconPickerEnlaceFila--activo' : ''}`}
                                    onClick={() => handleSelect(tipo.id)}
                                    title={tipo.label}
                                >
                                    <Icono size={16} />
                                    <span>{tipo.label}</span>
                                </button>
                            );
                        })}
                        {filtrados.length === 0 && (
                            <span className="iconPickerEnlaceVacio">Sin resultados</span>
                        )}
                    </div>
                </div>
            )}
        </div>
    );
};
