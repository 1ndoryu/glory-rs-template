/* [03AA-3 M3] HTTP del asistente Marketplace (token, borrador, audit).
 * Tres endpoints: `token` (emite el JWT mp con el JWT admin), `borrador`
 * (genera el borrador con la IA y la ficha stripeada) y `audit`
 * (observabilidad con HMAC, nunca texto en claro). La lógica pura vive en
 * `services::marketplace`; aquí solo boundary HTTP + 429 con `Retry-After`. */

use axum::async_trait;
use axum::extract::{FromRequestParts, State};
use axum::http::header::RETRY_AFTER;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::repositories::InmuebleRepository;
use crate::services::marketplace::{
    consumir_minuto, matriz_negativa, registrar_token, strip_ficha_para_prompt, sub_exento,
    validar_borrador, BorradorRequest, MpClaims, FALLBACK_BORRADOR, MATRIZ_NEGATIVA_VERSION,
    STRIP_VERSION,
};
use crate::AppState;

const ISS: &str = "mn-backend";
const AUD: &str = "mp";
const SCOPE: &str = "mp:borrador";
const TOKEN_MINUTOS: i64 = 15;
const TOPE_TOKEN_MINUTO: i64 = 5;
const TOPE_BORRADOR_MINUTO: i64 = 30;

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
        Ok(Self { sub: claims.sub })
    }
}

/// 429 con `Retry-After` (no cabe en `AppError`, que no lleva headers).
fn limite(reintento_segs: u64) -> Response {
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

/// Emite el JWT mp (`exp` 15 min). Tope 5/min por admin para que un bucle no
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

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BorradorResponse {
    pub borrador: String,
    pub fuente: String,
    pub aviso_conocido: bool,
    pub firma_version: String,
    pub matriz_version: u8,
}

/// Genera el borrador: valida schema (422), resuelve la ficha por UUID y la
/// stripea (sin ficha → no se afirma precio), llama la IA y aplica la matriz
/// negativa (cualquier fallo → fallback exacto, nunca error seco).
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/borrador",
    request_body = BorradorRequest,
    responses(
        (status = 200, description = "Borrador listo (ia o reserva)", body = BorradorResponse),
        (status = 422, description = "Schema inválido", body = crate::errors::ErrorResponse),
        (status = 429, description = "Tope por sub", body = crate::errors::ErrorResponse)
    )
)]
pub async fn borrador(
    State(state): State<AppState>,
    auth: MpAuth,
    r: Result<Json<BorradorRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    if !sub_exento(&auth.sub)
        && !consumir_minuto(
            &state.pool,
            &format!("bor:{}", auth.sub),
            TOPE_BORRADOR_MINUTO,
        )
        .await?
    {
        return Ok(limite(60));
    }
    let r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    let errores = validar_borrador(&r);
    if !errores.is_empty() {
        return Err(AppError::Validation(errores.join("; ")));
    }
    /* El avisoId del plugin es opaco: solo un UUID de `inmuebles` enlaza
     * ficha (M2 formaliza el catálogo). Lo demás es ruta sin-ficha: el
     * prompt lo declara y la IA no afirma precio. */
    let seguro = match r.aviso_id.as_deref() {
        Some(a) => match Uuid::parse_str(a) {
            Ok(id) => match InmuebleRepository::find_by_id(&state.pool, id).await? {
                Some(f) => Some(strip_ficha_para_prompt(&f, STRIP_VERSION)?),
                None => None,
            },
            Err(_) => None,
        },
        None => None,
    };
    let (borrador, fuente) = generar_borrador(&state, &r, seguro.as_ref()).await;
    Ok((
        StatusCode::OK,
        Json(BorradorResponse {
            borrador,
            fuente,
            aviso_conocido: seguro.is_some(),
            firma_version: "firma-v1".to_string(),
            matriz_version: MATRIZ_NEGATIVA_VERSION,
        }),
    )
        .into_response())
}

async fn generar_borrador(
    _state: &AppState, // M4: aquí se consulta `mp_respuestas_cache` antes de llamar a la IA.
    r: &BorradorRequest,
    seguro: Option<&crate::services::marketplace::PromptSeguro>,
) -> (String, String) {
    let datos = seguro.map_or_else(
        || "SIN FICHA: no conoces el inmueble; no afirmes precio ni medidas.".to_string(),
        |s| serde_json::to_string(s).unwrap_or_else(|_| "SIN FICHA".to_string()),
    );
    let tono = r.extras.as_ref().map_or("amable", |e| match e.tono {
        crate::services::marketplace::Tono::Corto => "corto",
        crate::services::marketplace::Tono::Amable => "amable",
        crate::services::marketplace::Tono::Formal => "formal",
    });
    let sistema = format!(
        "Eres el asistente de MN Inmobiliaria respondiendo en Marketplace. \
         Tono {tono}, máximo 3 líneas. Datos del inmueble: {datos}. \
         Reglas: responde solo disponibilidad, precio o visita según pregunten; \
         jamás inventes teléfono, email, dirección ni cifras fuera de los datos; \
         si preguntan precio y no hay datos, responde exactamente: {FALLBACK_BORRADOR}"
    );
    let texto = match crate::handlers::ia::completar_opencode(&sistema, &r.excerpt.texto, &[]).await
    {
        Ok((t, _)) => t,
        Err(e) => {
            tracing::warn!("borrador mp: IA caída ({e}), va fallback");
            return (FALLBACK_BORRADOR.to_string(), "reserva".to_string());
        }
    };
    if let Some(motivo) = matriz_negativa(&texto) {
        tracing::warn!("borrador mp: matriz negativa ({motivo}), va fallback");
        return (FALLBACK_BORRADOR.to_string(), "reserva".to_string());
    }
    (texto, "ia".to_string())
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum EventoAudit {
    Hit,
    Miss,
    Copiar,
    Regenerar,
    Emision,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AuditIn {
    pub thread_id: String,
    pub evento: EventoAudit,
}

/// Auditoría agregada: guarda `HMAC(secreto, threadId)` (sal del servidor,
/// jamás el texto) con la hora truncada. Sin PII en reposo ni en tránsito.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/audit",
    request_body = AuditIn,
    responses(
        (status = 201, description = "Evento registrado"),
        (status = 422, description = "Evento inválido", body = crate::errors::ErrorResponse)
    )
)]
pub async fn audit(
    State(state): State<AppState>,
    _auth: MpAuth,
    r: Result<Json<AuditIn>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    let r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    if r.thread_id.trim().is_empty() {
        return Err(AppError::Validation("thread_id requerido".to_string()));
    }
    let evento = match r.evento {
        EventoAudit::Hit => "hit",
        EventoAudit::Miss => "miss",
        EventoAudit::Copiar => "copiar",
        EventoAudit::Regenerar => "regenerar",
        EventoAudit::Emision => "emision",
    };
    /* HMAC sin dependencias nuevas: `sha256()` y `encode()` son nativos de
     * Postgres; el secreto viaja solo en el parámetro dentro del servidor. */
    sqlx::query(
        "INSERT INTO mp_auditoria (hilo_hmac, ts_hora, evento) \
         SELECT encode(sha256(($1 || $2)::bytea), 'hex'), date_trunc('hour', now()), $3",
    )
    .bind(&state.jwt_secret)
    .bind(r.thread_id.trim())
    .bind(evento)
    .execute(&state.pool)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"registrado": true})),
    )
        .into_response())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/marketplace/token", post(emitir_token))
        .route("/marketplace/borrador", post(borrador))
        .route("/marketplace/audit", post(audit))
}
