CREATE TABLE seo_settings (
    path VARCHAR(255) PRIMARY KEY,
    label VARCHAR(255) NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    og_image_url VARCHAR(500),
    json_ld_type VARCHAR(100),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE seo_settings IS 'SEO metadata for static pages, editable from admin panel';

INSERT INTO seo_settings (path, label, title, description, json_ld_type) VALUES
('/', 'Inicio', 'Nakomi Studio — Agencia Creativa Digital', 'Estudio creativo basado en Copenhague. Diseño web, apps e IA construidos con Rust para rendimiento real. Operamos en español, inglés y japonés.', 'Organization+WebSite'),
('/servicios', 'Servicios', 'Nuestros Servicios de Desarrollo Web y Diseño — Nakomi Studio', 'Servicios de desarrollo web, diseño UI/UX, branding y soluciones digitales a medida.', 'Organization'),
('/proyectos', 'Proyectos', 'Nuestros Proyectos y Casos de Éxito — Nakomi Studio', 'Explora nuestros proyectos y casos de éxito en desarrollo web, diseño digital y soluciones de software.', 'Organization'),
('/nosotros', 'Nosotros', 'Sobre Nosotros — Nakomi Studio', 'Conoce al equipo de Nakomi Studio: diseñadores y desarrolladores basados en Copenhague, especializados en web, apps e IA.', 'BreadcrumbList+Person'),
('/blog', 'Blog', 'Blog — Nakomi Studio', 'Artículos sobre desarrollo web, diseño, tecnología e inteligencia artificial.', NULL),
('/soluciones/hosting', 'Hosting', 'Hosting Administrado — Nakomi Studio', 'Hosting web administrado con SSL, backups automáticos y soporte técnico.', 'FAQPage'),
('/soluciones/hosting-wordpress', 'Hosting WordPress', 'Hosting WordPress — Nakomi Studio', 'WordPress hosting optimizado con WP-CLI, backups automáticos y soporte experto.', 'FAQPage'),
('/soluciones/vps', 'VPS', 'Servidores VPS — Nakomi Studio', 'Servidores VPS dedicados con acceso root, bootstrap inicial y precios transparentes.', 'FAQPage'),
('/contacto', 'Contacto', 'Contacto — Nakomi Studio', 'Contacta con Nakomi Studio para tu proyecto web, app o solución digital.', NULL),
('/politica-privacidad', 'Privacidad', 'Política de Privacidad — Nakomi Studio', 'Política de privacidad y protección de datos de Nakomi Studio.', NULL);
