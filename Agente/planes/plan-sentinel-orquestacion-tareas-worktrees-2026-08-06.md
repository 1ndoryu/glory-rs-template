# Plan — Orquestación universal de tareas con Sentinel, ramas y worktrees

> **Fecha:** 2026-08-06
> **Tarea:** 028A-18
> **Estado:** implementación parcial cerrada en Sentinel; integración del consumidor y GC pendientes
> **Ámbito:** Glory Sentinel universal; no depende de wandori.us ni de su stack

## 1. Objetivo y decisión

Sentinel debe coordinar trabajo paralelo por tarea sin compartir un checkout mutable: una tarea obtiene
ownership atómico, un `git worktree` y una rama exclusiva, pasa el gate dentro de ese árbol, se integra
solo con `--ff-only` a un target limpio y se limpia de forma verificable.

La unidad recomendada es **una tarea por agente activo**. Se pueden tomar varias tareas independientes,
pero cada una conserva su claim, worktree, rama, gate, reporte y cleanup. No se reutiliza una rama o
worktree para varias tareas. Las tareas acopladas o que tocan tooling/submódulos/contratos compartidos
son seriales y deben declararse como tales por el consumidor.

## 2. Invariantes

- Dos claims simultáneos del mismo ID dejan un único ganador.
- Cada tarea tiene como máximo un worktree y una rama `task/<task-id>`; nunca se reutilizan recursos.
- Los IDs de tarea/agente son allowlisted, acotados y no permiten traversal ni shell injection.
- El estado efímero se guarda en `.git/sentinel-task-coordination/`, fuera del checkout.
- `heartbeat` renueva ownership; una toma expirada requiere takeover explícito y no puede robar recursos
  aún registrados.
- `start` exige checkout de origen limpio, rama/base válidas y path sin ocupar; no hace reset ni force.
- `task gate` solo puede ejecutarse en el worktree registrado para esa tarea y delega a `sentinel check`.
- `integrate` exige agente propio, target limpio, worktree limpio, commit nuevo, base estable y
  `merge --ff-only`; detecta avance/divergencia y conserva la rama para recuperación manual.
- No hay commit implícito, push, deploy, rebase automático, resolución de conflictos ni borrado ajeno.
- `cleanup` es idempotente después de integrar; la recuperación forzada solo aplica a tomas expiradas,
  proceso emisor muerto, worktree limpio y rama determinista.
- `status` nunca borra: diagnostica metadata inválida, worktrees/ramas huérfanos y locks expirados.

## 3. Ciclo operativo

```text
claim → start → heartbeat/gate → commit explícito → integrate --ff-only → cleanup → release
```

Ejemplo portable:

```bash
sentinel task claim GAME-01 --project-root . --agent agent-a
sentinel task start GAME-01 --project-root . --agent agent-a --base main
sentinel task gate GAME-01 --project-root ../.sentinel-worktrees/repo-<hash>-GAME-01 --agent agent-a
sentinel task integrate GAME-01 --project-root . --agent agent-a --target main
sentinel task cleanup GAME-01 --project-root . --agent agent-a
sentinel task release GAME-01 --project-root . --agent agent-a
```

Para varias tareas independientes, repetir el ciclo completo por ID; no abrir dos agentes sobre el
mismo ID. El registro de toma del proyecto y el coordinador de Sentinel son capas complementarias:
el primero evita carreras entre agentes antes de iniciar; Sentinel evita carreras de Git durante el
ciclo y en otros proyectos.

## 4. Contrato CLI

Sentinel implementa:

```text
sentinel task claim <id> --project-root <dir> --agent <id> [--force] [--json]
sentinel task start <id> --project-root <dir> --agent <id> [--base main] [--path <dir>]
sentinel task heartbeat <id> --project-root <dir> --agent <id>
sentinel task status --project-root <dir> [--json]
sentinel task gate <id> --project-root <worktree> --agent <id> [--full|--ci]
sentinel task integrate <id> --project-root <dir> --agent <id> [--target main]
sentinel task cleanup <id> --project-root <dir> --agent <id> [--force]
sentinel task release <id> --project-root <dir> --agent <id>
```

La salida JSON es versionada por la metadata de tarea (`schemaVersion: 1`) y no contiene secretos,
tokens ni contenido de archivos. `task gate` conserva el exit code real del gate también en salida
humana. `status` devuelve `tasks`, `invalidMetadata`, `orphanWorktrees`, `orphanBranches` y
`expiredLocks`.

## 5. Implementación cerrada en Sentinel

- [x] Coordinador agnóstico en `tools/sentinel/src/core/taskCoordinator.ts`.
- [x] Claims serializados con directorios exclusivos; takeover de locks expirados mediante `rename`
  y liberación protegida con token de propietario.
- [x] TTL, heartbeat, takeover explícito, validación de agente e IDs y detección de colisiones Windows.
- [x] Worktree externo determinista con hash del Git common dir; rama exclusiva; rechazo de paths/ramas
  ocupados; checkout principal limpio.
- [x] Verificación de path real, rama y registro Git antes de ejecutar el gate.
- [x] Integración serializada por target con estado `INTEGRATING` reanudable tras crash; dirty checks,
  base/HEAD race detection y fast-forward únicamente.
- [x] Cleanup idempotente y recuperación forzada limitada por TTL, PID/host, árbol limpio y rama
  determinista.
- [x] CLI, ayuda, README y CHANGELOG de Sentinel actualizados.
- [x] Cobertura dirigida: claims concurrentes, takeover, worktree, integración, target sucio,
  release cruzado, gate en path incorrecto, heartbeat y CLI; suite Sentinel: **484 PASS, 1 pending**.
- [x] Commit upstream/local del submódulo: `31fb52f1dd20e1c86d1cc4b2fe5bb06c2275885e`.

## 6. Pendientes por fases

### Fase 1 — Integración del consumidor

- [x] Fijar el gitlink `tools/sentinel` al commit coordinador `31fb52f1dd20e1c86d1cc4b2fe5bb06c2275885e` y
  regenerar `sentinel.lock.json`/`quality-tools.json` sin rutas absolutas; `quality:lock --check` PASS.
- [ ] Ejecutar el quality gate completo del consumidor con el ID y el mismo `GLORY_AGENT_ID` del claim.
- [x] Mantener cambios preexistentes de otros agentes sin absorberlos: el submódulo `glory-rs` sucio
  actual no pertenece a esta tarea y permanece sin stage.

**Gate F1:** lock-check PASS y gitlink reproducible; el gate completo del consumidor y su commit final
siguen pendientes por la política de ownership/gate de la raíz.

### Fase 2 — Garbage collection y recuperación

- [ ] Añadir `sentinel task gc --dry-run` y modo aplicado con auditoría; nunca borrar árboles activos.
- [ ] Poda por TTL/cuota y detección de PID/host; incluir metadata sin worktree, ramas sin tarea y
  locks expirados, conservando todo lo que no pueda probarse seguro.
- [ ] Añadir runbook para crash antes/después de merge, target avanzado, conflicto, disco lleno y
  agente ausente.

**Gate F2:** simulación de crash y cleanup repetido dejan cero artefactos propios sin tocar recursos
ajenos.

### Fase 3 — Portabilidad y adopción

- [ ] Fixtures multi-proyecto Node/Rust/Python/no-policy y matriz Windows/macOS/Linux/CI.
- [ ] Definir política opcional de cuota/simultaneidad; por defecto no iniciar procesos destructivos
  ni imponer una cuota que rompa proyectos pequeños.
- [ ] Documentar adaptación mínima para consumidores: ID, agente, base/target, gate y raíz de worktrees.
- [ ] Versionar/publicar el release de Sentinel y migrar consumidores mediante lock reproducible.

**Gate F3:** CLI/core producen el mismo estado, JSON estable, documentación alineada y ningún
worktree/branch de test queda vivo.

## 7. Riesgos y mitigaciones

| Riesgo | Mitigación |
|---|---|
| Dos agentes reclaman el mismo ID | claim exclusivo y ownership del proyecto |
| Dos procesos recuperan el mismo lock | takeover por `rename`, token de liberación y TTL renovado |
| Target avanza mientras se integra | lock por target, HEAD/base revalidados y `ff-only` |
| Crash tras `merge` | estado `INTEGRATING` y reanudación basada en HEAD target/branch |
| Metadata manipulada | schema estricto, rama determinista, path real y worktree Git registrado |
| Agente desaparece | TTL, status y takeover explícito; nunca robo silencioso |
| Basura acumulada | cleanup idempotente ahora; GC auditable pendiente |
| Submódulo o checkout sucio | start/integrate bloquean y preservan cambios ajenos |
| Proyectos con mismo nombre | hash del Git common dir en el path por defecto |

## Definition of Done

- [x] Plan universal en MD y fuente enlazada desde el roadmap.
- [x] Sentinel ofrece claim/start/status/heartbeat/gate/integrate/cleanup/release.
- [x] Claims, worktrees, ramas, integración y cleanup tienen pruebas dirigidas.
- [x] Sentinel compila y pasa su suite completa: 484 PASS, 1 pending.
- [ ] GC/runbook/matriz multi-OS completos.
- [x] Gitlink y lock del consumidor fijados; commit final en la rama objetivo aprobada queda pendiente de autorización explícita del usuario.
