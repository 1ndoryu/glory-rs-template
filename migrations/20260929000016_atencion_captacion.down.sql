-- E14 rollback: las filas en `captacion` vuelven a `delegada`
-- (el humano las retoma igual) antes de restaurar el CHECK original.

UPDATE atencion_sesiones SET estado = 'delegada' WHERE estado = 'captacion';
ALTER TABLE atencion_sesiones DROP CONSTRAINT IF EXISTS atencion_sesiones_estado_check;
ALTER TABLE atencion_sesiones
  ADD CONSTRAINT atencion_sesiones_estado_check
  CHECK (estado = ANY (ARRAY['activa'::text, 'consultando'::text, 'delegada'::text]));
