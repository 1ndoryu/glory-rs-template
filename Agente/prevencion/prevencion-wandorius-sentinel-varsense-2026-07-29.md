# Prevención canónica: Glory Sentinel y VarSense para wandori.us

> **Fecha:** 2026-07-29  
> **Estado:** activo; infraestructura 297A-6 completada y reglas de dominio en rollout
> **Autoridad:** inventario único de reglas automatizables del proyecto  
> **Arquitectura:** `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`  
> **Identidad:** `Agente/documentacion/design-system/manual-identidad-visual-os-2026-07-29.md`

## Reglas de implementación

- Las reglas genéricas se implementan en Glory Sentinel/VarSense y permanecen agnósticas.
- wandori.us solo configura severidad, rutas y excepciones mínimas en archivos canónicos.
- Toda regla incluye fixture positiva, negativa, equivalencia CLI/LSP/editor y documentación.
- Baseline primero, corrección después, bloqueo CI al final.
- Prohibidas regex locales o scripts paralelos que dupliquen el motor.

## Checklist 1 — Seguridad inmediata

- [ ] Detectar endpoint admin protegido solo por autenticación y no por capacidad.
- [ ] Detectar request público que acepte rol/promoción.
- [ ] Detectar token/JWT de sesión persistido en Web Storage.
- [ ] Detectar credenciales o auto-registro en frontend.
- [ ] Detectar endpoint público sin predicados obligatorios de visibilidad/lifecycle.
- [ ] Detectar DTO público con `download_path`, `storage_key`, URL firmada o IDs internos de pago.
- [ ] Detectar directorio de entregables servido estáticamente.
- [ ] Detectar checkout que acepte precio/moneda/ruta/éxito desde cliente.
- [ ] Detectar webhook sin firma, evento único, validación de importe/moneda y transacción.
- [ ] Detectar descarga sin entitlement y grant revalidado.
- [ ] Detectar fallos silenciosos en pago, persistencia, email o fulfillment.
- [ ] Activar estas reglas como error después de corregir el baseline.

## Checklist 2 — Datos y publicación

- [ ] Detectar recurso creado sin defaults `draft`, `private`, `active`.
- [ ] Detectar un único `status` mezclando editorial, visibilidad, lifecycle o comercio.
- [ ] Detectar preferencia personal escrita en settings globales.
- [ ] Detectar publicación de workspace sin revisión, transacción y auditoría.
- [ ] Detectar persistencia desktop sin `schemaVersion` y validación.
- [ ] Detectar purga sin capacidad, retención, auditoría o capa explícita.
- [ ] Detectar portapapeles interno con HTML, funciones, secretos o URLs firmadas.

## Checklist 3 — Lifecycle y frontend

- [ ] `MountedView` async requiere `AbortSignal`.
- [ ] Listener global requiere signal/teardown.
- [ ] Fetch requiere cliente común y comprobación de resultado.
- [ ] Catch vacío o error convertido en éxito es error.
- [ ] HTML dinámico requiere sanitizador central.
- [ ] Router/history desde apps fuera del adaptador es error.
- [ ] Open externo exige `noopener,noreferrer`.
- [ ] Iframe externo exige sandbox y allowlist.

## Checklist 4 — Escritorio reutilizable

- [ ] Apps solo se definen en AppRegistry.
- [ ] AppRegistry usa capacidades, no booleano `public|admin` como autorización.
- [ ] Chrome `.desktop-window*` solo se crea en factory/componente autorizado.
- [ ] Drag, resize, focus, minimize y close solo viven en gestor/controlador común.
- [ ] Estado de ventanas solo cambia mediante comandos/reducer.
- [ ] Taskbar, escritorio y Aplicaciones no mantienen listas paralelas.
- [ ] Menús reciben modelos/comandos; no hardcodean dominios.
- [ ] z-index por app es error.
- [ ] Control de menú requiere foco/teclado y nombre accesible.
- [ ] Detectar una app/store móvil paralelo en vez de reutilizar AppRegistry/MountedView/comandos.
- [ ] Detectar DesktopWindow, barra superior o taskbar montadas en presentación móvil.

## Checklist 5 — API, backend y analytics

- [ ] Orval usa Fetch + `tags-split` y no genera archivo monolítico.
- [ ] Tipos generados no se editan ni duplican manualmente.
- [ ] SQL solo vive en repositories y usa macros/query builders tipados.
- [ ] Evento analítico requiere ID, versión, nombre allowlisted y metadata limitada.
- [ ] Batch analítico exige límite, idempotencia y transacción.
- [ ] El cliente no puede declarar éxito de pago, auth, publicación o descarga.
- [ ] Analytics no contiene email, contenido, token, URL firmada o datos de pago.
- [ ] Audit y analytics usan contratos/tablas separados.

## Checklist 6 — Identidad visual y VarSense

- [ ] Prohibir color literal fuera de `variables.css`.
- [ ] Prohibir sombra, blur y drop-shadow.
- [ ] Prohibir radio salvo círculo de marca documentado.
- [ ] Prohibir borde visual mayor de 1 px.
- [ ] Prohibir hover/animación puramente decorativos.
- [ ] Exigir Lucide oficial con stroke token de 1 px.
- [ ] Prohibir iconos manuales/emoji en chrome.
- [ ] Detectar CSS inline salvo variable funcional autorizada de geometría.
- [ ] Detectar token CSS inexistente o huérfano.
- [ ] Detectar clase usada sin definición y clase huérfana, con allowlist dinámica mínima.
- [ ] Detectar app con receta local equivalente a ventana, título, acciones, formulario o card compartidos.

## Checklist 7 — Configuración y CI

- [x] Crear `sentinel.config.json` canónico en raíz.
- [x] Crear/normalizar config VarSense canónica.
- [x] Excluir solo artefactos, dependencias y código generado; el runner se autosupervisa.
- [ ] Registrar excepción por regla, archivo, tarea, motivo y fecha de retirada.
- [x] Integrar ambos mediante el único script `task:check`, sin comandos paralelos duplicados.
- [x] Integrar ambos en `npm run self-check` y CI.
- [x] Reporte registra herramientas/config/fecha para comprobar vigencia.
- [x] CI falla ante errores nuevos de alta confianza.

## Criterio de cierre

- [ ] Todas las reglas de error tienen fixtures positivas/negativas.
- [ ] CLI, LSP, VS Code y Zed producen hallazgos equivalentes.
- [ ] Baseline del proyecto tiene cero errores.
- [ ] No quedan suppressions amplias.
- [ ] Self-check y CI bloquean una fixture regresiva de seguridad, arquitectura y visual.
