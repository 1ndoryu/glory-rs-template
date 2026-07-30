# wandori.us — Roadmap

> **Producto:** blog, portfolio y tienda digital dentro de un OS retro minimalista
> **Stack:** Rust/Axum + PostgreSQL + Vanilla TypeScript/Vite
> **Deploy:** no planificado
> **Epic:** 297A-4 — OS persistente, cuentas, programas y comercio
> **Visual:** identidad desktop aprobada; concepto móvil pendiente

## Fuentes canónicas

- Índice: `Agente/documentacion/indice-documentacion-2026-07-29.md`
- Arquitectura: `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`
- Identidad: `Agente/documentacion/design-system/manual-identidad-visual-os-2026-07-29.md`
- Plan maestro: `Agente/planes/plan-escritorio-persistente-cuentas-admin-apps-2026-07-29.md`
- Plan móvil: `Agente/planes/plan-experiencia-movil-launcher-2026-07-29.md`
- Quality gate: `Agente/planes/completados/plan-escalabilidad-sentinel-wandorius-2026-07-29.md`
- Prevención: `Agente/prevencion/prevencion-wandorius-sentinel-varsense-2026-07-29.md`

## Estado y reglas

- Concepto desktop aprobado; Finder/Reader siguen siendo previews.
- `/admin`, JWT en Web Storage, uploads públicos y comercio actual son legado.
- Ejecutar una tarea por vez y en este orden; no saltar dependencias.
- El plan maestro contiene checklists/gates. El roadmap conserva solo pendientes.
- El quality gate 297A-6 está operativo; toda tarea futura debe cerrarse con `npm run task:check -- {ID}`.

## Siguiente bloque habilitado

**297A-7 — ADRs y seguridad inmediata.** Quality gate completado; siguen decisiones y cierre de exposición legacy.

## Pendientes ordenados

### 297A-7 — ADRs y seguridad inmediata

**Depende de:** 297A-6.

- [ ] ADR SEO/indexabilidad, storage privado, modalidad Stripe y retención.
- [ ] Roles/capacidades y bootstrap admin seguro.
- [ ] Separar superficies public/me/admin/webhooks.
- [ ] Ocultar drafts/media privada/entregables y retirar modo demo.
- [ ] Eliminar auto-login/auto-registro y mantener registro apagado.
- [ ] Pruebas negativas de autorización/exposición.

**Salida:** usuario normal no administra y ningún recurso privado es público.

### 297A-8 — Sesiones seguras y Cuenta base

**Depende de:** 297A-7.

- [ ] Sesiones opacas revocables en cookie.
- [ ] CSRF/origin/rate limit.
- [ ] Cuenta interna: login/logout/me/sesiones.
- [ ] Verificación/recovery diseñados y probados.
- [ ] Registro continúa apagado hasta completar controles.

**Salida:** admin opera sin JWT en Web Storage y puede revocar sesiones.

### 297A-9 — Foundation del runtime

**Depende de:** 297A-6/7; coordina identidad 297A-8.

- [ ] Orval Fetch tags-split, sanitizador y Result común.
- [ ] MountedView/AbortSignal y RouteAppAdapter.
- [ ] AppRegistry, WindowManager y CommandRegistry.
- [ ] Taskbar/menús/ventanas derivados.
- [ ] Dispatcher analítico tipado.
- [ ] Prueba vertical Perfil→Finder→Reader.

**Salida:** runtime compartido funciona sin chrome/listas/listeners duplicados.

### 297A-10 — Recursos y migraciones

**Depende de:** 297A-7/9.

- [ ] Catálogo `resources`, estados ortogonales y defaults privados.
- [ ] About como artículo; producto independiente de artículo.
- [ ] Separar preview público y asset privado.
- [ ] Migrar status/is_visible/is_active/download_path legacy.
- [ ] DTO público/admin y OpenAPI completos.

**Salida:** organizar referencias no altera publicación y ninguna metadata privada se filtra.

### 297A-11 — Workspace público y overlay invitado

**Depende de:** 297A-9/10.

- [ ] Draft/release/preview/publicar/rollback.
- [ ] Organizador público separado del workspace admin personal.
- [ ] Árbol, referencias, initial windows y IDs estables.
- [ ] Overlay local: additions/overrides/tombstones.
- [ ] Clipboard, ciclos, papelera por capa y reset.

**Salida:** admin publica el preview exacto; visitante reorganiza sin escribir global.

### 297A-12 — Experiencia móvil tipo launcher

**Depende de:** 297A-9/11. Tablet conserva desktop.

- [ ] Prototipo visual móvil y aprobación del usuario.
- [ ] Launcher + MobileAppStack con las mismas apps.
- [ ] Apps full-screen, sin ventanas/barra superior/taskbar.
- [ ] Back/Home/long press, carpetas y `mobileOrder`.
- [ ] Cambio móvil↔tablet sin perder app/recurso.
- [ ] Pruebas 320/360/390 y tablet 768.

**Salida:** teléfono funciona como launcher sin duplicar lógica; tablet sigue como escritorio.

### 297A-13 — Registro y overlay remoto

**Depende de:** 297A-8/11; integra móvil 297A-12.

- [ ] Habilitar registro verificado.
- [ ] Overlay remoto y preferencias.
- [ ] Importar local/usar remoto/reset explícito.
- [ ] Merge por ID/campo y 409 visible.
- [ ] Pruebas dos dispositivos/release nuevo.

**Salida:** cuenta sincroniza sin sobrescribir ni restaurar recursos retirados.

### 297A-14 — Programas editoriales

**Depende de:** 297A-9/10/11.

- [ ] Editor de artículos/About.
- [ ] Editor de proyectos.
- [ ] Editor de productos privado/inactivo por defecto.
- [ ] Biblioteca de media.
- [ ] Menú Admin por capacidades y paridad sin ampliar `admin.ts`.

**Salida:** administración editorial vive en programas reutilizables del OS.

### 297A-15 — Comercio seguro

**Depende de:** 297A-7/10/14.

- [ ] Tienda como carpeta, Compra y Pedidos.
- [ ] Product versions y storage privado.
- [ ] Orden idempotente y webhook validado/transaccional.
- [ ] Entitlements, grants y outbox.
- [ ] Compra invitada, reembolso/revocación y pruebas negativas.

**Salida:** cliente no concede acceso; comprador recibe la versión adquirida.

### 297A-16 — Analytics, Estadísticas y retiro legado

**Depende de:** 297A-9/11–15.

- [ ] Consentimiento/retención y eventos esenciales/opcionales.
- [ ] Batch idempotente y eventos críticos server-side.
- [ ] Agregados, Estadísticas y audit separado.
- [ ] Paridad y eliminación de `/admin`, JWT, uploads y contratos/CSS legacy.

**Salida:** una sola administración y métricas privadas/tipadas.

### 297A-17 — Hardening, identidad, accesibilidad y SEO

**Depende de:** 297A-6–16.

- [ ] MFA/passkey, estrategia SEO, sitemap y metadata.
- [ ] Manual visual en desktop/tablet/móvil.
- [ ] Teclado, foco, zoom, reduced motion y multimedia accesible.
- [ ] Quality gate/CI completos y E2E críticos.
- [ ] Threat review, performance, observabilidad y runbook.

**Salida:** preparado para revisión de producción; deploy continúa fuera de alcance.
