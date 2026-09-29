# Plan batería de escenarios IA — WhatsApp MN Inmobiliaria (2026-09-29)

## Objetivo
Probar de punta a punta qué puede hoy la IA por WhatsApp (ficha cliente, ficha
inmueble, fotos, notas de voz, citas, captación, delegación) y construir lo que
falte. BD limpiada a cero el 2026-09-29 (`TRUNCATE ... CASCADE`, 0 sesiones):
cada escenario parte de conversación virgen.

## Alcance / no alcance
- Alcance: vía A (`wa_a`, IA) con batería sintética (`POST /api/agent/whatsapp/webhook`)
  + teléfono físico solo donde la batería no llega (fotos, audio real).
- No alcance: vía B real (muda), gateway QR, deploy, panel frontend (ya cerrado 289A-9).

## Auditoría previa (código leído 2026-09-29, no suposiciones)
- Tools IA (6): `buscar_inmuebles`, `detalle_inmueble`, `registrar_contacto`,
  `datos_contacto`, `escalar_a_humano`, `consultar_agente`. NO existen:
  agendar cita, registrar captación, enviar foto, ver foto, transcribir audio.
- Fotos entrantes: solo `imageMessage` → `media_url` temporal (TTL 30 min) + caption
  o `(foto sin pie)`; backend guarda `[foto]`; la IA **no la ve** (transporte
  `glory-agent` solo texto, cero `image_url` en el código).
- Notas de voz: `audioMessage`/`ptt` **sin manejo** en `sesion.mjs` → se descartan
  en silencio (inbound `null`, ni siquiera llega al backend).
- Fotos salientes: `/send` del gateway **sí acepta `media_url`** (`image`+`caption`),
  pero el worker del backend solo encola `texto`; la IA no tiene cómo enviar fotos.
- Ficha cliente: tabla `clientes` (`nombre,telefono,interes,presupuesto,zona,notas,origen`)
  + tool `registrar_contacto`. Base existe.
- Captación/citas: sin tool ni flujo. Tablas `solicitudes,fotos,notes` por auditar
  (Fase 0 dirá si sirven o son legacy).
- Delegación: `consultar`/`escalar` + `devolver_a_ia` existen; bug abierto: tras turno
  `Ok` con texto la sesión queda `consultando` en vez de volver a `activa`.

## Fase 0 — Auditoría (lectura, sin build)
1. `solicitudes,fotos,notes`: columnas y escritores reales (¿las usa algún handler?).
2. `sesion.mjs:21-46`: confirmar ruta exacta del descarte de audio.
3. Worker outbox (`mod.rs:471` zona + `/send`): confirmar que `media_url` nunca se encola.
4. Webhook: ¿acepta `media_url` sintético para simular foto sin teléfono físico?

## Fase 1 — Escenarios que YA deberían funcionar (solo batería, sin código)
| # | Escenario | Disparo | Verificación en BD |
|---|-----------|---------|--------------------|
| 1 | Búsqueda simple (apartamento 2 hab) | texto | `ai` con 2-3 opciones, outbox `sent` |
| 2 | Búsqueda con presupuesto + zona | texto | `buscar_inmueble…` log con filtros |
| 3 | Pedir detalle de una opción | texto | `3×detalle_inmueble` en log |
| 4 | Ficha cliente (nombre, interés, presupuesto) | texto | fila `clientes` + `registrar_contacto` en log |
| 5 | Pregunta fuera de catálogo → agente | texto | `consultando`, `consultar_agente` en log |
| 6 | Staff responde + `devolver_a_ia` | `POST chat_staff/devolver` | `ai` retoma con la respuesta |
| 7 | Escalar a humano | texto conflictivo | estado `delegada`, IA calla |
| 8 | Pedir teléfono/dirección oficina | texto | `datos_contacto` responde |
| 9 | Cambio de tema (apartamento→casa) | texto | no mezcla fichas |
| 10 | Saludo según hora + tono 289A-2 | texto | formato + texto plano 289A-10 |

## Fase 2 — Huecos que requieren build (diseño antes de picar)
- **E11.** IA ve foto entrante: pasar `media_url` al historial como imagen del Responses API.
- **E12.** Nota de voz: transcribir (Whisper vía tool `transcribir_audio`) o al menos avisar
  "no pude escuchar tu audio" en vez de silencio.
- **E13.** IA envía fotos: tool `enviar_fotos_inmueble(ids)` → outbox con `media_url` → `/send`.
- **E14.** Captación: tool `registrar_captacion` (propietario, teléfono, tipo, zona, precio
  pretendido) + ficha + delegación a captador humano.
- **E15.** Citas: tool `agendar_visita` (inmueble, fecha tentativa, nombre) + confirmación humana.
- **E16.** Bug `consultando→activa` tras turno `Ok` (ya diagnosticado, falta fix).

## Fase 3 — Batería completa y cierre
- Correr F1 entera en BD limpia, un escenario por sesión virgen (evita contaminación
  entre turnos). Teléfono físico solo para E11/E12 reales si la Fase 0 no logra sintetizarlos.
- Criterio de aceptación por escenario: respuesta `ai` correcta + outbox `sent` +
  fila de negocio (`clientes`/etc.) donde aplique + estado final esperado.
- Registrar en `Agente/completados/tareas-2026-09-29.md` qué pasa hoy en cada escenario
  (eso responde el "no sé qué pasa") y abrir IDs nuevos para cada build E11–E16.

## Orden de ejecución
Fase 0 → Fase 1 (responde "qué pasa hoy") → decidir con usuaria qué builds E11–E16
entran (propuesta: E16 + E12-aviso + E14 primero) → Fase 2 por bloques → Fase 3.
