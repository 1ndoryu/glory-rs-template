# Plan — Bosque multijugador 2D dentro del OS

> **Fecha:** 2026-08-01
> **ID:** GAME-01
> **Estado:** planificado y bloqueado; no iniciar implementación hasta cerrar las dependencias del runtime/OS y aprobar las decisiones abiertas.
> **Prioridad:** futura, después del bloque actualmente habilitado en `roadmap.md`.
> **Dependencias globales:** runtime `AppRegistry`/`MountedView`, ciclo de vida y carga lazy, sesiones/capacidades, contratos de workspace y quality gate.
> **Fuentes canónicas:** `roadmap.md`, `Agente/documentacion/arquitectura/manual-arquitectura-wandorius-2026-07-29.md`, `Agente/documentacion/arquitectura/guia-agregar-app-2026-07-31.md`, `Agente/documentacion/arquitectura/adr-carga-apps-pesadas-2026-07-31.md`, `Agente/documentacion/producto/referencia-visual-bosque-2026-08-01.md`.

## 1. Objetivo

Agregar una app del OS que abra y cierre como cualquier otro programa y que permita, en una primera versión controlada:

- explorar un bosque 2D con cámara centrada en el personaje;
- mover un personaje propio;
- ver a otros jugadores próximos en la misma sala;
- entrar como invitado temporal o como usuario con cuenta;
- editar mapa, assets de escenario y catálogo de personajes desde un modo admin dentro del juego;
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
| Geometría | Plano libre 2D para la presentación y colocación de objetos. El servidor usará una partición espacial fija para buscar vecinos y colisiones; no se hará una búsqueda contra todos los objetos. |
| Administración | Modo edición dentro de la misma app-juego. El modo se activa por capacidad server-side; no es una bandera confiable del cliente. |
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
8. Modo admin in-game para seleccionar, colocar, mover, duplicar, ocultar y eliminar objetos del borrador.
9. Subida/gestión de assets a través del pipeline existente de media; no habrá URLs de imagen arbitrarias ni scripts dentro del mapa.
10. Editor de personaje con slots y valores allowlisted: por ejemplo cuerpo, cabello, ropa y color; sin editor de píxeles ni contenido arbitrario.
11. Guardado de borrador, preview jugable y publicación de una versión inmutable.
12. Cierre de la app con liberación comprobable de `requestAnimationFrame`, listeners, WebSocket, timers, caches y object URLs.

### Fuera del MVP

- combate, daño, inventario, economía, comercio entre jugadores y NPCs;
- chat global o voz;
- físicas complejas, navegación de agentes, agua dinámica o destrucción del escenario;
- mundo único ilimitado sin salas;
- editor de tiles profesional, scripting de assets o código ejecutable por admin;
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
  ├─ renderer Canvas 2D
  └─ modo player/admin
```

- `app-registration.ts` solo registra metadatos y carga lazy; no importa un motor pesado.
- La app devuelve contenido full-bleed; nunca crea `DesktopWindow`, taskbar, launcher ni z-index.
- `destroy()` cancela el frame loop, aborta operaciones, cierra el socket, elimina listeners y libera recursos.
- El núcleo de movimiento, snapshots, spatial queries y comandos será independiente del DOM para probarlo con Vitest.
- Se prioriza Canvas 2D propio y pequeño. No se incorpora Phaser, Pixi ni otro motor hasta demostrar que el renderer propio no alcanza el presupuesto; una dependencia pesada requeriría una decisión documentada y medición.
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

### Fase 0 — Preflight de producto, arte e integración (sin código de juego)

Esta fase existe para que no se empiece por Canvas o WebSocket antes de saber qué se está construyendo, cómo encaja en el OS y qué se puede publicar.

- [ ] Redactar una ficha de vertical slice: mapa pequeño, un avatar, movimiento, un segundo jugador simulado y criterio de “jugable” en menos de cinco minutos.
- [ ] Aprobar qué significa “bosque multijugador” en el primer release: exploración/presencia únicamente, sin combate, chat, economía ni progresión.
- [ ] Aprobar la referencia visual guardada en `Agente/documentacion/producto/referencia-visual-bosque-2026-08-01.md` y producir tres bocetos originales: mapa, avatar y UI de estado.
- [ ] Fijar gramática visual: escala de cámara, grosor de línea, densidad, capas, siluetas, contraste local/remoto, agua/terreno y variante monocroma o paleta restringida.
- [ ] Verificar autoría/licencia de cada asset futuro; registrar la imagen compartida como inspiración, nunca como asset final ni como textura para calcar.
- [ ] Definir el contrato con el OS: `appId`, ruta allowlisted, deep link, capacidades, entrada en workspace, taskbar, full-bleed, comandos y presentación móvil.
- [ ] Decidir la restauración de sesión: se restaura el contenedor de la app si corresponde, pero nunca un WebSocket, ticket, identidad invitada ni sala automáticamente.
- [ ] Definir controles y accesibilidad del vertical slice: teclado, touch/joystick, foco, reduced motion, zoom, contraste y fallback si Canvas no es utilizable.
- [ ] Fijar presupuestos iniciales de frame, memoria, mapa, mensajes, latencia, reconexión y tiempo de apertura; cada presupuesto debe tener una métrica y un umbral.
- [ ] Crear fixtures originales y un mapa pequeño de prueba que no dependa de la imagen de referencia.
- [ ] Registrar qué lógica es agnóstica y candidata a Glory (validador de mapa, spatial index, reloj/simulación, protocolo) y qué queda específico de wandori.us.

**Gate:** ficha de vertical slice, bocetos aprobados, referencia/licencia documentada, contrato OS/realtime y presupuesto aceptados; no se escribe código de juego ni se instala un motor gráfico.

### Fase 1 — ADR, contratos y presupuesto

- [ ] Registrar GAME-01 en roadmap/índice y confirmar dependencias cerradas.
- [ ] Decidir sala única vs matchmaking/instancias pequeñas.
- [ ] Definir identidad de invitado y cómo se vincula posteriormente a una cuenta.
- [ ] Definir contrato de ticket compatible con UUID y separación Glory/wandori.us.
- [ ] Fijar esquema de mensajes, tick, límites, desconexión y códigos de error.
- [ ] Fijar licencia/dirección final de assets a partir de la referencia visual.

**Gate:** ADR realtime, ADR de identidad de invitado y contrato de mapa aprobados; sin código de juego todavía.

### Fase 2 — Esqueleto de app y renderer sin red

- [ ] Registrar `game` como app lazy/full-bleed.
- [ ] Montar Canvas 2D, cámara, bounds, input y loop abortable.
- [ ] Dibujar mapa fixture y avatar local con assets de prueba originales.
- [ ] Probar apertura/cierre repetidos en desktop, tablet y móvil.
- [ ] Confirmar que el bundle principal no descarga el chunk del juego antes de abrirlo.

**Gate:** app abre/cierra sin leaks visibles, el shell no cambia y el fixture se mueve offline.

### Fase 3 — Mundo estático y contratos de mapa

- [ ] Implementar parser/validador de snapshot.
- [ ] Implementar spatial index, capas, hitboxes simples y cámara.
- [ ] Cargar solo assets visibles con cache limitada.
- [ ] Crear endpoint/servicio de mapa publicado y fixture de versión.
- [ ] Probar documento inválido, exceso de entidades, asset inexistente y bounds malformados.

**Gate:** el mismo mapa validado se renderiza de forma determinista y no permite datos ejecutables o fuera de límites.

### Fase 4 — Realtime de una sala

- [ ] Integrar upgrade/ticket WebSocket en el backend de wandori.us.
- [ ] Crear actor de sala bajo demanda con TTL, cap de 8 y backpressure.
- [ ] Implementar inputs server-authoritative, snapshots, interpolación y presencia.
- [ ] Añadir reconexión, heartbeat, timeout y cierre al destruir la app.
- [ ] Medir CPU, memoria, mensajes, latencia y ancho de banda con 1, 4 y 8 clientes.

**Gate:** ocho clientes pueden moverse en una sala sin aceptar posiciones falsificadas, sin fanout ilimitado y sin dejar salas vivas vacías.

### Fase 5 — Invitados, cuentas y personaje base

- [ ] Emitir identidad temporal para invitados con límites de abuso.
- [ ] Asociar cuenta autenticada con perfil de juego persistente.
- [ ] Crear personaje base y selección de opciones allowlisted.
- [ ] Definir qué datos se conservan al pasar de invitado a cuenta.
- [ ] Probar logout, sesión revocada, reconexión y cambio de usuario.

**Gate:** ningún invitado puede invocar admin ni reclamar el estado de otra identidad; el perfil no depende de datos enviados sin validar.

### Fase 6 — Editor admin y publicación

- [ ] Añadir modo edición dentro de la app solo para admin.
- [ ] Añadir comandos de selección/colocación/movimiento/duplicado/borrado.
- [ ] Añadir asset catalog y pipeline de versiones.
- [ ] Persistir borrador con revisión optimista y conflicto visible.
- [ ] Añadir preview de borrador y publicación atómica.
- [ ] Auditar cambios sensibles y garantizar que la sala activa conserva su versión.

**Gate:** un admin puede editar bosque/roca/árbol, guardar, previsualizar y publicar; un usuario normal recibe rechazo server-side aunque fuerce el cliente.

### Fase 7 — Hardening y operación

- [ ] Tests de carga acotados hasta el límite de 8 por sala y prueba de rechazo al noveno.
- [ ] Soak de abrir/cerrar/reconectar y dos salas concurrentes dentro del presupuesto acordado.
- [ ] Pruebas negativas de tickets, mensajes grandes, inputs rápidos, velocidad, colisión, permisos y documentos corruptos.
- [ ] Métricas agregadas sin coordenadas precisas ni identidad innecesaria.
- [ ] `task:check`, type-check, tests, build, navegador y revisión de teardown.
- [ ] Runbook de rollback de versión de mapa y assets; deploy queda fuera de alcance salvo instrucción explícita.

**Gate:** Definition of Done completa, reporte de presupuesto y ausencia de errores bloqueantes.

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
| Asset malicioso o pesado | pipeline media existente, MIME/dimensiones/peso, versiones y sin scripts. |
| La referencia visual deriva en copia o assets sin licencia | moodboard separado, bocetos originales, registro de procedencia y revisión antes de importar cualquier recurso. |
| Fuga al cerrar ventana | `AbortSignal` + `destroy()` idempotente y prueba repetida. |
| Recarga restaura un ticket o duplica jugador | `window-session` solo restaura UI segura; ticket nuevo y join idempotente al reconectar. |
| Invitado suplanta cuenta/admin | ticket server-side, identidad temporal separada y capacidad comprobada en backend. |
| Editor se vuelve un motor general | catálogo y propiedades allowlisted, sin scripting ni físicas complejas. |
| Dependencia gráfica infla el OS | Canvas 2D propio y `registerLazy`; cualquier motor requiere medición/ADR. |
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
- [ ] Admin edita mapa/assets/personajes dentro de la app, guarda borrador, previsualiza y publica.
- [ ] Assets y personajes usan versiones/allowlists y no ejecutan contenido arbitrario.
- [ ] Capacidad, ancho de banda, memoria, reconexión y teardown tienen evidencia.
- [ ] Métricas operacionales, analytics/audit separados y política de consentimiento/retención documentada.
- [ ] Dirección visual aprobada sobre assets originales que reinterpretan la referencia sin copiarla.
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
9. **Accesibilidad de Canvas:** overlay DOM con estado y controles accesibles, modo reducido y fallback informativo, o alcance explícito si alguna parte no puede hacerse accesible.
10. **Feature flag/lanzamiento:** decidir quién puede ver la app durante el piloto y cómo se desactiva sin romper workspace ni sesiones.
11. **Métricas y consentimiento:** confirmar catálogo de eventos, retención y separación entre analytics de producto, audit de admin y telemetría operacional.
12. **Asset pipeline:** decidir si el blanco/negro es obligatorio para el mapa o si se permite una paleta restringida, siempre con assets originales/versionados.

Estas decisiones deben registrarse en ADR antes de Fase 1/3 según corresponda. Ninguna debe resolverse agregando flags ad hoc al cliente.
