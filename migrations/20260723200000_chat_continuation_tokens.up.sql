/* [237A-7j] Tabla de tokens de continuación de chat.
 * Permite al visitante recuperar su conversación desde un enlace firmado por email.
 * - token_hash: SHA-256 del token aleatorio (nunca se guarda el token en claro).
 * - expires_at: 7 días por defecto.
 * - used_at/revoke_at: control de un solo uso y revocación. */

CREATE TABLE chat_continuation_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    visitor_id TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '7 days',
    used_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_continuation_tokens_hash ON chat_continuation_tokens(token_hash)
    WHERE used_at IS NULL AND revoked_at IS NULL;
CREATE INDEX idx_continuation_tokens_session ON chat_continuation_tokens(session_id);
