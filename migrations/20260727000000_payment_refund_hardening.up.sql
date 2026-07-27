/* [277A-7] Endurecimiento de pagos y reembolsos (Fase F+G).
 * - Añade 'processing' y 'failed' al enum refund_status
 * - Añade campos de retry a order_refunds (attempts, max_attempts, next_retry_at)
 * - Constraint único parcial: un solo pago activo (pending/held) por orden
 * - Columna idempotency_key en order_payments para idempotencia Stripe */

/* 1. Expandir refund_status con nuevos estados */
ALTER TYPE refund_status ADD VALUE IF NOT EXISTS 'processing';
ALTER TYPE refund_status ADD VALUE IF NOT EXISTS 'failed';

/* 2. Campos de retry en order_refunds + updated_at para detectar stuck processing */
ALTER TABLE order_refunds ADD COLUMN IF NOT EXISTS attempts INT NOT NULL DEFAULT 0;
ALTER TABLE order_refunds ADD COLUMN IF NOT EXISTS max_attempts INT NOT NULL DEFAULT 3;
ALTER TABLE order_refunds ADD COLUMN IF NOT EXISTS next_retry_at TIMESTAMPTZ;
ALTER TABLE order_refunds ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

/* Trigger para actualizar updated_at automáticamente */
CREATE OR REPLACE FUNCTION update_order_refunds_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_order_refunds_updated_at ON order_refunds;
CREATE TRIGGER trigger_order_refunds_updated_at
    BEFORE UPDATE ON order_refunds
    FOR EACH ROW
    EXECUTE FUNCTION update_order_refunds_timestamp();

/* 3. Constraint único parcial: solo un pago activo por orden */
CREATE UNIQUE INDEX IF NOT EXISTS unique_active_payment_per_order
    ON order_payments (order_id)
    WHERE status IN ('pending', 'held');

/* 4. Columna idempotency_key para rastrear llamadas Stripe */
ALTER TABLE order_payments ADD COLUMN IF NOT EXISTS idempotency_key VARCHAR(100);
