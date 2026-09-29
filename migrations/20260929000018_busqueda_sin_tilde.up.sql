-- Fase3-v2 (2026-09-29): `sencilla(TEXT)` para búsqueda sin tildes ni
-- mayúsculas en `buscar_inmuebles` (`texto`/`zona`). El visitante escribe
-- "Caroni" y la BD guarda "Caroní": el ILIKE directo daba 0 filas y la IA
-- negaba oferta existente (F3: local de Riberas). IMMUTABLE para poder
-- indexar/expresiones; solo minúsculas ASCII + tildes es-ES en la lista.

CREATE OR REPLACE FUNCTION sencilla(t TEXT) RETURNS TEXT AS $$
  SELECT translate(lower(t), 'áéíóúüñ', 'aeiouun')
$$ LANGUAGE sql IMMUTABLE;
