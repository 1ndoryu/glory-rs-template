/* [259A-5] sentinel-disable-file menu-contextual-override-diseno: variantes por instancia via props API publica del DS; variantes canon al sistema en fase 5b. */
/**
 * Componente: HeaderPanel
 * Header minimalista exclusivo para el panel de usuario.
 * Solo muestra: logo, launcher de apps, notificaciones, "Inicio" y foto de usuario con submenú.
 * [044A-38 Fase 1] Logout real conectado a authStore.
 * [074A-45] "Salir" → "Inicio" navega sin desloguear. Avatar abre submenú con logout.
 * [T3-wallet-header] Añadido saldo mini junto a notificaciones. */
import React, {useCallback, useState} from 'react';
import {useTranslation} from 'react-i18next';
import {useNavigate} from 'react-router-dom';
import {LogOut, Wallet} from 'lucide-react';
import {GloryLink} from '../../core/router';
import {useAuthStore} from '../../stores/authStore';
import {useCurrentProfile} from '../../hooks/useCurrentProfile';
import {useWallet} from '../../hooks/useWallet';
import {formatBalance} from '../../api/wallet';
import {MenuContextual} from '../ui/ContextMenu';
import {Button} from '../ui/Button';
import {Logo} from '../ui/Logo';
import OptimizedImage from '../ui/OptimizedImage';
import NotificationBell from './NotificationBell';
import {AppLauncher} from '../layout/AppLauncher';
import './HeaderPanel.css';
import '../../styles/header.css';

export const HeaderPanel: React.FC = () => {
    const {t} = useTranslation();
    const {avatarUrl} = useCurrentProfile();
    const logout = useAuthStore(s => s.logout);
    const navigate = useNavigate();
    const [menuAbierto, setMenuAbierto] = useState(false);
    const {wallet} = useWallet();

    const handleLogout = useCallback(() => {
        setMenuAbierto(false);
        logout();
        navigate('/');
    }, [logout, navigate]);

    return (
        <header className="headerPanel" role="banner">
            <div className="headerPanelContenedor">
                {/* Logo */}
                <GloryLink to="/" className="headerPanelLogo" aria-label={t('accessibility.logo_home')}>
                    <Logo className="headerPanelLogoSvg" />
                </GloryLink>

                {/* Acciones: Chat, Notificaciones, Inicio, Avatar con submenú */}
                <div className="headerPanelAcciones">
                    {/* [T3-wallet-header] Saldo mini: clic dispara custom event para ir a wallet */}
                    <Button
                        variante="texto"
                        className="headerPanelSaldo"
                        type="button"
                        onClick={() => window.dispatchEvent(new CustomEvent('panel-cambiar-tab', {detail: 'wallet'}))}
                        title={t('panel.wallet', 'Mi saldo')}
                    >
                        <Wallet size={16} />
                        <span>{formatBalance(wallet?.balance_cents ?? 0, wallet?.currency)}</span>
                    </Button>
                    {/* [TEMP] AppLauncher oculto temporalmente */}
                    {false && <AppLauncher />}
                    <NotificationBell />
                    {/* [074A-45] Ir a inicio sin desloguear */}
                    <GloryLink to="/" className="headerPanelEnlace">
                        {t('nav.home', 'Inicio')}
                    </GloryLink>
                    {/* [084A-11] Avatar → MenuContextual en vez de dropdown artesanal.
                     * [114A-6] Alineado con Header.tsx: mismas clases + ícono LogOut. */}
                    <MenuContextual
                        abierto={menuAbierto}
                        onToggle={() => setMenuAbierto(prev => !prev)}
                        onCerrar={() => setMenuAbierto(false)}
                        ariaLabel={t('accessibility.user_actions')}
                        className="perfilDropdownWrapper"
                        triggerClassName="perfilAvatarBtn"
                        triggerContent={<OptimizedImage src={avatarUrl} alt={t('accessibility.profile_photo')} className="perfilAvatarImg" loading="eager" />}
                        items={[{
                            id: 'logout',
                            label: t('nav.logout'),
                            icon: <LogOut size={14} />,
                            onSelect: handleLogout,
                        }]}
                    />
                </div>
            </div>
        </header>
    );
};
