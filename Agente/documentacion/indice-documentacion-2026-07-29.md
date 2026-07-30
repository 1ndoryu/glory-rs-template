# Índice canónico de documentación de wandori.us

> **Fecha:** 2026-07-29  
> **Objetivo:** indicar qué documento decide cada aspecto y evitar duplicación.

## Fuentes de verdad

| Pregunta                                                | Documento canónico                                                              |
| ------------------------------------------------------- | ------------------------------------------------------------------------------- |
| ¿Qué está pendiente y qué sigue ahora?                  | `roadmap.md`                                                                    |
| ¿En qué orden se implementa toda la iniciativa?         | `Agente/planes/plan-escritorio-persistente-cuentas-admin-apps-2026-07-29.md`    |
| ¿Cómo se separan módulos, datos, seguridad y contratos? | `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md` |
| ¿Cómo debe verse y comportarse visualmente el OS?       | `Agente/documentacion/design-system/manual-identidad-visual-os-2026-07-29.md`   |
| ¿Cómo funciona el OS en teléfonos sin duplicar apps?    | `Agente/planes/plan-experiencia-movil-launcher-2026-07-29.md`                  |
| ¿Cómo funciona el quality gate?                        | `Agente/planes/completados/plan-escalabilidad-sentinel-wandorius-2026-07-29.md` |
| ¿Cómo funcionan comandos, menús, drag y estadísticas?  | `Agente/planes/plan-contratos-interaccion-comandos-medicion-2026-07-29.md`      |
| ¿Qué reglas automáticas faltan?                         | `Agente/prevencion/prevencion-wandorius-sentinel-varsense-2026-07-29.md`        |

## Regla de autoridad

1. El roadmap solo contiene trabajo pendiente y enlaces.
2. El plan maestro solo contiene secuencia, checklists, dependencias y criterios de salida.
3. Arquitectura e identidad no se duplican dentro de planes.
4. Prevención contiene reglas automatizables pendientes, no decisiones de producto.
5. Los planes superados o completados se mueven a `Agente/planes/completados/`.
6. Una contradicción se resuelve en el documento canónico y luego se actualizan referencias.

## Estado de consolidación

- Manual de arquitectura: creado.
- Manual visual: creado a partir del concepto aprobado y la interfaz real.
- Plan maestro: activo; debe ejecutarse por checklist.
- Contratos de interacción y medición: activos; se cierran dentro de las tareas dueñas 297A-9–17.
- Quality gate Sentinel/VarSense: implementado y archivado; CI y self-check usan el mismo core.
- Plan móvil: activo y bloqueado por runtime/workspace; tablet conserva desktop.
- Plan visual antiguo: referencia histórica del concepto aprobado.
- Plan wandori.us original: superado; no es especificación activa.
- Plan Sentinel/VarSense editor-agnóstico: trabajo histórico documentado en tareas completadas.
