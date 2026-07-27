/* [S7-H1] Prevenir que dos ventas locales mapeen al mismo BDP OrderId.
 * Índice parcial UNIQUE: solo aplica cuando bdp_order_id IS NOT NULL.
 * El advisory lock distribuido ya mitiga la carrera, pero esto es la
 * garantía dura a nivel de base de datos. */
CREATE UNIQUE INDEX IF NOT EXISTS idx_ventas_bdp_order_id_unique
    ON ventas(user_id, bdp_order_id)
    WHERE bdp_order_id IS NOT NULL;

/* [S7-H3] Prevenir intentos de facturar dos veces la misma orden en BDP.
 * Índice parcial UNIQUE: una orden BDP solo puede facturarse una vez por usuario.
 * La protección principal es el status check en BDP antes de InvoiceOrder,
 * pero esto añade una capa defensiva adicional a nivel de BD. */
CREATE UNIQUE INDEX IF NOT EXISTS idx_ventas_bdp_invoiced_order_unique
    ON ventas(user_id, bdp_order_id)
    WHERE bdp_invoiced = TRUE AND bdp_order_id IS NOT NULL;
