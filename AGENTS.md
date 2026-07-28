---
applyTo: '**'
---

# Protocolo de Desarrollo v7.0 — Modo Bloque (1 mayo 2026)
---

## I. REGLAS ABSOLUTAS (por prioridad)

<rule id="1" name="deploy-via-coolify-manager">
**NO HACER DEPLOY DIRECTO CON SSH.** Toda operacion en produccion (deploy, restart, logs, backup, restore, exec, health) pasa por `coolify-manager-rs` (ver seccion VII). Comandos directos (ssh, docker, scp, curl al servidor) eluden bind mounts, health checks y el flujo seguro. Excepcion: diagnostico puntual de emergencia, documentado luego como mejora a la herramienta.
</rule>

<rule id="2" name="">
Si tienes dudas, por favor, detente a consultar primero con el usuario, si tienes dudas de una tarea pide aclaración antes de empezar con la petición.
</rule>

<rule id="3" name="zero-patches">
Antes de implementar pregunta: "Es la mejor opcion arquitectonica o el camino facil?" Si es lo segundo, rediseña. Prohibido justificar con "es temporal" o "lo refactorizamos despues".
</rule>

<rule id="2.1" name="expansive-thinking">
Aunque la tarea parezca pequena, evalua primero si revela un problema mas profundo de arquitectura, sincronizacion, contratos, cache, observabilidad o UX. Resuelve la raiz cuando exista una solucion claramente superior al sintoma.
</rule>

<rule id="3" name="controlled-edits">
Edita por modulo o archivo, no parchees 10 archivos en un solo cambio masivo. Acumula cambios coherentes y valida al cierre del bloque (no entre archivos). 

**Edicion: solo herramientas integradas** (`replace_string_in_file`, `multi_replace_string_in_file`, `create_file`). Prohibido modificar codigo fuente con scripts PowerShell, heredocs (`cat > file << EOF`) o comandos de terminal — eso es solo para diagnostico, nunca para mutar codigo.
</rule>

<rule id="4" name="order-guardian">
Eres responsable absoluto del orden del proyecto. Al tocar un archivo, corrige toda violacion visible de bajo riesgo (imports muertos, hardcodeo, codigo muerto, nombres confusos). Si la correccion es compleja, deja `TODO` con contexto. No existe "no es mi tarea".
</rule>

<rule id="5" name="security-first">
- **SQL:** prepared statements, query builders, macros tipados (`query_as!`, `$wpdb->prepare()`). Nunca interpolar strings.
- **Backend:** manejo global de errores. SSL explicito en APIs de pago. Escapa argumentos de procesos externos (`escapeshellarg()`, `Command::new` con args separados).
- **Secrets:** siempre en variables de entorno. Permisos minimos.
- **Input:** validar/sanitizar en el boundary. Prohibido `eval()`, `innerHTML` con datos sin sanitizar, `unwrap()` sobre input externo.
</rule>

<rule id="6" name="no-silent-failures">
- I/O, red, BD, parsing: manejo de errores con logging util. Errores silenciados estan prohibidos.
- **Rust:** propaga con `?` y `thiserror`/`anyhow`. Cleanup en `Drop` o RAII. `.unwrap()` solo sobre invariantes probadas.
- **React:** errores retornan `ok: false`, nunca enmascarados como exito. Toda falla = feedback visible (toast). Updates optimistas con rollback. `useEffect` async con `AbortController`.
- Metodos criticos (INSERT/UPDATE/DELETE/APIs) retornan resultado, no void.
- Race conditions: usa upsert atomico o constraint UNIQUE, no buscar-crear secuencial.
</rule>

<rule id="7" name="performance">
- Prohibido N+1 o roundtrips innecesarios. Combina con CTEs/CASE/JOINs.
- Zustand: selectores especificos `useStore(s => s.campo)`, nunca el store completo.
- PostgreSQL INTERVAL: validar con whitelist, nunca interpolar.
</rule>

<rule id="8" name="solid-architecture">
- **SRP:** 1 componente = 1 responsabilidad. Max 3 `useState`. Logica >5 lineas va a hook (`useMiComponente.ts`).
- **Limites:** componentes/estilos max 300 lineas, hooks max 120, utils max 150. Si excede, divide.
- **Directorios jerarquicos por dominio** (`components/ui/`, `features/auth/`). Prohibido carpeta plana.
- **OCP/ISP/DIP:** extender por props/composicion, props minimas, depender de abstracciones.
</rule>

<rule id="9" name="code-standards">
- **JS/TS:** `camelCase` vars/funcs, `PascalCase` componentes/clases.
- **CSS:** nombres en espanol y `camelCase` (`.contenedorPrincipal`). Todo en archivos `.css` separados. Prohibido CSS inline. Variables obligatorias para colores/espaciados/tipografia.
- **Verifica que toda referencia exista** antes de usarla (variables CSS, imports, tipos). Si lo creas, conectalo.
- **UI atomica:** todo elemento reutilizable es su propio componente. Zustand para estado global.
- **Codegen API (Orval):** modo `tags-split` siempre. Si encuentras un `generated.ts` monolitico, divididlo de inmediato.
- **Carpetas y archivos en ingles.** `components/`, `styles/`, `services/` — nunca `componentes/`, `estilos/`, `servicios/`. Aplica a codigo fuente; la documentacion del agente sigue en `Agente/`, `completados/`, etc.

**Consistencia visual obligatoria.** Antes de crear o modificar CSS revisa, en orden:
1. Variables existentes en `styles/variables.css` (colores, espaciados, tipografia, radios).
2. Componentes atomicos en `components/ui/` (Badge, Button, Tarjeta, etc.) y sus CSS.
3. Patrones visuales de componentes similares ya implementados.

Si el estilo necesario ya existe como variable o componente, reutilizalo. No crees clases ad-hoc que dupliquen o contradigan el sistema. Toda tarea UI empieza leyendo los estilos existentes.
</rule>

<rule id="9.1" name="no-design-specs-in-components">
**Prohibido agregar especificaciones de diseno (colores hex, fuentes concretas, tamanos, gradientes, paletas, micro-interacciones decorativas) dentro de los componentes** salvo que sea estrictamente necesario para la funcionalidad. Toda decision visual debe vivir en variables CSS centralizadas (`styles/variables.css`) o en componentes atomicos del sistema de diseno.

Razon: Claude tiende por defecto a inyectar paletas, tipografias y "house style" creativo en cada componente. Eso fragmenta la identidad visual, duplica decisiones y rompe la consistencia del proyecto. La estetica se decide una sola vez en el sistema de diseno; los componentes solo la consumen.

**Excepciones validas (deben justificarse en el comentario de la tarea):**
- Diseno propuesto explicitamente por el usuario o Figma con detalles unicos.
- Estado one-off no parametrizable (ej: animacion contextual de un splash screen).
- Componente de marca (logo, hero unico) que por definicion no se reutiliza.

**Refuerzo operativo obligatorio para TODOS los componentes compartidos:**
- Reutiliza siempre la clase, receta o componente base del sistema antes de crear una variante local. Esto aplica a modales, paneles, cards, tablas, formularios, headers, footers, vacios, badges y cualquier otra pieza compartida.
- Queda prohibido crear `.algoTitulo`, `.algoTexto`, `.algoDescripcion`, `.algoAcciones`, `.algoHeader`, `.algoFooter`, `.algoCard`, `.algoInput` o equivalentes si solo redefinen tipografia, color, spacing, border o alineacion ya resueltos por el sistema.
- Los modales son solo un ejemplo visible del problema: `.modalTitulo`, `.modalTexto` y `.modalAcciones` son recetas compartidas, pero la misma regla aplica a cualquier componente base del proyecto.
- Si falta una receta compartida, se crea primero en el componente base del sistema y luego se consume desde el componente concreto.

Si dudas, consume tokens existentes; si falta un token, anadelo a `variables.css` y luego usalo.
</rule>

<rule id="10" name="comments-as-memory">
- Formato: bloques `/* ... */` que expliquen el "por que". Prohibido barras decorativas (`====`).
- Al completar tarea, deja comentario compacto: que se hizo, por que, gotchas, pendientes.
- No borres comentarios anteriores — son registro de evolucion. Actualizalos si quedan obsoletos.
- Lecciones aprendidas viven en `Agente/lecciones/lecciones-aprendidas.md` Y en los comentarios del codigo. Tras cada tarea evalua si hubo leccion nueva y registrala en ambos sitios.
</rule>

<rule id="11" name="block-validation">
**No valides despues de cada microcambio.** Acumula los cambios del bloque y al cierre ejecuta una sola ronda completa segun el stack (ver seccion VI). Excepciones para validacion puntual: editor muestra diagnosticos, cambio de alto riesgo, o duda tecnica que bloquea avanzar.

**Si los comandos reportan errores ajenos al bloque, corregirlos es tu responsabilidad.** No se cierra ni se commitea con errores pendientes. Errores pre-existentes se corrigen en el mismo commit, o en uno separado si son muchos.
</rule>

<rule id="12" name="commits">
- Prohibido `git add .` o `git add --all`. Siempre `git add archivo1 archivo2` explicito.
- Verifica `git diff --stat HEAD` y `git status` antes de commitear.
- Cada bloque coherente = un commit. Mensaje: `{id}: descripcion` o `{id1}+{id2}: descripcion` si son tareas relacionadas.
- Commit automatico al cerrar bloque, sin pedir permiso.
- **Prohibido cerrar sesion o cambiar de frente con tareas completadas sin commit + push.** Si push falla, resolver antes de seguir.
</rule>

<rule id="13" name="powershell-ssh">
- SQL complejo via SSH: usa base64 (`[Convert]::ToBase64String` + `base64 -d` en remoto). PS5 no tiene heredoc.
- Alternativa: crear `.sh` local, copiar con `scp`, ejecutar remoto.
</rule>

<rule id="14" name="glory-sentinel-disable">
Aplica `sentinel-disable-file` o `limite-lineas` solo a archivos con justificacion valida (utility classes centrales, controllers REST con muchas rutas, legacy temporal). Nunca para evitar refactorings necesarios. Documenta la razon cuando lo uses.
</rule>

<rule id="15" name="responsive-design">
Todo componente UI funciona en mobile (≥320px), tablet (≥768px) y desktop (≥1024px). Usa media queries o container queries con breakpoints estandar. Verifica visualmente en al menos 2 resoluciones antes de marcar UI como completada.
</rule>

<rule id="16" name="reread-roadmap">
Despues de cada commit y de cada resumen, **relee el roadmap completo** como ultima accion antes de cerrar o pasar al siguiente bloque. El usuario puede haber agregado tareas mientras trabajabas. No es opcional.
</rule>

<rule id="17" name="glory-framework">
- `/glory-rs` (Rust) y `/glory` (PHP) son los nucleos agnosticos reutilizables: logica, UI atomica, utilidades, fixtures/CMS, pagos, WebSocket, chat, etc.
- Deben permanecer **agnosticos**: nunca contienen logica especifica de un proyecto.
- Si la funcionalidad implementada es reutilizable entre proyectos, va al submodulo Glory.
- Al cerrar tarea pregunta: "Esto es especifico de este proyecto o agnostico?" Si es agnostico, muevelo.
</rule>

<rule id="18" name="continuous-process-improvement">
Eres responsable de mejorar tu propio proceso, reglas y herramientas. Si detectas una mejora que haga el flujo mas robusto o fiable, implementala sin pedir permiso. Si `scripts/self-check.ps1` no existe al iniciar sesion, crealo segun el formato vigente. El objetivo es ser mas preciso, no mas obediente.
</rule>

<rule id="19" name="deploy-only-coolify-manager">
Repite y refuerza la regla `-2`: todo deploy, restart, logs, backup y exec contra produccion **debe** pasar por `coolify-manager-rs`. Si una operacion no esta cubierta, dejar constancia para mejorar la herramienta. Excepcion: diagnostico de emergencia.
</rule>

<rule id="20" name="no-vscode-restart">
Nunca ejecutes comandos que reinicien, recarguen o cierren VS Code (`workbench.action.reloadWindow`, `Developer: Reload Window`, etc.). Si una extension se modifica, el usuario decide cuando reiniciar. Reiniciar interrumpe terminales, agentes y contexto de sesion.
</rule>

<rule id="21" name="bounded-terminal-execution">
**Prohibido quedarse esperando indefinidamente un comando ambiguo o de larga vida.** Antes de ejecutar cualquier comando, clasificalo como una de estas dos cosas:

- **Comando acotado** (build, test, lint, migracion, script puntual): debe tener criterio claro de salida/fracaso. Ejecutalo en modo no interactivo siempre que sea posible (`-y`, flags equivalentes, preinstalar dependencia). Si tras un timeout razonable o una sola lectura el estado sigue ambiguo, **no sigas esperando**: haz una comprobacion discriminante (exit code, proceso, puerto, archivo generado, HTTP probe, log final) y decide.
- **Comando de larga vida** (server, watch, tail, dev loop): ejecutalo en background/async y valida con una señal de readiness concreta (puerto escuchando, linea "listening", healthcheck, tarea viva). Nunca lo uses como validacion bloqueante ni te quedes pollando salida sin una hipotesis nueva.

Reglas operativas obligatorias:
- Si el comando pide confirmacion o instalacion interactiva, reejecutalo de forma no interactiva o responde de inmediato; no lo dejes colgado esperando input oculto.
- Si una herramienta devuelve salida larga o stale, no la releas en bucle: cambia a una verificacion puntual del estado real.
- Un comando no puede bloquear el ciclo solo porque "sigue corriendo"; o produce una senal verificable, o se trata como ambiguedad a resolver con una comprobacion mas barata.
</rule>

<rule id="21.1" name="anti-stall-remote-commands">
**Comandos remotos largos no se ejecutan como espera ciega.** Deploys, builds Docker/Rust, migraciones remotas, `ssh`, `coolify-manager-rs deploy-service`, `docker compose build`, restores y cualquier operacion que pueda tardar mas de 5 minutos deben cumplir una de estas condiciones antes de lanzarse:

- Tener heartbeat/log streaming real (`--progress=plain`, logs con timestamps, salida periodica o wrapper que imprima progreso) y timeout explicito.
- Ejecutarse en modo async/background con una senal de readiness concreta (health HTTP, contenedor healthy + probe real, puerto, archivo generado, exit code registrado).
- Dividirse en fases cortas verificables: preflight, build, image-exists, swap, health, logs finales.

Reglas anti-atasco:
- Nunca encadenes un build remoto largo y un health final en un unico comando opaco si no hay salida intermedia fiable.
- Si pasan 5-10 minutos sin nueva informacion util, no sigas esperando por inercia: comprueba proceso remoto, imagen creada, contenedor, logs recientes o health y decide.
- Para recoveries `--skip-build` o `--no-build`, primero verifica que la imagen local existe; si no existe, aborta antes de recrear contenedores.
- Si se ejecuta un binario release por path fijo (`target/release/*.exe`), verifica que el build actualizo ese mismo path. `CARGO_TARGET_DIR` puede redirigir la salida a otro directorio; usa `cargo build --release --target-dir target` o ejecuta el binario del target real, y confirma `LastWriteTime`/version/salida esperada antes de probar produccion.
- Si una herramienta queda colgada o oculta stdout/stderr critico, arregla la herramienta en el mismo bloque o documenta la mejora pendiente; no normalices el cuelgue como parte del flujo.
</rule>

<rule id="21.2" name="no-terminal-probing-credit-guard">
**Prohibido usar terminal como prueba exploratoria cuando pueda fallar por ruta, cwd, config, puerto, dependencia o modo interactivo.** Cada comando fallido consume tiempo y credito; antes de ejecutarlo debe existir una hipotesis concreta y una alta confianza de exito.

Reglas obligatorias:
- Si existe una herramienta integrada para leer, buscar, diagnosticar, editar o consultar estado, usala antes que terminal.
- Antes de ejecutar un comando, verifica por lectura o estado local: directorio correcto, archivo/config existente, binario esperado, puerto libre si aplica, y si el comando es acotado o larga vida.
- No ejecutes `cargo run`, servidores, watchers, deploys, builds release o CLIs con config dudosa como "smoke test". Usa binario ya construido, tests unitarios, `get_errors`, lectura de logs, o comprobaciones estaticas primero.
- Si no puedes garantizar el preflight, no ejecutes el comando: documenta el bloqueo, ajusta el plan, o usa una comprobacion mas barata.
- Tras un `exit code != 0`, queda prohibido reintentar variantes a ciegas. Primero identifica la causa exacta por salida, archivo, config o codigo; luego ejecuta solo una correccion justificada.
</rule>

<rule id="22" name="subagent-for-efficiency">
**Delegar tareas a subagentes siempre que sea posible, usando DeepSeek V4 Flash para ahorrar tokens.** El modelo Pro (cadena completa) se reserva para el agente principal; toda subtarea, investigacion, busqueda de codigo, lectura de archivos, ejecucion de comandos acotados y diagn ostico debe delegarse a un subagente, que automaticamente usara Flash (primero intenta opdecode-zen gratis, luego opencode-go pago).

**Costo/beneficio:** Flash cuesta ~1/10 de Pro por token. Delegar una busqueda o lectura a subagente es practicamente gratis comparado con procesarlo en el agente principal. El unico costo es un round-trip de contexto — que es menor que procesar la misma operacion con el modelo caro.

**Reglas operativas:**
1. **Investigacion y busqueda de codigo:** usar `runSubagent` con `Explore` agent o busqueda directa. No hagas 5 `grep_search`/`read_file` en el agente principal si un subagente puede hacerlo en un solo llamado.
2. **Lectura de archivos grandes:** si necesitas leer multiples secciones de un archivo, usa subagente. El costo de procesar miles de tokens con Pro se elimina.
3. **Validaciones y comprobaciones:** `get_errors`, `testFailure`, estados de compilacion, verificaciones de sintaxis — delega a subagente.
4. **Tareas paralelizables:** si hay 3 archivos para leer o 3 comandos para ejecutar, lanza subagentes en paralelo. El flujo del bloque se acelera y el costo se reduce.
5. **Excepcion (NO delegar):** logica de edicion, diseno arquitectonico, decisiones de implementacion, validacion final del bloque. El agente principal retiene el control de las decisiones.

**Mecanismo tecnico:** La extension `vizards.deepseek-v4-for-copilot` clasifica cada request como `main-agent` o subagente segun el system prompt que VS Code le envia. Cuando un subagente ejecuta con el modelo Pro seleccionado, la extension intercepta y cambia el modelo a Flash antes de enviarlo a FreeLLMAPI. FreeLLMAPI entonces usa la cadena restringida Flash (opencode-zen gratis → opencode-go pago). El agente principal nunca se ve afectado.

**Compatibilidad con modelos:** Si el usuario selecciona "DeepSeek V4 Flash" en el picker, todo (principal y subagentes) usa Flash — no hay routing diferencial. La optimizacion solo opera cuando el picker esta en "DeepSeek V4 Pro".
</rule>

---

## II. FLUJO DE TRABAJO (ciclo continuo, por bloque coherente)

<task_id_format>
`{DD}{M}{A}-{N}`
- `DD` = dia 01-31
- `M` = mes 1-9, A=oct, B=nov, C=dic
- `A` = ano de proyecto: A=2026, B=2027, C=2028...
- `N` = secuencial del dia (1, 2, 3...)

Ejemplo: 17 marzo 2026 tarea 1 → `173A-1`. 5 noviembre 2027 tarea 1 → `05BB-1`.
</task_id_format>

<step n="1" name="leer-roadmap-y-planes">
Lee `roadmap.md` completo. Identifica tareas pendientes. Revisa `Agente/planes/` por planes activos. Si una tarea es ambigua, dejar nota pidiendo aclaracion y saltarla; no bloquear el ciclo entero por una ambiguedad aislada.
</step>

<step n="2" name="ejecutar-bloque">
Toma el bloque de tareas abordables (relacionadas, mismo dominio o barrido). Reglas:
- **2.1** Cada bloque = un commit separado.
- **2.2** Completa todas las tareas del bloque antes de validar.
- **2.3** Deja comentario en el codigo con `[id]`: que se hizo, instrucciones clave, gotchas, pendientes. No borres comentarios anteriores.
- **2.4** No cierres el bloque sin: archivado, roadmap actualizado, commit y push.
- **2.5** Edita por modulo, sin validaciones pesadas entre archivos.
- **2.6** Si la tarea es compleja (multi-sesion, multi-fase) o un problema repetido, crea plan en `Agente/planes/plan-tema-YYYY-MM-DD.md` con fases, estado y proximos pasos.
- **2.7** **Delegar a subagentes (regla 22):** tareas de investigacion, busqueda, lectura, diagnostico y comandos acotados se delegan a `runSubagent`. El agente principal se reserva para decisiones, edicion y validacion final. Esto minimiza el consumo de tokens del modelo Pro.
</step>

<step n="3" name="validar-y-corregir">
Ejecuta validaciones del stack (seccion VI) **una sola vez** al cierre del bloque. Corrige errores aunque sean ajenos al bloque actual.

Si tocaste React, hooks, stores, islands o servicios frontend, no basta el type-check abstracto: revisa que el flujo renderizado siga funcionando (hydration, estados vacios, modales, menus, contadores, navegacion).

Si Glory Sentinel reporta un **falso positivo**, crea MD en `Agente/prevencion/` describiendo el caso y la correccion necesaria a la regla.
</step>

<step n="4" name="testear">
Verifica que la funcionalidad implementada funciona realmente:
- Ejecutala en local y confirma resultado esperado.
- Si el cambio es UI/HTML/CSS o texto renderizado: abre el flujo y comprueba el elemento exacto. Type-check + lectura de codigo no son suficientes.
- Ejecuta tests existentes. Si la tarea lo amerita y es viable, agrega un test.
- Si no es posible testear local (terceros, hardware), justifica la omision en el commit.

**Un bloque no se marca como completado sin verificacion confirmada.** Prohibido archivar si el sintoma original sigue visible localmente cuando se podia comprobar.
</step>

<step n="5" name="archivar">
Mueve las tareas completadas del roadmap a `Agente/completados/tareas-YYYY-MM-DD.md` (agrupadas por fecha). El roadmap nunca acumula completadas. Si la tarea tenia plan en `Agente/planes/`, muevelo a `Agente/planes/completados/`.
</step>

<step n="6" name="documentar">
Si la tarea cambio arquitectura, flujos de usuario, contratos, sincronizacion, algoritmos, integraciones, tooling o comportamiento reutilizable, actualiza/crea documentacion en `Agente/documentacion/`. Nunca dupliques: si ya existe MD del tema, actualiza el archivo y su fecha.
</step>

<step n="7" name="prevencion">
Pregunta: "Se puede detectar o prevenir esto automaticamente con Code Sentinel/VarSense?" Si si, crea MD en `Agente/prevencion/prevencion-tema-YYYY-MM-DD.md` describiendo la regla, y deja referencia en el roadmap.
</step>

<step n="8" name="implementar-prevenciones-pendientes">
Lee `Agente/prevencion/`. Si hay MDs pendientes:
1. Implementa la regla en `code-sentinel` o `varsense` (ver seccion VII para ubicacion).
2. Pruebala contra el caso original.
3. Reinstala la extension (`vsce package` + instalar `.vsix`).
4. Confirma deteccion exitosa, elimina el MD y marca pendiente como completada.
</step>

<step n="9" name="commit-push-deploy">
Commit final del bloque. Sincroniza con remoto (`git pull --rebase` u equivalente no interactivo). Push.

Si el roadmap del proyecto indica deploy: usa `coolify-manager-rs` (seccion VII). **Comandos directos prohibidos.** Tras deploy verifica health (URL produccion, logs). Si rompe algo, restore inmediato.

Si el flujo descubre un escenario que `coolify-manager-rs` no cubre bien, deja constancia para mejorarlo.
</step>

<step n="10" name="volver-al-paso-1">
Relee el roadmap completo. Ejecuta `npm run self-check -- -TareaId {ID}` (o IDs agrupados) y confirma cada punto del checklist. Repite el ciclo hasta vaciar pendientes. Solo entonces cierra con resumen breve.
</step>

---

## III. FORMATOS

### Tarea completada en `Agente/completados/`
```markdown
## 173A-1 — Titulo breve
- **Que:** descripcion de lo hecho
- **Archivos:** lista de archivos modificados
- **Gotchas:** problemas encontrados (si los hubo)
- **Sentinel:** nueva regla / falso positivo / no aplica
- **GLORY:** que se movio al submodulo agnostico, o "No aplica"
```

### Comentario en codigo
```javascript
/* [173A-1] Que se hizo y por que.
 * Gotcha: detalle relevante para futuras ediciones.
 * Pendiente: lo que queda en esta area. */
```

### Commits
```
173A-1: descripcion breve
173A-1+173A-2: descripcion si son tareas relacionadas
```

### Nomenclatura
- JS/TS: `camelCase` vars/funcs, `PascalCase` componentes
- CSS: espanol + `camelCase` (`.contenedorPrincipal`)
- Archivos MD: `nombre-descriptivo-YYYY-MM-DD.md`

---

## IV. ESTRUCTURA DE MDs

### Plantilla `roadmap.md`
```markdown
# {Proyecto} — Roadmap

> **Descripcion:** breve
> **Stack:** breve
> **URL produccion:** si aplica
> **Servidor:** IP y acceso (si aplica)
> **Deploy:** Coolify / manual / N/A
> **Repositorio:** rama principal y convenciones

## Herramientas del agente
- coolify-manager-rs, code-sentinel, varsense (ver protocolo seccion VII)

## Tareas pendientes
(el usuario escribe en cualquier formato; el agente asigna IDs y ejecuta)
```

### Arbol de archivos
```
{raiz}/
  roadmap.md                          ← solo pendientes del usuario
  Agente/
    completados/
      tareas-YYYY-MM-DD.md
    documentacion/
      {categoria}/
        tema-YYYY-MM-DD.md
    lecciones/
      lecciones-aprendidas.md
    planes/
      plan-tema-YYYY-MM-DD.md         ← activos
      completados/
        plan-tema-YYYY-MM-DD.md
    prevencion/
      prevencion-tema-YYYY-MM-DD.md   ← reglas pendientes para Sentinel/VarSense
```

### Reglas de los MDs
1. `roadmap.md` solo contiene pendientes; nunca acumula completadas.
2. Todo MD lleva fecha (`YYYY-MM-DD`) en el nombre.
3. `documentacion/` se organiza en subcarpetas por categoria.
4. Nunca dupliques documentacion: actualiza el archivo existente y su fecha.
5. Cada completada indica `Sentinel:` (regla nueva / falso positivo / no aplica).
6. Lecciones viven en `Agente/lecciones/lecciones-aprendidas.md` Y en comentarios.
7. Si la estructura de MDs esta desorganizada al iniciar sesion, reorganizar como primera accion.
8. Archivos legacy del proyecto: no mover ni modificar sin instruccion explicita.

---

## V. (reservado)

---

## VI. COMANDOS DE VALIDACION

<stack_detection>
`npm run self-check` (o `scripts/self-check.ps1`) detecta el stack:
- **Rust** (`Cargo.toml`): `cargo check`, `cargo clippy -- -D warnings`, `cargo test`
- **Frontend** (`frontend/package.json`): `npx tsc --noEmit`
- **PHP** (`composer.json` o `style.css`): validacion manual
- **Node** (`package.json` raiz, sin `Cargo.toml`): `npm test`, `npm run lint`
</stack_detection>

<validation_table>
| Cuando | Comando |
|---|---|
| Edicion `.rs` (durante) | Sin validacion pesada; `get_errors` puntual si hay diagnosticos visibles o bloqueo |
| Cierre de bloque con `.rs` | `cargo fmt --check && cargo check && cargo clippy -- -D warnings && cargo test` |
| Edicion `.ts`/`.tsx` (durante) | Sin `type-check` por archivo; `get_errors` puntual si hace falta |
| Cierre de bloque frontend | `npx tsc --noEmit` (en `frontend/`) o equivalente del proyecto |
| Edicion `.php` (durante) | Revision manual puntual; validacion completa al cierre |
| Cierre de bloque CSS | VarSense `cssVarsValidator.scanAllDiagnostics` o build/lint que compile CSS |
| Generacion masiva (>3 archivos) | Code Sentinel `codeSentinel.analyzeWorkspace` al cierre |
| Antes del commit del bloque | `npm run self-check -- -TareaId {ID}` |
| Lint + types integrado | `codeSentinel.runExternalTools` |
</validation_table>

<execution_discipline>
- Para servidores, watchers y procesos que permanecen vivos: usar modo background/async + una comprobacion de readiness concreta. Nunca esperar "a ver si termina".
- Para `npx`, instaladores o CLIs que puedan pedir confirmacion: forzar modo no interactivo (`-y` o equivalente) antes de ejecutarlos.
- Si una validacion larga queda ambigua por timeout o salida truncada, la siguiente accion no es esperar mas: es una comprobacion puntual del estado real (exit code, puerto, proceso, health, archivo generado, ultimas lineas del log).
</execution_discipline>

<other_commands>
- `codeSentinel.analyzeFile` — analizar archivo actual
- `cssVarsValidator.exportReport` — reporte CSS exportable
- `cssVarsValidator.scanOrphanClasses` — clases CSS sin uso
</other_commands>

---

## VII. UBICACION DE HERRAMIENTAS DEL AGENTE

<tool_locations>
Las tres herramientas viven como workspaces hermanos en el repositorio del tema WordPress base, accesibles via los multi-root del workspace o por path absoluto.

### coolify-manager-rs
- **Workspace folder:** `c:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs`
- **Path relativo (cuando el proyecto vive dentro de `themes/glorytemplate/`):** `.agent/coolify-manager-rs`
- **Binario release (path absoluto, siempre valido desde cualquier proyecto):**
  `C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs\target\release\coolify-manager.exe`
- **Repo:** github.com/1ndoryu/coolify-manager-rs (rama `main`)
- **Para modificarlo o recompilar:** abrir ese workspace folder, editar `src/`, ejecutar `cargo build --release --target-dir target`. El binario quedara en `target/release/`.

**Como encontrarlo desde cualquier proyecto:**
```powershell
# Path absoluto universal (usar siempre si el proyecto NO esta bajo themes/glorytemplate/)
$cm = "C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs\target\release\coolify-manager.exe"
& $cm deploy --name <sitio> --update

# Alternativa: cd al workspace y usar ruta relativa
cd "C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs"
.\target\release\coolify-manager.exe health --name <sitio>
```
**Antes de invocar:** verificar que `target\release\coolify-manager.exe` existe. Si no existe o esta desactualizado, compilar primero:
```powershell
cd "C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs"
cargo build --release --target-dir target
```

#### Comandos principales
| Comando | Uso |
|---|---|
| `deploy --name <sitio> --update` | **Para todos los proyectos** (WordPress Y Rust). Clona/rebuild vía Coolify API. |
| `deploy --name <sitio> --update --skip-backup` | Deploy sin backup (cambios de bajo riesgo). |
| `deploy-service --name <sitio>` | **NO USAR para Rust** — falla en build Docker paso 3/6. Solo para gestión interna del manager. |
| `redeploy --name <sitio>` | Fuerza redeploy via Coolify API (sin cambios de codigo). Alternativa si deploy falla. |
| `health --name <sitio>` | Health check remoto + HTTP. Usar siempre post-deploy |
| `logs --name <sitio>` | Logs del contenedor |
| `restart --name <sitio>` | Reinicia servicios del sitio |
| `backup --name <sitio>` | Backup externo |
| `restore --name <sitio>` | Restaura backup (usar si deploy rompe algo) |
| `exec --name <sitio> -- <cmd>` | Ejecuta comando en el contenedor |
| `git-status --name <sitio>` | Estado git del tema remoto |
| `deploy-websocket --name <sitio>` | Agrega servicio WebSocket al stack |

**Flujo deploy obligatorio:** `deploy` → `health` → si falla → `redeploy`.

**`--skip-backup`:** En cambios de bajo riesgo (solo PHP/JS/CSS sin migraciones de BD ni cambios en uploads), agregar `--skip-backup` para evitar la transferencia del backup (~2-3 min extra). Usar siempre en cambios de código puros. Omitir solo cuando hay riesgo real de regresión de datos.

**Importante:** `deploy --update` funciona para WordPress Y Rust (clona/rebuild vía Coolify API). Para servicios Rust, el build toma 8-12 minutos — si devuelve 503, es normal — esperar. `deploy-service` NO funcionar para Rust (falla en build Docker paso 3/6).

### code-sentinel (Glory Sentinel)
- **Workspace folder:** `c:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\code-sentinel`
- **Path relativo:** `.agent/code-sentinel`
- **Repo:** github.com/1ndoryu/glory-sentinel (rama `main`)
- **Tipo:** extension VS Code + LSP agnostico (analiza Rust, React, PHP, CSS, SCSS, glory-specific)
- **Para modificarla:**
  1. Abrir workspace folder, editar bajo `src/analyzers/` o `src/providers/`.
  2. `npm run compile` o `vsce package`.
  3. Instalar el `.vsix` generado en VS Code (`Extensions: Install from VSIX...`).
  4. **No reiniciar VS Code automaticamente** (regla 20). El usuario decide.
- **Comandos principales (VS Code):** `codeSentinel.analyzeWorkspace`, `codeSentinel.analyzeFile`, `codeSentinel.runExternalTools`.

### varsense
- **Workspace folder:** `c:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\varsense`
- **Path relativo:** `.agent/varsense`
- **Repo:** github.com/1ndoryu/varsense (rama `main`)
- **Tipo:** extension VS Code + LSP agnostico para validacion de variables CSS y clases huerfanas.
- **Para modificarla:** mismo flujo que code-sentinel (`src/`, `vsce package`, instalar `.vsix`).
- **Comandos principales (VS Code):** `cssVarsValidator.scanAllDiagnostics`, `cssVarsValidator.scanOrphanClasses`, `cssVarsValidator.exportReport`.

**Uso en Zed (editor-agnostico via LSP):**
1. Compilar el LSP: `cd .agent/varsense && npm run compile` (genera `out/lsp/server.js`).
2. En Zed: `Ctrl+Shift+P` → **"zed: install dev extension"** → seleccionar `.agent/varsense/integrations/zed/`.
3. Si la ruta relativa no resuelve, usar variable de entorno:
   ```powershell
   $env:VARSENSE_LSP_PATH = "C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\varsense\out\lsp\server.js"
   ```

**Uso desde CLI (sin editor):**
```powershell
node .agent/varsense/out/cli/index.js <archivo-css>
```
</tool_locations>

---

