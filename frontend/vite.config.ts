import path from 'node:path';
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { '@': path.resolve(import.meta.dirname, 'src') },
  },
  server: {
    port: 5199,
    host: '127.0.0.1',
    strictPort: true,
    // Mismo-origen hacia GloryAPI: evita CORS sin tocar gloryapi.
    proxy: {
      '/glory': {
        target: 'http://127.0.0.1:3101',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/glory/, ''),
      },
      // Mini-backend local de mejora (server/mejora.mjs en :3122).
      '/api': {
        target: 'http://127.0.0.1:3122',
        changeOrigin: true,
      },
      /* [299A-3] Media del backend Rust (:3000): el hilo muestra
       * `/uploads/...` relativo (en prod es mismo origen); en dev el
       * proxy lo lleva al backend para que `<img>`/`<audio>` carguen. */
      '/uploads': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      },
    },
  },
  preview: {
    port: 5199,
    host: '127.0.0.1',
  },
})
