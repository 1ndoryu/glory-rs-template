-- E15 (2026-09-29): visitas que agenda la IA por WhatsApp. La tool
-- `agendar_visita` deja la fila en `pendiente` (el agente confirma día y
-- hora con el visitante) y congela la IA en `consultando` hasta que el
-- humano responde; el admin confirma/cancela con fecha desde el panel.
-- `cuando` guarda lo que dijo el visitante ("el sábado en la mañana");
-- `fecha` solo se fija al confirmar (NULL = aún sin día cerrado).

CREATE TABLE visitas (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  inmueble_id UUID NOT NULL REFERENCES inmuebles(id) ON DELETE CASCADE,
  session_id UUID NOT NULL,
  nombre TEXT NOT NULL CHECK (char_length(nombre) BETWEEN 1 AND 200),
  telefono TEXT NOT NULL CHECK (char_length(telefono) BETWEEN 6 AND 40),
  cuando TEXT NOT NULL CHECK (char_length(cuando) BETWEEN 1 AND 200),
  fecha DATE NULL,
  estado TEXT NOT NULL DEFAULT 'pendiente'
    CHECK (estado IN ('pendiente', 'confirmada', 'cancelada')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_visitas_estado ON visitas(estado);
CREATE INDEX idx_visitas_inmueble ON visitas(inmueble_id);
