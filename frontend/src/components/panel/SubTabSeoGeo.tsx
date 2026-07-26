/* [SEO-A] Sub-tab GEO: checklist de tareas de visibilidad en IA. */
import React from 'react';
import {Check, X} from 'lucide-react';
import type {GeoCheck} from '../../api/admin-seo';

interface Props {
    checks: GeoCheck[];
}

export const SubTabSeoGeo: React.FC<Props> = ({checks}) => {
    if (checks.length === 0) {
        return <div className="contenidoPlaceholder"><span className="contenidoPlaceholderTexto">Sin checks GEO configurados</span></div>;
    }

    return (
        <div className="seoGeoLista">
            {checks.map(check => (
                <div key={check.id} className="seoGeoFila">
                    <span className={`seoGeoIcono ${check.passed ? 'seoGeoIcono--ok' : 'seoGeoIcono--fail'}`}>
                        {check.passed ? <Check size={16} /> : <X size={16} />}
                    </span>
                    <div className="seoGeoInfo">
                        <span className="seoGeoLabel">{check.label}</span>
                        {check.detail && <span className="seoGeoDetalle">{check.detail}</span>}
                    </div>
                </div>
            ))}
        </div>
    );
};
