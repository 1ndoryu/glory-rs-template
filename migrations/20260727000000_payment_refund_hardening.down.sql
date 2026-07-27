/* Rollback [277A-7] */
DROP INDEX IF EXISTS unique_active_payment_per_order;
ALTER TABLE order_payments DROP COLUMN IF EXISTS idempotency_key;
DROP TRIGGER IF EXISTS trigger_order_refunds_updated_at ON order_refunds;
DROP FUNCTION IF EXISTS update_order_refunds_timestamp();
ALTER TABLE order_refunds DROP COLUMN IF EXISTS updated_at;
ALTER TABLE order_refunds DROP COLUMN IF EXISTS next_retry_at;
ALTER TABLE order_refunds DROP COLUMN IF EXISTS max_attempts;
ALTER TABLE order_refunds DROP COLUMN IF EXISTS attempts;
/* Nota: no se puede eliminar valores de un enum en PostgreSQL sin recrear el tipo.
 * Si se necesita rollback completo, recrear refund_status sin processing/failed. */
