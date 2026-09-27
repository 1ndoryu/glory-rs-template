/* [259A-5] sentinel-disable-file menu-contextual-override-diseno: variantes por instancia via props API publica del DS; variantes canon al sistema en fase 5b. */
/* [205A-2] SelectDropdown — dropdown personalizado que reemplaza <Select> genérico.
 * Auto-contenido: gestiona su propio estado abierto/cerrado internamente.
 * Usar para toda selección de opción única en formularios.
 * NUNCA usar <Select> ni <select> nativo fuera de Select.tsx — Sentinel detecta ambos. */
import React, {useState} from 'react';
import {ChevronDown} from 'lucide-react';
import {MenuContextual} from './ContextMenu';
import './SelectDropdown.css';

export interface SelectDropdownOpcion {
    value: string;
    label: string;
}

interface SelectDropdownProps {
    value: string;
    opciones: SelectDropdownOpcion[];
    onChange: (value: string) => void;
    ariaLabel: string;
    className?: string;
    triggerClassName?: string;
    variante?: 'primario' | 'secundario' | 'outline' | 'texto';
    tamano?: 'pequeno' | 'mediano' | 'grande';
}

export const SelectDropdown: React.FC<SelectDropdownProps> = ({
    value,
    opciones,
    onChange,
    ariaLabel,
    className = '',
    triggerClassName = '',
    variante = 'outline',
    tamano = 'pequeno',
}) => {
    const [abierto, setAbierto] = useState(false);
    const seleccionada = opciones.find(o => o.value === value);

    const items = opciones.map(opt => ({
        id: opt.value,
        label: opt.label,
        onSelect: () => onChange(opt.value),
    }));

    return (
        <MenuContextual
            abierto={abierto}
            onToggle={() => setAbierto(prev => !prev)}
            onCerrar={() => setAbierto(false)}
            items={items}
            ariaLabel={ariaLabel}
            className={`selectDropdown ${className}`.trim()}
            triggerVariante={variante}
            triggerTamano={tamano}
            triggerContent={
                <>
                    <span className="selectDropdownLabel">{seleccionada?.label ?? value}</span>
                    <ChevronDown size={14} aria-hidden />
                </>
            }
            triggerClassName={triggerClassName}
        />
    );
};
