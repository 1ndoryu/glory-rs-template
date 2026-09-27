/* [044A-43] Hook de perfil: carga perfil desde backend, sube avatar.
 * [074A-23] handleGuardar conectado a PATCH /api/profile (antes era stub).
 * [205A-2] Añade cambio de contraseña desde configuración, separado del perfil general. */
import {useState, useEffect} from 'react';
import {useQueryClient} from '@tanstack/react-query';
import {extraerMensajeError} from '../api/auth';
import {subirAvatar, actualizarPerfil, cambiarPasswordPerfil} from '../api/profile';
import type {PerfilResponse} from '../api/profile';
import {currentProfileKey, useCurrentProfile} from './useCurrentProfile';
import {useAuthStore} from '../stores/authStore';

interface EstadoPerfil {
    email: string;
    nombre: string;
    descripcion: string;
    linkedin: string;
    twitter: string;
    website: string;
}

interface EstadoPassword {
    actual: string;
    nueva: string;
    confirmar: string;
}

/* [259A-5] Retorno segregado por area (ISP, regla large-interface-isp).
 * El hook sigue devolviendo RetornoUsePerfil plano via extends. */
interface PerfilDatos {
    estado: EstadoPerfil;
    perfil: PerfilResponse | null;
    avatarUrl: string;
}

interface PerfilGuardado {
    guardado: boolean;
    guardando: boolean;
    errorGuardar: string | null;
    cargando: boolean;
    actualizarCampo: (campo: keyof EstadoPerfil, valor: string) => void;
    guardarPerfil: () => Promise<boolean>;
}

interface PerfilAvatar {
    subiendoAvatar: boolean;
    handleSubirAvatar: (archivo: File) => Promise<void>;
}

interface PerfilPassword {
    estadoPassword: EstadoPassword;
    passwordActualizada: boolean;
    guardandoPassword: boolean;
    errorPassword: string | null;
    actualizarPasswordCampo: (campo: keyof EstadoPassword, valor: string) => void;
    handleGuardarPassword: (e: React.FormEvent) => void;
}

interface RetornoUsePerfil extends PerfilDatos, PerfilGuardado, PerfilAvatar, PerfilPassword {
}

export const usePerfil = (): RetornoUsePerfil => {
    const queryClient = useQueryClient();
    const userId = useAuthStore(s => s.user?.userId);
    const actualizarEmailAuth = useAuthStore(s => s.actualizarEmail);
    const {perfil, cargando: cargandoPerfil, avatarUrl} = useCurrentProfile();
    const [subiendoAvatar, setSubiendoAvatar] = useState(false);

    const [estado, setEstado] = useState<EstadoPerfil>({
        email: '',
        nombre: '',
        descripcion: '',
        linkedin: '',
        twitter: '',
        website: ''
    });

    const [guardado, setGuardado] = useState(false);
    const [guardando, setGuardando] = useState(false);
    const [errorGuardar, setErrorGuardar] = useState<string | null>(null);
    const [estadoPassword, setEstadoPassword] = useState<EstadoPassword>({
        actual: '',
        nueva: '',
        confirmar: ''
    });
    const [passwordActualizada, setPasswordActualizada] = useState(false);
    const [guardandoPassword, setGuardandoPassword] = useState(false);
    const [errorPassword, setErrorPassword] = useState<string | null>(null);

    /* Sincroniza los datos editables cuando el backend devuelve el perfil actual */
    useEffect(() => {
        if (!perfil) return;
        setEstado(prev => ({
            ...prev,
            email: prev.email || perfil.email || '',
            nombre: prev.nombre || perfil.display_name || ''
        }));
    }, [perfil]);

    const actualizarCampo = (campo: keyof EstadoPerfil, valor: string) => {
        setEstado(prev => ({...prev, [campo]: valor}));
    };

    const actualizarPasswordCampo = (campo: keyof EstadoPassword, valor: string) => {
        setEstadoPassword(prev => ({...prev, [campo]: valor}));
    };

    /* [074A-23] Guardar perfil llamando PATCH /api/profile */
    const guardarPerfil = async (): Promise<boolean> => {
        setGuardando(true);
        setErrorGuardar(null);
        try {
            const resp = await actualizarPerfil({
                email: estado.email.trim() || undefined,
                display_name: estado.nombre || undefined,
                bio: estado.descripcion || undefined,
                linkedin: estado.linkedin || undefined,
                twitter: estado.twitter || undefined,
                website: estado.website || undefined,
            });
            queryClient.setQueryData<PerfilResponse | undefined>(
                currentProfileKey(userId),
                (prev) => prev ? {...prev, email: resp.email, display_name: resp.display_name} : prev,
            );
            actualizarEmailAuth(resp.email);
            setEstado(prev => ({...prev, email: resp.email}));
            setGuardado(true);
            setTimeout(() => setGuardado(false), 3000);
            return true;
        } catch (err: unknown) {
            const axiosData = (err as { response?: { data?: { message?: string } } })?.response?.data;
            setErrorGuardar(axiosData?.message ?? (err instanceof Error ? err.message : 'Error guardando perfil'));
            return false;
        } finally {
            setGuardando(false);
        }
    };

    const handleGuardarPassword = async (e: React.FormEvent) => {
        e.preventDefault();
        setGuardandoPassword(true);
        setErrorPassword(null);
        setPasswordActualizada(false);
        try {
            if (!estadoPassword.actual || !estadoPassword.nueva || !estadoPassword.confirmar) {
                throw new Error('Completa los tres campos de contraseña.');
            }
            if (estadoPassword.nueva.length < 8) {
                throw new Error('La nueva contraseña debe tener al menos 8 caracteres.');
            }
            if (estadoPassword.nueva !== estadoPassword.confirmar) {
                throw new Error('La confirmación de la nueva contraseña no coincide.');
            }
            if (estadoPassword.actual === estadoPassword.nueva) {
                throw new Error('La nueva contraseña debe ser distinta a la actual.');
            }

            await cambiarPasswordPerfil({
                current_password: estadoPassword.actual,
                new_password: estadoPassword.nueva,
            });

            setEstadoPassword({actual: '', nueva: '', confirmar: ''});
            setPasswordActualizada(true);
            setTimeout(() => setPasswordActualizada(false), 3000);
        } catch (err: unknown) {
            const hasHttpResponse = typeof err === 'object' && err !== null && 'response' in err;
            setErrorPassword(
                hasHttpResponse
                    ? extraerMensajeError(err)
                    : err instanceof Error
                        ? err.message
                        : 'Error actualizando contraseña'
            );
        } finally {
            setGuardandoPassword(false);
        }
    };

    const handleSubirAvatar = async (archivo: File) => {
        setSubiendoAvatar(true);
        try {
            const resp = await subirAvatar(archivo);
            queryClient.setQueryData<PerfilResponse | undefined>(
                currentProfileKey(userId),
                (prev) => prev ? {...prev, avatar_url: resp.avatar_url} : prev,
            );
        } finally {
            setSubiendoAvatar(false);
        }
    };

    return {
        estado, guardado, guardando, errorGuardar, cargando: cargandoPerfil, perfil, avatarUrl,
        subiendoAvatar, estadoPassword, passwordActualizada, guardandoPassword, errorPassword,
        actualizarCampo, actualizarPasswordCampo, guardarPerfil, handleGuardarPassword,
        handleSubirAvatar
    };
};
