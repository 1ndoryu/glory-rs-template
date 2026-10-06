/* [03AA-3 M4] Caché de respuestas del asistente Marketplace.
 * Clave (firma, precio_hash, catalog_hash): la misma pregunta sobre el mismo
 * catálogo reutiliza la respuesta sin gastar IA. `corregida` marca texto de
 * la dueña (gana sobre generaciones futuras). `valida_hasta` default +90d
 * (retención revisable 2026-10-05); la purga borra lo vencido. */
CREATE TABLE mp_respuestas_cache (
    firma TEXT NOT NULL,
    precio_hash TEXT NOT NULL,
    catalog_hash TEXT NOT NULL,
    respuesta TEXT NOT NULL,
    valida_hasta TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '90 days',
    usos INTEGER NOT NULL DEFAULT 0,
    corregida BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (firma, precio_hash, catalog_hash)
);
