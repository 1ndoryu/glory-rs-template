use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AdminUser;
use crate::models::article::{
    Article, ArticleQueryParams, CreateArticleRequest, PaginatedArticles, UpdateArticleRequest,
};
use crate::services::article::ArticleService;
use crate::AppState;

/// Crear un articulo (admin)
#[utoipa::path(
    post,
    path = "/api/articles",
    request_body = CreateArticleRequest,
    responses(
        (status = 201, description = "Articulo creado", body = Article),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validacion", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_article(
    State(state): State<AppState>,
    _auth: AdminUser,
    Json(req): Json<CreateArticleRequest>,
) -> Result<(StatusCode, Json<Article>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let article = ArticleService::create(&state.pool, req).await?;
    Ok((StatusCode::CREATED, Json(article)))
}

/// Obtener articulo por ID (admin — incluye borradores)
#[utoipa::path(
    get,
    path = "/api/articles/{id}",
    params(("id" = Uuid, Path, description = "ID del articulo")),
    responses(
        (status = 200, description = "Articulo encontrado", body = Article),
        (status = 404, description = "No encontrado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_article(
    State(state): State<AppState>,
    _auth: AdminUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Article>, AppError> {
    let article = ArticleService::get(&state.pool, id).await?;
    Ok(Json(article))
}

/// Obtener articulo por slug (publico)
#[utoipa::path(
    get,
    path = "/api/articles/slug/{slug}",
    params(("slug" = String, Path, description = "Slug del articulo")),
    responses(
        (status = 200, description = "Articulo encontrado", body = Article),
        (status = 404, description = "No encontrado", body = crate::errors::ErrorResponse)
    )
)]
pub async fn get_article_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Article>, AppError> {
    let article = ArticleService::get_by_slug(&state.pool, &slug).await?;
    Ok(Json(article))
}

/// Listar articulos publicados (publico)
/// [297A-7] Solo artículos con status='published'
#[utoipa::path(
    get,
    path = "/api/articles",
    params(ArticleQueryParams),
    responses(
        (status = 200, description = "Lista de articulos", body = PaginatedArticles)
    )
)]
pub async fn list_articles(
    State(state): State<AppState>,
    Query(params): Query<ArticleQueryParams>,
) -> Result<Json<PaginatedArticles>, AppError> {
    let articles =
        ArticleService::list(&state.pool, Some("published"), params.page, params.per_page).await?;
    Ok(Json(articles))
}

/// Listar todos los articulos incluyendo borradores (admin)
#[utoipa::path(
    get,
    path = "/api/admin/articles",
    params(ArticleQueryParams),
    responses(
        (status = 200, description = "Lista de articulos", body = PaginatedArticles)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_articles_admin(
    State(state): State<AppState>,
    _auth: AdminUser,
    Query(params): Query<ArticleQueryParams>,
) -> Result<Json<PaginatedArticles>, AppError> {
    let articles = ArticleService::list(
        &state.pool,
        params.status.as_deref(),
        params.page,
        params.per_page,
    )
    .await?;
    Ok(Json(articles))
}

/// Actualizar articulo (admin)
#[utoipa::path(
    put,
    path = "/api/articles/{id}",
    params(("id" = Uuid, Path, description = "ID del articulo")),
    request_body = UpdateArticleRequest,
    responses(
        (status = 200, description = "Articulo actualizado", body = Article),
        (status = 404, description = "No encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_article(
    State(state): State<AppState>,
    _auth: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateArticleRequest>,
) -> Result<Json<Article>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let article = ArticleService::update(&state.pool, id, req).await?;
    Ok(Json(article))
}

/// Eliminar articulo (admin)
#[utoipa::path(
    delete,
    path = "/api/articles/{id}",
    params(("id" = Uuid, Path, description = "ID del articulo")),
    responses(
        (status = 204, description = "Articulo eliminado"),
        (status = 404, description = "No encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_article(
    State(state): State<AppState>,
    _auth: AdminUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    ArticleService::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        /* Públicos: solo artículos publicados */
        .route("/articles", get(list_articles))
        .route("/articles/slug/{slug}", get(get_article_by_slug))
        /* Admin: CRUD completo */
        .route(
            "/admin/articles",
            post(create_article).get(list_articles_admin),
        )
        .route(
            "/admin/articles/{id}",
            get(get_article).put(update_article).delete(delete_article),
        )
}
