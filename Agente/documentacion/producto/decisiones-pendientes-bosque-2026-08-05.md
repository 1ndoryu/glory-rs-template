# Decisiones de producto pendientes — Bosque (GAME-01)

> **Fecha:** 2026-08-05
> **Origen:** Fase 0 y sección 12 de `Agente/planes/plan-juego-bosque-multijugador-2026-08-01.md`.
> **Uso:** checklist único para confirmar las decisiones de producto que bloquean el cierre
> formal de GAME-01. Nada de esto bloquea el código ya entregado; son confirmaciones para
> cerrar el DoD y guiar las fases futuras (9 en particular).

---

## 1. Referencia visual como atmósfera

**Estado:** pendiente de aprobación explícita.

**Contexto:** la imagen `referencias/bosque-tinta-mapa-2026-08-01.png` tiene autoría/licencia
no verificada. El plan exige usarla solo como moodboard: nunca como textura, sprite, tileset,
asset distribuible, ni calcando árboles/lagos/composición exacta.

**Para aprobar:**
- [ ] Confirmar que la referencia se usa como atmósfera y no como asset para copiar.
- [ ] Confirmar que los elementos del boceto serán originales (ya implementados como tales en
  el fixture y los modelos 297A-30/33).

## 2. Gramática visual

**Estado:** pendiente de decisión.

**Contexto:** el fixture ya renderiza con cámara isométrica Three.js y tinta monocroma base.
Falta fijar los parámetros formales que la referencia exige:

- [ ] Escala de cámara y tamaño de avatar en el mundo.
- [ ] Grosor de línea y densidad máxima de elementos.
- [ ] Capas de profundidad y tratamiento de agua/terreno/sombras.
- [ ] Contraste de jugador local, jugadores remotos, selección, colisiones y estados de conexión.
- [ ] Variante: **tinta monocroma** frente a **paleta muy restringida** (ver sección 5).
- [ ] Modo oscuro y `prefers-reduced-motion` (sin movimiento parpadeante).

## 3. Capturas y aprobación visual

**Estado:** pendiente de revisión con capturas.

- [ ] Revisar capturas y la app real (`/forest-playable`, `/forest-3d`) e iterar densidad,
  escala, árboles, agua, avatar y contraste hasta aprobación explícita.

> La validación técnica del fixture ya está cerrada (05-ago): WebGL2, GPU Intel Iris Xe y
> personaje `forest-scout`. Esta casilla es la aprobación de **dirección artística**, no de funcionamiento.

## 4. Sala única vs instancias (matchmaking)

**Estado:** implementado como sala única por mapa; confirmación pendiente.

**Contexto:** `GameRoomState` crea una sala por `map.map_version()` con cap de 8 jugadores y
TTL independiente (297A-44/75). La pregunta de producto:

- [ ] ¿Los jugadores deben verse **siempre todos entre sí**? → la sala única actual ya lo garantiza.
- [ ] ¿O se necesitan instancias múltiples con matchmaking/elección de sala por disponibilidad?

**Recomendación del plan:** mapa lógico único con salas de hasta 8 jugadores elegidas por
disponibilidad; confirmar si el requisito es "todos ven a todos" o no.

## 5. Dirección cromática

**Estado:** pendiente de decisión.

- [ ] **Tinta monocroma** (como la referencia): jugador, agua y estados se distinguen por
  densidad/silueta, no por color.
- [ ] **Paleta muy restringida** (2–4 tintes): para diferenciar jugador, agua y estados
  interactivos sin romper la identidad de tinta.

**Nota:** el chrome del OS permanece monocromo en ambos casos; solo el contenido del juego
podría usar color.

## 6. Controles móviles

**Estado:** teclado + D-pad DOM implementado; decisión de producto pendiente.

- [ ] Confirmar controles móviles: D-pad táctil actual, joystick virtual, o solo teclado en
  la primera prueba pública.

## 7. Persistencia del invitado

**Estado:** decidido en 297A-51 y cerrado en 297A-76 (nada se transfiere; la cuenta aplica).

- [ ] Confirmar que el invitado pierde su identidad al cerrar (cookie `guest_game` con TTL de
  2 h, revocada al autenticarse) y que no se reclamará identidad invitada al registrarse.

## 8. Publicación en vivo de mapas

**Estado:** decidido (solo salas nuevas); confirmación pendiente.

- [ ] Confirmar que la publicación de una versión nueva aplica **solo a salas nuevas**
  (recomendado; las salas activas conservan su snapshot inmutable) y no requiere transición
  coordinada de salas en vivo.

## 9. Escalado futuro

**Estado:** decidido (single-instance primero); registro pendiente.

- [ ] Confirmar single-instance para la primera publicación; si se requieren réplicas, se
  elegirá almacenamiento/coordinación realtime en un ADR aparte.

---

## Resumen para confirmar de una vez

| # | Decisión | Alternativas | Recomendación |
|---|---|---|---|
| 1 | Referencia visual | Atmósfera vs asset | Atmósfera |
| 2 | Gramática visual | — | Parámetros formales del fixture |
| 3 | Capturas/aprobación | — | Revisar app real |
| 4 | Salas | Única vs instancias | Única por mapa, cap 8 |
| 5 | Cromática | Monocroma vs paleta restringida | Monocroma base + contraste por densidad |
| 6 | Controles móviles | D-pad vs joystick | D-pad actual |
| 7 | Invitado | Perder vs reclamar | Perder (297A-76) |
| 8 | Publicación | Solo salas nuevas vs transición | Solo salas nuevas |
| 9 | Escalado | Single-instance vs réplicas | Single-instance |
