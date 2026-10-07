/* [07AA-7 F1] Panel por chat: qué conversación produjo cada borrador.
 * thread_id = clave estable de ventana del puente (trae nombre+aviso);
 * excerpt_texto = foto del chat (<=2000) al momento de generar. PII
 * solo-admin, misma retención 90d + purga. Filas viejas quedan 'sin-hilo'. */
ALTER TABLE mp_respuestas_cache
  ADD COLUMN thread_id TEXT NOT NULL DEFAULT 'sin-hilo',
  ADD COLUMN excerpt_texto TEXT NOT NULL DEFAULT '';
CREATE INDEX mp_cache_thread_idx ON mp_respuestas_cache (thread_id, valida_hasta DESC);
