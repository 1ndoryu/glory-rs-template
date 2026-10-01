-- 011A-5 Fase0/1: espejo del DDL del nucleo glory-agent@8560269 que el
-- compilador exige (los `query!` del core se verifican contra esta BD):
-- mitad `agent_messages` + mitad `agent_outbox` de `0004_canal.sql` y
-- `0005_handoff.sql` completos. Todo aditivo (`IF NOT EXISTS`); el binario
-- aplica migraciones al arrancar (`main.rs`), asi cubre dev y prod.
-- El canal de MN sigue viviendo en `canal_sesiones`: estas columnas son
-- del nucleo (dedup ingress, idempotencia outbox, auditoria handoff).

-- F10/F2 (nucleo 0004_canal.sql): columnas de canal en mensajes.
ALTER TABLE agent_messages
  ADD COLUMN IF NOT EXISTS canal TEXT NOT NULL DEFAULT 'web',
  ADD COLUMN IF NOT EXISTS numero_destino_hash TEXT,
  ADD COLUMN IF NOT EXISTS id_externo TEXT,
  ADD COLUMN IF NOT EXISTS via TEXT,
  ADD COLUMN IF NOT EXISTS media_ref JSONB,
  ADD COLUMN IF NOT EXISTS imported BOOLEAN NOT NULL DEFAULT FALSE;

-- El import puntual no trae usage (fuera de la ventana del plan del nucleo).
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'chk_agent_messages_import_sin_usage'
  ) THEN
    ALTER TABLE agent_messages
      ADD CONSTRAINT chk_agent_messages_import_sin_usage
      CHECK (imported = FALSE OR (input_tokens IS NULL AND output_tokens IS NULL));
  END IF;
END $$;

-- Dedup parcial del nucleo: solo filas con id externo del canal.
CREATE UNIQUE INDEX IF NOT EXISTS uq_agent_messages_canal_externo
  ON agent_messages (canal, id_externo) WHERE id_externo IS NOT NULL;

-- F10/F2 (nucleo 0004_canal.sql): clave de idempotencia en outbox. La
-- calcula el bin (hash de contenido, ver plan 011A-5); el UNIQUE parcial
-- es la fuente unica anti-duplicados (filas viejas NULL, sin dedup).
ALTER TABLE agent_outbox
  ADD COLUMN IF NOT EXISTS idempotency_key TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS uq_agent_outbox_idem
  ON agent_outbox (idempotency_key) WHERE idempotency_key IS NOT NULL;

-- F10/F4 (nucleo 0005_handoff.sql): auditoria append-only de la consola.
CREATE TABLE IF NOT EXISTS agent_eventos (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id UUID NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
  tipo TEXT NOT NULL CHECK (tipo IN (
    'handoff.tomar', 'handoff.devolver', 'handoff.cerrar', 'handoff.reabrir',
    'config.cambio', 'import.puntual'
  )),
  actor TEXT NOT NULL CHECK (char_length(actor) BETWEEN 1 AND 200),
  detalle JSONB NOT NULL DEFAULT '{}',
  creado_en TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_agent_eventos_sesion
  ON agent_eventos (session_id, creado_en DESC);
