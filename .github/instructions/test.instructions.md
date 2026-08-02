---
applyTo: '**'
---

# Protocolo de Desarrollo v5.0 — Rust + React + OpenAPI

## I. REGLAS ABSOLUTAS (por prioridad)

**-1. LO MAS DIFICIL PRIMERO.**
Siempre abordar primero lo mas complejo, la tarea mas dificil primero.

**0. El flujo es obligatorio e innegociable.**
Antes de ejecutar cualquier tarea, la primera respuesta al usuario SIEMPRE debe ser un anuncio breve con este formato exacto:

> **Flujo que voy a seguir:**
> 1. Leer roadmap completo
> 2. Por cada tarea: ejecutar → validar errores → testear → archivar en completados/ → actualizar roadmap → commit y push. Seguir cada paso estrictamente, paso 1 a 10 y repetir.
> 3. Repetir hasta vaciar pendientes.
>
> **Tareas identificadas:** [lista de IDs y títulos]

Sin este anuncio, no se inicia ninguna tarea. Esta regla existe para que el agente no optimice el flujo a conveniencia propia, salte pasos o agrupe lo que no debe agruparse.

**Prohibido explícitamente:**
- Completar una tarea sin archivarla inmediatamente en `completados/` antes de pasar a la siguiente.
- Hacer commit de una tarea sin haber actualizado el roadmap (quitar la tarea de pendientes).
- Avanzar a la siguiente tarea si la anterior no tiene: commit + entrada en `completados/` + roadmap actualizado.
- Enfocarse en varias tareas al mismo tiempo al menos que esten muy relacionadas.
- Detenerte a mitad de ejecución o pedir confirmacion para tareas triviales o pasos del flujo. El flujo es un ciclo continuo, no un checklist individual.

**1. Autonomia total.** Trabaja continua y prolongadamente sin detenerte. Prohibido pedir confirmacion trivial, dividir tareas artificialmente o interrumpir el flujo. Maxima eficiencia por interaccion.

**2. Cero parches.** Toda solucion debe escalar 10x sin reescritura. Antes de implementar: "Es la mejor opcion arquitectonica o el camino facil?" Si es lo segundo, redisenar. Prohibido justificar con "es temporal" o "lo refactorizamos despues".

**2.1 Pensamiento expansivo obligatorio.** Incluso si la tarea parece pequena, primero evaluar si revela un problema de arquitectura, sincronizacion, contratos, cache, observabilidad o UX mas profundo. No limitarse al sintoma pedido si existe una solucion raiz claramente superior. Cada tarea es una oportunidad para mejorar el sistema, no solo para apagar un fuego local.

**3. Ediciones controladas.** Prohibido editar muchos archivos simultaneamente en un solo parche. Los cambios grandes fallan — dividir en ediciones pequenas, archivo por archivo, validando despues de cada uno. Un parche que toca 10 archivos a la vez es un parche que rompe cosas. Secuencia: editar archivo → validar → siguiente archivo.

**4. Guardian del orden.** Eres responsable absoluto de que el proyecto no se desordene. Al tocar un archivo, corregir toda violacion visible de bajo riesgo (imports muertos, hardcodeo, codigo muerto, nombres confusos). Si la correccion es compleja, dejar TO-DO en el codigo. No existe "no es mi tarea".

**5. Seguridad primero.** 
  - SQL: siempre prepared statements / query builders (usar `sqlx::query_as!` para detectar problemas en tiempo real). Prohibido interpolar strings en queries. Usar parametros `$1, $2...` siempre.
  - Rust: todo input externo se valida con tipos fuertes y `serde` + validadores (`validator` crate o validacion manual en constructores). Newtypes para IDs y valores de dominio (`UserId(i64)`, `Email(String)`) — nunca `String` crudo para datos semanticos.
  - Secrets: siempre variables de entorno (`.env` + `dotenvy`), nunca en codigo fuente. Secrets nunca en logs ni en respuestas de error.
  - Endpoints: autenticacion/autorizacion explicita por ruta. Prohibido endpoint sin guard de permisos a menos que sea intencionalmente publico (documentar con comentario).
  - Frontend: sanitizar toda entrada antes de renderizar. Prohibido `dangerouslySetInnerHTML` con datos dinamicos sin sanitizar. CORS configurado explicitamente en el backend.
  - Dependencias: `cargo audit` y `npm audit` como parte de la validacion. Vulnerabilidades criticas se corrigen antes de avanzar.

**6. Sin fallos silenciosos.**
  - Rust: todo `Result` y `Option` se maneja explicitamente. Prohibido `.unwrap()` en codigo de produccion excepto en inicializacion con comentario justificando por que no puede fallar. Usar `?`, `map_err`, `context()` (anyhow/thiserror). Prohibido `let _ = operacion_que_puede_fallar()`.
  - Errores de dominio usan `thiserror` con variantes semanticas. Errores internos usan `anyhow` con contexto. Los handlers traducen errores de dominio a HTTP status codes apropiados.
  - Logging estructurado obligatorio: `tracing` con spans por request. Toda operacion I/O, BD, red: debe tener tracing con contexto suficiente para debuggear en produccion.
  - React: errores de API retornan estructura tipada, nunca enmascarar como exito. Toda falla = feedback visible al usuario (toast/banner). Updates optimistas con rollback si falla. React Query maneja reintentos y cache.
  - OpenAPI: toda respuesta de error documentada en el schema (`utoipa::path` responses). El codegen genera tipos de error que el frontend usa.

**7. Rendimiento.**
  - Prohibido queries N+1 o roundtrips innecesarios. Combinar con CTEs/CASE/JOINs. `sqlx::query!` valida queries en compile-time contra la BD.
  - Connection pooling obligatorio (sqlx `PgPool`). Configurar `max_connections`, `min_connections`, timeouts.
  - Zustand/React Query: selectores especificos, nunca store completo. React Query para server state, Zustand solo para client state.
  - Paginacion obligatoria en endpoints que retornan listas. Cursor-based preferido sobre offset.
  - Indices de BD documentados junto al schema. Toda query nueva: verificar que existe indice apropiado.

**8. Arquitectura SOLID.**
  - Backend (Rust): separacion en capas — `handlers/` (HTTP), `services/` (logica), `repositories/` (BD), `models/` (structs/domain), `errors/` (tipos de error). Handlers delgados: extraer body → llamar service → retornar response.
  - Newtypes y enums para todo valor de dominio. Prohibido `String` o `i64` desnudo como parametro de funcion de dominio.
  - Frontend (React): componentes max 300 lineas, hooks max 120, utils max 150. Logica >5 lineas va en hook separado. Max 3 `useState` por componente.
  - Directorios por dominio/feature: `features/users/`, `features/auth/`. Componentes compartidos en `components/ui/`. API generada en `src/api/` (no tocar manualmente).
  - OCP (extender por props/composicion), ISP (props minimas), DIP (depender de traits/interfaces, no implementaciones concretas).

**9. Estandares de codigo.**
  - Rust: `snake_case` funciones/variables, `PascalCase` structs/enums/traits, `SCREAMING_SNAKE` constantes. `cargo fmt` + `cargo clippy` obligatorios antes de commit. Clippy con `#![deny(clippy::all)]` en `main.rs`/`lib.rs`.
  - TypeScript: `camelCase` vars/funcs, `PascalCase` componentes/tipos/interfaces.
  - CSS: nombres en espanol y `camelCase` (`.contenedorPrincipal`). Todo en archivos `.css` separados. Prohibido CSS inline. Variables obligatorias para colores/espaciados/tipografia.
  - Verificar que toda referencia existe antes de usarla (variables CSS, imports, tipos). Si lo creas, conectalo.
  - Codigo generado por Orval/codegen: NUNCA editar manualmente. Cambios van en el schema OpenAPI (Rust) → regenerar.
  - UI atomica: todo elemento reutilizable es su propio componente. Zustand para estado global de cliente. React Query para estado de servidor.

**10. Comentarios = memoria del proyecto.**
  - Formato: bloques `/* ... */` o `// ...` explicando el "por que". Prohibido barras decorativas (`====`).
  - Rust: `///` para doc comments publicos (structs, funciones, traits). `//` para comentarios internos.
  - Al completar una tarea, dejar comentario compacto en el codigo con: que se hizo, por que, gotchas encontrados, que queda pendiente.
  - No borrar comentarios de tareas anteriores — son registro de evolucion. Actualizar si quedan obsoletos.
  - Las lecciones aprendidas viven en los comentarios del codigo, no en MDs.

**11. Validacion obligatoria — errores ajenos incluidos.**
  - Despues de editar cualquier archivo Rust: `cargo check` sobre el workspace.
  - Despues de editar `.ts`/`.tsx`: ejecutar `npm run type-check`.
  - Despues de editar `.css`: validar variables/clases referenciadas.
  - Para Rust usa `npm run task:check -- <ID>` (modo local ligero: fmt/check). `cargo clippy`/`cargo test` completos solo en cierre de fase, CI o con `--full`; el guard aplica un cooldown de 3 horas y bloquea ejecuciones repetidas.
  - Antes de cada commit frontend: `npm run type-check` como minimo.
  - **Si los comandos reportan errores — aunque no esten relacionados con tu tarea — corregirlos es tu responsabilidad.** No se avanza ni se commitea con errores pendientes. Los errores pre-existentes encontrados se corrigen en el mismo commit o en uno separado si son muchos.
  - Despues de cambios en endpoints/schemas de Rust: regenerar cliente con `npm run codegen` y verificar que el frontend compila.

**12. Commits.**
  - Prohibido `git add .` o `git add --all`. Siempre `git add archivo1 archivo2` explicito.
  - Verificar `git diff --stat HEAD` y `git status` antes de commitear.
  - Cada tarea = un commit separado. Mensaje claro: `{id}: descripcion breve`.
  - Commit automatico al completar tarea, sin pedir permiso.
  - Archivos generados por Orval/codegen se commitean junto con los cambios de schema que los causaron.

**13. OpenAPI como contrato unico.**
  - El schema OpenAPI se genera desde Rust (`utoipa`). Es la unica fuente de verdad del contrato API.
  - Prohibido escribir tipos de API manualmente en el frontend. Todo sale del codegen.
  - Cambiar un endpoint = cambiar la anotacion `utoipa` → regenerar → verificar que el frontend compila → si hay breaking change, actualizar componentes afectados.
  - El archivo `openapi.json` generado se commitea en el repositorio para trazabilidad.
  - Todo campo opcional debe ser `Option<T>` en Rust y se refleja como `T | undefined` en el tipo generado. No hay ambiguedad.

**14. Migraciones de BD.**
  - Toda modificacion de schema de base de datos usa migraciones versionadas (`sqlx migrate`).
  - Naming: `{YYYYMMDDHHMMSS}_{descripcion}.sql` (generado por `sqlx migrate add`).
  - Migraciones son inmutables una vez commiteadas. Correciones van en migraciones nuevas.
  - Toda migracion incluye tanto `up` como `down` cuando sea posible.
  - `sqlx::query!` valida contra el schema actual — si cambias la BD, los queries que no compilen se detectan automaticamente.

---

## II. FLUJO DE TRABAJO (ciclo continuo)

El roadmap (`App/roadmap.md`) es el canal de comunicacion. El usuario escribe tareas ahi, tu las ejecutas. El flujo es un ciclo **tarea por tarea**: los 10 pasos se ejecutan completos para UNA tarea antes de tomar la siguiente. No se acumulan tareas ni se saltan pasos.

### ID de tarea
Cada tarea recibe un ID unico basado en la fecha: `{DD}{M}{A}-{N}`
- `DD` = dia (01-31)
- `M` = mes (1-9, A=oct, B=nov, C=dic)
- `A` = ano del proyecto (A=2026, B=2027, C=2028...)
- `N` = numero secuencial de tarea ese dia (1, 2, 3...)
- Ejemplo: 17 marzo 2026, tarea 1 = `173A-1`. Tarea 2 ese dia = `173A-2`.

### Paso 1 — Leer roadmap y planes
Leer `App/roadmap.md` completo. Identificar tareas pendientes. Revisar `App/Agente/planes/` por planes activos que requieran continuacion.

Si una tarea del roadmap no es suficientemente clara para ejecutarse con seguridad tecnica, dejar una nota breve pidiendo aclaracion en el lugar adecuado del flujo del agente, saltar a la siguiente tarea y volver luego. Prohibido bloquear el ciclo completo por una ambiguedad aislada.

### Paso 2 — Ejecutar tarea
Tomar una tarea pendiente y completarla. Reglas:
- **2.1** Cada tarea = un commit separado con mensaje claro.
- **2.2** Completar una tarea individualmente antes de pasar a otra. Se permite agrupar solo tareas completamente relacionadas.
- **2.3** Dejar comentarios en el codigo referenciando la tarea: que se hizo, instrucciones clave, problemas enfrentados, pendientes sobre esa funcionalidad. No borrar comentarios anteriores.
- **2.4** Prohibido avanzar sin marcar la tarea como completada, hacer commit y organizar los MDs.
- **2.5** Editar archivo por archivo. No acumular cambios en muchos archivos sin validar entre cada uno.
- **2.6** Si la tarea es compleja (>1 sesion o multiples fases) o es un problema repetitivo que ya reaparecio, crear un plan en `App/Agente/planes/` con nombre `plan-tema-YYYY-MM-DD.md` describiendo fases, estado actual y proximos pasos. Continuar desde donde se quedo.
- **2.7** Si la tarea modifica un endpoint o schema de Rust: despues de compilar el backend, regenerar el cliente frontend (`npm run codegen`) y verificar que `npm run type-check` pasa. Si rompe componentes, arreglarlos como parte de la misma tarea.

### Paso 3 — Validar y corregir errores reportados
Despues de cada tarea, ejecutar los comandos de validacion correspondientes (ver seccion V). **Si los comandos reportan errores — aunque no tengan relacion con la tarea actual — corregirlos antes de continuar.** Los errores reportados por herramientas son tu responsabilidad. No se avanza con errores pendientes.

Backend:
- `npm run task:check -- <ID>` → `npm run task:check -- <ID> --full` solo cuando el gate de fase/CI lo exige; `cargo test 2>&1` sigue siendo el mismo comando pesado y no evita el guard.
- Si se modifico schema BD: verificar que migraciones estan al dia y `sqlx prepare` actualizado.

Frontend:
- `npm run codegen` (si cambio el schema OpenAPI) → `npm run type-check`
- Si la tarea toca React, hooks, stores o servicios frontend: revisar que el flujo renderizado afectado siga funcionando y que no haya regresiones en estados vacios, modales, navegacion o carga de datos.

### Paso 4 — Testear la tarea
Antes de marcar como completada, verificar que la funcionalidad implementada o corregida funciona:
- Backend: ejecutar los tests afectados mediante el gate. Para endpoints nuevos/modificados: hacer request manual (herramienta equivalente) y verificar response body y status code; reservar la suite completa para el cierre de fase/CI.
- Frontend: verificar que la UI renderiza correctamente y que los datos fluyen del backend al componente.
- Si hay tests existentes, ejecutarlos. Si la tarea lo amerita y es viable, agregar un test.
- Solo si no es posible testear en local (dependencia de terceros, hardware, etc.), omitir con justificacion en el comentario del commit.
- **Una tarea no se marca como completada hasta que este testeada y confirmada.**

### Regla adicional de cierre
- Prohibido mover una tarea a completados si el sintoma original sigue visible localmente o si no se verifico el flujo exacto reportado por el usuario cuando el entorno local permite hacerlo.

### Paso 5 — Archivar tarea completada
Mover la tarea completada del roadmap a un archivo en `App/Agente/completados/` con nombre `tareas-YYYY-MM-DD.md`. Si ya existe uno con la fecha de hoy, agregar ahi. El roadmap nunca acumula tareas completadas. Si la tarea tenia un plan en `App/Agente/planes/`, mover el plan a `App/Agente/planes/completados/`.

### Paso 6 — Documentar (obligatorio cuando se toca funcionalidad)
Despues de completar una tarea, revisar si la funcionalidad o flujo tocado ya tiene documentacion vigente en `App/Agente/documentacion/`. Si no existe, crearla; si existe, actualizarla. Esto es obligatorio para toda tarea que cambie arquitectura, flujos de usuario, contratos API (endpoints, schemas), migraciones de BD, integraciones, tooling o comportamiento reutilizable. Nunca duplicar documentacion existente sobre el mismo tema — actualizar el archivo existente y cambiar la fecha en el nombre.

### Paso 7 — Prevencion (si aplica)
Preguntarse: "Se puede detectar o prevenir automaticamente la proxima vez?" Si si, dejar nota en el roadmap o crear issue. Para Rust, considerar si un clippy lint custom, un test de integracion, o una regla de CI cubre el caso. Para frontend, considerar si un lint rule o test e2e lo previene.

### Paso 8 — Revisar pendientes de prevencion
Leer `App/Agente/prevencion/`. Si hay MDs pendientes de implementar:
1. Implementar la regla, test o configuracion de lint descrita.
2. Verificar que detecta el problema original.
3. Confirmar deteccion exitosa, eliminar el MD de prevencion y marcar como completada.
- Si no hay pendientes, saltar este paso.

### Paso 9 — Commit, push y deploy
Hacer commit final. Luego sincronizar la rama local con remoto (`git pull --rebase`) antes del push. Si el roadmap del proyecto indica que aplica deploy:
- Backend: build release (`cargo build --release`), deploy segun infra del proyecto (Docker, servicio systemd, plataforma cloud).
- Frontend: build produccion (`npm run build`), deploy segun infra.
- **Despues de cada deploy, verificar que el servidor sigue funcionando** (health check a la URL de produccion, revisar logs si hay errores). Si el deploy rompe algo, revertir antes de continuar.

### Paso 10 — Volver al Paso 1
Releer el roadmap completo (el usuario puede haber agregado tareas mientras trabajabas). Repetir el ciclo hasta que no queden tareas pendientes. Solo entonces, cerrar con un resumen breve de lo realizado.

---

## III. FORMATOS

### ID de tareas
Formato: `{DD}{M}{A}-{N}` donde DD=dia, M=mes (1-9, A-C para oct-dic), A=ano proyecto (A=2026, B=2027...), N=secuencial del dia.
Ejemplo: 17 marzo 2026, tarea 3 = `173A-3`. 5 noviembre 2027, tarea 1 = `05BB-1`.

### Tareas en el roadmap (formato del agente al completar)
