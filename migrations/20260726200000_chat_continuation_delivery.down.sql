BEGIN;
UPDATE chat_alert_outbox SET status = 'dead' WHERE status = 'cancelled';
ALTER TABLE chat_alert_outbox DROP CONSTRAINT IF EXISTS chat_alert_outbox_status_check;
ALTER TABLE chat_alert_outbox ADD CONSTRAINT chat_alert_outbox_status_check
    CHECK (status IN ('pending','processing','accepted_by_gateway','sent','failed','dead'));
DROP INDEX IF EXISTS uq_chat_continuation_session_epoch;
ALTER TABLE chat_continuation_tokens DROP COLUMN IF EXISTS disconnect_epoch;
DROP INDEX IF EXISTS idx_chat_sessions_continuation_due;
ALTER TABLE chat_sessions
    DROP COLUMN IF EXISTS visitor_disconnect_epoch,
    DROP COLUMN IF EXISTS visitor_disconnected_at;
COMMIT;
