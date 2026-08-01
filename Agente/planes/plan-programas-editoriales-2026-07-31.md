# Plan 297A-14 — Programas editoriales

> **Fecha:** 2026-07-31
> **Estado:** vertical de artículos/editor completado; proyectos, productos, media y E2E visual pendientes.
> **Epic:** 297A-4 — OS persistente, cuentas, programas y comercio.
> **Depende de:** 297A-9, 297A-10 y 297A-11.
> **Bloquea:** cierre completo de la administración editorial y 297A-15 Comercio.

## Objetivo

Migrar la administración editorial desde el monolito Admin hacia programas reutilizables del OS. Cada programa debe usar `AppRegistry`, capacidades server-side, `MountedView`/`AbortSignal`, servicios tipados y el chrome compartido; el shell no conoce formularios ni editores concretos.

## Límites arquitectónicos

- Admin conserva orquestación/listados; no crea ventanas ni importa Tiptap.
- `articleId`, `resourceId` y otros IDs internos son parámetros de instancia, no deep links públicos.
- Los cambios entre apps se comunican mediante eventos de dominio tipados o invalidación de servicio, nunca mediante referencias DOM entre ventanas.
- La app monta una vista de carga inmediatamente; red y dependencias pesadas se hidratan de forma abortable.
- Crear y editar deben ser operaciones idempotentes respecto a la instancia: después de crear se conserva el ID y los guardados siguientes actualizan.
- Listados con recarga reactiva deben invalidar caché, descartar respuestas fuera de orden y liberar suscripciones al desmontar.

## Fases y checklist

### Fase 1 — Editor de artículos/About (vertical cerrado)

- [x] Registrar `article-editor` como app lazy `requires: 'admin'`, multiinstancia, sin `deepLink` público.
- [x] Extraer el editor de Tiptap de `admin-articles.ts`; Admin queda como listado/orquestador.
- [x] Montar loading síncrono y cargar artículo/Tiptap dentro del lifecycle.
- [x] Cancelar hidratación y destruir Tiptap con `AbortSignal`/`MountedView.destroy()`.
- [x] Manejar errores de apertura, carga del Admin y carga del editor con feedback visible.
- [x] Mantener `currentArticleId` después de crear para evitar duplicados; multimedia usa el ID actual.
- [x] Publicar evento tipado de guardado e invalidar/refrescar listados sin acoplarlos al shell.
- [x] Proteger el listado contra respuestas fuera de orden y liberar listeners al desmontar.
- [x] Regresiones de registro/capacidad/deep-link y canal de eventos.
- [x] TypeScript, Vitest, build, backend, `task:check` y `self-check` PASS.

**Evidencia F1 — 2026-07-31:** frontend typecheck PASS; Vitest **281/281** en 35 archivos; build PASS; `npm run check:back` PASS; `npm test` **17/17**; `npm run task:check -- 297A-14 --fresh` PASS; `npm run self-check -- -TareaId 297A-14` PASS. Sentinel: 0 errores; VarSense: 2 avisos; custom: 5 avisos informativos preexistentes/no bloqueantes. Revisión code-reviewer-luna: sin bloqueantes tras corregir carga asíncrona, create→update, carreras del listado y asociación multimedia.

### Fase 2 — Editor de proyectos

- [ ] Convertir el editor modal heredado en app lazy `project-editor` con el mismo contrato de loading/lifecycle.
- [ ] Separar listado/orquestación de formulario y usar evento tipado de guardado.
- [ ] Añadir pruebas de capacidad, cleanup, error, create→update y refresh sin respuestas obsoletas.
- [ ] Validar envelope `resource` y estados editoriales antes de ampliar el formulario.

### Fase 3 — Productos versionados

- [ ] Diseñar `product-editor` admin-only con producto inactivo/private por defecto.
- [ ] Mantener versiones de entrega inmutables y separar metadatos editables de archivos privados.
- [ ] Validar precio, moneda, disponibilidad y MIME únicamente en backend.
- [ ] Probar permisos negativos y no exposición de drafts/assets/grants.

### Fase 4 — Biblioteca de media

- [ ] Crear `media-library` como programa separado del editor de artículos.
- [ ] Asociar media mediante referencias, sin mover ni mutar el recurso propietario al reorganizar workspace.
- [ ] Validar estados `processing/clean/rejected`, límites, MIME y cleanup de object URLs.
- [ ] Probar selección, eliminación, papelera, restauración y permisos.

### Fase 5 — Paridad y cierre

- [ ] Congelar matriz de paridad del Admin legacy antes de retirar cada superficie.
- [ ] Migrar publish/preview/rollback, draft/private/public, autosave y papelera por tipo de recurso.
- [ ] Ejecutar E2E visual desktop/tablet/móvil de apertura, foco, minimizar, cierre, error y transición de presentación.
- [ ] Ejecutar quality gate y self-check por fase; ningún vertical se marca completo con pruebas condicionadas o documentación sin evidencia.
- [ ] Retirar gradualmente el editor legacy solo después de paridad y rollback verificados.

## Definition of Done por vertical

- App lazy registrada y capacidad autorizada en la frontera de apertura.
- Sin chrome, ventana, router ni SQL dentro de la app.
- Loading inmediato, error visible y lifecycle abortable.
- Parámetros internos no aparecen en URL pública.
- Create/update, cache invalidation y eventos de dominio probados.
- Typecheck, tests, build, backend, task-check y self-check PASS.
- Revisión SOLID: SRP/ISP/OCP/DIP, límites de tamaño, sin listeners o I/O silencioso.
- E2E visual queda pendiente explícito o PASS con evidencia real; nunca se infiere desde unit tests.

## Enlaces

- Guía de apps: `Agente/documentacion/arquitectura/guia-agregar-app-2026-07-31.md`
- Auditoría v4: `Agente/documentacion/arquitectura/auditoria-arquitectura-v4-2026-07-30.md`
- Manual de arquitectura: `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`
- Roadmap: `roadmap.md` (§297A-14)
