-- 011A-5: reverso de `20261001000019_nucleo_f10.up.sql`. Solo se usa en
-- rama dev (prod nunca baja migraciones); respeta el orden inverso.

DROP INDEX IF EXISTS idx_agent_eventos_sesion;
DROP TABLE IF EXISTS agent_eventos;

DROP INDEX IF EXISTS uq_agent_outbox_idem;
ALTER TABLE agent_outbox DROP COLUMN IF EXISTS idempotency_key;

DROP INDEX IF EXISTS uq_agent_messages_canal_externo;
ALTER TABLE agent_messages DROP CONSTRAINT IF EXISTS chk_agent_messages_import_sin_usage;
ALTER TABLE agent_messages
  DROP COLUMN IF EXISTS imported,
  DROP COLUMN IF EXISTS media_ref,
  DROP COLUMN IF EXISTS via,
  DROP COLUMN IF EXISTS id_externo,
  DROP COLUMN IF EXISTS numero_destino_hash,
  DROP COLUMN IF EXISTS canal;
