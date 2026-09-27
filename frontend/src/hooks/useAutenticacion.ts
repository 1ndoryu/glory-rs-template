/**
 * Hook: useAutenticacion
 * Encapsula toda la logica de estado del modal de autenticacion.
 * Reduce useState en ModalAutenticacion de 9 a 0 (SRP).
 * [044A-13] Conectado con backend REST API (login/registro via JWT).
 * [155A-1] handleGoogleLogin: redirige a Google OAuth; el callback lo procesa App.tsx.
 * Pendiente: recuperación de contraseña.
 */
import {useState, useCallback} from 'react';
import {useNavigate} from 'react-router-dom';
import {apiLogin, apiRegister, apiGoogleLoginUrl, extraerMensajeError} from '../api/auth';
import {useAuthStore} from '../stores/authStore';
/* [259A-5] Acceso window via platform/navigation (boundary sentinel). */
import {redirigir} from '../platform/navigation';

export type VistaModal = 'login' | 'registro' | 'recuperar';

interface EstadoLogin {
    email: string;
    password: string;
}

interface EstadoRegistro {
    nombre: string;
    email: string;
    password: string;
    confirmar: string;
}

interface EstadoRecuperar {
    email: string;
    enviado: boolean;
}

/* [259A-5] Retorno segregado por subformulario (ISP, regla large-interface-isp).
 * El hook sigue devolviendo RetornoUseAutenticacion plano via extends. */
interface AutenticacionVista {
    vista: VistaModal;
    setVista: (v: VistaModal) => void;
    cargando: boolean;
    error: string | null;
}

interface AutenticacionLogin {
    login: EstadoLogin;
    actualizarLogin: (campo: keyof EstadoLogin, valor: string) => void;
    handleLogin: (e: React.FormEvent) => void;
}

interface AutenticacionRegistro {
    registro: EstadoRegistro;
    actualizarRegistro: (campo: keyof EstadoRegistro, valor: string) => void;
    handleRegistro: (e: React.FormEvent) => void;
}

interface AutenticacionRecuperar {
    recuperar: EstadoRecuperar;
    actualizarRecuperar: (campo: keyof EstadoRecuperar, valor: string) => void;
    handleRecuperar: (e: React.FormEvent) => void;
    resetRecuperacion: () => void;
}

interface AutenticacionGoogle {
    handleGoogleLogin: () => void;
}

interface RetornoUseAutenticacion extends AutenticacionVista, AutenticacionLogin, AutenticacionRegistro, AutenticacionRecuperar, AutenticacionGoogle {
}

export const useAutenticacion = (onCerrar: () => void): RetornoUseAutenticacion => {
    const [vista, setVista] = useState<VistaModal>('login');
    const [cargando, setCargando] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const authLogin = useAuthStore(s => s.login);
    const navigate = useNavigate();

    const [login, setLogin] = useState<EstadoLogin>({email: '', password: ''});

    const [registro, setRegistro] = useState<EstadoRegistro>({nombre: '', email: '', password: '', confirmar: ''});
    const [recuperar, setRecuperar] = useState<EstadoRecuperar>({email: '', enviado: false});

    const actualizarLogin = useCallback((campo: keyof EstadoLogin, valor: string) => {
        setLogin(prev => ({...prev, [campo]: valor}));
    }, []);

    const actualizarRegistro = useCallback((campo: keyof EstadoRegistro, valor: string) => {
        setRegistro(prev => ({...prev, [campo]: valor}));
    }, []);

    const actualizarRecuperar = useCallback((campo: keyof EstadoRecuperar, valor: string) => {
        setRecuperar(prev => ({...prev, [campo]: valor}));
    }, []);

    /* [044A-13] Login real contra backend REST API
     * [044A-38 Fase 1] Ahora pasa role/effective_role al store
     * [064A-14] Navega a /panel tras login exitoso */
    const handleLogin = useCallback(async (e: React.FormEvent) => {
        e.preventDefault();
        setError(null);
        setCargando(true);
        try {
            const resp = await apiLogin(login.email, login.password);
            authLogin(resp.token, resp.user_id, login.email, resp.role, resp.effective_role, resp.needs_password);
            onCerrar();
            navigate('/panel');
        } catch (err) {
            setError(extraerMensajeError(err));
        } finally {
            setCargando(false);
        }
    }, [login.email, login.password, authLogin, onCerrar, navigate]);

    /* [044A-13] Registro real contra backend REST API
     * [044A-38 Fase 1] Ahora pasa role/effective_role al store
     * [064A-14] Navega a /panel tras registro exitoso */
    const handleRegistro = useCallback(
        async (e: React.FormEvent) => {
            e.preventDefault();
            if (registro.password !== registro.confirmar) {
                setError('Las contraseñas no coinciden.');
                return;
            }
            setError(null);
            setCargando(true);
            try {
                const resp = await apiRegister(registro.email, registro.password);
                authLogin(resp.token, resp.user_id, registro.email, resp.role, resp.effective_role, resp.needs_password);
                onCerrar();
                navigate('/panel');
            } catch (err) {
                setError(extraerMensajeError(err));
            } finally {
                setCargando(false);
            }
        },
        [registro.email, registro.password, registro.confirmar, authLogin, onCerrar, navigate]
    );

    const handleRecuperar = useCallback((e: React.FormEvent) => {
        e.preventDefault();
        setCargando(true);
        setTimeout(() => {
            setCargando(false);
            setRecuperar(prev => ({...prev, enviado: true}));
        }, 500);
    }, []);

    /* [155A-1] Google OAuth — obtiene URL de redirección y redirige al usuario.
     * Google devolverá al usuario a la raíz del SPA con ?code=... que se
     * procesa en GoogleAuthCallback (App.tsx). */
    const handleGoogleLogin = useCallback(async () => {
        setError(null);
        setCargando(true);
        try {
            const {url} = await apiGoogleLoginUrl();
            redirigir(url);
        } catch (err) {
            setError(extraerMensajeError(err));
            setCargando(false);
        }
    }, []);

    const resetRecuperacion = useCallback(() => {
        setVista('login');
        setRecuperar({email: '', enviado: false});
    }, []);

    return {
        vista,
        setVista,
        cargando,
        error,
        login,
        registro,
        recuperar,
        actualizarLogin,
        actualizarRegistro,
        actualizarRecuperar,
        handleLogin,
        handleRegistro,
        handleRecuperar,
        handleGoogleLogin,
        resetRecuperacion
    };
};
