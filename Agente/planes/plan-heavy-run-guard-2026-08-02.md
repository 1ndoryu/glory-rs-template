# Plan 028A-3 — Guard global de ejecuciones pesadas y targets Cargo

> **Fecha:** 2026-08-02
> **Estado:** activo; núcleo implementado, activación del perfil PowerShell pendiente de autorización explícita.
> **Dueño:** `roadmap.md` y `roadmap-sentinel.md`.

## Objetivo y límites

Evitar que una tarea o un agente lance `cargo test`, `cargo clippy`, `cargo bench` o un quality full cada pocos minutos, y detener la acumulación ilimitada de targets bajo `C:\tmp`. El control es por proyecto, pero se ejecuta globalmente a través del estado compartido del guard y los wrappers de Cargo. CI puede ejecutar full sin cooldown.

## Política efectiva

- Cooldown por proyecto: **180 minutos** después de cualquier intento pesado, incluso si terminó con error.
- Concurrencia: un proceso pesado a la vez por target base.
- Excepción manual: `--allow-heavy` o `GLORY_QUALITY_ALLOW_HEAVY=1`; debe quedar visible en el reporte.
- Full bloqueado: se degrada a `local-light` y no deja al agente esperando ni recompilando tests.
- Targets: cuota de 15 GB, retención de 7 días, limpieza únicamente bajo una raíz validada `*/tmp/glory-target` y preservando marcadores activos.
- `cargo test 2>&1`: se clasifica como el mismo `cargo test`; la redirección no cambia el coste.

## Checklist

### Fase 1 — Guard del quality gate

- [x] Añadir estado compartido por proyecto y cooldown configurable en `quality.config.json`.
- [x] Bloquear concurrencia pesada y liberar el lock aun con error/señal.
- [x] Convertir full bloqueado a local-light con razón y siguiente comando reproducible.
- [x] Separar el fingerprint/report mode y recordar `--allow-heavy`.

### Fase 2 — Cargo directo y wrappers

- [x] Aplicar el guard a `scripts/run-with-db.mjs` para `test/clippy/bench`.
- [x] Añadir `cargo.cmd` y wrapper PowerShell que localizan el proyecto y pasan comandos pesados al guard.
- [x] Instalar el shim por PATH sin modificar perfiles automáticamente.
- [ ] Revisar y activar ambos perfiles PowerShell tras backup y autorización explícita; nunca reescribir perfiles ajenos sin revisión.

### Fase 3 — Targets y mantenimiento

- [x] Añadir limpieza dry-run/real, cuota, retención y detección de procesos activos.
- [x] Ejecutar limpieza inicial de los targets huérfanos bajo `C:\tmp\glory-target`.
- [ ] Añadir ejecución periódica opcional al cierre de `task:check` sin escanear/borrar fuera de la raíz validada.

## Validación y Definition of Done

- [x] `npm run quality:test` pasa con las pruebas del guard y del parser.
- [x] `npm run quality:cleanup:dry` muestra candidatos sin borrar; la limpieza real preserva el target activo.
- [x] `cmd /c where cargo` muestra primero el shim administrado y después el Cargo real.
- [ ] Perfil PowerShell cargado sin errores y `cargo test` bloqueado durante cooldown en una nueva sesión.
- [ ] Auditoría SOLID/rendimiento/seguridad: estado atómico, rutas verificadas, sin secretos en mensajes, una sola política reutilizable.
