# Plan posterior: migrar consumidores de Glory RS a `main`

> **Estado:** pendiente; ejecutar únicamente después de estabilizar el freezer de Nakomi.
> **Repositorio núcleo:** `1ndoryu/glory-rs-framework`
> **Commit unificado inicial:** `0249f5a`

## Objetivo

Todos los proyectos, scripts, Dockerfiles, settings y automatizaciones que hoy
consuman `master` o `085A-dev-launcher-centralizado` deben pasar a consumir
`main`, que contiene tanto el launcher centralizado como el runtime/watchdog.

## Orden obligatorio

1. No iniciar mientras el incidente freeze/Bad Gateway de Nakomi siga abierto.
2. Cambiar primero la rama predeterminada de GitHub a `main`.
3. Inventariar referencias sin editar:
   - dependencias Git de Cargo;
   - dependencias `path` y submódulos/checkouts locales;
   - `Dockerfile` y scripts con `git clone`, `checkout` o `--branch`;
   - configuraciones de `coolify-manager-rs` (`library_branch`, templates y defaults);
   - workflows CI/CD;
   - launchers y workspaces locales;
   - documentación que instruya usar `master` o la rama dev.
4. Crear una tabla por consumidor con repositorio, archivo, rama anterior,
   cambio requerido, validación y estado de deploy.
5. Migrar un proyecto canary de bajo riesgo.
6. Validar en el canary:
   - launcher centralizado;
   - `cargo check`, Clippy y tests;
   - build Docker reproducible;
   - watchdog/runtime disponible;
   - health post-deploy.
7. Migrar los demás proyectos uno por uno, con commit y rollback independientes.
8. Confirmar que ninguna búsqueda global encuentra referencias operativas a
   `master` o `085A-dev-launcher-centralizado`.
9. Mantener ambas ramas antiguas como rollback durante al menos un ciclo de
   despliegue estable. No borrarlas ni forzar su historial durante la migración.
10. Solo después actualizar protecciones, CI y documentación para declarar
    `main` como la única rama soportada.

## Criterio de cierre

- GitHub usa `main` como rama predeterminada.
- Todos los consumidores conocidos están inventariados.
- Cada consumidor compila y pasa sus tests con `main`.
- Los proyectos desplegados pasan health después del cambio.
- No quedan referencias operativas a las ramas anteriores.
- Existe rollback documentado al commit previo de cada consumidor.

## Prohibiciones

- No hacer reemplazo global ciego de la palabra `master`.
- No actualizar todos los proyectos en un único commit.
- No borrar `master` ni `085A-dev-launcher-centralizado` antes de cerrar la
  migración completa.
- No mezclar esta migración con el hotfix del freezer.
