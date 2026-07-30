use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::product::{CreateProductRequest, Product};
use crate::repositories::product_repo::ProductRepository;

pub struct ProductService;

impl ProductService {
    pub async fn create(pool: &PgPool, req: CreateProductRequest) -> Result<Product, AppError> {
        let product = ProductRepository::create(
            pool,
            req.article_id,
            &req.name,
            &req.description,
            req.price_cents,
            &req.currency,
            req.download_path.as_deref(),
        )
        .await?;
        Ok(product)
    }

    pub async fn get(pool: &PgPool, id: Uuid) -> Result<Product, AppError> {
        ProductRepository::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Producto no encontrado".into()))
    }

    pub async fn list_by_article(pool: &PgPool, article_id: Uuid) -> Result<Vec<Product>, AppError> {
        Ok(ProductRepository::find_by_article(pool, article_id).await?)
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        if !ProductRepository::delete(pool, id).await? {
            return Err(AppError::NotFound("Producto no encontrado".into()));
        }
        Ok(())
    }
}
