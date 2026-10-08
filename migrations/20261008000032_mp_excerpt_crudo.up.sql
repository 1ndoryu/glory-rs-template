/* [08AA-21] Guarda el excerpt tal como llegó del puente, junto al limpio.
 * El filtro por líneas (`normalizar_excerpt`) no se puede calibrar a ciegas:
 * el puente aplana el DOM a texto plano y lo que llega pegado
 * (`OrdazDetalles`, `Mensaje enviado 3:18 pm por: Wilmery`) no se ve en
 * ningún lado. Con el crudo a la vista se corrige el filtro (08AA-8).
 * PII solo-admin, misma retención 90d + purga; NULL en filas viejas. */
ALTER TABLE mp_respuestas_cache
  ADD COLUMN excerpt_crudo TEXT;
