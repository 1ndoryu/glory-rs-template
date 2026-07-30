use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::product::{CreateProductRequest, Product};
use crate::models::resource::{CreateResourceParams, EditorialState, ResourceKind, VisibilityState};
use crate::repositories::product_repo::ProductRepository;
use crate::repositories::resource_repo::ResourceRepository;

pub struct ProductService;

impl ProductService {
    /// [297A-10] Crear producto con resource envelope en transacción. Defaults: draft, private, inactive.
    pub async fn create(pool: &PgPool, req: CreateProductRequest) -> Result<Product, AppError> {
        let id = uuid::Uuid::new_v4();

        let mut tx = pool.begin().await?;

        /* 1. Insertar resource envelope (producto: draft + private por defecto) */
        ResourceRepository::create(
            &mut *tx,
            CreateResourceParams {
                id,
                kind: ResourceKind::Product,
                title: &req.name,
                editorial: EditorialState::Draft,
                visibility: VisibilityState::Private,
            },
        )
        .await?;

        /* 2. Insertar producto */
        let product = ProductRepository::create(
            &mut *tx,
            id,
            req.article_id,
            &req.name,
            &req.description,
            req.price_cents,
            &req.currency,
            req.download_path.as_deref(),
        )
        .await?;

        tx.commit().await?;
        Ok(product)
    }

    pub async fn get(pool: &PgPool, id: Uuid) -> Result<Product, AppError> {
        ProductRepository::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Producto no encontrado".into()))
    }

    /// Listar productos activos de un artículo (público)
    pub async fn list_by_article(
        pool: &PgPool,
        article_id: Uuid,
    ) -> Result<Vec<Product>, AppError> {
        let products = ProductRepository::find_by_article(pool, article_id).await?;
        /* [297A-7] Solo exponer productos activos al público */
        Ok(products.into_iter().filter(|p| p.is_active).collect())
    }

    /// Listar todos los productos de un artículo (admin)
    pub async fn list_by_article_admin(
        pool: &PgPool,
        article_id: Uuid,
    ) -> Result<Vec<Product>, AppError> {
        Ok(ProductRepository::find_by_article(pool, article_id).await?)
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        if !ProductRepository::delete(pool, id).await? {
            return Err(AppError::NotFound("Producto no encontrado".into()));
        }
        Ok(())
    }
}
