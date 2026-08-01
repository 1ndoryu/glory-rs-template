# Plan de notificaciones — 2026-08-01

> Estado: bloque autónomo parcial; la entrega por cuenta y el panel admin quedan diferidos hasta cerrar registro verificado/overlay remoto.

## Objetivo y límites

Avisar de releases públicos sin duplicar el catálogo de workspace ni inventar un canal de publicación paralelo. La primera fase usa el release público como fuente canónica y conserva el estado leído en el navegador.

## Fases

### Fase 1 — Fuente y política local (completada)

- [x] Una notificación estable por `workspace-release:{version}`.
- [x] Solo releases públicos; no exponer drafts, overlays privados ni contenido editorial.
- [x] Estado leído idempotente y acotado a 100 IDs en `localStorage`.
- [x] Error de red visible en la app, sin convertirlo en lista vacía silenciosa.

**Gate:** type-check y suite frontend pasan; no hay endpoint nuevo ni migración que duplicar.

### Fase 2 — Presentación del OS (completada)

- [x] App pública `notifications` con URL `/notifications`.
- [x] Campana Lucide de 1px en barra desktop y launcher móvil.
- [x] Contador de no leídas y apertura de la misma app desde ambos shells.
- [x] Vista accesible con foco, Enter/Espacio y botón de recarga.

**Gate:** app registrada por `AppRegistry`; chrome no conoce la fuente de datos.

### Fase 3 — Cuenta y administración (diferida)

- [ ] Endpoint de lectura por usuario y sincronización entre dispositivos.
- [ ] Panel admin para crear, publicar y descartar avisos sin publicar manualmente un release.
- [ ] Política anti-spam por recurso/evento y dedupe server-side.
- [ ] Casos E2E: overlay personalizado, logout/login, dos dispositivos y permisos.

**Bloquea:** registro verificado, overlay remoto estable y decisión del formato de avisos editoriales/comerciales.

## Definition of Done de la fase autónoma

- [x] `npm run task:check -- 297A-21` pasa para el alcance frontend.
- [x] La fuente no crea un segundo estado de publicación.
- [x] El roadmap deja explícitos los pendientes que requieren decisión/credenciales.
