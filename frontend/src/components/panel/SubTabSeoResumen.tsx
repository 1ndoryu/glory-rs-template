/* [SEO-A] Sub-tab Resumen: tarjetas de conteo + issues críticos. */
import React from 'react';
import type {SeoAuditSummary, SeoPageEntry} from '../../api/admin-seo';

interface Props {
    summary: SeoAuditSummary;
    pages: SeoPageEntry[];
}

export const SubTabSeoResumen: React.FC<Props> = ({summary, pages}) => {
    const criticalIssues = pages
        .filter(p => p.status === 'error' || p.status === 'warning')
        .flatMap(p => p.issues.map(issue => ({path: p.path, label: p.label, issue, status: p.status})))
        .slice(0, 10);

    return (
        <>
            <div className="seoResumenTarjetas">
                <div className="seoResumenTarjeta">
                    <span className="seoResumenValor">{summary.total_pages}</span>
                    <span className="seoResumenLabel">Páginas</span>
                </div>
                <div className="seoResumenTarjeta seoResumenTarjeta--error">
                    <span className="seoResumenValor">{summary.errors}</span>
                    <span className="seoResumenLabel">Problemas</span>
                </div>
                <div className="seoResumenTarjeta">
                    <span className="seoResumenValor">{summary.warnings}</span>
                    <span className="seoResumenLabel">Mejorables</span>
                </div>
                <div className="seoResumenTarjeta">
                    <span className="seoResumenValor">{summary.ok}</span>
                    <span className="seoResumenLabel">OK</span>
                </div>
            </div>
            <div className="seoResumenTarjetas">
                <div className="seoResumenTarjeta">
                    <span className="seoResumenValor">{summary.services_active}</span>
                    <span className="seoResumenLabel">Servicios</span>
                </div>
                <div className="seoResumenTarjeta">
                    <span className="seoResumenValor">{summary.projects_published}</span>
                    <span className="seoResumenLabel">Proyectos</span>
                </div>
                <div className="seoResumenTarjeta">
                    <span className="seoResumenValor">{summary.blog_published}</span>
                    <span className="seoResumenLabel">Blog posts</span>
                </div>
            </div>
            {criticalIssues.length > 0 && (
                <>
                    <h3 style={{fontSize: 'var(--text-sm)', color: 'var(--text-primary)', margin: 'var(--spacing-md) 0 var(--spacing-sm)'}}>
                        Issues ({criticalIssues.length})
                    </h3>
                    <div className="seoIssuesLista">
                        {criticalIssues.map((item, i) => (
                            <div key={`${item.path}-${item.issue}-${i}`} className="seoIssueFila">
                                <span className={`seoIssueBadge ${item.status === 'error' ? '' : ''}`}>
                                    {item.issue.replace(/_/g, ' ').toLowerCase()}
                                </span>
                                <span className="seoIssuePath">{item.label}</span>
                                <span className="seoIssueMensaje">{item.path}</span>
                            </div>
                        ))}
                    </div>
                </>
            )}
        </>
    );
};
