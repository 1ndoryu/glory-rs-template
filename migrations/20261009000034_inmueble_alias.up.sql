/* [09AA-24] Nombres alternativos del inmueble (`alias_titulos TEXT[]`).
 * Un mismo inmueble puede publicarse con varios nombres ("Residencias
 * Caroní Plaza" = "Residencias Río Aro Plaza"): el emparejado por título
 * (`ficha_por_titulo` + badge del panel) puntúa el título y cada alias.
 * Array vacío = sin alias. Sin UNIQUE: dos fichas distintas podrían
 * compartir un alias por error y el empate del emparejado ya devuelve
 * `None` en ese caso (nunca se cita un precio dudoso). */
ALTER TABLE inmuebles
  ADD COLUMN alias_titulos TEXT[] NOT NULL DEFAULT '{}';
