use sqlx::PgPool;
use uuid::Uuid;

use crate::models::product::{Order, Product};

pub struct ProductRepository;

impl ProductRepository {
    /// [297A-10] Crear producto dentro de una transacción. article_id es opcional.
    pub async fn create(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
        article_id: Option<Uuid>,
        name: &str,
        description: &str,
        price_cents: i32,
        currency: &str,
        download_path: Option<&str>,
    ) -> Result<Product, sqlx::Error> {
        sqlx::query_as::<_, Product>(
            "INSERT INTO products (id, article_id, name, description, price_cents, currency, download_path) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             RETURNING id, article_id, name, description, price_cents, currency, stripe_product_id, stripe_price_id, download_path, is_active, created_at",
        )
        .bind(id)
        .bind(article_id)
        .bind(name)
        .bind(description)
        .bind(price_cents)
        .bind(currency)
        .bind(download_path)
        .fetch_one(&mut *conn)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Product>, sqlx::Error> {
        sqlx::query_as::<_, Product>(
            "SELECT id, article_id, name, description, price_cents, currency, stripe_product_id, stripe_price_id, download_path, is_active, created_at \
             FROM products WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_article(
        pool: &PgPool,
        article_id: Uuid,
    ) -> Result<Vec<Product>, sqlx::Error> {
        sqlx::query_as::<_, Product>(
            "SELECT id, article_id, name, description, price_cents, currency, stripe_product_id, stripe_price_id, download_path, is_active, created_at \
             FROM products WHERE article_id = $1 ORDER BY created_at DESC",
        )
        .bind(article_id)
        .fetch_all(pool)
        .await
    }

    pub async fn update_stripe_ids(
        pool: &PgPool,
        id: Uuid,
        stripe_product_id: &str,
        stripe_price_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE products SET stripe_product_id = $1, stripe_price_id = $2 WHERE id = $3",
        )
        .bind(stripe_product_id)
        .bind(stripe_price_id)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM products WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

pub struct OrderRepository;

impl OrderRepository {
    pub async fn create(
        pool: &PgPool,
        product_id: Uuid,
        customer_email: &str,
        stripe_session_id: Option<&str>,
    ) -> Result<Order, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Order>(
            "INSERT INTO orders (id, product_id, customer_email, stripe_session_id) \
             VALUES ($1, $2, $3, $4) \
             RETURNING id, product_id, stripe_session_id, stripe_payment_intent, customer_email, status, paid_at, delivered_at, created_at",
        )
        .bind(id)
        .bind(product_id)
        .bind(customer_email)
        .bind(stripe_session_id)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_session(
        pool: &PgPool,
        session_id: &str,
    ) -> Result<Option<Order>, sqlx::Error> {
        sqlx::query_as::<_, Order>(
            "SELECT id, product_id, stripe_session_id, stripe_payment_intent, customer_email, status, paid_at, delivered_at, created_at \
             FROM orders WHERE stripe_session_id = $1",
        )
        .bind(session_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn mark_paid(
        pool: &PgPool,
        id: Uuid,
        payment_intent: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE orders SET status = 'paid', paid_at = NOW(), stripe_payment_intent = $1 WHERE id = $2",
        )
        .bind(payment_intent)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn mark_delivered(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE orders SET status = 'delivered', delivered_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Marcar orden como fallida (webhook expirado)
    pub async fn mark_failed(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE orders SET status = 'failed' WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Buscar orden por ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Order>, sqlx::Error> {
        sqlx::query_as::<_, Order>(
            "SELECT id, product_id, stripe_session_id, stripe_payment_intent, \
             customer_email, status, paid_at, delivered_at, created_at \
             FROM orders WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Actualizar `stripe_session_id` de una orden
    pub async fn update_stripe_session(
        pool: &PgPool,
        id: Uuid,
        stripe_session_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE orders SET stripe_session_id = $1 WHERE id = $2")
            .bind(stripe_session_id)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
