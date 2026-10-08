/* [08AA-21] Revierte el excerpt crudo. */
ALTER TABLE mp_respuestas_cache
  DROP COLUMN IF EXISTS excerpt_crudo;
