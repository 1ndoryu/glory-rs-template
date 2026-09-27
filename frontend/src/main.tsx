/* [044A-1] Entry point - importa estilos globales antes de montar el app */
/* [044A-2] Inicializa i18n antes del render para que esté disponible globalmente */
/* [044A-13] Restaura sesión JWT desde localStorage al iniciar */
/* [044A-28] HelmetProvider para gestión dinámica de meta tags SEO */
import React from 'react';
import ReactDOM from 'react-dom/client';
import {HelmetProvider} from 'react-helmet-async';
import './i18n';
import './styles/variables.css';
import './styles/init.css';
import App from './App';
import {useAuthStore} from './stores/authStore';
/* [259A-5] Bootstrap: raiz del documento via platform/dom (boundary sentinel). */
import {obtenerElementoPorId} from './platform/dom';

useAuthStore.getState().inicializar();

ReactDOM.createRoot(obtenerElementoPorId('root')!).render(
  <React.StrictMode>
    <HelmetProvider>
      <App />
    </HelmetProvider>
  </React.StrictMode>,
);
