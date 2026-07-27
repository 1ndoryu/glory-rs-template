/* [277A-15] Corregir datos sembrados de seo_settings: resolver los 10 issues del audit.
 * - og_image_url: poner imagen default para que el audit no flaggea "default og image"
 * - titles: Blog y Contacto por encima de 30 chars mínimo
 * - json_ld_type: Blog y Contacto ya no son NULL
 * - description: Privacidad por encima de 70 chars mínimo */
UPDATE seo_settings SET
    og_image_url = '/assets/Proyectos%20portadas/Kamples%20portada.jpg'
WHERE og_image_url IS NULL;

UPDATE seo_settings SET
    title = 'Blog de Desarrollo Web y Tecnología — Nakomi Studio'
WHERE path = '/blog';

UPDATE seo_settings SET
    json_ld_type = 'CollectionPage'
WHERE path = '/blog';

UPDATE seo_settings SET
    title = 'Contacta con Nosotros — Nakomi Studio'
WHERE path = '/contacto';

UPDATE seo_settings SET
    json_ld_type = 'ContactPage'
WHERE path = '/contacto';

UPDATE seo_settings SET
    description = 'Política de privacidad y protección de datos de Nakomi Studio. Información sobre cómo recopilamos, usamos y protegemos tu información personal.'
WHERE path = '/politica-privacidad';
