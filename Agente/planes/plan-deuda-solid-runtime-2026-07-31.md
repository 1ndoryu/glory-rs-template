# Plan 297A-23 — Deuda SOLID del runtime de apps (hipótesis)

> **Fecha:** 2026-07-31
> **Estado:** hipótesis pendientes de validar; no ejecutar hasta cerrar 297A-19 (deep links) y aprobar fases.
> **Epic:** 297A-4 (OS persistente, cuentas, programas y comercio).
> **Origen:** revisión SOLID de la guía canónica `Agente/documentacion/arquitectura/guia-agregar-app-2026-07-31.md`; hallazgos verificados contra `route-app-adapter.ts`, `app-registry.ts` y `app-registration.ts`.

## Objetivo y límites

Reducir la deuda SOLID detectada antes de que crezcan los dominios de 297A-14 (editors) y 297A-15 (comercio), sin cambiar comportamiento visible del OS.

- **Dentro:** refactor de `route-app-adapter.ts`, centralización de capacidades, resolución de la rama `authenticated` y test anti-drift registry ↔ workspace.
- **Fuera:** hot registration/plugin system, inyección de dependencias, migración de `routePatterns` → `deepLink` (ya planificada en 297A-19) y cualquier cambio de contrato público de `AppDefinition`.

## Hipótesis (validar antes y durante cada fase)

**H1 — SRP del adapter.** *"Dividir `route-app-adapter.ts` en un coordinador delgado + módulos extraídos (validación de capacidad, dedup de instancia, delegación móvil) mejora la testabilidad sin regresiones ni cambio de contrato."*
- Falsación: la refactor introduce más superficie que lógica (el coordinador vuelve a coordinar todo) o algún flujo (interceptor, `openAppWindow`, móvil) cambia de comportamiento.

**H2 — Capacidades en un solo punto.** *"Centralizar la jerarquía `public < authenticated < admin` en un módulo único (`capability.ts`) consumido por `AppRegistry.getAvailable` y `route-app-adapter` elimina la duplicación OCP/DRY."*
- Falsación: los dos consumidores no pueden expresar su intención con el mismo contrato (p. ej. el adapter necesita orden + comparación y el registry solo filtro) sin añadir acoplamiento nuevo.

**H3 — Rama muerta `authenticated`.** *"Decidir el destino de `authenticated`: la app Cuenta (297A-13) la usará para estados intermedios, o se retira del tipo `Capability`."*
- Falsación: tras decidir, queda una tercera capacidad sin consumidor o se fuerza un uso artificial solo para justificar el tipo.

**H4 — Anti-drift registry ↔ workspace.** *"Un test que verifica que todo nodo `type:'app'` en `default-release.ts`/`ADMIN_NODES` tiene `AppRegistry.register` (y viceversa) previene el bug real ya sufrido (admin sin registro)."*
- Falsación: el test genera falsos positivos (p. ej. nodos de apps planificadas aún no registradas) que obligan a suppressions en lugar de detectar drift real.

## Fases

### Fase 1 — Validación de hipótesis (sin código)
- [ ] Releer `route-app-adapter.ts` completo y listar cada responsabilidad con sus consumidores (interceptor, `openAppWindow`, móvil).
- [ ] Confirmar el único lugar donde se repite la jerarquía de capacidades (registry + adapter) y si hay más copias (buscar `'authenticated'`).
- [ ] Decidir con el usuario el destino de `authenticated` (usarla en Cuenta o retirarla).
- [ ] Mapear los nodos `type:'app'` actuales frente a registros para dimensionar el test anti-drift.

**Gate F1:** cada hipótesis queda marcada como validada/falsada con evidencia citada (archivo/línea), y H3 tiene decisión del usuario.

### Fase 2 — Centralizar capacidades (H2)
- [ ] Crear `frontend/src/features/runtime/capability.ts` con el tipo `Capability`, el orden y helpers de comparación/filtro.
- [ ] Consumirlo en `app-registry.ts` (`getAvailable`) y `route-app-adapter.ts` (frontera).
- [ ] Según decisión H3: usar `authenticated` en Cuenta o retirarla del tipo y del orden.
- [ ] `npx tsc --noEmit` + tests de registry/adapter; sin cambios de contrato externo.

**Gate F2:** una sola fuente de verdad de capacidades; cero copias de la jerarquía en el repo; type-check y tests PASS.

### Fase 3 — SRP del adapter (H1)
- [ ] Extraer validación de capacidad (usa H2) a helper puro con test unitario.
- [ ] Extraer dedup de instancia (non-singleton con params / singleton) a helper puro con test.
- [ ] Extraer la delegación móvil y `clearRuntimeApps` a módulo de presentación.
- [ ] Dejar `initRouteAppAdapter`/`openAppWindow` como coordinadores delgados que orquestan los helpers.
- [ ] Correr tests existentes del router/adapter + 209 del frontend; verificar flujo real en navegador (desktop + móvil, deep links, focus, dedup).

**Gate F3:** adapter ≤ 120 líneas efectivas de coordinación; todos los helpers con test; sin regresión visual ni de rutas (prueba en 2 resoluciones).

### Fase 4 — Test anti-drift (H4)
- [ ] Crear test que cruza `AppRegistry` registrado con nodos `type:'app'` de `default-release.ts` + `ADMIN_NODES` (refId ↔ id).
- [ ] Resolver falsos positivos reales antes de añadir suppressions.
- [ ] Verificar que detecta el caso histórico (nodo sin registro).

**Gate F4:** test PASS y detecta el caso histórico; cero suppressions injustificadas.

### Fase 5 — Cierre
- [ ] Actualizar la guía `guia-agregar-app-2026-07-31.md` si algún paso cambió (p. ej. helper de capacidades en la receta).
- [ ] Ejecutar `npm run task:check -- 297A-23` y registrar evidencia S1–S5 en el plan/completados.
- [ ] Archivar en `Agente/completados/tareas-2026-07-31.md`, actualizar roadmap y commit.

**Gate F5:** quality gate PASS; guía sincronizada; sin cambios de comportamiento observables.

## Pruebas obligatorias y Definition of Done

- [ ] Tests unitarios de cada helper extraído (positivo, negativo y regresión).
- [ ] Prueba visual en navegador: desktop ≥768px y móvil <768px; deep links, focus/restore, dedup, singleton y cambio de breakpoint.
- [ ] Evidencia S1–S5 enlazada (checkpoints SOLID).
- [ ] `npm run task:check -- 297A-23` PASS.
- [ ] Documentación (guía, roadmap, completados) sincronizada.

## Enlaces

- Guía canónica de apps: `Agente/documentacion/arquitectura/guia-agregar-app-2026-07-31.md`
- Checkpoints SOLID: `Agente/documentacion/arquitectura/checkpoints-solid-escalabilidad-2026-07-31.md`
- Manual de arquitectura: `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`
- Roadmap: `roadmap.md` (297A-23)
