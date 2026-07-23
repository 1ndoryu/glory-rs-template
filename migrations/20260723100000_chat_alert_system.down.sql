-- [237A-7d] Rollback: remove all chat alert system objects.

BEGIN;

-- 7. visitor_profiles columns
ALTER TABLE visitor_profiles DROP COLUMN IF EXISTS email_source;
ALTER TABLE visitor_profiles DROP COLUMN IF EXISTS continuation_declined_at;
ALTER TABLE visitor_profiles DROP COLUMN IF EXISTS continuation_consent_at;
ALTER TABLE visitor_profiles DROP COLUMN IF EXISTS email_captured_at;
ALTER TABLE visitor_profiles DROP COLUMN IF EXISTS email_normalized;

-- 6. chat_messages sequence
DROP INDEX IF EXISTS idx_chat_messages_session_sequence;
ALTER TABLE chat_messages DROP COLUMN IF EXISTS sequence_num;

-- 5. chat_sessions columns
ALTER TABLE chat_sessions DROP COLUMN IF EXISTS ai_mode;
ALTER TABLE chat_sessions DROP COLUMN IF EXISTS next_message_sequence;

-- 4. chat_response_cycles
DROP INDEX IF EXISTS idx_chat_response_cycles_claim;
DROP INDEX IF EXISTS uq_chat_response_cycles_open;
DROP TABLE IF EXISTS chat_response_cycles;

-- 3. chat_escalations
DROP INDEX IF EXISTS uq_chat_escalations_open;
DROP TABLE IF EXISTS chat_escalations;

-- 2. notifications dedup
DROP INDEX IF EXISTS uq_notifications_dedup;

-- 1. chat_alert_outbox
DROP INDEX IF EXISTS idx_chat_alert_outbox_claim;
DROP INDEX IF EXISTS uq_chat_alert_outbox_idempotency;
DROP TABLE IF EXISTS chat_alert_outbox;

COMMIT;
