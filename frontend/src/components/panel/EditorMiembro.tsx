/* [074A-13] Editor modal para miembros del equipo.
 * [084A-8] Migrado a componente base <Modal> para consistencia
 * (focus trap, Escape listener, scroll lock, overlay). */
/* [259A-5 5b-6] sentinel-disable-file html-nativo-en-vez-de-componente: inputs y textarea
 * del formulario usan elementos nativos con clases locales (fuera de alcance 5b-6: solo botones/selects). */

import React from 'react';
import {useEditorMiembro} from '../../hooks/useEditorMiembro';
import {AdminTeamMember, CreateTeamMemberBody, UpdateTeamMemberBody} from '../../api/admin-team';
import {UploadImage} from '../ui/UploadImage';
import {Modal} from '../ui/Modal';
import {Button} from '../ui/Button';
import './EditorMiembro.css';

interface EditorMiembroProps {
    miembro: AdminTeamMember | null;
    abierto: boolean;
    guardando: boolean;
    onGuardar: (id: string | null, body: CreateTeamMemberBody | UpdateTeamMemberBody) => void;
    onCerrar: () => void;
}

export const EditorMiembro: React.FC<EditorMiembroProps> = ({miembro, abierto, guardando, onGuardar, onCerrar}) => {
    const editor = useEditorMiembro();

    React.useEffect(() => {
        if (miembro) {
            editor.cargarDesde(miembro);
        } else {
            editor.resetear();
        }
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [miembro]);

    if (!abierto) return null;

    const handleSubmit = () => {
        if (miembro) {
            onGuardar(miembro.id, editor.buildUpdateBody());
        } else {
            onGuardar(null, editor.buildCreateBody());
        }
    };

    return (
        <Modal abierto={abierto} onCerrar={onCerrar} className="modalMedio">
            <div className="editorMiembroContenido">
                <div className="modalCampo">
                    <label>Nombre</label>
                    <input type="text" value={editor.nombre} onChange={e => editor.setNombre(e.target.value)} placeholder="Nombre del miembro" />
                </div>
                <div className="modalCampo">
                    <label>Slug</label>
                    <input type="text" value={editor.slug} onChange={e => editor.setSlug(e.target.value)} placeholder="slug-unico" />
                </div>
                <div className="modalCampo">
                    <label>Cargo</label>
                    <input type="text" value={editor.cargo} onChange={e => editor.setCargo(e.target.value)} placeholder="CEO, Developer, etc." />
                </div>
                <div className="modalCampo">
                    <label>Bio</label>
                    <textarea value={editor.bio} onChange={e => editor.setBio(e.target.value)} placeholder="Descripción breve del miembro" rows={3} />
                </div>
                <div className="modalCampo">
                    <label>Avatar</label>
                    <UploadImage valor={editor.avatar} onChange={editor.setAvatar} etiqueta="Subir avatar" />
                </div>
                <div className="editorMiembroFila">
                    <div className="modalCampo">
                        <label>LinkedIn</label>
                        <input type="text" value={editor.linkedin} onChange={e => editor.setLinkedin(e.target.value)} placeholder="URL LinkedIn" />
                    </div>
                    <div className="modalCampo">
                        <label>GitHub</label>
                        <input type="text" value={editor.github} onChange={e => editor.setGithub(e.target.value)} placeholder="URL GitHub" />
                    </div>
                </div>
                <div className="editorMiembroFila">
                    <div className="modalCampo">
                        <label>Twitter</label>
                        <input type="text" value={editor.twitter} onChange={e => editor.setTwitter(e.target.value)} placeholder="URL Twitter/X" />
                    </div>
                </div>
                <div className="modalCampo">
                    <label>Estado</label>
                    <div className="editorMiembroEstados">
                        {['published', 'draft', 'archived'].map(s => (
                            <Button
                                key={s}
                                type="button"
                                variante="outline"
                                tamano="pequeno"
                                className={editor.status === s ? 'editorMiembroEstadoBtn--activo' : ''}
                                onClick={() => editor.setStatus(s)}
                            >
                                {s === 'published' ? 'Publicado' : s === 'draft' ? 'Borrador' : 'Archivado'}
                            </Button>
                        ))}
                    </div>
                </div>
            </div>
            <div className="modalAcciones">
                <Button variante="secundario" tamano="pequeno" type="button" onClick={onCerrar}>Cancelar</Button>
                <Button variante="primario" tamano="pequeno" type="button" onClick={handleSubmit} disabled={guardando || !editor.nombre || !editor.slug}>
                    {guardando ? 'Guardando...' : miembro ? 'Actualizar' : 'Crear'}
                </Button>
            </div>
        </Modal>
    );
};
