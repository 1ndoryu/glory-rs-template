/* [237A-7j] Rollback: eliminar tabla de tokens de continuación. */
DROP INDEX IF EXISTS idx_continuation_tokens_session;
DROP INDEX IF EXISTS idx_continuation_tokens_hash;
DROP TABLE IF EXISTS chat_continuation_tokens;
