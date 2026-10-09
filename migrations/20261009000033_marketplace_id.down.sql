/* [09AA-21] Revierte el vínculo exacto con el aviso. */
ALTER TABLE inmuebles
  DROP COLUMN IF EXISTS marketplace_id;
