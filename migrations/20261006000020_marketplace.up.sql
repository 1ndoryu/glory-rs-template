-- [03AA-3 M3] Tablas del asistente Marketplace: tokens mp, cubo por minuto y auditoria.
CREATE TABLE mp_tokens_emitidos (
    jti TEXT PRIMARY KEY,
    sub TEXT NOT NULL,
    emitida_en TIMESTAMPTZ NOT NULL DEFAULT now(),
    expira_en TIMESTAMPTZ NOT NULL,
    revocada BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE mp_uso_minuto (
    clave TEXT NOT NULL,
    ventana TIMESTAMPTZ NOT NULL,
    n INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (clave, ventana)
);

CREATE TABLE mp_auditoria (
    id BIGSERIAL PRIMARY KEY,
    hilo_hmac TEXT NOT NULL,
    ts_hora TIMESTAMPTZ NOT NULL,
    evento TEXT NOT NULL CHECK (evento IN ('hit', 'miss', 'copiar', 'regenerar', 'emision'))
);
CREATE INDEX mp_auditoria_hora_idx ON mp_auditoria (ts_hora);
