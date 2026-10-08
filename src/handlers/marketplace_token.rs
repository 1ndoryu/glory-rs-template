//! [08AA-8] Token mp del asistente Marketplace (emisión panel + CLI).
//!
//! Extraído de `marketplace.rs` (límite 500): extractor `MpAuth` (JWT mp +
//! binding `X-MP-Maquina` para CLI), 429 `limite` con `Retry-After` y las
//! dos rutas de emisión (`emitir_token` panel 15 min, `emitir_token_cli`
//! 8h atado a máquina). `marketplace.rs` conserva borrador/regenerar/
//! releer/corregir/audit/uso/chats y re-exporta estos nombres para que las
//! rutas utoipa (`marketplace::emitir_token`…) sigan resolviendo.

use axum::async_trait;
use axum::extract::{FromRequestParts, State};
use axum::http::header::RETRY_AFTER;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::marketplace::{consumir_minuto, registrar_token, MpClaims};
use crate::AppState;

const ISS: &str = "mn-backend";
const AUD: &str = "mp";
const SCOPE: &str = "mp:borrador";
const TOKEN_MINUTOS: i64 = 15;
const TOPE_TOKEN_MINUTO: i64 = 5;

/// JWT mp verificado: `iss mn-backend`, `aud mp`, `scope mp:borrador`, sin
/// expirar y con `jti` conocido y no revocado.
pub struct MpAuth {
    pub sub: String,
}

#[async_trait]
impl FromRequestParts<AppState> for MpAuth {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;
        let mut validacion = Validation::new(jsonwebtoken::Algorithm::HS256);
        validacion.set_issuer(&[ISS]);
        validacion.set_audience(&[AUD]);
        let claims = decode::<MpClaims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &validacion,
        )
        .map(|d| d.claims)
        .map_err(|_| AppError::Unauthorized)?;
        if claims.scope != SCOPE {
            return Err(AppError::Forbidden("alcance insuficiente".to_string()));
        }
        let vigente: Option<bool> = sqlx::query_scalar(
            "SELECT NOT revocada AND expira_en > now() FROM mp_tokens_emitidos WHERE jti = $1",
        )
        .bind(&claims.jti)
        .fetch_optional(&state.pool)
        .await?;
        if vigente != Some(true) {
            return Err(AppError::Unauthorized);
        }
        /* E3: token CLI atado a máquina: exige `X-MP-Maquina` igual al `mid`
         * del token. Sin `mid` (panel) no se pide nada. Fallo = 401 seco,
         * sin decir si fue máquina o token (sin oráculo). */
        let maquina = parts
            .headers
            .get("x-mp-maquina")
            .and_then(|v| v.to_str().ok());
        if !crate::services::marketplace::maquina_autorizada(claims.mid.as_deref(), maquina) {
            return Err(AppError::Unauthorized);
        }
        Ok(Self { sub: claims.sub })
    }
}

/// 429 con `Retry-After` (no cabe en `AppError`, que no lleva headers).
pub(super) fn limite(reintento_segs: u64) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(RETRY_AFTER, reintento_segs.to_string())],
        Json(serde_json::json!({
            "error": "limite_excedido",
            "message": "Demasiadas peticiones; reintenta en un minuto",
        })),
    )
        .into_response()
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TokenResponse {
    pub token: String,
    pub expira_en_minutos: i64,
}

/// Emite el JWT mp de panel (`exp` 15 min, sin binding). Tope 5/min por admin para que un bucle no
/// fabrique tokens sin parar.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/token",
    responses(
        (status = 201, description = "Token mp emitido", body = TokenResponse),
        (status = 429, description = "Tope de emisión", body = crate::errors::ErrorResponse)
    )
)]
pub async fn emitir_token(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Response, AppError> {
    let clave = format!("tok:{}", auth.user_id);
    if !consumir_minuto(&state.pool, &clave, TOPE_TOKEN_MINUTO).await? {
        return Ok(limite(60));
    }
    let expira = Utc::now() + chrono::Duration::minutes(TOKEN_MINUTOS);
    let sub = auth.user_id.to_string();
    let jti = registrar_token(&state.pool, &sub, &expira).await?;
    let exp = usize::try_from(expira.timestamp())
        .map_err(|_| AppError::Internal("Timestamp fuera de rango".to_string()))?;
    let token = encode(
        &Header::default(),
        &MpClaims {
            iss: ISS.to_string(),
            sub,
            aud: AUD.to_string(),
            scope: SCOPE.to_string(),
            exp,
            jti,
            mid: None,
        },
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Error generando token mp: {e}")))?;
    Ok((
        StatusCode::CREATED,
        Json(TokenResponse {
            token,
            expira_en_minutos: TOKEN_MINUTOS,
        }),
    )
        .into_response())
}

/// [03AA-3 E3] Token CLI: `exp` 8h atado a máquina (`mid` = hash hex64 que el
/// CLI deriva localmente; el id real jamás viaja). `borrador`/`audit` con
/// este token exigen `X-MP-Maquina` igual o devuelven 401. Cubo propio
/// 5/min para que un bucle CLI no fabrique tokens sin parar.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CliTokenRequest {
    pub maquina_hash: String,
}

#[utoipa::path(
    post,
    path = "/api/admin/marketplace/token/cli",
    request_body = CliTokenRequest,
    responses(
        (status = 201, description = "Token CLI emitido (8h, atado a máquina)", body = TokenResponse),
        (status = 422, description = "maquina_hash inválido", body = crate::errors::ErrorResponse),
        (status = 429, description = "Tope de emisión", body = crate::errors::ErrorResponse)
    )
)]
pub async fn emitir_token_cli(
    State(state): State<AppState>,
    auth: AuthUser,
    r: Result<Json<CliTokenRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    use crate::services::marketplace::{maquina_valida, minutos_para_cli};
    let r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    let mid = r.maquina_hash.trim().to_string();
    if !maquina_valida(&mid) {
        return Err(AppError::Validation(
            "maquina_hash debe ser 64 hex (hash local, nunca el id en claro)".to_string(),
        ));
    }
    let clave = format!("tokcli:{}", auth.user_id);
    if !consumir_minuto(&state.pool, &clave, TOPE_TOKEN_MINUTO).await? {
        return Ok(limite(60));
    }
    let minutos = minutos_para_cli(true);
    let expira = Utc::now() + chrono::Duration::minutes(minutos);
    let sub = auth.user_id.to_string();
    let jti = registrar_token(&state.pool, &sub, &expira).await?;
    let exp = usize::try_from(expira.timestamp())
        .map_err(|_| AppError::Internal("Timestamp fuera de rango".to_string()))?;
    let token = encode(
        &Header::default(),
        &MpClaims {
            iss: ISS.to_string(),
            sub,
            aud: AUD.to_string(),
            scope: SCOPE.to_string(),
            exp,
            jti,
            mid: Some(mid),
        },
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Error generando token CLI: {e}")))?;
    Ok((
        StatusCode::CREATED,
        Json(TokenResponse {
            token,
            expira_en_minutos: minutos,
        }),
    )
        .into_response())
}
