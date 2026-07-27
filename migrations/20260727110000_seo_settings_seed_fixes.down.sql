/* [277A-15] Revert seed data fixes. No-op: los datos originales se pierden
 * pero no afectan funcionalidad (el audit simplemente mostraría warnings). */
UPDATE seo_settings SET og_image_url = NULL;
UPDATE seo_settings SET title = 'Blog — Nakomi Studio' WHERE path = '/blog';
UPDATE seo_settings SET json_ld_type = NULL WHERE path = '/blog';
UPDATE seo_settings SET title = 'Contacto — Nakomi Studio' WHERE path = '/contacto';
UPDATE seo_settings SET json_ld_type = NULL WHERE path = '/contacto';
UPDATE seo_settings SET description = 'Política de privacidad y protección de datos de Nakomi Studio.' WHERE path = '/politica-privacidad';
