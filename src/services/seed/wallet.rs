/* sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro: seed usa runtime queries
 * porque ejecuta SQL dinámico con datos de prueba y tipos genéricos. */
//! Seed de wallet: balances, historial y solicitudes de retiro de prueba.

use sqlx::PgPool;
use uuid::Uuid;

use super::SeedService;

async fn upsert_seed_wallet(
    pool: &PgPool,
    user_id: Uuid,
    balance_cents: i32,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO user_wallets (user_id, balance_cents, currency)
         VALUES ($1, $2, 'USD')
         ON CONFLICT (user_id) DO UPDATE SET balance_cents = $2
         RETURNING id",
    )
    .bind(user_id)
    .bind(balance_cents)
    .fetch_one(pool)
    .await
}

async fn insert_wallet_history(
    pool: &PgPool,
    wallet_id: Uuid,
    user_id: Uuid,
    transactions: &[(i32, &str, &str)],
) -> Result<(), sqlx::Error> {
    let mut balance_after = 0i32;
    for (amount, tx_type, description) in transactions {
        balance_after += amount;
        sqlx::query(
            "INSERT INTO wallet_transactions (wallet_id, user_id, amount_cents, transaction_type, description, balance_after_cents)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(wallet_id)
        .bind(user_id)
        .bind(amount)
        .bind(tx_type)
        .bind(description)
        .bind(balance_after)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn insert_seed_withdrawal_request(
    pool: &PgPool,
    user_id: Uuid,
    amount_cents: i32,
    status: &str,
    payment_method: &str,
    payment_details: &str,
    admin_notes: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO withdrawal_requests (user_id, amount_cents, status, payment_method, payment_details, admin_notes)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(user_id)
    .bind(amount_cents)
    .bind(status)
    .bind(payment_method)
    .bind(payment_details)
    .bind(admin_notes)
    .execute(pool)
    .await?;
    Ok(())
}

impl SeedService {
    /* [045A-1] Limpieza puntual de wallet para la cuenta que ejecuta el seed.
     * No tocamos sus notificaciones ni otras tablas ajenas al problema del panel wallet. */
    async fn delete_wallet_seed_for_user(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM withdrawal_requests WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM wallet_transactions WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM user_wallets WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /* [025B-2] Crea wallet con balance de prueba + withdrawal_requests en diferentes estados.
     * El cliente tiene saldo de $120 (liberado por órdenes completadas).
     * El empleado tiene saldo de $38.40 (80% comisión de orden completada).
     * Retornamos (balance_cents_cliente, num_withdrawals). */
    #[allow(clippy::too_many_lines)]
    async fn create_seed_wallet(
        pool: &PgPool,
        client_id: Uuid,
        employee_id: Uuid,
        seed_requester_id: Uuid,
    ) -> Result<(i32, u32), sqlx::Error> {
        /* Wallet cliente: $120.00 (pagos devueltos / créditos demo) */
        let client_wallet_id = upsert_seed_wallet(pool, client_id, 12000).await?;

        /* Wallet empleado: $38.40 (comisión 80% de orden completada $48) */
        let employee_wallet_id = upsert_seed_wallet(pool, employee_id, 3840).await?;

        /* Historial de transacciones del cliente */
        let client_txs: &[(i32, &str, &str)] = &[
            (15000, "credit", "Crédito inicial de bienvenida"),
            (-3000, "debit", "Pago parcial — Diseño Web Básico fase 1"),
            (3000, "refund", "Reembolso aprobado — Agentes IA parcial"),
            (-3000, "withdrawal", "Retiro procesado vía PayPal"),
        ];
        insert_wallet_history(pool, client_wallet_id, client_id, client_txs).await?;

        /* Historial del empleado */
        let emp_txs: &[(i32, &str, &str)] = &[
            (
                4800,
                "commission",
                "Comisión 80% — Agentes IA Básico completado",
            ),
            (
                -960,
                "withdrawal",
                "Retiro procesado vía transferencia bancaria",
            ),
        ];
        insert_wallet_history(pool, employee_wallet_id, employee_id, emp_txs).await?;

        /* Withdrawal requests del cliente: pending + approved + rejected */
        let withdrawals: &[(&str, i32, &str, &str, Option<&str>)] = &[
            (
                "pending",
                5000,
                "PayPal",
                "nakomi_cliente@gmail.com",
                Some("Retiro mensual de saldo acumulado"),
            ),
            (
                "approved",
                3000,
                "bank",
                "ES12 3456 7890 0123 4567",
                Some("Retiro por transferencia SEPA"),
            ),
            (
                "rejected",
                8000,
                "crypto",
                "3FZbgi29cpjq2GjdwV8eyHuJJnkLtktZc5",
                Some("Método de pago no soportado actualmente."),
            ),
        ];
        let mut withdrawal_count = 0u32;
        for (status, amount, method, details, admin_notes) in withdrawals {
            insert_seed_withdrawal_request(
                pool,
                client_id,
                *amount,
                status,
                method,
                details,
                *admin_notes,
            )
            .await?;
            withdrawal_count += 1;
        }

        /* Withdrawal request del empleado: pending */
        insert_seed_withdrawal_request(
            pool,
            employee_id,
            3840,
            "pending",
            "bank",
            "DE89 3704 0044 0532 0130 00",
            None,
        )
        .await?;
        withdrawal_count += 1;

        /* [045A-1] También sembramos la cuenta que ejecuta el seed, salvo que ya sea uno
         * de los usuarios demo anteriores, para que el panel wallet muestre datos reales
         * sin obligar a iniciar sesión con `cliente@test.com`. */
        if seed_requester_id != client_id && seed_requester_id != employee_id {
            let requester_wallet_id = upsert_seed_wallet(pool, seed_requester_id, 25000).await?;
            let requester_txs: &[(i32, &str, &str)] = &[
                (30000, "commission", "Comisión plataforma — Mayo 2026"),
                (-5000, "withdrawal", "Retiro procesado vía PayPal"),
            ];
            insert_wallet_history(pool, requester_wallet_id, seed_requester_id, requester_txs)
                .await?;
            insert_seed_withdrawal_request(
                pool,
                seed_requester_id,
                10000,
                "pending",
                "bank",
                "ES89 3704 0044 0532 0130 00",
                Some("Retiro mensual de comisiones"),
            )
            .await?;
            withdrawal_count += 1;
        }

        Ok((12000, withdrawal_count))
    }
}
