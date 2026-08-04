---
applyTo: '**'
---

# AGENTS — wandori.us

## 1. Prioridad actual

1. Leer `roadmap.md` (debe listar siempre los planes pendientes de `Agente/planes/` y estar actualizado) y revisar `Agente/planes/` por planes activos; ejecutar solo el primer bloque habilitado.
2. **Primero Sentinel/VarSense:** cerrar cada tarea con el quality gate unificado antes de avanzar.
3. No iniciar seguridad, runtime, workspace, móvil, programas o comercio hasta cerrar su dependencia documental/técnica.
4. Identidad visual aprobada; cambios materiales requieren actualizar manual y aprobación visual.
5. Deploy fuera de alcance hasta instrucción explícita.

## 2. Fuentes canónicas

- Pendientes/orden: `roadmap.md`.
- Plan maestro/checklists: `Agente/planes/plan-escritorio-persistente-cuentas-admin-apps-2026-07-29.md`.
- Arquitectura: `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`.
- Identidad OS: `Agente/documentacion/design-system/manual-identidad-visual-os-2026-07-29.md`.
- Móvil: `Agente/planes/plan-experiencia-movil-launcher-2026-07-29.md`.
- Quality gate: `Agente/planes/completados/plan-escalabilidad-sentinel-wandorius-2026-07-29.md`.
- Interacción/comandos/medición: `Agente/planes/plan-contratos-interaccion-comandos-medicion-2026-07-29.md`.
- Reglas pendientes: `Agente/prevencion/prevencion-wandorius-sentinel-varsense-2026-07-29.md`.
- Motor futuro agnóstico: `Agente/planes/plan-glory-render-motor-juegos-2026-08-01.md` y ADR `Agente/documentacion/arquitectura/adr-glory-render-repositorio-agnostico-2026-08-01.md`.
- Índice: `Agente/documentacion/indice-documentacion-2026-07-29.md`.

No dupliques decisiones: actualiza primero la fuente correspondiente y luego sus referencias.

## 3. Arquitectura resumida

**Stack:** Axum/SQLx/PostgreSQL + Vanilla TypeScript/Vite. No React, Zustand ni CSS-in-JS.

**Frontend mínimo:**

- `MountedView` + AbortSignal/teardown.
- AppRegistry local con capacidades.
- WindowManager/reducer para desktop/tablet.
- MobileAppStack/launcher como presentación móvil del mismo runtime.
- CommandRegistry para barra/clic derecho/long press.
- RouteAppAdapter para URL ↔ app/recurso.
- AnalyticsDispatcher tipado.
- Una app devuelve contenido; solo el shell crea ventana/chrome.

**Juegos reutilizables:** `frontend/src/features/game-core/` es provisional. Después de GAME-01/Fase 8, la lógica agnóstica se extraerá a `glory-render/` dentro de este workspace, con Git/CI/versionado propios; no entran identidad, OS, backend, salas ni reglas de Bosque.

**Backend por dominios:** identity, workspace, content, media, commerce, analytics y audit. Flujo obligatorio: `handler -> service/command -> repository/adaptador`. SQL solo en repositories; transacciones/outbox en services.

**Datos:**

- Workspace guarda nodos/referencias; contenido vive en recursos tipados.
- Release público inmutable + overlay personal + estado de sesión.
- Carpetas son nodos; un recurso puede tener varias referencias.
- Estados independientes: editorial, visibilidad, lifecycle y comercio.
- Todo recurso nace `draft + private + active`; producto además inactivo.

**Seguridad:** capacidades siempre server-side; sesiones opacas revocables en cookie; CSRF/origin/rate limit; públicos nunca reciben drafts/assets privados. Precio/pago/entrega nunca confían en navegador. Webhook verificado e idempotente; descargables privados mediante entitlement + grant corto. Analytics y audit separados.

## 4. Presentaciones del OS

- **Desktop/tablet (>=768):** escritorio, ventanas flotantes, barra superior y taskbar.
- **Móvil (<768):** launcher tipo teléfono; sin ventanas, barra superior ni taskbar; apps a pantalla completa.
- Mismas apps, recursos, permisos, comandos y rutas en ambos modos. No duplicar lógica ni crear `MobileFooApp`.
- Cambiar de breakpoint transforma la presentación y preserva app/recurso activo; tablet conserva escritorio.

## 5. Identidad visual resumida

- Macintosh 1984/Mac OS 9 minimalista, no emulación literal.
- Chrome blanco/negro, sin sombras, radios, blur, gradientes suaves ni bordes >1px.
- JetBrains Mono en todo el OS.
- Solo Lucide oficial, stroke 1px; excepción de radio: círculo negro de marca.
- Una sola DesktopWindow: X izquierda, título centrado, Minus derecha, resize por bordes, sin grip.
- Menús compactos, 2px menores, sin separadores; teclado completo.
- Taskbar sin Inicio: Nav + ventanas abiertas/estado/cerrar.
- Tokens en `variables.css`; sin CSS inline ni recetas visuales locales por app.
- Contenido multimedia puede conservar color; el chrome permanece monocromo.
- Verificar 1440×900, 1024×768, 390×844 y 320px, además de foco, teclado y zoom 200%.

## 6. Flujo obligatorio por tarea

1. Leer roadmap completo (listado de planes pendientes y estado siempre actualizado), revisar `Agente/planes/` por planes activos, y leer manuales aplicables y checklist del bloque.
2. Delegar a subagentes investigación, búsquedas, lecturas grandes y diagnósticos; el agente principal decide/edita/valida.
3. Confirmar dependencias y aclarar solo dudas que cambien materialmente el resultado.
4. Preguntar internamente: “¿es la mejor opción arquitectónica o el camino fácil?” Resolver raíz, no parche.
5. Editar por módulo con `apply_patch`; preservar cambios ajenos y corregir desorden visible de bajo riesgo.
6. Completar el bloque antes de validar; no ejecutar pruebas pesadas tras cada microcambio.
7. Ejecutar el quality gate único de tarea cuando exista; hasta entonces usar validaciones del stack.
8. Probar el flujo real. UI requiere navegador y viewports; seguridad/comercio requieren casos negativos.
9. Actualizar roadmap, completados, manuales, prevención y lecciones. Todo plan activo usa checklists, dependencia, gate y criterio de salida.
10. Revisar `git status`/`diff`; stage explícito (nunca `git add .`), commit `{ID}: descripción`, pull/rebase y push.
11. Releer roadmap y planes activos de `Agente/planes/` como última acción, actualizar el roadmap con el estado real de los planes, y elegir solo el siguiente bloque habilitado.

> **Commit por tarea (obligatorio):** primero el gate, después el commit. Al cerrar una tarea entregable: ejecutar `npm run task:check -- {ID}` y, si pasa, commitear de inmediato. Cada tarea terminada = un commit `{ID}: descripción`; prohibido acumular archivos modificados pasando de una tarea a otra sin commitear. Diagnósticos, prototipos intermedios o cambios compartidos que aún no estén listos no se fuerzan como commit de bloque: en ese caso el reporte deja el recordatorio de revisar `git status` y documentar el estado, y no se inicia la siguiente tarea con cambios pendientes sin resolver.

## 7. Quality gate por tarea

El orquestador Node multiplataforma se ejecuta con un único comando público:

```text
npm run task:check -- 297A-N
```

Cuando el gate está configurado (como en este proyecto), el cierre de tarea es siempre por `task:check` y los comandos directos pesados quedan bloqueados por el guard. La skill global **`quality-gate-setup`** documenta esta configuración (archivos, scripts, lock, guard) y cómo replicarla en proyectos desde cero; si un proyecto no tiene gate configurado, se valida por stack hasta configurarlo.

Orden obligatorio:

1. Preflight de config/binarios/paths sin instalar ni mutar.
2. Glory Sentinel con config canónica.
3. VarSense sobre CSS/clases/tokens.
4. Validaciones del stack afectado.
5. Reporte combinado Markdown + JSON con task ID, commit/config y tiempos.
6. Terminal limitada a tres hallazgos y máximo cuatro recordatorios contextuales.
7. Indicar exactamente qué corregir y qué comando repetir.
8. Exit code no cero ante error de herramienta, regla bloqueante o test fallido.

El script decide alcance automáticamente, es incremental local y full en CI. Debe ser no interactivo, acotado, redactor de secretos y con cache segura de etapas PASS. No reemplaza tests ni convierte fallos en warnings. Sentinel/VarSense usan su core/CLI oficial; el script no duplica reglas. El detalle vive en `.quality-reports/`, no en stdout/contexto. `cargo test`, `cargo clippy`, `cargo bench` y `task:check --full` están protegidos por un cooldown de 180 minutos y una sola ejecución pesada; un full diferido cae a `local-light`. `--allow-heavy` es una excepción manual auditable.

## 8. Estándares esenciales

- TypeScript: camelCase/PascalCase; archivos/carpetas en inglés por dominio.
- CSS del proyecto: clases en español camelCase, tokens centralizados; migrar legacy por componente.
- Máximos: componentes/CSS 300 líneas, lifecycle/store/hook 120, utils 150; dividir por responsabilidad.
- Orval Fetch `tags-split`; generated no se edita.
- Input se valida en boundary; SQL preparado/tipado; secretos solo env.
- Prohibido `eval`, HTML no sanitizado, unwrap sobre input externo, catch vacío, I/O silencioso y operaciones críticas sin resultado.
- Evitar N+1/roundtrips, estados redundantes y abstracción sin segundo caso real.
- Comentarios `[ID]` explican por qué/gotchas; registrar lección si existe.
- Funcionalidad agnóstica/reutilizable pertenece a Glory; lógica wandori.us permanece aquí.

## 9. Validación

- Cierre normal: `npm run task:check -- {ID}`; el alcance se calcula automáticamente.
- Rust local: no ejecutar `cargo test 2>&1` por tarea; la redirección sigue siendo el mismo test pesado. Usar el gate y reservar `--full` para cierre de fase/CI o una excepción justificada.
- Targets: `npm run quality:cleanup:dry` revisa `C:\tmp\glory-target`; `npm run quality:cleanup` aplica cuota/retención sin tocar targets con proceso activo.
- Compatibilidad: `npm run self-check -- -TareaId {ID}` llama al mismo core y no duplica validaciones.
- CI usa el mismo core en modo full y publica `.quality-reports/`.
- UI todavía exige prueba visual real; el gate no sustituye navegador ni casos negativos.
- Un fallo preexistente se registra y corrige en bloque separado antes de cerrar; no se oculta con suppressions.

## 10. Git, terminal y deploy

- Comandos acotados, no interactivos y con timeout. Servidores/watchers en background con señal de readiness.
- Tras un fallo, identificar causa antes de reintentar; no probar variantes a ciegas.
- Nunca borrar/mover recursivamente sin resolver y verificar path absoluto.
- No reiniciar/recargar VS Code automáticamente.
- Producción exclusivamente mediante `coolify-manager-rs`; prohibido SSH/docker/scp/curl directo al servidor.
- Flujo deploy futuro: `deploy --update` -> `health` -> si falla `redeploy`/restore. Nunca desplegar sin instrucción y roadmap.
- Concurrencia: el checkout es compartido con otros agentes/IDE/usuario. Stage únicamente tus propios archivos y hunks; prohibido `git add -A`/`git add .`; no descartar, stashear, commitear ni empujar cambios ajenos. Si la propiedad de un cambio es ambigua, déjalo sin commitear y documenta qué queda pendiente.

## 11. Herramientas obligatorias

### Coolify

- Toda operación de producción —deploy, restart, logs, backup, restore, exec y health— usa `coolify-manager-rs`.
- Prohibido SSH, Docker, SCP o curl directo al servidor, salvo emergencia documentada para mejorar la herramienta.
- Binario: `C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs\target\release\coolify-manager.exe`.
- Flujo: verificar binario -> `deploy --name <sitio> --update` -> `health --name <sitio>` -> si falla `redeploy` o `restore`.
- Rust puede tardar; ejecución remota larga requiere progreso/timeout/readiness, nunca espera ciega.
- Si una operación no está soportada, crear tarea para ampliar manager; no saltárselo.

### Glory Sentinel

- Repo: `C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\code-sentinel`; reglas bajo core/analyzers, agnósticas del proyecto.
- wandori.us configura reglas en `sentinel.config.json`; no crea regex/scripts paralelos.
- CLI, LSP, VS Code y Zed deben producir hallazgos equivalentes mediante fixtures.
- No usar `sentinel-disable-file` salvo justificación por regla/archivo/tarea/fecha de retirada.
- No empaquetar/instalar/reiniciar editor sin completar pruebas y autorización aplicable; nunca recargar VS Code automáticamente.

### VarSense

- Repo: `C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\varsense`; valida tokens, hardcodes y clases CSS.
- Config del proyecto define includes/excludes mínimos; no duplicar validación en scripts locales.
- Ejecutar después de cambios CSS mediante `task:check`; corregir referencias inexistentes y huérfanas antes de cerrar.
- Reglas visuales provienen del manual de identidad, no de decisiones ad-hoc del analizador.

## 12. Organización de `Agente/`

```text
Agente/
  completados/
    tareas-YYYY-MM-DD.md
  documentacion/
    indice-documentacion-YYYY-MM-DD.md
    arquitectura/
    design-system/
    producto/                 # solo si existe visión vigente separada
    herramientas/
  lecciones/
    lecciones-aprendidas.md
  planes/
    plan-tema-YYYY-MM-DD.md   # únicamente activos
    completados/              # cerrados o históricos
  prevencion/
    prevencion-tema-YYYY-MM-DD.md
```

Reglas:

- Carpetas/código fuente en inglés; documentación del agente conserva estructura española anterior.
- Todo MD nuevo lleva fecha; actualizar documento existente antes de crear duplicado.
- Un tema tiene una fuente canónica enlazada desde el índice.
- Plan superado/completado sale de `planes/` y entra en `planes/completados/` con estado histórico.
- Prevención contiene solo reglas automatizables pendientes; al implementarlas, archivar/eliminar según completados y actualizar roadmap.
- Lecciones registran causas y patrones reutilizables, no el resumen de cada cambio trivial.

## 13. Cómo organizar `roadmap.md`

- Solo pendientes ejecutables; nunca visión histórica, completados, notas sueltas o especificaciones extensas.
- Encabezado breve: producto, stack, deploy, epic y estado visual.
- Enlaces a fuentes canónicas.
- Planes pendientes: el roadmap lista los planes activos de `Agente/planes/` con su estado y se actualiza ante cualquier cambio (plan nuevo, avanzado, completado o movido a `planes/completados/`); nunca queda desactualizado respecto a los planes.
- Un único “Siguiente bloque” claramente habilitado.
- Tareas en orden real con ID, dependencia, checklist corto y criterio de salida.
- Epic agrupa; tarea ejecutable cabe en un bloque/commit. Si no cabe, subdividir en plan, no inflar roadmap.
- Al completar: retirar del roadmap y registrar en `Agente/completados/tareas-YYYY-MM-DD.md`.
- Una nueva petición se ubica por dependencia/prioridad; no se ejecuta automáticamente si rompe el gate actual.

## 14. Cómo crear y mantener planes

Todo plan activo debe incluir:

- Tarea/epic, fecha, estado y siguiente paso.
- Objetivo y límites explícitos.
- Dependencias y qué bloquea.
- Fases en orden con `- [ ]` para cada acción verificable.
- Gate/criterio de salida por fase.
- Pruebas obligatorias y Definition of Done.
- Enlaces a manuales; no copiar arquitectura/identidad completa.

Marcar un checkbox solo con evidencia. Si cambia el orden, actualizar primero plan maestro, luego roadmap y planes especializados. Las decisiones duraderas con alternativas se registran como ADR en `documentacion/arquitectura/`.

## 15. Documentación al cerrar

- `roadmap.md` contiene solo pendientes.
- Activos en `Agente/planes/`; completados/históricos en `Agente/planes/completados/`.
- Completadas en `Agente/completados/tareas-YYYY-MM-DD.md` con Qué/Archivos/Gotchas/Sentinel/GLORY.
- Arquitectura, contratos o comportamiento cambian manual/documentación; estética cambia manual visual.
- Prevención solo contiene reglas automatizables pendientes.
- No marcar checklist sin evidencia; no cerrar plan mientras quede un ítem requerido.
