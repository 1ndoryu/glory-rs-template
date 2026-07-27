/* [S7-H1] Revertir índices únicos BDP */
DROP INDEX IF EXISTS idx_ventas_bdp_order_id_unique;
DROP INDEX IF EXISTS idx_ventas_bdp_invoiced_order_unique;
