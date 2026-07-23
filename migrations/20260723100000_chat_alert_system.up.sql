-- [237A-7d] Chat alert outbox system: durable alerts for every client message.
-- Outbox guarantees at-most-once delivery per (event, channel, recipient).
-- Escalations track open/resolved cycles for WhatsApp CTA dedup.
-- Response cycles manage the 10-minute human takeover / AI fallback window.

BEGIN;

-- ============================================================
-- 1. chat_alert_outbox: durable queue for email + WhatsApp alerts
-- ============================================================
CREATE TABLE IF NOT EXISTS chat_alert_outbox (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    idempotency_key TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    channel         TEXT NOT NULL CHECK (channel IN ('email', 'whatsapp')),
    recipient       TEXT NOT NULL,
    reference_type  TEXT,
    reference_id    UUID,
    payload         JSONB NOT NULL DEFAULT '{}',
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending','processing','accepted_by_gateway','sent','failed','dead')),
    attempts        INT NOT NULL DEFAULT 0,
    available_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_at       TIMESTAMPTZ,
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at         TIMESTAMPTZ
);

-- Unique idempotency: same key → same row (INSERT ON CONFLICT DO NOTHING)
CREATE UNIQUE INDEX IF NOT EXISTS uq_chat_alert_outbox_idempotency
    ON chat_alert_outbox (idempotency_key);

-- Worker claim index: fetch pending rows ordered by available_at
CREATE INDEX IF NOT EXISTS idx_chat_alert_outbox_claim
    ON chat_alert_outbox (status, available_at, created_at)
    WHERE status IN ('pending', 'processing');

-- ============================================================
-- 2. Unique constraint on notifications (prevent per-message dupes)
-- ============================================================
-- Reconcile existing duplicates before adding the constraint.
WITH ranked AS (
    SELECT id, ROW_NUMBER() OVER (
        PARTITION BY user_id, notification_type, COALESCE(reference_type, ''), COALESCE(reference_id, '00000000-0000-0000-0000-000000000000'::uuid)
        ORDER BY created_at DESC
    ) AS rn
    FROM notifications
    WHERE reference_type IS NOT NULL AND reference_id IS NOT NULL
)
DELETE FROM notifications WHERE id IN (SELECT id FROM ranked WHERE rn > 1);

CREATE UNIQUE INDEX IF NOT EXISTS uq_notifications_dedup
    ON notifications (user_id, notification_type, reference_type, reference_id)
    WHERE reference_type IS NOT NULL AND reference_id IS NOT NULL;

-- ============================================================
-- 3. chat_escalations: open/resolved cycles per session
-- ============================================================
CREATE TABLE IF NOT EXISTS chat_escalations (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id           UUID NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    opened_by_message_id UUID,
    status               TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'resolved')),
    reason               TEXT,
    cta_message_id       UUID,
    opened_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at          TIMESTAMPTZ,
    resolved_by          UUID REFERENCES users(id)
);

-- One open escalation per session
CREATE UNIQUE INDEX IF NOT EXISTS uq_chat_escalations_open
    ON chat_escalations (session_id)
    WHERE status = 'open';

-- ============================================================
-- 4. chat_response_cycles: 10-min human takeover / AI fallback
-- ============================================================
CREATE TABLE IF NOT EXISTS chat_response_cycles (
    id                     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id             UUID NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    opened_by_message_id   UUID NOT NULL,
    first_client_message_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deadline_at            TIMESTAMPTZ NOT NULL,
    status                 TEXT NOT NULL DEFAULT 'waiting'
                           CHECK (status IN ('waiting','claimed','answered_human','answered_ai','cancelled')),
    claimed_at             TIMESTAMPTZ,
    answered_message_id    UUID,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One open (waiting) cycle per session
CREATE UNIQUE INDEX IF NOT EXISTS uq_chat_response_cycles_open
    ON chat_response_cycles (session_id)
    WHERE status = 'waiting';

-- Worker claim index
CREATE INDEX IF NOT EXISTS idx_chat_response_cycles_claim
    ON chat_response_cycles (status, deadline_at)
    WHERE status = 'waiting';

-- ============================================================
-- 5. chat_sessions: new columns for sequence, AI mode, escalation
-- ============================================================
ALTER TABLE chat_sessions
    ADD COLUMN IF NOT EXISTS next_message_sequence BIGINT NOT NULL DEFAULT 0;

ALTER TABLE chat_sessions
    ADD COLUMN IF NOT EXISTS ai_mode TEXT NOT NULL DEFAULT 'automatic'
    CHECK (ai_mode IN ('automatic', 'human_priority', 'manual_pause'));

-- ============================================================
-- 6. chat_messages: sequence number for gap detection
-- ============================================================
ALTER TABLE chat_messages
    ADD COLUMN IF NOT EXISTS sequence_num BIGINT;

CREATE INDEX IF NOT EXISTS idx_chat_messages_session_sequence
    ON chat_messages (session_id, sequence_num)
    WHERE sequence_num IS NOT NULL;

-- ============================================================
-- 7. visitor_profiles: email capture and continuation fields
-- ============================================================
ALTER TABLE visitor_profiles
    ADD COLUMN IF NOT EXISTS email_normalized TEXT;

ALTER TABLE visitor_profiles
    ADD COLUMN IF NOT EXISTS email_captured_at TIMESTAMPTZ;

ALTER TABLE visitor_profiles
    ADD COLUMN IF NOT EXISTS continuation_consent_at TIMESTAMPTZ;

ALTER TABLE visitor_profiles
    ADD COLUMN IF NOT EXISTS continuation_declined_at TIMESTAMPTZ;

ALTER TABLE visitor_profiles
    ADD COLUMN IF NOT EXISTS email_source TEXT;

-- Case-insensitive email lookup for visitor profiles
CREATE INDEX IF NOT EXISTS idx_visitor_profiles_email_lower
    ON visitor_profiles (LOWER(email_normalized))
    WHERE email_normalized IS NOT NULL;

COMMIT;
