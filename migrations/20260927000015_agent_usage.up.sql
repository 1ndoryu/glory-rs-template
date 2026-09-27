-- 279A-2 F0 (consume núcleo b235771): `usage` exacto por turno.
-- El núcleo inserta los mensajes `ai` con `input_tokens/output_tokens`
-- (Responses `usage`; NULL en filas viejas y mensajes no-IA), así que
-- `agent_messages` necesita las columnas o ese INSERT falla. El trigger
-- copia el exacto a `uso_mensajes.tokens_in/out`; la estima `len/4` queda
-- como fallback para lo que no traiga exacto. Espejo de
-- glory-agent/migrations/0003_usage.sql.

ALTER TABLE agent_messages
  ADD COLUMN IF NOT EXISTS input_tokens INTEGER CHECK (input_tokens IS NULL OR input_tokens >= 0),
  ADD COLUMN IF NOT EXISTS output_tokens INTEGER CHECK (output_tokens IS NULL OR output_tokens >= 0);

CREATE OR REPLACE FUNCTION registrar_uso_estimado() RETURNS TRIGGER AS $$
BEGIN
  INSERT INTO uso_mensajes (message_id, session_id, sender, tokens_est, tokens_in, tokens_out)
  VALUES (NEW.id, NEW.session_id, NEW.sender,
          GREATEST(1, (char_length(NEW.body) + 3) / 4),
          NEW.input_tokens, NEW.output_tokens)
  ON CONFLICT (message_id) DO NOTHING;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_uso_mensajes ON agent_messages;
CREATE TRIGGER trg_uso_mensajes
AFTER INSERT ON agent_messages
FOR EACH ROW EXECUTE FUNCTION registrar_uso_estimado();
