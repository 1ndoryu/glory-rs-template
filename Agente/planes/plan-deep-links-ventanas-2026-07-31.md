# Plan — URLs canónicas, deep links y foco del OS

> **Fecha:** 2026-07-31  
> **Estado:** pendiente de implementación  
> **Dependencias:** 297A-9 runtime, 297A-11 workspace, 297A-12 móvil; integra permisos de 297A-13.

## Objetivo

Cada aplicación y cada recurso abierto debe tener una URL canónica compartible. La URL representa la ventana enfocada, no todo el estado visual de la sesión: al abrirla, otra persona llega a esa app/recurso y el OS la abre o enfoca según su presentación.

## Contrato de URL

- [ ] Definir un formato versionado y allowlisted para `app`, `resourceKind`, alias/slug público, versión y parámetros de instancia; no exponer IDs internos, tokens, clipboard, posiciones, tamaños, z-index ni overlays privados.
- [ ] Hacer que cada entrada de `AppRegistry` declare parser/serializer de ruta, capacidades requeridas, parámetros permitidos y fallback seguro.
- [ ] Distinguir URL canónica pública, URL autenticada de cuenta y URL local de sesión; una ruta privada sin permiso muestra login/not-found sin filtrar existencia ni metadata.
- [ ] Resolver recursos por alias/slug estable cuando sea público; versiones y descargas privadas solo mediante entitlement/grant válido.

**Gate:** una app nueva puede declarar su ruta sin modificar un router monolítico ni copiar lógica de otra app.

## Ventana enfocada como URL

- [ ] Al abrir/focalizar una ventana, serializar únicamente la instancia enfocada mediante `replaceState`; las aperturas explícitas usan `pushState` para conservar navegación útil.
- [ ] Al cargar una URL, `RouteAppAdapter` debe abrir la app, hidratar el recurso, enfocarla y reutilizar una instancia equivalente en vez de duplicarla.
- [ ] No serializar las demás ventanas, posiciones, tamaños, orden de taskbar, overlay local ni estado transitorio; esos datos siguen siendo de la sesión/dispositivo.
- [ ] Definir Back/Forward, refresh y cambio de foco: volver a una ruta restaura app/recurso/foco permitidos sin sobrescribir el workspace público.
- [ ] En móvil, la misma ruta abre la app a pantalla completa; en tablet conserva escritorio; al cambiar breakpoint se preservan app, recurso y parámetros válidos.

**Gate:** copiar la URL con varias ventanas abiertas y abrirla en una sesión limpia enfoca exactamente la app/recurso compartido.

## Compartir y seguridad

- [ ] Añadir comando `Copiar URL` al menú contextual/toolbar de cada app y recurso; mostrar feedback visible y medir la acción.
- [ ] Validar y normalizar rutas en el boundary: sin open redirects, parámetros arbitrarios, HTML no sanitizado, secretos en query/hash ni acceso a drafts/privados.
- [ ] Aplicar capacidades server-side y no confiar en que ocultar una app en el cliente sea autorización; comprobar release/overlay/entitlement antes de hidratar.
- [ ] Resolver rutas antiguas con redirección canónica documentada, sin romper enlaces existentes ni crear bucles.

**Gate:** rutas inválidas, privadas, expiradas y manipuladas producen un estado seguro, trazable y comprensible.

## Historial, analítica y SEO

- [ ] Diferenciar `pushState` (navegación intencional) de `replaceState` (foco/cambio visual) para no llenar el historial al cambiar ventanas.
- [ ] Emitir `deep_link_opened`, `window_focus_changed` y `share_url_copied` con `routeName`, tipo de app, `presentationMode` y resultado; nunca enviar contenido ni IDs sensibles.
- [ ] Preparar metadata/sitemap solo para recursos públicos; no indexar rutas del Admin, overlays, drafts ni grants.
- [ ] Documentar canonical URL y título accesible por app para compartir y lectores de pantalla.

**Gate:** la misma acción produce el mismo evento semántico en desktop, tablet y móvil, con consentimiento y retención aplicables.

## Pruebas obligatorias

- [ ] Abrir una URL de Finder, Reader/artículo, About, proyecto, producto, Configuración y Estadísticas desde sesión limpia.
- [ ] Probar varias ventanas, foco alterno, Copiar URL, refresh, Back/Forward, deep link directo y colisión de instancia.
- [ ] Probar 1440x900, 1024x768, 768px, 390px y 320px; incluir usuario anónimo, admin, usuario sin capacidad, recurso privado y grant expirado.
- [ ] Ejecutar type-check, tests de RouteAppAdapter/WindowManager, E2E de navegación y quality gate; registrar evidencia SOLID S1–S5.

## Definition of Done

- [ ] Cada app/recurso soportado tiene URL versionada, parser, serializer, permisos y fallback.
- [ ] La URL compartida abre/enfoca solo la ventana representada, sin filtrar la sesión del emisor.
- [ ] Historial, móvil/tablet, seguridad, analítica y accesibilidad están probados.
- [ ] Manual de arquitectura, contratos, roadmap e índice se actualizan con la decisión final.
