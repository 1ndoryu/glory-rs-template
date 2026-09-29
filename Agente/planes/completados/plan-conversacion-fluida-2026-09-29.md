# Plan: conversación fluida multi-mensaje (post E14/E15)

> Decisión usuaria 2026-09-29: la IA debe conversar como persona en WhatsApp
> (acuse breve → trabajo → entrega por partes), no en un solo bloque tras
> 30–60s de silencio. Implementar DESPUÉS de E14/E15 y ANTES de Fase 3
> (cambia la forma de los mensajes; la batería final debe medir lo nuevo).

## Reglas de producto (usuaria)
1. Mensajes breves (tope ~300 chars cada uno; nunca párrafos largos).
2. Pocos mensajes separados (máx 4 por turno: ej. acuse + hasta 2 piezas + cierre).
3. Orden garantizado (los id de `agent_outbox` ya lo dan; no inventar otro).
4. Propiedades: UNA por mensaje (tarjeta breve), jamás la lista completa en un
   solo mensaje (molesto de leer). Con más de 5 resultados: 5 tarjetas + cierre
   "tengo N más, ¿te muestro otras?".

## Diseño
- **F1 Intermedios del loop (núcleo `glory-agent`, loop F6):** cuando el modelo
  devuelve texto JUNTO con tool calls, hoy se descarta; entregarlo por hook
  `on_texto_intermedio` que el consumidor registra. Sin llamadas extra al LLM.
- **F2 Tarjetas deterministas (backend, sin depender del modelo):** tras
  `buscar_inmuebles`, el backend encola 1 mensaje por propiedad (formato fijo
  desde `Tarjeta`: título, tipo/operación, precio, ubicación) y devuelve al
  modelo `{"tarjetas_enviadas": N}` para que NO las repita: solo intro breve +
  cierre breve. Precisión + brevedad sin pelear con el prompt.
- **F3 Tope y ritmo:** máx 4 mensajes/turno (acuse + tarjetas/fotos + cierre);
  `max` de fotos ya existe (3); tope tarjetas 5/turno.
- **F4 Fallback de fallo:** si el turno falla tras intermedios, enviar
  "se me complicó buscarlo, ¿me lo repites en un momento?" (hoy el fallo es
  silencio total; esto lo tapa).
- **F5 (opcional, si el acuse mete ruido en turnos rápidos):** retener el
  intermedio 8s; si el turno ya terminó, descartarlo y entregar solo el final.

## Fases
1. Hook en `glory-agent` + registro en `whatsapp.rs` (acuse en vivo).
2. Tarjetas 1-propiedad-por-mensaje + topes + `{"tarjetas_enviadas"}`.
3. Fallback de fallo (con test).
4. E2E sintético: pedir fotos (acuse+cierre separados), pedir lista (N tarjetas
   1-por-mensaje + cierre), fallo simulado (fallback). Criterio: textos ≤300
   chars, ≤4 mensajes, orden por id, outbox `sent`.
5. Recién entonces: Fase 3 batería completa.

## No alcance
- Streaming token a token (overkill; el grano es "mensaje").
- Presence "escribiendo..." (mejora menor; evaluar en fase 1 si el acuse tarda
  por el poll de 15s del worker).
