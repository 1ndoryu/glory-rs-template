/* [09AA-24] Revierte los nombres alternativos del inmueble. */
ALTER TABLE inmuebles
  DROP COLUMN IF EXISTS alias_titulos;
