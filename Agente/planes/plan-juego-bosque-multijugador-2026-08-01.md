# Plan — Bosque multijugador dentro del OS

> **Fecha:** 2026-08-01
> **ID:** GAME-01
> **Estado:** dirección Three.js 3D aprobada; fixture offline, persistencia, publicación admin de mapas y la primera sala realtime server-authoritative están integrados; identidad invitada, reconexión persistente y editor siguen pendientes.
> **Prioridad:** futura, después del bloque actualmente habilitado en `roadmap.md`.
> **Dependencias globales:** runtime `AppRegistry`/`MountedView`, ciclo de vida y carga lazy, sesiones/capacidades, contratos de workspace y quality gate.
> **Fuentes canónicas:** `roadmap.md`, `Agente/documentacion/arquitectura/adr-bosque-3d-assets-terreno-2d-2026-08-01.md`, `Agente/planes/plan-assets-terreno-bosque-3d-2026-08-01.md`, `Agente/planes/plan-glory-render-motor-juegos-2026-08-01.md`, `Agente/documentacion/arquitectura/adr-glory-render-repositorio-agnostico-2026-08-01.md`, `Agente/documentacion/arquitectura/adr-carga-apps-pesadas-2026-07-31.md`, `Agente/documentacion/producto/referencia-visual-bosque-2026-08-01.md`.

## 1. Objetivo

Agregar una app del OS que abra y cierre como cualquier otro programa y que permita, en una primera versión controlada:

- explorar un bosque 3D isométrico renderizado con Three.js sobre un mundo lógico finito X/Z;
- mover un personaje propio;
- ver a otros jugadores próximos en la misma sala;
- entrar como invitado temporal o como usuario con cuenta;
- editar terreno/colocación desde un programa 2D y administrar modelos externos desde `Assets 3D`, sin modelar geometría dentro del OS;
- editar un personaje sencillo mediante opciones previamente autorizadas;
- publicar cambios de forma versionada sin romper una partida activa.

El objetivo no es construir todavía un MMO ni un motor general. La primera entrega debe demostrar un loop pequeño, medible, reversible y capaz de ejecutarse en el servidor actual sin reservar procesos permanentes para un mundo vacío.

## 1.1 Revisión crítica del plan

### Lo que está bien

- El alcance inicial está acotado a exploración, presencia y edición de un mapa; combate, economía, chat y mundo infinito quedan fuera.
- La app respeta la arquitectura del OS: registro lazy, `MountedView`, teardown, mismas reglas para desktop/móvil y ningún chrome duplicado.
- La autoridad de movimiento, permisos, publicación y versiones está en el servidor; el cliente solo expresa intención y renderiza.
- Salas pequeñas, spatial index, interés por proximidad, TTL de salas y límites de mensajes evitan diseñar un MMO accidentalmente.
- El plan incluye negativas, carga, reconexión, auditoría y rollback, no solo el camino feliz.

### Omisiones corregidas en este documento

- Se añade una fase previa sin código para cerrar producto, referencia visual, licencia, integración con el OS, persistencia de sesión y presupuestos.
- Se define que restaurar la ventana del OS no debe restaurar automáticamente una sala ni un ticket WebSocket; reconectar será una decisión explícita y segura.
- Se separan los contratos de app/ruta/capacidad, juego/render y realtime; esto evita que el juego contamine `WindowManager`, `MobileAppStack` o el workspace.
- Se incorporan pruebas deterministas con reloj/transporte falsos, fuzzing de mapas/mensajes y simulación de reconexión antes de una prueba de carga.
- Se agregan criterios de accesibilidad, reduced motion, background-tab, feature flag, observabilidad y operación Coolify.
- La imagen se guarda como moodboard versionado; no se tratará como asset, textura ni fuente para calcar.

## 2. Decisiones confirmadas

| Área | Decisión inicial |
|---|---|
| Acceso | Invitado + cuenta. El invitado tiene identidad temporal; la cuenta permite conservar personaje y preferencias. |
| Renderer | Three.js 3D isométrico, lazy y con teardown GPU. El boceto cenital se conserva como referencia histórica, no como renderer candidato. |
| Geometría | Presentación 3D sobre simulación lógica X/Z. La altura proviene de un terreno determinista finito y el servidor mantiene spatial index/colisión autoritativos. |
| Administración | Dos programas admin: `Assets 3D` gestiona GLB/versiones y `Editor de mapa` edita terreno/instancias en 2D con preview 3D. Ninguno modela mallas. |
| Assets | Autoría externa; runtime GLB/glTF 2.0, versiones inmutables, storage por hash, metadata/proxies allowlisted y validación server-side. |
| Sala inicial | Máximo objetivo de 8 jugadores por sala. El límite debe ser configurable y rechazará conexiones nuevas antes de degradar la sala. |
| Dirección visual | Bosque dibujado de tinta/mapa: árboles, rocas, agua y terreno con líneas orgánicas, capas y lectura clara. La imagen compartida se registra como referencia de atmósfera y composición, no como asset final ni como textura que se copie sin licencia. |
| Presentación OS | Una app `registerLazy`, `full-bleed`, con el mismo `MountedView` en desktop/tablet y móvil. No habrá `MobileGameApp` paralelo. |

### Registro de la referencia visual

La referencia proporcionada el 2026-08-01 queda descrita como: bosque visto desde arriba, composición de mapa dibujado a tinta, masas de árboles con dos siluetas principales, lagos, caminos/terreno tramado, cuadrícula conceptual y catálogo visual de símbolos. Servirá para moodboard, densidad, capas y lenguaje de assets. Antes de producir assets finales habrá que definir autoría/licencia y decidir si se usa una reinterpretación original en blanco y negro o una paleta de color muy restringida.

## 3. Límites del MVP

### Incluido

1. Un mapa publicado inicial, acotado y navegable.
2. Cámara que sigue al personaje con límites del mapa.
3. Movimiento de cuatro u ocho direcciones, según la decisión de controles; colisiones estáticas simples.
4. Presencia de otros jugadores: aparecer, moverse interpoladamente, salir y reconectar.
5. Salas pequeñas, con un snapshot server-authoritative y visibilidad por proximidad.
6. Invitado temporal y usuario autenticado.
7. Un catálogo pequeño de assets: terreno, árbol, roca, agua y al menos un punto de spawn.
8. `Editor de mapa` admin en vista 2D para pintar altura/superficie y seleccionar, colocar, mover, duplicar u ocultar instancias.
9. `Assets 3D` admin para importar GLB privado, analizar, previsualizar, configurar proxy/metadata, versionar y publicar.
10. Editor de personaje con slots y valores allowlisted: por ejemplo cuerpo, cabello, ropa y color; sin editor de píxeles ni contenido arbitrario.
11. Guardado de borrador, preview jugable y publicación de una versión inmutable.
12. Cierre de la app con liberación comprobable de `requestAnimationFrame`, listeners, WebSocket, timers, caches y object URLs.

### Fuera del MVP

- combate, daño, inventario, economía, comercio entre jugadores y NPCs;
- chat global o voz;
- físicas complejas, navegación de agentes, agua dinámica o destrucción del escenario;
- mundo único ilimitado sin salas;
- editor de mallas/UV/materiales/rigs, scripting de assets o código ejecutable por admin;
- cuevas, voladizos, terreno infinito, escultura libre o generación procedural en el MVP;
- generación procedural, login social, ranking y moderación avanzada;
- sincronización de cada frame o posición de todos los usuarios del mundo completo.

Cada elemento excluido deberá convertirse en una tarea independiente con presupuesto y amenaza de escalabilidad explícitos.

## 4. Arquitectura propuesta

### 4.1 Frontend dentro del OS

```text
AppRegistry/registerLazy
        ↓
MountedView + AbortSignal
        ↓
GameController
  ├─ input
  ├─ simulation visual/interpolación
  ├─ cámara
  ├─ WebSocket client
  ├─ ThreeRendererAdapter (mundo lógico X/Z + terreno por chunks)
  └─ modo player/admin

El núcleo puro offline vive en `frontend/src/features/game-core/` y no importa DOM,
Three.js, WebSocket ni persistencia. El fixture `game-playable` lo consume mediante
un controlador y adaptador visual separados; `game` y `game-3d` siguen siendo
previews visuales independientes.
```

- `app-registration.ts` solo registra metadatos y carga lazy; no importa un motor pesado.
- La app devuelve contenido full-bleed; nunca crea `DesktopWindow`, taskbar, launcher ni z-index.
- `destroy()` cancela el frame loop, aborta operaciones, cierra el socket, elimina listeners y libera recursos.
- El núcleo de movimiento, snapshots, spatial queries y comandos será independiente del DOM para probarlo con Vitest.
- Three.js queda elegido para la dirección 3D y permanece detrás de un adaptador/lazy chunk. Su presupuesto de memoria/GPU, accesibilidad y pipeline de assets todavía debe medirse; si falla, se reduce alcance/calidad visible antes de contaminar el shell o abandonar la autoridad server-side.
- El mismo contenido recibe controles de teclado/pointer/touch según la presentación. El shell móvil solo cambia el marco, no las reglas del juego.

### 4.2 Mundo libre con estructuras eficientes

El mundo se representará con coordenadas continuas para respetar la decisión de plano libre, pero no se procesará como una lista plana:

- el mapa tendrá bounds explícitos;
- los objetos estáticos se indexarán en una cuadrícula espacial/hash de celdas, aunque visualmente no estén encajados a tiles;
- cada objeto tendrá una forma de colisión limitada inicialmente a AABB, círculo o polígono simple validado;
- el servidor buscará colisiones y jugadores solo en celdas vecinas;
- el cliente renderizará por cámara + margen, con capas/layers ordenadas;
- el editor podrá ofrecer snapping opcional para facilitar alineación sin convertir el mundo en un tilemap.

No se permitirá que un mapa publicado contenga un número ilimitado de entidades, polígonos, capas o tamaños de textura.

### 4.3 Backend realtime

El backend de wandori.us deberá integrar una capa de salas encima de la base reusable de WebSocket disponible en `glory-rs/backend/src/websocket/`. El hub actual agrupa conexiones por usuario y no representa todavía salas de juego; no se debe usarlo directamente como si ya resolviera el multiplayer.

Propuesta para la primera versión:

- una tarea/actor por sala activa, creada bajo demanda al entrar el primer jugador y destruida después de un TTL sin jugadores;
- estado de sala en memoria para la primera instancia del servidor: jugadores, mapa publicado cargado, spatial index y reloj de tick;
- límite estricto de 8 jugadores por sala y límites globales configurables de salas, conexiones y mensajes;
- tick server-authoritative de 10 Hz inicialmente; el cliente puede renderizar a 30/60 Hz interpolando snapshots;
- el cliente envía intención (`move`, dirección, secuencia y heartbeat), nunca una posición final;
- el servidor valida velocidad, cooldown, bounds y colisión y emite snapshots/versiones de estado;
- snapshots filtrados por radio de interés y con delta/quantización cuando la medición lo justifique;
- desconexión, reconexión, heartbeat, timeout, backpressure y cierre ordenado como contratos explícitos;
- ningún loop de mapa permanece vivo si la sala está vacía.

El envelope JSON existente sirve para el primer MVP de 8 jugadores si se versionan mensajes y se prueban límites. No se debe optimizar prematuramente con binario; se abrirá una decisión binaria solo si la medición supera los presupuestos. La serialización no puede bloquear el thread que atiende HTTP.

### 4.4 Identidad y tickets

- La sesión web opaca existente sigue siendo la autoridad para usuarios autenticados.
- El invitado necesita una identidad de juego temporal, rate-limited y sin derechos administrativos; no se confiará solo en un `userId` enviado por el navegador.
- El ticket WebSocket será corto, de un solo propósito y ligado a la sesión/identidad server-side.
- El ticket reusable actual de Glory usa `i32`, mientras wandori.us usa UUID para usuarios. Antes de implementar se debe definir un contrato compatible con UUID/subject opaco; no se hará una conversión implícita ni se asociará una cuenta por un entero cliente.
- La capacidad admin se comprueba al abrir el modo edición y en cada comando de guardado/publicación. Ocultar botones no es autorización.

### 4.5 Contrato con el OS, rutas y sesión

- `game` se registra como una app lazy/full-bleed con `MountedView`, `AbortSignal`, `destroy()` idempotente y capacidades explícitas (`game:play`, `game:admin`); el renderer no crea ventanas, taskbar ni listeners globales.
- La ruta pública debe representar solo el estado compartible y allowlisted (por ejemplo, mapa/sala pública si finalmente se permite); nunca serializa ticket, identidad invitada, coordenadas precisas, token, snapshot privado ni posición de cámara.
- La ventana puede restaurarse como contenedor, pero `window-session` no guarda WebSocket, sala, ticket, input pendiente ni identidad temporal. Al abrir/reanudar se solicita un ticket nuevo y se muestra estado claro `desconectado/conectando/conectado`.
- Cerrar, minimizar, cambiar de breakpoint, pasar la pestaña a background o perder la sesión deben tener políticas explícitas: pausar render, cerrar o degradar socket, conservar solo estado seguro y permitir reconectar sin duplicar jugador.
- Desktop/tablet conserva la ventana; móvil usa app a pantalla completa con los mismos comandos y ruta. Los controles táctiles viven en una capa de presentación, no en la simulación.
- El catálogo del workspace contiene una referencia a la app, no una copia del mapa publicado. El mapa y sus assets se resuelven mediante servicios de juego/versiones y capacidades.

### 4.6 Observabilidad y operación

- El servidor debe exponer salud y métricas agregadas de salas activas, joins/rechazos, latencia, tick, mensajes, desconexiones, backpressure y memoria; no se registran coordenadas precisas ni payloads de usuario.
- Coolify debe poder iniciar/detener la instancia con cierre ordenado: dejar de aceptar joins, cerrar tickets, drenar salas dentro de un límite y registrar qué versión de mapa estaba activa.
- La primera topología será single-instance documentada. Si se requiere más de una réplica, se abre un ADR de coordinación/estado antes de cambiar el contrato; no se asume que el hub WebSocket actual resuelve salas distribuidas.
- El juego debe estar detrás de una feature flag o capacidad de lanzamiento hasta que el vertical slice y el hardening pasen sus gates.

## 5. Modelo de datos y versionado

Se conservará el envelope de `resources` cuando el mapa o asset necesite identidad editorial, visibilidad y lifecycle. El contenido específico vivirá en tablas de dominio, no en el workspace ni en el frontend.

### Entidades iniciales propuestas

- `game_maps`: identidad, nombre interno, estado, revisión de borrador, versión publicada y bounds.
- `game_map_versions`: snapshot inmutable publicado, hash/schema version, autor, timestamp y referencia a assets/versiones.
- `game_map_drafts`: documento editable o revisión activa con `expected_revision`.
- `game_map_entities`: solo si la medición demuestra que JSONB deja de ser manejable; no crear una fila SQL por cada árbol del primer MVP sin necesidad.
- `game_assets`: metadata normalizada, tipo (`terrain`, `tree`, `rock`, `water`, etc.), asset media autorizado, dimensiones, hitbox, layer y estado.
- `game_asset_versions`: cambios inmutables de asset para que una publicación antigua no cambie al editar el original.
- `game_character_definitions`: catálogo de piezas/slots y opciones permitidas.
- `user_game_profiles`: personaje seleccionado, configuración allowlisted y datos mínimos persistibles por usuario.
- `game_audit_events`: publicación, modificación de assets, cambios de catálogo, expulsión y acciones admin sensibles; separado de analytics.

### Documento de mapa

El snapshot inicial puede ser JSONB versionado, validado por servicio, con límites de profundidad/tamaño:

```text
mapVersion {
  schemaVersion,
  bounds,
  layers,
  entities: [{ id, assetVersionId, x, y, rotation, scale, collision, properties }],
  spawnPoints,
  publishedAssets
}
```

- `properties` será una allowlist tipada; nunca HTML, JavaScript, URLs firmadas ni handlers.
- Un mapa publicado es inmutable. Editar crea borrador y publicar crea otra versión.
- Una sala conserva la versión con la que inició; las publicaciones nuevas afectan salas nuevas o una transición explícita y segura, nunca mutan una partida en curso silenciosamente.
- El backend rechaza ciclos conceptuales, bounds inválidos, IDs de assets no autorizados, duplicados, entidades fuera de límites, hitboxes abusivas y documentos que excedan cuota.

## 6. Editor admin dentro del juego

El modo admin reutiliza la misma cámara, renderer, assets y selección del juego, pero añade herramientas registradas por comandos y autorizadas server-side:

1. **Modo:** entrar/salir de edición; mostrar estado `borrador`, `versión publicada`, `guardando`, `conflicto`.
2. **Paleta:** listar assets activos autorizados con preview, búsqueda y tipo.
3. **Escenario:** seleccionar, colocar, mover, rotar dentro de límites, duplicar, cambiar layer y eliminar.
4. **Colisiones:** visualizar hitbox; editar solo formas permitidas; probar el movimiento contra el borrador.
5. **Spawn:** crear y marcar puntos de aparición válidos.
6. **Borrador:** deshacer local acotado, guardar por comando al terminar una operación y manejar `409 expectedRevision` sin overwrite silencioso.
7. **Preview:** abrir una sesión de prueba del borrador aislada de las salas públicas.
8. **Publicar:** validar todo el documento, crear versión inmutable, auditar y mostrar resultado; no publicar desde un `if` visual del cliente.
9. **Assets:** cargar una imagen mediante media autorizada, validar MIME/dimensiones/peso, definir hitbox/layer/metadata y crear una versión; eliminar será soft delete si existen referencias.
10. **Personajes:** administrar catálogo de slots/opciones y límites; las cuentas solo pueden elegir combinaciones permitidas.

La edición en vivo no debe modificar el snapshot que usan otros jugadores. El modo admin podrá mostrar una sala de preview separada o un overlay visual del borrador, pero el contrato de publicación será transaccional.

## 7. Fases de ejecución

### Fase 0 — Dirección visual 3D aprobada

Esta fase define únicamente qué debe verse. No decide todavía movimiento, red, salas, persistencia, colisiones ni editor.

- [ ] Aprobar la referencia guardada en `Agente/documentacion/producto/referencia-visual-bosque-2026-08-01.md` como atmósfera, no como asset para copiar.
- [x] Comparar ambos bocetos y elegir Three.js 3D isométrico; conservar el cenital sin usarlo como renderer final.
- [ ] Fijar gramática visual: escala de cámara, grosor de línea, densidad, capas, siluetas, agua/terreno y variante monocroma o paleta restringida.
- [ ] Definir el marco mínimo del OS: nombre `Bosque`, icono Lucide, ventana full-bleed en desktop/tablet y pantalla completa móvil.
- [ ] Confirmar que los elementos del boceto serán originales y que la referencia no se incrusta, calca ni distribuye dentro de la app.

**Gate:** dirección 3D aprobada; todavía no existe movimiento de personaje, WebSocket, backend, base de datos ni editor.

**Auditoría de cierre — Fase 0:**
- [ ] **SOLID/arquitectura:** renderer, shell, contratos, assets y realtime tienen límites explícitos; no aparece un segundo runtime paralelo.
- [ ] **Rendimiento/escalabilidad:** presupuesto inicial, perfiles de dispositivo y límite de mapa/sala están medidos o marcados como hipótesis verificable.
- [ ] **Seguridad/observabilidad:** procedencia de referencia, permisos, datos no sensibles y eventos mínimos están documentados; la fase no avanza sin evidencia en el plan/ADR.

### Fase 1 — Dos bocetos visuales ejecutables dentro del OS

Planes específicos: `Agente/planes/plan-boceto-visual-bosque-2026-08-01.md` y `Agente/planes/plan-boceto-visual-bosque-3d-2026-08-01.md`.

- [ ] Registrar una app lazy `game`/`Bosque` que abra y cierre mediante el runtime existente.
- [ ] Mostrar una escena estática original con HTML/SVG y CSS dedicado; no usar game loop, Canvas animado ni estado de juego.
- [ ] Registrar `game-3d`/`Bosque 3D` sin reemplazar el primero, con Three.js lazy, primitivas low-poly y cámara orbital limitada.
- [ ] Liberar en el boceto 3D controles, observers, animation loop, geometrías, materiales, renderer y contexto WebGL al cerrar.
- [ ] Integrar el boceto como contenido full-bleed, sin crear ventanas, taskbar, menús o z-index propios.
- [ ] Verificar desktop 1440×900, tablet 1024×768, móvil 390×844 y 320px.
- [ ] Presentar capturas y la app real al usuario; iterar densidad, escala, árboles, agua, avatar, contraste y posible paleta hasta recibir aprobación explícita.
- [ ] Mantener fuera del boceto: movimiento, controles, colisiones, salas, jugadores reales, login, guardado, analytics propio, admin y publicación.

**Gate:** superado para la dirección 3D; faltan cierre técnico/commit del prototipo y parámetros de cámara, relieve y assets.

**Auditoría de cierre — Fase 1:**
- [ ] **SOLID:** ambos bocetos usan `AppRegistry`/`MountedView` y no duplican chrome, navegación ni estado del shell.
- [ ] **Rendimiento:** la app es lazy, el chunk pesado no afecta el arranque y abrir/cerrar repetidamente no deja canvas, listeners, timers ni GPU vivos.
- [ ] **Escalabilidad/UX:** la dirección elegida conserva una ruta para mapa finito, assets externos, móvil/tablet, accesibilidad y métricas sin rehacer la app.

### Fase 2A — Núcleo lógico offline defensivo

Esta subfase no abre salas ni convierte los previews en gameplay. Entrega una base
pura que podrá reutilizar el futuro cliente y cuya semántica podrá portarse al
servidor sin depender de Three.js.

- [x] Definir contratos JSON-safe de bounds, colliders estáticos, jugadores, inputs y snapshots en `frontend/src/features/game-core/contracts.ts`.
- [x] Validar mapas con esquema/bounds/IDs/formas/cuotas y rechazo fail-closed de documentos JSON corruptos.
- [x] Implementar spatial hash determinista con límites de celdas y consultas acotadas.
- [x] Implementar movimiento X/Z determinista con velocidad, delta y subpasos presupuestados; solo obstáculos estáticos, sin colisión jugador-jugador.
- [x] Implementar deduplicación/rechazo de secuencias e inputs, normalización de dirección e interpolación de snapshots.
- [x] Proteger diccionarios contra prototipos, IDs especiales y entradas no finitas.
- [x] Cubrir invariantes con 18 tests deterministas y validar type-check, regresiones de previews/registro/workspace, build y `git diff --check`.

**Evidencia:** `frontend/src/features/game-core/`; type-check PASS; 18 tests del núcleo PASS; 18 tests de regresión PASS; build PASS; diff-check PASS.

**Gate:** base lógica offline estable; no implica que exista movimiento visible, servidor autoritativo, WebSocket, identidad, persistencia ni editor.

### Fase 2 — ADR, contratos y presupuesto

- [x] Aceptar `adr-bosque-3d-assets-terreno-2d-2026-08-01.md`: Three.js, GLB externo, mundo X/Z y terreno 2D finito.
- [ ] Ejecutar `plan-assets-terreno-bosque-3d-2026-08-01.md` por fases, sin adelantar editor o importador.
- [ ] Registrar GAME-01 en roadmap/índice y confirmar dependencias cerradas.
- [ ] Decidir sala única vs matchmaking/instancias pequeñas.
- [ ] Medir y aprobar presupuesto de chunk, GPU, memoria, mapa, assets, móvil y teardown para completar el ADR.
- [ ] Definir identidad de invitado y cómo se vincula posteriormente a una cuenta.
- [x] Definir contrato de ticket compatible con UUID y separación Glory/wandori.us; el ticket opaco y el store server-side quedan implementados en 297A-40/41.
- [ ] Fijar esquema de mensajes, tick, límites, desconexión y códigos de error.
- [ ] Fijar licencia/dirección final de assets a partir de la referencia visual.
- [ ] Redactar la ficha del vertical slice jugable: mapa pequeño, avatar con movimiento, segundo jugador simulado y criterio de “jugable”.
- [ ] Confirmar que el primer release es exploración/presencia, sin combate, chat, economía ni progresión.
- [ ] Decidir la restauración segura de sesión y fijar presupuestos de frame, memoria, mapa, mensajes, latencia y reconexión.
- [ ] Registrar qué lógica es agnóstica y candidata a Glory y qué queda específica de wandori.us.

#### 297A-39 — Contrato realtime v1 sin transporte

- [x] Definir envelope versionado `v:1` para join, move, heartbeat, client ack, joined, snapshot, heartbeat ack y error.
- [x] Mantener el ticket de join opaco y separar el `playerId` efímero de cualquier UUID de cuenta o subject interno.
- [x] Validar límites antes de deserializar: 512 bytes cliente, 4 KiB servidor, strings acotados, 8 entidades, secuencias y rate budget.
- [x] Alinear frontend/Rust en `deny_unknown_fields`, UTF-8 estricto, Unicode por puntos de código, controles C0/C1/DEL, vectores finitos, errores allowlisted y snapshots filtrados/deterministas.
- [x] Cubrir negativos: JSON/campos desconocidos, versión, dirección, replay/jump, entidades duplicadas, payload sobredimensionado, UTF-8 inválido y timestamps negativos.
- [x] Validar sin abrir conexiones: frontend type-check, 26 tests dirigidos y build; Rust fmt/check y 8 tests del modelo.

#### 297A-40 — Ticket de juego firmado sin transporte

- [x] Emitir tickets `g1.game` con subject UUID resuelto por servidor, propósito fijo, nonce aleatorio y firma HMAC.
- [x] Acotar TTL a 30 segundos por defecto y 60 segundos máximo; rechazar reloj inválido, secreto vacío y expiración vencida.
- [x] Rechazar tokens mayores de 512 bytes antes de dividir/decodificar y mantener errores de verificación sin revelar detalles sensibles.
- [x] Consumir cada nonce una sola vez con un replay store local acotado a 4096 entradas y poda de entradas expiradas.
- [x] Cubrir manipulación, secreto incorrecto, propósito incorrecto, UUID inválido, expiración, replay, poda, token sobredimensionado y clocks inválidos.

#### 297A-41 — Emisión HTTP autenticada del ticket

- [x] Añadir `GLORY_GAME_TICKET_SECRET` opcional a `AppConfig`/`AppState`, sin hardcodear secretos; la emisión falla cerrado con 500 si no está configurado.
- [x] Exponer `POST /api/game/ticket` con `AuthUser` y CSRF; el subject procede de la sesión server-side y el cliente no puede elegirlo.
- [x] Mantener el ticket opaco: el UUID queda en `GameTicketStore`, la respuesta solo contiene `{ ticket }` y el router de pruebas comparte el mismo estado.
- [x] Registrar la ruta y el schema en OpenAPI y conservar `create_router` compatible mediante `create_router_with_state` para pruebas/adaptadores.
- [x] Cubrir 401 sin sesión, 403 por CSRF ausente/incorrecto, 500 por secreto ausente y emisión positiva en 3 pruebas HTTP reales sobre PostgreSQL temporal migrado.

#### 297A-42 — Frontera de upgrade WebSocket del juego

- [x] Activar `axum/ws` y registrar `GET /api/game/ws` en el router de producción, sin reutilizar el hub Glory `i32` como autoridad de salas UUID.
- [x] Exigir `join` como primer mensaje WebSocket, usando el contrato realtime v1 y sin aceptar ticket por query string ni cookie.
- [x] Aplicar timeout de handshake de 5 segundos, límite global inicial de 64 conexiones y guard RAII para liberar capacidad al cerrar.
- [x] Resolver el ticket con `GameTicketStore` de forma single-use y fail-closed; cubrir secreto ausente, secreto incorrecto, replay y subject UUID solo en memoria.
- [x] Enviar error allowlisted `map_unavailable` y cerrar después de autenticar mientras no exista mapa/sala; no aceptar todavía `move`, snapshots ni presencia.
- [x] Cubrir capacidad/teardown, timeout, parseo del join y resolución/replay del ticket con tests dirigidos; la prueba de upgrade TCP real queda para el siguiente bloque.

#### 297A-43 — Prueba TCP real del upgrade WebSocket

- [x] Añadir `tokio-tungstenite` y `futures-util` solo como dependencias de desarrollo para probar el cliente contra Axum real.
- [x] Levantar un servidor efímero en `127.0.0.1:0` con `create_router_with_state`, `PgPool::connect_lazy` y shutdown ordenado mediante `oneshot`/`JoinHandle`.
- [x] Verificar por TCP el upgrade real, `join` válido con `map_unavailable`, cierre fatal, replay `unauthorized` y primer mensaje inválido `invalid_message`.
- [x] Verificar por TCP el límite global: con capacidad 1, el segundo upgrade recibe HTTP 409 antes de abrir WebSocket.
- [x] Mantener los tests independientes de PostgreSQL real y sin abrir actor de sala, movimiento ni snapshots.

#### 297A-45 — Cliente realtime autenticado del Bosque

- [x] Añadir adaptador `game-realtime-client.ts` separado de `game-core`, con ticket provider, WebSocket inyectable, estados de conexión y URL `ws/wss` derivada del host.
- [x] Enviar `join`, `move` y heartbeat solo después de conexión/join; validar mensajes server-side en el boundary, ignorar snapshots atrasados e interpolar snapshots sucesivos.
- [x] Integrar el adaptador en `game-playable` solo para cuentas autenticadas; mantener fallback offline para usuarios públicos y errores de transporte.
- [x] Usar el `playerId` efímero server-side para distinguir el avatar local de entidades remotas; liberar socket, heartbeat, listeners, RAF y escena en `destroy()`.
- [x] Cubrir handshake, ticket, joined, snapshots, interpolación, heartbeat, error fatal/no fatal, snapshot stale, URL segura y teardown con 20 tests frontend.

**Límite de 297A-45:** no se implementan identidad invitada, reconexión persistente, dos salas, editor admin, publicación en vivo ni métricas operacionales.

**Gate técnico:** PASS verificado con type-check, 20 tests frontend dirigidos, build y `git diff --check`; la validación visual de `/forest-playable` queda pendiente porque la automatización de navegador no produjo una sesión/pestaña válida. No se abre socket para usuarios públicos.

#### 297A-46 — Harness de medición realtime 1/4/8 clientes

- [x] Añadir `tests/game_ws_benchmark.rs` como test manual ignorado (`--ignored --nocapture`) contra el router WebSocket real y un mapa fixture inyectado.
- [x] Medir por escenario de 1, 4 y 8 clientes la latencia p50/p95 de `joined` y primer snapshot, snapshots recibidos, mensajes y bytes de payload cliente/servidor durante una ventana acotada.
- [x] Mantener el benchmark fuera de la suite normal, con timeout, cierre de sockets y TTL de sala `0` solo para que cada escenario retire su actor antes del shutdown.
- [x] Ejecutar `cargo check --test game_ws_benchmark` y `cargo test --test game_ws_benchmark -- --ignored --nocapture` en un entorno con espacio suficiente; registrar resultados reproducibles.
- [x] Capturar CPU y memoria del proceso externamente; el harness sigue reportando payload JSON y no se presenta como medición de bytes físicos de transporte.

**Estado de validación:** PASS reproducible con `CARGO_TARGET_DIR=C:/tmp/glory-target/game_ws_benchmark_check`: `cargo check --test game_ws_benchmark` y `cargo test --test game_ws_benchmark -- --ignored --nocapture`. Escenarios 1/4/8: 1 cliente `join p50/p95 0.63/0.63 ms`, primer snapshot `0.65/0.65 ms`, 21 snapshots, 23 mensajes servidor y 4.548 bytes de payload servidor; 4 clientes `1.04/1.09 ms`, `1.06/1.11 ms`, 84 snapshots, 92 mensajes y 48.876 bytes; 8 clientes `1.69/1.97 ms`, `1.71/1.98 ms`, 168 snapshots, 184 mensajes y 179.576 bytes. La ejecución duró 6.56 s, pasó 1/1 test y la monitorización externa observó pico de 56.34 MiB working set y 1.031 s de CPU acumulada. Los bytes son payload JSON del harness; el tráfico físico de red queda fuera de esta evidencia.

**Límite de 297A-46:** no cambia el protocolo productivo, no añade métricas operacionales, no habilita reconexión ni invitados y no sustituye una prueba de carga distribuida.

#### 297A-44 — Actor de sala server-authoritative

- [x] Crear `GameRoomState` single-instance bajo demanda con actor Tokio de propietario único, cap estricto de 8 jugadores y TTL configurable de sala vacía.
- [x] Materializar el mapa publicado como `GameRoomMap` inmutable, verificar metadata/hash, transformar colliders/spawns y construir un spatial index con presupuesto global de referencias.
- [x] Conectar el ticket single-use al actor: `joined`, snapshot inicial, heartbeat, intents `move` server-authoritative, secuencias replay/jump, rate limit y snapshot filtrado por radio de interés.
- [x] Aplicar backpressure bounded: `try_send` para comandos normales/snapshots, expulsión de conexiones lentas y `Disconnect` prioritario para evitar jugadores zombis.
- [x] Manejar frames inválidos/oversized, Ping/Pong, cierre y teardown sin bloquear el loop; el mapa se carga desde `GAME_MAP_ID` o desde fixture inyectado solo en pruebas.
- [x] Cubrir determinísticamente TTL/recreación, capacidad 8, identidad duplicada, movimiento/colisión y, por TCP, joined/snapshot/move/heartbeat/replay, map unavailable, replay de ticket, mensajes inválidos textuales/binarios, cierre, sala llena y capacidad HTTP 409.

**Límite de 297A-44:** no se implementan identidad invitada, editor admin, matchmaking, dos salas, reconexión persistente, métricas operacionales ni publicación en vivo; el actor conserva la versión de mapa con la que inició.

**Gate:** PASS verificado con `cargo fmt --check`, `cargo check --tests`, Clippy del alcance, tests dirigidos del actor/mapa/handler/contrato, 7 tests TCP, `git diff --check` y `GLORY_CARGO_TARGET_DIR=C:/tmp/glory-target/game_room npm run task:check -- 297A-44`. El gate reporta warnings preexistentes no bloqueantes fuera de este bloque.

**Límite de estas entregas:** `297A-42`/`297A-43` establecen y prueban la frontera de transporte y autenticación WebSocket; no crean actor de sala, mapa activo para realtime, snapshots, presencia, autoridad de movimiento, reconexión ni identidad invitada. El `GameTicketStore`/`GameWsState` en memoria solo es válido para la primera instancia; antes de escalar se requiere un store/coordinador compartido. Estos contratos de ejecución permanecen en Fase 5/6.

**Gate:** ADR realtime, ADR de identidad de invitado y contrato de mapa aprobados; el núcleo offline puede existir, pero no se habilita gameplay conectado hasta cerrar estos contratos.

**Auditoría de cierre — Fase 2:**
- [ ] **SOLID/OCP/DIP:** cada contrato puede extenderse por versión/adaptador; ninguna decisión futura exige `if` repartidos por renderer, shell y backend.
- [ ] **Rendimiento/escalabilidad:** están definidos límites de bytes, entidades, frecuencia, chunks, concurrencia y estrategia single-instance antes de escribir código.
- [ ] **Seguridad/observabilidad:** capacidades server-side, threat model, nombres/unidades/cardinalidad de métricas y retención tienen ADR y casos negativos asociados.

### Fase 3 — Esqueleto Three.js sin red

- [x] Registrar `game-playable` como app lazy/full-bleed separada de `game` y `game-3d`.
- [x] Montar Three.js detrás de un adaptador, con cámara ortográfica limitada, bounds, input y loop abortable.
- [x] Dibujar mapa fixture, props originales y avatar local desde `game-core`.
- [x] Añadir teclado WASD/flechas y controles táctiles DOM con etiquetas accesibles.
- [x] Pausar el loop al pasar a background, observar resize y liberar input, observers, RAF, geometrías, materiales, renderer y contexto WebGL al cerrar.
- [x] Proteger la carrera de carga lazy cuando `AbortSignal` ya está abortado.
- [ ] Probar apertura/cierre repetidos en desktop, tablet y móvil con evidencia de memoria/GPU.
- [x] Confirmar por build que el juego se mantiene en carga lazy; la medición Network detallada queda pendiente.

**Evidencia del bloque offline:**
`frontend/src/features/desktop/apps/game-playable/`,
`frontend/src/features/desktop/apps/game-shared/forest-models.ts`,
`frontend/src/features/runtime/app-registration-game-playable.ts` y
`frontend/src/styles/desktop/desktop-game-playable.css`.
Type-check PASS; 40 tests del bloque/regresiones PASS; build PASS; diff-check PASS.
Navegador PASS en `/forest-playable`: `section.juegoFixture`, canvas, control
`Mover a la derecha` y consola sin errores.

**Gate:** fixture offline funcional y aislado del realtime; faltan mediciones
repetidas de memoria/GPU y validación multi-viewport antes de cerrar la fase completa.

**Auditoría de cierre — Fase 3:**
- [ ] **SOLID:** `WorldQuery`, cámara, input, renderer y lifecycle son interfaces separadas; Three.js no se filtra a lógica de dominio.
- [ ] **Rendimiento:** se registra frame p50/p95, memoria y carga del chunk; se prueba pausa background, resize, minimized y destrucción idempotente.
- [ ] **Escalabilidad/calidad:** el fixture permite agregar un segundo asset/personaje sin duplicar escena; Sentinel/VarSense detectan imports eager, loops sin teardown y módulos sobredimensionados.

### Fase 4 — Mundo estático y contratos de mapa

- [x] Implementar en frontend el contrato JSON-safe versionado de `TerrainDocument`, `GameAssetVersion`, instancias, spawns y `MapVersion`, con validador fail-closed y cuotas hard.
- [x] Adaptar `MapVersion` a `WorldMap` mediante proxies estáticos allowlisted; el núcleo sigue siendo X/Z y no inventa todavía altura de gameplay.
- [x] **297A-33 — Terreno visible por chunks y cache visual:** `buildTerrainMeshData` convierte cada `TerrainChunk` validado en posiciones/índices/superficies puras; `GamePlayableVisualCache` crea y retira `BufferGeometry` solo para chunks visibles y reutiliza geometría/materiales de props mediante `clone(true)` y prototipos. El teardown dispone el terreno y prototipos de forma idempotente.
- [x] **297A-34 — Batching InstancedMesh y métricas locales del renderer:** `GamePlayableVisualCache` agrupa por tipo los sólidos repetidos en `THREE.InstancedMesh` con máximo 128 instancias, mantiene contornos reutilizables y aplica posición/escala/rotación desde `AssetInstance`. `readRendererMetrics` normaliza `renderer.info` y `performance.memory` opcional; el controller publica `data-renderer-*`/`data-js-heap-*` sin analytics. 40 tests dirigidos PASS, incluyendo transformación de instancia y teardown idempotente.
- [x] **297A-35 — Cache visual persistente y culling por batch:** el cache conserva hasta 12 `TerrainChunk` materializados, los separa de la escena durante una eviction temporal y los reutiliza al volver; expulsa y dispone solo los chunks no activos cuando supera el límite. Los `InstancedMesh` activan frustum culling y recalculan `boundingSphere` tras sincronizar matrices. 42 tests dirigidos PASS, incluyendo eviction, reutilización y bounding sphere.
- [x] **297A-36 — Presupuesto local medible del renderer:** `evaluateGamePerformanceBudget` evalúa p95 de frame, draw calls, triángulos, geometrías, texturas y heap JS opcional con estados `pass`/`fail`/`unknown`; exige 30 muestras para frame, separa la disponibilidad de `renderer.info.render` y `renderer.info.memory` para no tratar métricas parciales como cero y publica únicamente `data-renderer-budget-*` locales. 17 tests dirigidos PASS, type-check, build y diff-check PASS.
- [x] **297A-37 — Diagnóstico WebGL y pérdida de contexto:** `detectWebGL` prueba WebGL2/WebGL, libera el contexto temporal cuando existe `WEBGL_lose_context` y el fixture muestra fallback accesible antes de montar Three.js. El controller escucha `webglcontextlost` sobre el canvas real, detiene RAF y libera listeners/input/scene al cerrar. 23 tests dirigidos PASS, type-check, build y diff-check PASS; no sustituye una medición física de GPU.
- [x] **297A-38 — Lazy loading y lifecycle repetido del fixture:** `AppRegistry.isLazy('game-playable')` verifica que el registro no resuelve la app pesada antes de instanciarla. Las pruebas cubren 12 abortos antes del montaje y 12 ciclos reales de mount/destroy con handles independientes, sin acumular input, escena ni RAF. 36 tests dirigidos PASS, type-check, build y diff-check PASS. La evidencia es de carga/lifecycle lógico, no de memoria GPU física.
- [ ] Cargar solo chunks/assets visibles con cache limitada e instancing para props repetidos; el cache/culling básico y el evaluador local ya están implementados, pero faltan culling avanzado, batching por materiales y medición física de GPU/memoria.
- [x] Crear el endpoint/servicio de lectura de mapa publicado y la migración de snapshots persistidos.
- [x] Crear el flujo admin de publicación versionada: `AdminUser`, CSRF, revisión optimista, hash canónico, activación atómica y snapshots inmutables.
- [x] Crear un fixture de versión persistido mediante el flujo autorizado y cubrir integración HTTP/DB real de autorización, CSRF, 413, revisión stale, concurrencia, activación única y trigger de inmutabilidad.
- [x] Probar documento inválido, exceso de chunks, referencias de asset inexistentes, IDs reservados, transforms, spawns y bounds malformados.
- [x] Validar en Rust el mismo JSON `MapVersion` con `serde` camelCase, `deny_unknown_fields`, proxy opcional, límites de bytes previos a la deserialización y 11 tests deterministas.
- [x] Alinear el frontend con rechazo de campos desconocidos en raíz, terreno, chunks, assets, colliders, instancias y spawns.
- [x] Proteger snapshots publicados con límite SQL de JSONB, hash SHA-256 verificado al servir, índice de una versión activa y trigger de inmutabilidad.

**Evidencia parcial:** `frontend/src/features/game-core/map-version.ts`,
`map-version.test.ts`, `src/models/game_map.rs`, `src/repositories/game_map_repo.rs`,
`src/services/game_map_svc.rs`, `src/handlers/game_map_handler.rs` y la migración
`20260801140000_game_map_versions` y la corrección incremental
`20260801150000_297a30_game_map_author_fk`. El endpoint público es
`GET /api/game/maps/:map_id`; solo consulta `is_active`, valida el documento y no
expone UUID interno, `published_by` ni `is_active`. `contentHash` se calcula con
`document_json_bytes` sobre el `JsonValue` normalizado que se persiste, y el service
lo verifica antes de responder. Los bloques `297A-32`, `297A-33` y `297A-34` añaden `map-streaming.ts`,
`map-streaming-contracts.ts`, `performance-monitor.ts`, `terrain-mesh.ts`,
el adaptador visual `game-playable-visual-cache.ts` y el normalizador
`game-renderer-metrics.ts`: la selección y la malla son puras y testeables, el
fixture actualiza `data-visible-chunks`, `data-visible-instances`,
`data-frame-p95-ms`, `data-renderer-draw-calls`, `data-renderer-triangles`,
`data-renderer-geometries`, `data-renderer-textures` y heap JS cuando el navegador
lo ofrece. El renderer crea/retira terreno por chunk, agrupa sólidos repetidos en`THREE.InstancedMesh` hasta 128 por tipo y libera recursos en el teardown. Los
contornos conservan la gramática visual y reciben la transformación de
`AssetInstance`. La ventana y el LRU tienen límites hard; `297A-35` añade un LRU
visual de 12 chunks y culling por `InstancedMesh.boundingSphere`. Las superficies
3–15 sin material específico usan el material base. Las métricas son locales y no
representan memoria GPU física ni latencia de red. El endpoint admin es `POST /api/admin/game/maps`,
protegido por `AdminUser`/CSRF, con `expectedVersion`, advisory lock por mapa,
activación atómica y límite de body de 4 MiB antes de deserializar. El fixture
`tests/game_map_publish.rs` ejercita el router de producción contra PostgreSQL real:
7/7 tests PASS para 401, admin/no-admin, CSRF, mapId incoherente, persistencia + GET
público, 413, revisión stale, segunda versión, concurrencia, una sola activa y
trigger UPDATE/DELETE. Los snapshots de prueba usan IDs únicos y se conservan porque
son inmutables; los autores se conservan por `ON DELETE RESTRICT`. La preparación de
la BD de rama se hizo aplicando solo las migraciones faltantes, ya que
`prepare-ci-db.mjs` no es idempotente sobre una BD existente. Backend: `cargo fmt
--check`, `cargo check --tests` y la integración real PASS. Frontend `297A-32`:
type-check PASS, 33 tests dirigidos PASS, build PASS y diff-check PASS. Frontend
`297A-33`: type-check PASS, 36 tests dirigidos PASS, build PASS y diff-check PASS.
Frontend `297A-34`: type-check PASS, 40 tests dirigidos PASS, build PASS y
diff-check PASS. No implica culling avanzado, cache persistente entre evictions,
medición física de GPU/memoria, realtime ni editor.

**Gate:** lectura pública y publicación admin con fixture HTTP/DB real, autorización,
concurrencia e invariantes de persistencia validadas; la selección lógica, medición
local, terreno visible por chunks y batching básico quedan evidenciados por
`297A-32`/`297A-33`/`297A-34`. La fase completa queda pendiente hasta implementar
culling/batching avanzado, completar la medición de GPU/memoria y avanzar después a
realtime.

**Auditoría de cierre — Fase 4:**
- [ ] **SOLID/OCP:** parser, validación, navegación, serialización y renderer consumen el contrato versionado sin acoplamiento circular.
- [ ] **Rendimiento/escalabilidad:** chunks, índices y manifests tienen tamaño máximo, consulta por lote y coste medido; no se usa JSON monolítico ni escaneo global.
- [ ] **Seguridad/observabilidad:** bounds, schema, extensiones, URIs y payloads se rechazan en el boundary; métricas de parseo/error no contienen coordenadas privadas.

### Fase 5 — Realtime de una sala

- [x] Integrar y probar la frontera inicial de upgrade/ticket WebSocket en el backend de wandori.us (`297A-42`/`297A-43`); el test TCP efímero cubre join, errores, replay, cierre y capacidad.
- [x] Crear actor de sala bajo demanda con TTL, cap de 8 y backpressure (`297A-44`).
- [x] Implementar inputs server-authoritative, snapshots, interpolación y presencia (`297A-44`).
- [x] Añadir heartbeat, timeout de handshake y cierre ordenado al destruir la sesión (`297A-44`); la reconexión persistente queda pendiente.
- [x] Conectar `game-playable` al transporte autenticado con fallback offline público (`297A-45`).
- [x] Preparar y ejecutar el harness manual de mensajes, payload, latencia y snapshots para 1/4/8 clientes (`297A-46`); la ejecución local obtuvo evidencia externa de CPU/memoria.
- [ ] Medir bytes físicos de transporte y comparar CPU/memoria/ancho de banda contra un presupuesto operativo en un entorno dedicado o distribuido; el benchmark local no pretende sustituir esa medición.

**Gate:** ocho clientes pueden moverse en una sala sin aceptar posiciones falsificadas, sin fanout ilimitado y sin dejar salas vivas vacías.

**Auditoría de cierre — Fase 5:**
- [ ] **SOLID:** autoridad de movimiento, transporte, sala, spatial index y broadcast son módulos sustituibles; el cliente nunca decide estado válido.
- [ ] **Rendimiento/escalabilidad:** se mide p95 de tick/join/snapshot, bytes por jugador, cola de conexión, 1/4/8 jugadores, sala llena y dos salas; se verifica TTL y límite global.
- [ ] **Seguridad/observabilidad:** rate limit, secuencias, reconexión y mensajes inválidos dejan eventos agregados y auditables sin identidad innecesaria.

### Fase 6 — Invitados, cuentas y personaje base

- [ ] Emitir identidad temporal para invitados con límites de abuso.
- [ ] Asociar cuenta autenticada con perfil de juego persistente.
- [ ] Crear personaje base y selección de opciones allowlisted.
- [ ] Definir qué datos se conservan al pasar de invitado a cuenta.
- [ ] Probar logout, sesión revocada, reconexión y cambio de usuario.

**Gate:** ningún invitado puede invocar admin ni reclamar el estado de otra identidad; el perfil no depende de datos enviados sin validar.

**Auditoría de cierre — Fase 6:**
- [ ] **SOLID/seguridad:** identidad temporal, cuenta, personaje, capacidades y ticket tienen servicios separados y validación server-side.
- [ ] **Rendimiento/escalabilidad:** join/leave/reconnect, expiración y migración invitado→cuenta se prueban bajo concurrencia y sin duplicar jugadores o sockets.
- [ ] **Observabilidad/privacidad:** audit y analytics están separados, con retención definida; no se registran tokens, coordenadas precisas ni datos privados.

### Fase 7 — Assets 3D, editor 2D y publicación

- [ ] Crear `Assets 3D` admin para importar/analizar/previsualizar/versionar GLB; no editar geometría.
- [ ] Crear `Editor de mapa` admin 2D para altura, superficie, agua, caminos, spawn y colocación de instancias.
- [ ] Reutilizar el renderer del juego para preview 3D; no crear un segundo motor dentro del editor.
- [ ] Añadir command stack de selección/colocación/movimiento/duplicado/borrado y undo/redo.
- [ ] Persistir borrador con revisión optimista y conflicto visible.
- [ ] Añadir preview de borrador y publicación atómica.
- [ ] Auditar cambios sensibles y garantizar que la sala activa conserva su versión.

**Gate:** un admin importa un GLB, crea terreno 2D, coloca instancias, guarda, previsualiza y publica; un usuario normal recibe rechazo server-side aunque fuerce el cliente.

**Auditoría de cierre — Fase 7:**
- [ ] **SOLID/OCP:** `Assets 3D`, `Editor de mapa`, publicación y runtime reutilizan servicios/contratos; agregar otra categoría no duplica analizadores ni escenas.
- [ ] **Rendimiento/escalabilidad:** análisis, manifests y referencias se procesan por lote; se mide tamaño de GLB, draw calls, memoria y coste de preview antes de publicar.
- [ ] **Seguridad/operación:** validación server-side, revisión optimista, rollback, dependency checks y auditoría de cambios sensibles tienen casos negativos y transacciones claras.

### Fase 8 — Hardening y operación

- [ ] Tests de carga acotados hasta el límite de 8 por sala y prueba de rechazo al noveno.
- [ ] Soak de abrir/cerrar/reconectar y dos salas concurrentes dentro del presupuesto acordado.
- [ ] Pruebas negativas de tickets, mensajes grandes, inputs rápidos, velocidad, colisión, permisos y documentos corruptos.
- [ ] Métricas agregadas sin coordenadas precisas ni identidad innecesaria.
- [ ] `task:check`, type-check, tests, build, navegador y revisión de teardown.
- [ ] Runbook de rollback de versión de mapa y assets; deploy queda fuera de alcance salvo instrucción explícita.

**Gate:** Definition of Done completa, reporte de presupuesto y ausencia de errores bloqueantes.

**Auditoría de cierre — Fase 8:**
- [ ] **SOLID:** Sentinel confirma límites de módulos, dependencias dirigidas y ausencia de suppressions sin ADR; se registra cualquier deuda aceptada.
- [ ] **Rendimiento/escalabilidad:** carga 1/4/8, dos salas, soak, background, reconexión, memoria GPU/CPU y rollback tienen comparación contra presupuesto y criterio de regresión.
- [ ] **Seguridad/observabilidad/operación:** negativos, consentimiento, métricas, alertas, runbook y recuperación están probados; no se marca DoD con warnings bloqueantes.

### Fase 9 — Extracción del motor agnóstico `glory-render`

Esta fase ocurre después de estabilizar Fase 8. El `game-core` provisional que ya vive en `frontend/src/features/game-core/` se considera candidato, no API definitiva.

- [ ] Auditar qué lógica se repite o puede probarse con un segundo juego: bounds, colisión, spatial hash, simulación, snapshots, interpolación, reloj y contratos de lifecycle.
- [ ] Crear `glory-render/` dentro de `glory-rust-template/` con repositorio Git, CI, versionado y quality gate propios; no mover código antes de aprobar la frontera del ADR.
- [ ] Separar `packages/core`, `packages/contracts` y `packages/three`; el core no importa DOM, Vite, Three, Axum, SQLx, AppRegistry, cuentas, red ni secretos.
- [ ] Migrar Bosque mediante exports públicos fijados por commit/submódulo, sin copiar ni mantener una segunda implementación.
- [ ] Añadir un juego mínimo de conformidad que consuma el mismo motor y fixtures deterministas; si el segundo caso no existe, no extraer abstracciones especulativas.
- [ ] Documentar SemVer, deprecaciones, changelog, compatibilidad, rollback y procedimiento para añadir nuevas utilidades agnósticas.

**Auditoría de cierre — Fase 9:**
- [ ] **SOLID/DIP:** el núcleo depende de contratos; renderer, OS, transporte, persistencia e identidad son adaptadores reemplazables.
- [ ] **Rendimiento/escalabilidad:** Bosque y el segundo juego comparan bundle, frame p50/p95, memoria, entidades y teardown; importar `core` no arrastra Three ni el shell.
- [ ] **Seguridad/operación:** el repo no ejecuta assets/scripts externos ni conoce secretos; sus releases son reproducibles, auditables y reversibles.

**Gate:** Bosque y un segundo juego consumen `glory-render` por API pública, sin código duplicado ni dependencia específica de wandori.us; el repositorio tiene CI y rollback documentados.

## 8. Presupuesto inicial de seguridad y rendimiento

Son presupuestos de planificación y deben medirse con el primer prototipo, no convertirse en optimizaciones ciegas:

- el bundle principal del OS no crece por cargar el juego;
- el juego se descarga únicamente al abrirse y se libera al cerrarse;
- tick server-authoritative inicial: 10 Hz;
- render local: hasta 60 Hz, sin enviar frames;
- input: máximo 15 mensajes/s por conexión más heartbeat controlado;
- sala: máximo 8 jugadores y límite configurable de salas activas;
- snapshot: solo jugadores/entidades en radio de interés + margen;
- cliente: límite inicial de entidades visibles y de bytes cacheados por mapa;
- WebSocket: límite de tamaño de mensaje, cola por conexión y política de desconexión lenta;
- mapa: bounds, capas, entidades, polígonos, dimensiones de assets y peso total acotados;
- admin: guardado por operación terminada, no por `pointermove`, y publicación transaccional;
- analytics: eventos semánticos de join/leave/error/publicación, nunca por frame ni con coordenadas precisas.
- renderer: objetivo de frame p95 ≤16,7 ms en desktop/tablet y degradación explícita en móvil; no se acepta ocultar errores de frame con una reducción silenciosa de calidad;
- realtime: medir p95 de tick, join, snapshot y reconexión, además de bytes por jugador y cola máxima de mensajes;
- background: una pestaña oculta pausa render y reduce actividad sin perder el contrato de heartbeat; al volver debe reconciliar snapshot, no simular tiempo local indefinido;
- observabilidad: cada métrica debe tener nombre, unidad, cardinalidad y política de retención antes de instrumentarse.

Los valores finales deberán quedar en el ADR de presupuesto después de medir 1/4/8 jugadores. Si el presupuesto falla, se reduce primero el alcance (radio, entidades, frecuencia o tamaño de mapa), no se desactiva la autoridad del servidor.

## 9. Pruebas obligatorias

### Frontend

- unitarias: movimiento, cámara, colisión, spatial index, interpolación, snapshots, parser y selección;
- deterministas: reloj falso, semillas controladas, simulación offline repetible y transporte WebSocket falso; ningún test depende de tiempo real o red pública;
- DOM/lifecycle: abort, destroy idempotente, WebSocket cerrado, frame loop detenido y object URLs revocadas;
- app registry: lazy load, capacidad, apertura/cierre y no duplicación;
- navegador: 1440×900, 1024×768, 390×844 y 320px; teclado, touch, foco, zoom 200% y reduced motion;
- visual: snapshots de tres assets originales, mapa fixture, selección, jugador local/remoto, desconectado y modo admin; no comparar contra una copia pixel a pixel de la referencia.

### Backend

- integración: ticket UUID/guest, capacidad admin, join/leave, cap de sala, timeout y publicación;
- negativas: posición falsificada, velocidad excesiva, mensaje sobredimensionado, sala llena, revisión stale, asset privado, mapa corrupto y usuario sin admin;
- concurrencia: dos updates del mismo borrador con una sola victoria y sin overwrite;
- concurrencia realtime: orden fuera de secuencia, replay de secuencia, reloj adelantado, reconexión duplicada y cliente lento;
- carga acotada: 1/4/8 jugadores, dos salas, reconexiones y conexiones lentas;
- fuzz/property: parser de mapa, límites de entidades/hitboxes, mensajes JSON y reglas de movimiento con invariantes de bounds/velocidad.

### Gate

Cada fase ejecutable cerrará con `npm run task:check -- GAME-01-Fn` o el ID que se asigne al subdividirla, además del type-check/tests del stack afectado. El quality gate no sustituye la prueba realtime ni el navegador.

## 10. Riesgos y mitigaciones

| Riesgo | Mitigación |
|---|---|
| Fanout y JSON consumen CPU | salas pequeñas, interés por proximidad, 10 Hz, mensajes limitados y medición antes de binario. |
| El mundo libre genera colisiones costosas | spatial hash fijo y formas simples; no escanear entidades globalmente. |
| Salas permanentes consumen memoria | creación bajo demanda, TTL vacío y límites globales. |
| Admin modifica una partida activa | snapshots inmutables por sala; publicación afecta nuevas salas. |
| Asset malicioso o pesado | parser GLB server-side, extensiones/URIs allowlisted, límites por perfil, storage privado, versiones y sin scripts/shaders arbitrarios. |
| La referencia visual deriva en copia o assets sin licencia | moodboard separado, bocetos originales, registro de procedencia y revisión antes de importar cualquier recurso. |
| Fuga al cerrar ventana | `AbortSignal` + `destroy()` idempotente y prueba repetida. |
| Recarga restaura un ticket o duplica jugador | `window-session` solo restaura UI segura; ticket nuevo y join idempotente al reconectar. |
| Invitado suplanta cuenta/admin | ticket server-side, identidad temporal separada y capacidad comprobada en backend. |
| Editor se vuelve un motor general | catálogo y propiedades allowlisted, sin scripting ni físicas complejas. |
| Dependencia gráfica infla el OS | Three.js lazy en chunk separado; medir Network/GPU y reducir mapa, draw calls o perfil visible antes de afectar el arranque del OS. |
| Terreno 3D vuelve complejo el editor | documento finito de celdas/alturas/superficies en vista 2D; Three.js genera chunks y no existe escultura libre. |
| Pestaña en background produce estado divergente | política de pausa/reconciliación y snapshot server-authoritative al volver al primer plano. |
| Escalado horizontal prematuro | primera fase single-instance documentada; no prometer multiinstancia hasta decidir estado compartido. |

## 11. Definition of Done de GAME-01

- [ ] Fase 0 aprobada: vertical slice, bocetos originales, licencia/procedencia, contrato OS, ruta/sesión, accesibilidad y presupuestos documentados.
- [ ] App registrada en AppRegistry, lazy, full-bleed y sin app móvil duplicada.
- [ ] Abrir/cerrar libera todos los recursos del juego.
- [ ] Restaurar la ventana no restaura tickets, sockets, salas ni identidades temporales; reconectar es explícito e idempotente.
- [ ] Mapa publicado versionado y validado; ningún cliente modifica el snapshot activo.
- [ ] Jugador local y otros 7 como máximo se ven y se mueven con interpolación.
- [ ] Servidor autoritativo rechaza posiciones, velocidades y comandos inválidos.
- [ ] Invitado y cuenta tienen identidades separadas y capacidades correctas.
- [ ] Admin gestiona GLB/versiones en `Assets 3D` y edita terreno/instancias en `Editor de mapa` 2D, guarda, previsualiza y publica.
- [ ] Assets y personajes usan versiones/allowlists y no ejecutan contenido arbitrario.
- [ ] Capacidad, ancho de banda, memoria, reconexión y teardown tienen evidencia.
- [ ] Métricas operacionales, analytics/audit separados y política de consentimiento/retención documentada.
- [ ] Dirección visual aprobada sobre assets originales que reinterpretan la referencia sin copiarla.
- [ ] El motor reutilizable está aislado en `glory-render/` cuando exista un segundo consumidor real; no se extraen piezas específicas de Bosque.
- [ ] Tests, navegador, quality gate, documentación y rollback están completos.

## 12. Decisiones aún abiertas

1. **Sala única vs instancias:** recomendación: mapa lógico único con salas de hasta 8 jugadores, elegidas por disponibilidad; el usuario debe confirmar si necesita que todos vean siempre a todos.
2. **Controles:** teclado + WASD/flechas en desktop y joystick/tap en móvil, o solo teclado en la primera prueba.
3. **Persistencia del invitado:** perder identidad al cerrar, conservarla en una cookie temporal o permitir reclamarla al registrar una cuenta.
4. **Dirección cromática:** tinta monocroma como la referencia o tinta con una paleta muy restringida para diferenciar jugador, agua y estados interactivos.
5. **Publicación en vivo:** aplicar la nueva versión solo a salas nuevas (recomendado) o permitir una transición coordinada de salas activas.
6. **Escalado futuro:** single-instance primero; si se requieren varias réplicas habrá que elegir almacenamiento/coordination realtime antes de prometerlo.
7. **Restauración de sesión:** restaurar solo el contenedor de la app (recomendado) o reingresar automáticamente a la última sala; la segunda opción exige consentimiento y ticket nuevo.
8. **Contrato de URL:** decidir si se comparte un mapa público/sala pública o si el deep link solo abre la app sin unirse; no se expondrán salas privadas ni identidades invitadas.
9. **Accesibilidad del renderer:** overlay DOM con estado y controles accesibles, modo reducido y fallback informativo, o alcance explícito si alguna parte no puede hacerse accesible.
10. **Feature flag/lanzamiento:** decidir quién puede ver la app durante el piloto y cómo se desactiva sin romper workspace ni sesiones.
11. **Métricas y consentimiento:** confirmar catálogo de eventos, retención y separación entre analytics de producto, audit de admin y telemetría operacional.
12. **Relieve y assets iniciales:** confirmar alturas discretas sin cuevas, cámara orbital limitada, monocromo/paleta y si el primer personaje necesita rig `idle/walk`.
13. **Extracción:** confirmar el commit de `glory-render` que consumirá Bosque y el tipo de integración (submódulo/artefacto); la carpeta permanece dentro de `glory-rust-template`, pero su historial Git es independiente.

Estas decisiones deben registrarse en ADR antes de Fase 2/5 según corresponda. Ninguna debe resolverse agregando flags ad hoc al cliente.
