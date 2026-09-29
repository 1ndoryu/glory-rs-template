-- E14 (2026-09-29): la captación (el visitante quiere VENDER o ALQUILAR
-- su propiedad) merece estado propio en la máquina de atención: no es una
-- duda (`consultando`, la IA espera y retoma) ni una delegación total
-- (`delegada`, triple freno). En `captacion` la IA confirma y sigue
-- disponible; el captador contacta por teléfono fuera del chat.

ALTER TABLE atencion_sesiones DROP CONSTRAINT IF EXISTS atencion_sesiones_estado_check;
ALTER TABLE atencion_sesiones
  ADD CONSTRAINT atencion_sesiones_estado_check
  CHECK (estado = ANY (ARRAY['activa'::text, 'consultando'::text, 'delegada'::text, 'captacion'::text]));
