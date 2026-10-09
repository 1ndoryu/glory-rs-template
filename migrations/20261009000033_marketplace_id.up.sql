/* [09AA-21] Vínculo exacto con el aviso de Marketplace (`/marketplace/item/<id>`).
 * `marketplace_id` = dígitos del aviso (`NULL` = sin vincular). UNIQUE para que
 * dos fichas no reclamen el mismo aviso; NULL múltiples permitidos (Postgres
 * no considera iguales dos NULL). El emparejado por título sigue como fallback
 * (`ficha_por_titulo`); esta columna es la rama prioritaria en `claves_cache`. */
ALTER TABLE inmuebles
  ADD COLUMN marketplace_id TEXT UNIQUE;
