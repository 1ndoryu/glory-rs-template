/* [07AA-7 F1] Revierte el panel por chat. */
DROP INDEX IF EXISTS mp_cache_thread_idx;
ALTER TABLE mp_respuestas_cache
  DROP COLUMN IF EXISTS excerpt_texto,
  DROP COLUMN IF EXISTS thread_id;
