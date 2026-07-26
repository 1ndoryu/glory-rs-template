/* [257A-9] Cada intervención humana o toggle invalida generaciones IA en vuelo.
 * El contador vive en PostgreSQL para funcionar entre workers y reinicios. */
ALTER TABLE chat_sessions
    ADD COLUMN IF NOT EXISTS ai_generation_epoch BIGINT NOT NULL DEFAULT 0;

