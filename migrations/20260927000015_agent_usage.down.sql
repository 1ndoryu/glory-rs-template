-- 279A-2 F0 rollback: vuelve al trigger solo-estima y quita las columnas
-- (datos de `uso_mensajes` ya escritos se conservan; solo cambia el futuro).

CREATE OR REPLACE FUNCTION registrar_uso_estimado() RETURNS TRIGGER AS $$
BEGIN
  INSERT INTO uso_mensajes (message_id, session_id, sender, tokens_est)
  VALUES (NEW.id, NEW.session_id, NEW.sender, GREATEST(1, (char_length(NEW.body) + 3) / 4))
  ON CONFLICT (message_id) DO NOTHING;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

ALTER TABLE agent_messages
  DROP COLUMN IF EXISTS input_tokens,
  DROP COLUMN IF EXISTS output_tokens;
