# 07AA-7 — Panel admin Marketplace por chat (2026-10-07)

## Objetivo
Ver en el admin qué genera el asistente Marketplace por cada chat:
lista de chats + conversación (extracto) + borrador guardado + usos +
corregida + vigencia.

## Decisión de ella (2026-10-07)
**Chat + borrador**: se guarda el texto del chat (extracto, ≤2000) y el
borrador por conversación, solo visible en admin. Nota de privacidad:
`thread_id` (= clave de ventana del puente, trae nombre+aviso) y el
extracto son PII; viven solo en la misma BD, con la misma retención 90d
y purga. Las filas viejas quedan `thread_id='sin-hilo'`.

## Fases
- **F1 — Migración** `20261007000031_mp_panel_chat`: `thread_id TEXT
  NOT NULL DEFAULT 'sin-hilo'` + `excerpt_texto TEXT NOT NULL
  DEFAULT ''` en `mp_respuestas_cache` (+ índice por `thread_id`).
- **F2 — Guardar**: `guardar_cache`/`reemplazar_cache`/`corregir_cache`
  aceptan `(thread_id, excerpt)` y los persisten en los 3 caminos
  (`borrador`, `regenerar`, `corregir`); tests de roundtrip.
- **F3 — Endpoint admin**: `GET /api/admin/marketplace/chats`
  (hilos distintos + último + conteos) y
  `GET /api/admin/marketplace/chats/:thread` (filas: extracto,
  respuesta, usos, corregida, vigencia). Solo JWT admin.
- **F4 — Vista admin** (frontend `frontend/`): pestaña Marketplace en
  Mensajes (lista + detalle), reutilizando sesión/tabs existentes.
- **F5 — Gate + humo + commit**: fmt+clippy+test+tsc, humo HTTP con JWT,
  commit + push.
## Estado

- Completada 2026-10-07 (F1–F5 verdes, commit pendiente).

## Verificación
- Humo: generar en el lab → el panel muestra el chat con su texto.
- `cargo test --lib marketplace` verde + `tsc` 0.
