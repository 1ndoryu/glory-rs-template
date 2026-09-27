-- 279A-2 F1: tablas propias del agente (clientes, canal, atención, uso).
-- CREATE IF NOT EXISTS por idempotencia (estilo 20260916000005_agent_chat).
-- `uso_mensajes` se rellena con trigger (estima len/4) hasta que F0 traiga
-- `usage` real del núcleo; entonces el trigger solo cubre lo que falte.

CREATE TABLE IF NOT EXISTS clientes (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  nombre TEXT,
  telefono TEXT NOT NULL UNIQUE,
  origen TEXT NOT NULL DEFAULT 'web' CHECK (origen IN ('web','wa_a','wa_b')),
  interes TEXT,
  presupuesto TEXT,
  zona TEXT,
  notas TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_clientes_telefono ON clientes (telefono);

CREATE TABLE IF NOT EXISTS canal_sesiones (
  session_id UUID PRIMARY KEY REFERENCES agent_sessions(id) ON DELETE CASCADE,
  cliente_id UUID REFERENCES clientes(id) ON DELETE SET NULL,
  canal TEXT NOT NULL CHECK (canal IN ('web','wa_a','wa_b')),
  telefono TEXT,
  modo TEXT NOT NULL DEFAULT 'completo' CHECK (modo IN ('completo','inicial'))
);

CREATE TABLE IF NOT EXISTS atencion_sesiones (
  session_id UUID PRIMARY KEY REFERENCES agent_sessions(id) ON DELETE CASCADE,
  estado TEXT NOT NULL DEFAULT 'activa' CHECK (estado IN ('activa','consultando','delegada')),
  modo TEXT NOT NULL DEFAULT 'completo' CHECK (modo IN ('completo','inicial')),
  asignado_a TEXT,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS uso_mensajes (
  message_id UUID PRIMARY KEY REFERENCES agent_messages(id) ON DELETE CASCADE,
  session_id UUID NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
  sender TEXT NOT NULL,
  modelo TEXT,
  tokens_est INTEGER NOT NULL CHECK (tokens_est >= 1),
  tokens_in INTEGER,
  tokens_out INTEGER,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_uso_mensajes_sesion ON uso_mensajes (session_id, created_at);

CREATE OR REPLACE FUNCTION registrar_uso_estimado() RETURNS TRIGGER AS $$
BEGIN
  INSERT INTO uso_mensajes (message_id, session_id, sender, tokens_est)
  VALUES (NEW.id, NEW.session_id, NEW.sender, GREATEST(1, (char_length(NEW.body) + 3) / 4))
  ON CONFLICT (message_id) DO NOTHING;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_uso_mensajes ON agent_messages;
CREATE TRIGGER trg_uso_mensajes
AFTER INSERT ON agent_messages
FOR EACH ROW EXECUTE FUNCTION registrar_uso_estimado();
