/* [237A-5] La consolidación histórica no se puede deshacer sin volver a
 * fragmentar mensajes. El rollback solo retira la restricción preventiva. */

DROP INDEX IF EXISTS idx_chat_sessions_unique_order;
