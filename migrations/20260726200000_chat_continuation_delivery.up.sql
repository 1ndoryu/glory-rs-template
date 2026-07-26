/* [267A-3] Presencia durable y ciclo idempotente de continuación por desconexión. */
BEGIN;

ALTER TABLE chat_sessions
    ADD COLUMN IF NOT EXISTS visitor_disconnected_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS visitor_disconnect_epoch BIGINT NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_chat_sessions_continuation_due
    ON chat_sessions (visitor_disconnected_at)
    WHERE visitor_disconnected_at IS NOT NULL AND status <> 'closed';

ALTER TABLE chat_continuation_tokens
    ADD COLUMN IF NOT EXISTS disconnect_epoch BIGINT NOT NULL DEFAULT 0;

CREATE UNIQUE INDEX IF NOT EXISTS uq_chat_continuation_session_epoch
    ON chat_continuation_tokens (session_id, disconnect_epoch);

ALTER TABLE chat_alert_outbox DROP CONSTRAINT IF EXISTS chat_alert_outbox_status_check;
ALTER TABLE chat_alert_outbox ADD CONSTRAINT chat_alert_outbox_status_check
    CHECK (status IN ('pending','processing','accepted_by_gateway','sent','failed','dead','cancelled'));

COMMIT;
