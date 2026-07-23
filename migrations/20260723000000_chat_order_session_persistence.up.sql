/* [237A-5] Una orden conserva una única conversación. Antes de imponer la
 * unicidad, consolida cualquier duplicado histórico en la sesión referenciada
 * por orders.chat_session_id o, si no existe, en la más antigua. Los mensajes
 * y notas se reasignan; typing es efímero y se descarta en duplicados. */

CREATE TEMP TABLE chat_order_session_canonical ON COMMIT DROP AS
SELECT order_id, id AS keep_session_id
FROM (
    SELECT
        sessions.order_id,
        sessions.id,
        ROW_NUMBER() OVER (
            PARTITION BY sessions.order_id
            ORDER BY
                (orders.chat_session_id = sessions.id) DESC NULLS LAST,
                sessions.created_at ASC,
                sessions.id ASC
        ) AS position
    FROM chat_sessions AS sessions
    LEFT JOIN orders ON orders.id = sessions.order_id
    WHERE sessions.order_id IS NOT NULL
) AS ranked
WHERE position = 1;

CREATE TEMP TABLE chat_order_session_duplicates ON COMMIT DROP AS
SELECT sessions.id AS duplicate_session_id, canonical.keep_session_id
FROM chat_sessions AS sessions
JOIN chat_order_session_canonical AS canonical
    ON canonical.order_id = sessions.order_id
WHERE sessions.id <> canonical.keep_session_id;

UPDATE chat_messages AS messages
SET session_id = duplicates.keep_session_id
FROM chat_order_session_duplicates AS duplicates
WHERE messages.session_id = duplicates.duplicate_session_id;

UPDATE chat_session_notes AS notes
SET session_id = duplicates.keep_session_id
FROM chat_order_session_duplicates AS duplicates
WHERE notes.session_id = duplicates.duplicate_session_id;

DELETE FROM chat_typing AS typing
USING chat_order_session_duplicates AS duplicates
WHERE typing.session_id = duplicates.duplicate_session_id;

UPDATE orders
SET chat_session_id = canonical.keep_session_id
FROM chat_order_session_canonical AS canonical
WHERE orders.id = canonical.order_id
  AND orders.chat_session_id IS DISTINCT FROM canonical.keep_session_id;

DELETE FROM chat_sessions AS sessions
USING chat_order_session_duplicates AS duplicates
WHERE sessions.id = duplicates.duplicate_session_id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_chat_sessions_unique_order
ON chat_sessions(order_id)
WHERE order_id IS NOT NULL;
