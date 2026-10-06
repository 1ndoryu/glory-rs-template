-- [03AA-3 M3] Rollback: suelta las tres tablas del asistente Marketplace.
DROP TABLE IF EXISTS mp_auditoria;
DROP TABLE IF EXISTS mp_uso_minuto;
DROP TABLE IF EXISTS mp_tokens_emitidos;
