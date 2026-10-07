/* [03AA-3 M3] HTTP del asistente Marketplace (token, borrador, audit).
 * [03AA-3 M2] suma `uso`: dashboard agregado (día+evento+conteo, sin PII)
 * con el JWT admin. [03AA-3 M4] suma caché (`regenerar`, `corregir`):
 * hit/miss por (firma, precio, catálogo), sin servir precio viejo.
 * La lógica pura vive en `services::marketplace`; aquí
 * solo boundary HTTP + 429 con `Retry-After`. */

use axum::async_trait;
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::header::RETRY_AFTER;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
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
    borrar_cache, buscar_cache, consumir_minuto, corregir_cache, detalle_chat, guardar_cache,
    hash_ficha, matriz_negativa, precio_hash_seguro, reemplazar_cache, registrar_token,
    resumen_chats, resumen_uso, strip_ficha_para_prompt, sub_exento, validar_borrador,
    BorradorRequest, MpClaims, FALLBACK_BORRADOR, MATRIZ_NEGATIVA_VERSION, SIN_FICHA,
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

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BorradorResponse {
    pub borrador: String,
    pub fuente: String,
    pub aviso_conocido: bool,
    pub firma_version: String,
    pub matriz_version: u8,
    /// `true` si el texto es corrección de la dueña (vía `corregir`).
    pub corregida: bool,
}

/// Claves de caché para un `avisoId` opaco: UUID de `inmuebles` → hashes de
/// la fila recién leída; lo demás (o UUID inexistente) es ruta sin-ficha.
/// Mismo cálculo en `borrador`, `regenerar` y `corregir` para que los tres
/// hablen de la misma fila (si la ficha cambia entre llamadas, miss honesto).
async fn claves_cache(
    pool: &sqlx::PgPool,
    aviso_id: Option<&str>,
) -> Result<
    (
        Option<crate::services::marketplace::PromptSeguro>,
        String,
        String,
        bool,
    ),
    AppError,
> {
    match aviso_id {
        Some(a) => match Uuid::parse_str(a) {
            Ok(id) => match InmuebleRepository::find_by_id(pool, id).await? {
                Some(f) => {
                    let seguro = strip_ficha_para_prompt(&f, STRIP_VERSION)?;
                    let precio = precio_hash_seguro(&seguro);
                    let catalogo = hash_ficha(&f);
                    Ok((Some(seguro), precio, catalogo, true))
                }
                None => Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false)),
            },
            Err(_) => Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false)),
        },
        None => Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false)),
    }
}

/// Genera el borrador: valida schema (422), resuelve la ficha por UUID y la
/// stripea (sin ficha → no se afirma precio), consulta la caché (hit → sin
/// gastar IA) y si es miss genera con singleflight (un doble clic = una IA)
/// y guarda. Solo se cachea `fuente=ia`; el fallback nunca (ver M4).
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/borrador",
    request_body = BorradorRequest,
    responses(
        (status = 200, description = "Borrador listo (cache, ia o reserva)", body = BorradorResponse),
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
    let (seguro, precio_hash, catalog_hash, conocido) =
        claves_cache(&state.pool, r.aviso_id.as_deref()).await?;
    if let Some((texto, corregida)) =
        buscar_cache(&state.pool, &r.firma, &precio_hash, &catalog_hash).await?
    {
        /* Hit: el plugin audita `hit`; aquí no se audita nada (el conteo de
         * usos ya subió en la misma sentencia del `UPDATE ... RETURNING`). */
        return Ok((
            StatusCode::OK,
            Json(BorradorResponse {
                borrador: texto,
                fuente: "cache".to_string(),
                aviso_conocido: conocido,
                firma_version: "firma-v1".to_string(),
                matriz_version: MATRIZ_NEGATIVA_VERSION,
                corregida,
            }),
        )
            .into_response());
    }
    /* Miss (el plugin audita `miss`): una sola IA por clave en vuelo. */
    let clave_vuelo = format!("{}:{precio_hash}:{catalog_hash}", r.firma);
    let gen = state
        .mp_vuelo
        .ejecutar(&clave_vuelo, || {
            generar_borrador(&r, seguro.as_ref(), &state.pool)
        })
        .await;
    if gen.fuente == "ia" {
        guardar_cache(
            &state.pool,
            &r.firma,
            &precio_hash,
            &catalog_hash,
            &gen.texto,
            r.thread_id.trim(),
            &r.excerpt.texto,
        )
        .await?;
    }
    Ok((
        StatusCode::OK,
        Json(BorradorResponse {
            borrador: gen.texto.clone(),
            fuente: gen.fuente.clone(),
            aviso_conocido: conocido,
            firma_version: "firma-v1".to_string(),
            matriz_version: MATRIZ_NEGATIVA_VERSION,
            corregida: false,
        }),
    )
        .into_response())
}

/// Regenerar explícito de la dueña: `DELETE` + bypass de lectura (nueva IA
/// siempre) + reemplazo (pisa incluso correcciones: lo pidió ella).
/// Sin tope por minuto por decisión 2026-10-05 (freno = ritmo humano); el
/// resto del flujo (schema 422, matriz → reserva, no cachear fallback)
/// es idéntico al `borrador`.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/regenerar",
    request_body = BorradorRequest,
    responses(
        (status = 200, description = "Borrador regenerado (ia o reserva)", body = BorradorResponse),
        (status = 422, description = "Schema inválido", body = crate::errors::ErrorResponse)
    )
)]
pub async fn regenerar(
    State(state): State<AppState>,
    _auth: MpAuth,
    r: Result<Json<BorradorRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    let r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    let errores = validar_borrador(&r);
    if !errores.is_empty() {
        return Err(AppError::Validation(errores.join("; ")));
    }
    let (seguro, precio_hash, catalog_hash, conocido) =
        claves_cache(&state.pool, r.aviso_id.as_deref()).await?;
    borrar_cache(&state.pool, &r.firma, &precio_hash, &catalog_hash).await?;
    /* Bypass: directo a la IA, sin vuelo (Regenerar es gesto explícito; si
     * dos llegan juntas, la última que escribe gana por `reemplazar`). */
    let gen = generar_borrador(&r, seguro.as_ref(), &state.pool).await;
    if gen.fuente == "ia" {
        reemplazar_cache(
            &state.pool,
            &r.firma,
            &precio_hash,
            &catalog_hash,
            &gen.texto,
            r.thread_id.trim(),
            &r.excerpt.texto,
        )
        .await?;
    }
    Ok((
        StatusCode::OK,
        Json(BorradorResponse {
            borrador: gen.texto,
            fuente: gen.fuente,
            aviso_conocido: conocido,
            firma_version: "firma-v1".to_string(),
            matriz_version: MATRIZ_NEGATIVA_VERSION,
            corregida: false,
        }),
    )
        .into_response())
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CorregirRequest {
    pub firma: String,
    pub firma_version: String,
    #[serde(rename = "avisoId")]
    pub aviso_id: Option<String>,
    pub texto: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CorregirResponse {
    pub corregida: bool,
}

/// Guarda la corrección de la dueña (`corregida=TRUE`, vigencia +90d). Cubo
/// propio 30/min (escritura barata pero no gratis); la matriz vale también
/// para su texto (422 con motivo si trae contacto).
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/corregir",
    request_body = CorregirRequest,
    responses(
        (status = 200, description = "Corrección guardada", body = CorregirResponse),
        (status = 422, description = "Texto inválido o con contacto", body = crate::errors::ErrorResponse),
        (status = 429, description = "Tope por sub", body = crate::errors::ErrorResponse)
    )
)]
pub async fn corregir(
    State(state): State<AppState>,
    auth: MpAuth,
    r: Result<Json<CorregirRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    if !sub_exento(&auth.sub)
        && !consumir_minuto(
            &state.pool,
            &format!("corr:{}", auth.sub),
            TOPE_BORRADOR_MINUTO,
        )
        .await?
    {
        return Ok(limite(60));
    }
    let r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    if r.firma.len() != 64 || !r.firma.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::Validation("firma debe ser hex64".to_string()));
    }
    if r.firma_version != "firma-v1" {
        return Err(AppError::Validation(
            "firma_version debe ser firma-v1".to_string(),
        ));
    }
    let (_, precio_hash, catalog_hash, _) =
        claves_cache(&state.pool, r.aviso_id.as_deref()).await?;
    corregir_cache(&state.pool, &r.firma, &precio_hash, &catalog_hash, &r.texto).await?;
    Ok((StatusCode::OK, Json(CorregirResponse { corregida: true })).into_response())
}

async fn generar_borrador(
    r: &BorradorRequest,
    seguro: Option<&crate::services::marketplace::PromptSeguro>,
    pool: &sqlx::PgPool,
) -> crate::services::marketplace::Generado {
    use crate::services::marketplace::{
        aviso_fb_de_thread, hilo_previo, Generado, CONTACTO_TEL, CONTACTO_WA,
    };
    let datos = seguro.map_or_else(
        || "SIN FICHA: no conoces el inmueble; no afirmes precio ni medidas.".to_string(),
        |s| serde_json::to_string(s).unwrap_or_else(|_| "SIN FICHA".to_string()),
    );
    /* [07AA-8] El aviso de Facebook viaja en el hilo (`comprador|aviso`):
     * contexto aproximado para abrir con la ficha breve en el piloto. */
    let aviso = aviso_fb_de_thread(r.thread_id.trim()).unwrap_or_else(|| "desconocido".to_string());
    /* [07AA-8] Lo ya dicho en este hilo: la IA avanza, no repite. Si la BD
     * falla aquí, se genera sin contexto (nunca se bloquea el borrador). */
    let previas = hilo_previo(pool, r.thread_id.trim())
        .await
        .unwrap_or_default();
    let ya_dicho = if previas.is_empty() {
        "nada todavía".to_string()
    } else {
        previas.join(" / ")
    };
    let tono = r.extras.as_ref().map_or("amable", |e| match e.tono {
        crate::services::marketplace::Tono::Corto => "corto",
        crate::services::marketplace::Tono::Amable => "amable",
        crate::services::marketplace::Tono::Formal => "formal",
    });
    let sistema = format!(
        "Eres el asistente de MN Inmobiliaria respondiendo en Marketplace. \
         Tono {tono}, máximo 6 líneas. Datos del inmueble: {datos}. \
         Aviso en Facebook: {aviso}. \
         La conversación trae marcas: `Cliente:` es el comprador, `Dueña:` \
         es la dueña (tú no eres la dueña: no repitas lo que ella ya dijo). \
         Responde la última pregunta del Cliente con coherencia. \
         Formato obligatorio, en este orden exacto de 4 partes: abre con la \
         ficha breve (propiedad, precio y zona según los datos o el aviso); \
         responde la pregunta del Cliente; incluye siempre «cualquier cosa \
         escríbeme al {CONTACTO_TEL}»; cierra siempre con {CONTACTO_WA}. \
         Reglas: jamás inventes teléfono, email, dirección ni cifras fuera \
         de los datos y el aviso; \
         si preguntan precio y no hay datos, responde exactamente: {FALLBACK_BORRADOR} \
         (el sistema agrega el contacto y el enlace al final). \
         Ya le dijiste (no lo repitas igual): {ya_dicho}"
    );
    let texto = match crate::handlers::ia::completar_opencode(&sistema, &r.excerpt.texto, &[]).await
    {
        Ok((t, _)) => t,
        Err(e) => {
            tracing::warn!("borrador mp: IA caída ({e}), va fallback");
            return Generado {
                texto: crate::services::marketplace::asegurar_contacto(FALLBACK_BORRADOR),
                fuente: "reserva".to_string(),
            };
        }
    };
    if let Some(motivo) = matriz_negativa(&texto) {
        tracing::warn!("borrador mp: matriz negativa ({motivo}), va fallback");
        return Generado {
            texto: crate::services::marketplace::asegurar_contacto(FALLBACK_BORRADOR),
            fuente: "reserva".to_string(),
        };
    }
    Generado {
        texto: crate::services::marketplace::asegurar_contacto(&texto),
        fuente: "ia".to_string(),
    }
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
        .route("/marketplace/token/cli", post(emitir_token_cli))
        .route("/marketplace/borrador", post(borrador))
        .route("/marketplace/regenerar", post(regenerar))
        .route("/marketplace/corregir", post(corregir))
        .route("/marketplace/audit", post(audit))
        .route("/marketplace/uso", get(uso))
        .route("/marketplace/chats", get(chats))
        .route("/marketplace/chats/:thread", get(chat_detalle))
}

/// [03AA-3 M2] Dashboard agregado para el panel: conteos por día y evento de
/// los últimos `dias` (default 7, tope 90). Solo JWT admin; sin PII.
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct UsoQuery {
    pub dias: Option<i32>,
}

#[utoipa::path(
    get,
    path = "/api/admin/marketplace/uso",
    params(UsoQuery),
    responses(
        (status = 200, description = "Uso agregado por día", body = Vec<crate::services::marketplace::UsoDia>)
    )
)]
pub async fn uso(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<UsoQuery>,
) -> Result<Response, AppError> {
    let filas = resumen_uso(&state.pool, q.dias.unwrap_or(7)).await?;
    Ok((StatusCode::OK, Json(filas)).into_response())
}

/// [07AA-7] Panel por chat: lista de hilos con conteos. Solo JWT admin.
#[utoipa::path(
    get,
    path = "/api/admin/marketplace/chats",
    responses(
        (status = 200, description = "Chats con borradores", body = Vec<crate::services::marketplace::ChatResumen>)
    )
)]
pub async fn chats(State(state): State<AppState>, _auth: AuthUser) -> Result<Response, AppError> {
    let filas = resumen_chats(&state.pool).await?;
    Ok((StatusCode::OK, Json(filas)).into_response())
}

/// [07AA-7] Panel por chat: filas de un hilo (extracto + respuesta).
/// Solo JWT admin.
#[utoipa::path(
    get,
    path = "/api/admin/marketplace/chats/{thread}",
    params(("thread" = String, Path, description = "Clave del hilo")),
    responses(
        (status = 200, description = "Borradores del hilo", body = Vec<crate::services::marketplace::ChatFila>),
        (status = 422, description = "Hilo vacío", body = crate::errors::ErrorResponse)
    )
)]
pub async fn chat_detalle(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(thread): Path<String>,
) -> Result<Response, AppError> {
    let hilo = thread.trim();
    if hilo.is_empty() {
        return Err(AppError::Validation("thread requerido".to_string()));
    }
    let filas = detalle_chat(&state.pool, hilo).await?;
    Ok((StatusCode::OK, Json(filas)).into_response())
}
