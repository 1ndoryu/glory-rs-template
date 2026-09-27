/* [074A-66] Tab de edición de planes dentro de EditorServicio.
 * Cada plan es una card expandible con campos editables inline.
 * Las fases se gestionan dentro de cada plan expandido.
 * sentinel-disable-file html-nativo-en-vez-de-componente: solo los checkboxes usan
 * <input> nativo (sin componente Checkbox en el DS); botones ya migrados a <Button>.
 * sentinel-disable-file componente-sin-hook: Estado vive en el padre (planes prop),
 * los callbacks son wiring trivial que no justifica un hook separado.
 * sentinel-disable-file limite-lineas: Editor de planes con cards expandibles inline;
 * refactor generaría prop-drilling excesivo sin ganancia real de mantenibilidad. */
import React, {useCallback} from 'react';
import {Input} from '../ui/Input';
import {Textarea} from '../ui/Textarea';
import {Button} from '../ui/Button';
import {Plus, Trash2, ChevronDown, ChevronUp, Star} from 'lucide-react';
import type {PlanEditable, PhaseEditable} from '../../hooks/useEditorServicio';
import './TabPlanes.css';

interface TabPlanesProps {
    planes: PlanEditable[];
    onChange: (planes: PlanEditable[]) => void;
}

let keyCounter = 0;
function nextKey(): string {
    keyCounter += 1;
    return `new-${keyCounter}-${Date.now()}`;
}

function crearPlanVacio(): PlanEditable {
    return {
        key: nextKey(),
        slug: '',
        name: '',
        priceCents: 0,
        description: '',
        features: [],
        isHighlighted: false,
        isCustom: false,
        sortOrder: 0,
        phases: [],
    };
}

function crearFaseVacia(num: number): PhaseEditable {
    return {
        phaseNumber: num,
        title: '',
        description: '',
        percentageOfTotal: 0,
        estimatedDays: 7,
        maxRevisions: 2,
    };
}

export const TabPlanes: React.FC<TabPlanesProps> = ({planes, onChange}) => {
    const [expandido, setExpandido] = React.useState<string | null>(null);

    const actualizarPlan = useCallback((key: string, patch: Partial<PlanEditable>) => {
        onChange(planes.map(p => p.key === key ? {...p, ...patch} : p));
    }, [planes, onChange]);

    const agregarPlan = useCallback(() => {
        onChange([...planes, crearPlanVacio()]);
    }, [planes, onChange]);

    const eliminarPlan = useCallback((key: string) => {
        onChange(planes.filter(p => p.key !== key));
    }, [planes, onChange]);

    const toggleExpandido = useCallback((key: string) => {
        setExpandido(prev => prev === key ? null : key);
    }, []);

    const actualizarFeature = useCallback((planKey: string, idx: number, valor: string) => {
        const plan = planes.find(p => p.key === planKey);
        if (!plan) return;
        const features = [...plan.features];
        features[idx] = valor;
        actualizarPlan(planKey, {features});
    }, [planes, actualizarPlan]);

    const agregarFeature = useCallback((planKey: string) => {
        const plan = planes.find(p => p.key === planKey);
        if (!plan) return;
        actualizarPlan(planKey, {features: [...plan.features, '']});
    }, [planes, actualizarPlan]);

    const eliminarFeature = useCallback((planKey: string, idx: number) => {
        const plan = planes.find(p => p.key === planKey);
        if (!plan) return;
        actualizarPlan(planKey, {features: plan.features.filter((_, i) => i !== idx)});
    }, [planes, actualizarPlan]);

    const actualizarFase = useCallback((planKey: string, phaseNum: number, patch: Partial<PhaseEditable>) => {
        const plan = planes.find(p => p.key === planKey);
        if (!plan) return;
        const phases = plan.phases.map(ph =>
            ph.phaseNumber === phaseNum ? {...ph, ...patch} : ph
        );
        actualizarPlan(planKey, {phases});
    }, [planes, actualizarPlan]);

    /* [035A-30] Auto-balance: cuando % de una fase cambia, ajusta el resto
     * proporcionalmente para que el total siempre sea 100.
     * Gotcha: rounding errors se corrigen en la última fase "otra". */
    const actualizarPorcentajeFase = useCallback((planKey: string, phaseNum: number, newPct: number) => {
        const plan = planes.find(p => p.key === planKey);
        if (!plan) return;
        if (plan.phases.length <= 1) {
            actualizarFase(planKey, phaseNum, {percentageOfTotal: newPct});
            return;
        }
        const others = plan.phases.filter(ph => ph.phaseNumber !== phaseNum);
        const remaining = Math.max(0, 100 - newPct);
        const othersSum = others.reduce((s, ph) => s + ph.percentageOfTotal, 0);
        let updatedPhases = plan.phases.map(ph => {
            if (ph.phaseNumber === phaseNum) return {...ph, percentageOfTotal: newPct};
            const scaled = othersSum > 0
                ? Math.round((ph.percentageOfTotal / othersSum) * remaining)
                : Math.round(remaining / others.length);
            return {...ph, percentageOfTotal: scaled};
        });
        /* Corrige diferencia de redondeo en la última fase distinta */
        const total = updatedPhases.reduce((s, ph) => s + ph.percentageOfTotal, 0);
        const gap = 100 - total;
        if (gap !== 0) {
            const lastOtherNum = others[others.length - 1].phaseNumber;
            updatedPhases = updatedPhases.map(ph =>
                ph.phaseNumber === lastOtherNum
                    ? {...ph, percentageOfTotal: Math.max(0, ph.percentageOfTotal + gap)}
                    : ph
            );
        }
        actualizarPlan(planKey, {phases: updatedPhases});
    }, [planes, actualizarPlan, actualizarFase]);

    const agregarFase = useCallback((planKey: string) => {
        const plan = planes.find(p => p.key === planKey);
        if (!plan) return;
        const nextNum = plan.phases.length > 0
            ? Math.max(...plan.phases.map(ph => ph.phaseNumber)) + 1
            : 1;
        actualizarPlan(planKey, {phases: [...plan.phases, crearFaseVacia(nextNum)]});
    }, [planes, actualizarPlan]);

    const eliminarFase = useCallback((planKey: string, phaseNum: number) => {
        const plan = planes.find(p => p.key === planKey);
        if (!plan) return;
        actualizarPlan(planKey, {phases: plan.phases.filter(ph => ph.phaseNumber !== phaseNum)});
    }, [planes, actualizarPlan]);

    return (
        <div className="tabPlanes">
            {planes.length === 0 && (
                <p className="tabPlanesSinPlanes">No hay planes configurados.</p>
            )}

            {planes.map(plan => {
                const abierto = expandido === plan.key;
                return (
                    <div key={plan.key} className={`tabPlanCard ${abierto ? 'tabPlanCard--abierto' : ''}`}>
                        <div className="tabPlanCardHeader" onClick={() => toggleExpandido(plan.key)}>
                            <div className="tabPlanCardInfo">
                                {plan.isHighlighted && <Star size={14} className="tabPlanStar" />}
                                <span className="tabPlanCardNombre">{plan.name || 'Plan sin nombre'}</span>
                                <span className="tabPlanCardPrecio">
                                    ${(plan.priceCents / 100).toFixed(2)}
                                </span>
                            </div>
                            <div className="tabPlanCardAcciones">
                                <Button
                                    type="button"
                                    variante="texto"
                                    tamano="pequeno"
                                    className="tabPlanBtnIcono tabPlanBtnEliminar"
                                    onClick={e => { e.stopPropagation(); eliminarPlan(plan.key); }}
                                    title="Eliminar plan"
                                >
                                    <Trash2 size={14} />
                                </Button>
                                {abierto ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
                            </div>
                        </div>

                        {abierto && (
                            <div className="tabPlanCardBody">
                                <div className="tabPlanFila">
                                    <label className="tabPlanLabel">
                                        Nombre
                                        <Input
                                            value={plan.name}
                                            onChange={e => actualizarPlan(plan.key, {name: e.target.value})}
                                            placeholder="Ej: Básico, Pro, Enterprise"
                                        />
                                    </label>
                                    <label className="tabPlanLabel">
                                        Slug
                                        <Input
                                            value={plan.slug}
                                            onChange={e => actualizarPlan(plan.key, {slug: e.target.value})}
                                            placeholder="basico"
                                        />
                                    </label>
                                </div>

                                <div className="tabPlanFila">
                                    <label className="tabPlanLabel">
                                        Precio (cents)
                                        <Input
                                            type="number"
                                            value={plan.priceCents}
                                            onChange={e => actualizarPlan(plan.key, {priceCents: Number(e.target.value)})}
                                            min={0}
                                        />
                                    </label>
                                    <label className="tabPlanLabelCheck">
                                        <input
                                            type="checkbox"
                                            checked={plan.isHighlighted}
                                            onChange={e => actualizarPlan(plan.key, {isHighlighted: e.target.checked})}
                                        />
                                        Destacado
                                    </label>
                                    <label className="tabPlanLabelCheck">
                                        <input
                                            type="checkbox"
                                            checked={plan.isCustom}
                                            onChange={e => actualizarPlan(plan.key, {isCustom: e.target.checked})}
                                        />
                                        Personalizado
                                    </label>
                                </div>

                                <label className="tabPlanLabel">
                                    Descripción
                                    <Textarea
                                        className="tabPlanTextarea"
                                        value={plan.description}
                                        onChange={e => actualizarPlan(plan.key, {description: e.target.value})}
                                        placeholder="Descripción breve del plan"
                                        rows={2}
                                    />
                                </label>

                                <div className="tabPlanSeccion">
                                    <span className="tabPlanSeccionTitulo">Características</span>
                                    {/* sentinel-disable-next-line key-index-lista: features are plain strings without IDs, idx is the only stable key */}
                                    {plan.features.map((feat, idx) => (
                                        <div key={`${plan.key}-${idx}`} className="tabPlanFeatureFila">
                                            <Input
                                                value={feat}
                                                onChange={e => actualizarFeature(plan.key, idx, e.target.value)}
                                                placeholder={`Característica ${idx + 1}`}
                                            />
                                            <Button
                                                type="button"
                                                variante="texto"
                                                tamano="pequeno"
                                                className="tabPlanBtnIcono tabPlanBtnEliminar"
                                                onClick={() => eliminarFeature(plan.key, idx)}
                                            >
                                                <Trash2 size={12} />
                                            </Button>
                                        </div>
                                    ))}
                                    <Button
                                        type="button"
                                        variante="outline"
                                        tamano="pequeno"
                                        className="tabPlanBtnAgregar"
                                        onClick={() => agregarFeature(plan.key)}
                                    >
                                        <Plus size={12} /> Característica
                                    </Button>
                                </div>

                                <div className="tabPlanSeccion">
                                    <span className="tabPlanSeccionTitulo">Fases</span>
                                    {plan.phases.map(fase => (
                                        <div key={fase.phaseNumber} className="tabPlanFase">
                                            <div className="tabPlanFaseHeader">
                                                <span className="tabPlanFaseNum">Fase {fase.phaseNumber}</span>
                                                <Button
                                                    type="button"
                                                    variante="texto"
                                                    tamano="pequeno"
                                                    className="tabPlanBtnIcono tabPlanBtnEliminar"
                                                    onClick={() => eliminarFase(plan.key, fase.phaseNumber)}
                                                >
                                                    <Trash2 size={12} />
                                                </Button>
                                            </div>
                                            <div className="tabPlanFila">
                                                <label className="tabPlanLabel">
                                                    Título
                                                    <Input
                                                        value={fase.title}
                                                        onChange={e => actualizarFase(plan.key, fase.phaseNumber, {title: e.target.value})}
                                                        placeholder="Nombre de la fase"
                                                    />
                                                </label>
                                                <label className="tabPlanLabel tabPlanLabelCorto">
                                                    % Total
                                                    <Input
                                                        type="number"
                                                        value={fase.percentageOfTotal}
                                                        onChange={e => actualizarPorcentajeFase(plan.key, fase.phaseNumber, Number(e.target.value))}
                                                        min={0}
                                                        max={100}
                                                    />
                                                </label>
                                            </div>
                                            <div className="tabPlanFila">
                                                <label className="tabPlanLabel tabPlanLabelCorto">
                                                    Días est.
                                                    <Input
                                                        type="number"
                                                        value={fase.estimatedDays}
                                                        onChange={e => actualizarFase(plan.key, fase.phaseNumber, {estimatedDays: Number(e.target.value)})}
                                                        min={1}
                                                    />
                                                </label>
                                                <label className="tabPlanLabel tabPlanLabelCorto">
                                                    Max revisiones
                                                    <Input
                                                        type="number"
                                                        value={fase.maxRevisions}
                                                        onChange={e => actualizarFase(plan.key, fase.phaseNumber, {maxRevisions: Number(e.target.value)})}
                                                        min={0}
                                                    />
                                                </label>
                                            </div>
                                        </div>
                                    ))}
                                    <Button
                                        type="button"
                                        variante="outline"
                                        tamano="pequeno"
                                        className="tabPlanBtnAgregar"
                                        onClick={() => agregarFase(plan.key)}
                                    >
                                        <Plus size={12} /> Fase
                                    </Button>
                                </div>
                            </div>
                        )}
                    </div>
                );
            })}

            <Button
                variante="secundario"
                tamano="pequeno"
                onClick={agregarPlan}
                className="tabPlanesBtnNuevo"
            >
                <Plus size={14} /> Agregar plan
            </Button>
        </div>
    );
};
