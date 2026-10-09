/* [03AA-3 M3] HTTP del asistente Marketplace (token, borrador, audit).
 * [03AA-3 M2] suma `uso`: dashboard agregado (día+evento+conteo, sin PII)
 * con el JWT admin. [03AA-3 M4] suma caché (`regenerar`, `corregir`):
 * hit/miss por (firma, precio, catálogo), sin servir precio viejo.
 * La lógica pura vive en `services::marketplace`; aquí
 * solo boundary HTTP + 429 con `Retry-After`. */

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::repositories::InmuebleRepository;
use crate::services::marketplace::{
    aviso_fb_de_thread, borrar_hilo_no_corregidas, borrar_todo_cache, clave_hilo, consumir_minuto,
    corregir_cache, detalle_chat, filas_para_regenerar, formatear_parrafos, guardar_cache,
    hash_ficha, nombre_de_thread, normalizar_excerpt_hilo, precio_hash_seguro, reemplazar_cache,
    releer_foto, resumen_chats, resumen_uso, sha_hex, strip_ficha_para_prompt, sub_exento,
    validar_borrador, BorradorRequest, ExcerptIn, FotoHilo, FALLBACK_BORRADOR, FIRMA_VERSION_V2,
    SIN_FICHA, STRIP_VERSION,
};
use crate::AppState;

/* F0 estructuradas/idempotencia vive en `marketplace_estructuradas.rs`
 * (split god-object): aquí solo el wiring para `borrador`/`regenerar`. */
/* [09AA-22] F3 suma al wiring: verificación de la llave contra hint+firma
 * + lectura con convivencia v2→v1 (lógica en el módulo, aquí llamadas). */
use super::marketplace_estructuradas::{
    buscar_cache_convivencia, clave_idempotencia, con_idempotencia, resolver_fuente,
    verificar_idempotencia_conversacion, FuenteBorrador,
};

/* [08AA-8] Token mp (extractor + emisión) vive en `marketplace_token.rs`
 * (split límite 500). Re-export `pub` para las rutas utoipa de `mod.rs`
 * (`marketplace::emitir_token`…); `MpAuth`/`limite` para este boundary. */
use super::marketplace_token::limite;
pub use super::marketplace_token::{
    emitir_token, emitir_token_cli, CliTokenRequest, MpAuth, TokenResponse,
};
/* [09AA-5] Buffer de eventos del puente para la tab de Logs (sin PII). */
use super::mp_logs::{hilo8, mp_log, LogNivel};

const TOPE_BORRADOR_MINUTO: i64 = 30;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BorradorResponse {
    pub borrador: String,
    pub fuente: String,
    pub aviso_conocido: bool,
    pub firma_version: String,
    /// `true` si el texto es corrección de la dueña (vía `corregir`).
    pub corregida: bool,
}

/// Claves de caché para un `avisoId` opaco: UUID de `inmuebles` → hashes de
/// la fila recién leída; lo demás (o UUID inexistente) es ruta sin-ficha.
/// Mismo cálculo en `borrador`, `regenerar` y `corregir` para que los tres
/// hablen de la misma fila (si la ficha cambia entre llamadas, miss honesto).
/// [08AA-10] Sin UUID (piloto: siempre) se empareja el título del hilo
/// contra publicados (`ficha_por_titulo`): el precio del catálogo entra al
/// prompt en vez del dodge. Sin emparejamiento o con la BD caída, sin ficha
/// (el borrador jamás se bloquea por esto).
/// [09AA-21] Rama prioritaria exacta: si hay `avisoId` con dígitos de aviso
/// (`/marketplace/item/<id>` o dígitos 5–32) va `WHERE marketplace_id = $1`
/// sin pasar por el título. UUID legacy intacto (misma ruta de siempre);
/// ID exacto inexistente = sin ficha (no se cita un precio dudoso);
/// forma no-UUID-no-dígitos o sin ID = fallback por título.
async fn claves_cache(
    pool: &sqlx::PgPool,
    aviso_id: Option<&str>,
    titulo_fb: Option<&str>,
) -> Result<
    (
        Option<crate::services::marketplace::PromptSeguro>,
        String,
        String,
        bool,
    ),
    AppError,
> {
    match aviso_id.map(str::trim) {
        Some(a) if !a.is_empty() => {
            /* UUID legacy: ruta intacta (existe → ficha, no existe → sin ficha). */
            if let Ok(id) = Uuid::parse_str(a) {
                match InmuebleRepository::find_by_id(pool, id).await? {
                    Some(f) => {
                        let seguro = strip_ficha_para_prompt(&f, STRIP_VERSION)?;
                        let precio = precio_hash_seguro(&seguro);
                        let catalogo = hash_ficha(&f);
                        return Ok((Some(seguro), precio, catalogo, true));
                    }
                    None => {
                        return Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false));
                    }
                }
            }
            /* Dígitos de aviso: rama exacta prioritaria. */
            match crate::services::InmuebleService::normalizar_marketplace_id(Some(a)) {
                Ok(Some(mp)) => claves_por_marketplace_id(pool, &mp).await,
                /* Vacío normalizado o forma inválida: cae al título como antes. */
                Ok(None) | Err(_) => claves_por_titulo(pool, titulo_fb).await,
            }
        }
        _ => claves_por_titulo(pool, titulo_fb).await,
    }
}

/// Ficha por ID exacto de aviso (`WHERE marketplace_id = $1`): con vínculo,
/// hashes reales de la fila; si el ID no lo reclama nadie, ruta sin-ficha
/// (sin fallback al título: un ID explícito que no existe no debe citar el
/// precio de otra ficha por aproximación). La BD caída sí degrada a sin ficha.
async fn claves_por_marketplace_id(
    pool: &sqlx::PgPool,
    marketplace_id: &str,
) -> Result<
    (
        Option<crate::services::marketplace::PromptSeguro>,
        String,
        String,
        bool,
    ),
    AppError,
> {
    match InmuebleRepository::find_by_marketplace_id(pool, marketplace_id).await {
        Ok(Some(f)) => {
            let seguro = strip_ficha_para_prompt(&f, STRIP_VERSION)?;
            let precio = precio_hash_seguro(&seguro);
            let catalogo = hash_ficha(&f);
            Ok((Some(seguro), precio, catalogo, true))
        }
        Ok(None) => Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false)),
        Err(e) => {
            tracing::warn!("claves_cache: buscar por marketplace_id falló ({e}), va sin ficha");
            Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false))
        }
    }
}

/// Ficha por título del hilo (ver `ficha_por_titulo`): con emparejamiento,
/// hashes reales de la fila; si no, ruta sin-ficha.
async fn claves_por_titulo(
    pool: &sqlx::PgPool,
    titulo_fb: Option<&str>,
) -> Result<
    (
        Option<crate::services::marketplace::PromptSeguro>,
        String,
        String,
        bool,
    ),
    AppError,
> {
    let titulo = titulo_fb.map(str::trim).unwrap_or_default();
    if titulo.is_empty() {
        return Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false));
    }
    match crate::services::marketplace::ficha_por_titulo(pool, titulo).await {
        Ok(Some(f)) => {
            let seguro = strip_ficha_para_prompt(&f, STRIP_VERSION)?;
            let precio = precio_hash_seguro(&seguro);
            let catalogo = hash_ficha(&f);
            Ok((Some(seguro), precio, catalogo, true))
        }
        Ok(None) => Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false)),
        Err(e) => {
            tracing::warn!("claves_cache: emparejar por título falló ({e}), va sin ficha");
            Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false))
        }
    }
}

/// Genera el borrador: valida schema (422), resuelve la ficha por UUID y la
/// stripea (sin ficha → no se afirma precio), consulta la caché (hit → sin
/// gastar IA) y si es miss genera con singleflight (un doble clic = una IA)
/// y guarda. Solo se cachea `fuente=ia`; el fallback nunca (ver M4).
/* [09AA-5] Eventos para la tab de Logs, fuera de `borrador` (tope 100
 * líneas): un hit viejo aquí explica la «plantilla fantasma» (texto de otra
 * época servido como fresco). Sin PII: solo hash-8 del hilo. */
fn log_borrador_cache(thread_id: &str, corregida: bool) {
    mp_log(
        LogNivel::Info,
        "borrador.cache",
        "cache",
        format!(
            "hit de caché (firma conocida{})",
            if corregida {
                ", corrección de la dueña"
            } else {
                ""
            }
        ),
        &[
            ("hilo", serde_json::json!(hilo8(thread_id))),
            ("corregida", serde_json::json!(corregida)),
        ],
    );
}

fn log_borrador_ia(thread_id: &str, fuente: &str, latencia_ms: u64, conocido: bool) {
    mp_log(
        LogNivel::Info,
        "borrador.ia",
        fuente,
        format!("pasada generada en {latencia_ms} ms (aviso conocido: {conocido})"),
        &[
            ("hilo", serde_json::json!(hilo8(thread_id))),
            ("latencia_ms", serde_json::json!(latencia_ms)),
            ("aviso_conocido", serde_json::json!(conocido)),
        ],
    );
}
/* F0 estructuradas/idempotencia → `marketplace_estructuradas.rs`
 * (`FuenteBorrador`, `resolver_fuente`, `fuente_v1/v2`,
 * `clave_idempotencia`, `con_idempotencia`): aquí solo wiring. */

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
    headers: HeaderMap,
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
    let mut r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    let errores = validar_borrador(&r);
    if !errores.is_empty() {
        return Err(AppError::Validation(errores.join("; ")));
    }
    /* [09AA-20] `Idempotency-Key` opcional (422 si es basura) + F0: la
     * estructurada deja `excerpt.texto` renderizado y su firma v2; el texto
     * plano sigue el camino de siempre (`fuente_v1`: limpia excerpt,
     * conserva el original si solo había ruido, crudo para calibrar).
     * [09AA-22] F3: la llave se verifica contra hint+firma (422 si es de
     * otro hilo) y la lectura convive v2→v1 en transición. */
    let clave_idem = clave_idempotencia(&headers)?;
    let fuente = resolver_fuente(&mut r)?;
    verificar_idempotencia_conversacion(&r, &fuente, clave_idem.as_ref())?;
    let FuenteBorrador {
        firma_cache,
        firma_version,
        crudo,
        firma_legacy,
    } = fuente;
    let titulo_fb = aviso_fb_de_thread(r.thread_id.trim());
    let (seguro, precio_hash, catalog_hash, conocido) =
        claves_cache(&state.pool, r.aviso_id.as_deref(), titulo_fb.as_deref()).await?;
    if let Some((texto, corregida)) = buscar_cache_convivencia(
        &state.pool,
        &firma_cache,
        firma_legacy.as_deref(),
        &precio_hash,
        &catalog_hash,
    )
    .await?
    {
        /* Hit: el plugin audita `hit`; aquí no se audita nada (el conteo de
         * usos ya subió en la misma sentencia del `UPDATE ... RETURNING`). */
        log_borrador_cache(r.thread_id.trim(), corregida);
        let resp = (
            StatusCode::OK,
            Json(BorradorResponse {
                borrador: texto,
                fuente: "cache".to_string(),
                aviso_conocido: conocido,
                firma_version: firma_version.clone(),
                corregida,
            }),
        )
            .into_response();
        return Ok(con_idempotencia(resp, clave_idem.as_ref()));
    }
    /* Miss (el plugin audita `miss`): una sola IA por clave en vuelo. La
     * llave de idempotencia entra al vuelo para que un reintento colapse
     * con el original en vez de disparar otra IA. */
    let huella_idem = clave_idem.as_deref().unwrap_or("sin-clave");
    let clave_vuelo = format!("{firma_cache}:{precio_hash}:{catalog_hash}:{huella_idem}");
    /* [09AA-5] Latencia real de la pasada para la tab de Logs. */
    let inicio = std::time::Instant::now();
    let gen = state
        .mp_vuelo
        .ejecutar(&clave_vuelo, || {
            generar_borrador(&r, seguro.as_ref(), &state.pool)
        })
        .await;
    /* `u128` sin cast: `serde_json` no lo representa; si algún día no
     * cupiera en `u64`, se satura en vez de envolver. */
    let latencia_ms = u64::try_from(inicio.elapsed().as_millis()).unwrap_or(u64::MAX);
    log_borrador_ia(
        r.thread_id.trim(),
        gen.fuente.as_str(),
        latencia_ms,
        conocido,
    );
    if gen.fuente == "ia" {
        let foto = FotoHilo {
            thread_id: r.thread_id.trim(),
            excerpt: &r.excerpt.texto,
            excerpt_crudo: &crudo,
        };
        guardar_cache(
            &state.pool,
            &firma_cache,
            &precio_hash,
            &catalog_hash,
            &gen.texto,
            &foto,
        )
        .await?;
    }
    let resp = (
        StatusCode::OK,
        Json(BorradorResponse {
            borrador: gen.texto.clone(),
            fuente: gen.fuente.clone(),
            aviso_conocido: conocido,
            firma_version,
            corregida: false,
        }),
    )
        .into_response();
    Ok(con_idempotencia(resp, clave_idem.as_ref()))
}

/// Regenerar explícito de la dueña: borra las filas no-corregidas del hilo +
/// bypass de lectura (nueva IA siempre) + reemplazo (pisa incluso
/// correcciones de la MISMA firma: lo pidió ella). Si la IA cae no se guarda
/// nada ([09AA-4]: antes se conservaba el viejo y el panel lo seguía
/// mostrando al abrir). Sin tope por minuto por decisión 2026-10-05 (freno
/// = ritmo humano); el resto del flujo (schema 422, no cachear fallback)
/// es idéntico al `borrador`. [08AA-14] Sin matriz negativa por decisión de
/// ella 2026-10-08: el texto de la IA pasa por `imponer_forma_borrador`
/// (09AA-2) y conserva la regla de no inventar contacto.
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
    headers: HeaderMap,
    r: Result<Json<BorradorRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    let r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    let errores = validar_borrador(&r);
    if !errores.is_empty() {
        return Err(AppError::Validation(errores.join("; ")));
    }
    /* [09AA-20] La llave se valida y se devuelve igual que en `borrador`
     * (aquí no hay vuelo: gesto explícito, la última que escribe gana).
     * [09AA-22] F3: además se verifica contra hint+firma dentro de
     * `regenerar_uno` (422 si es de otro hilo). */
    let clave_idem = clave_idempotencia(&headers)?;
    let resp = regenerar_uno(&state.pool, r.0, clave_idem.as_ref()).await?;
    let resp = (StatusCode::OK, Json(resp)).into_response();
    Ok(con_idempotencia(resp, clave_idem.as_ref()))
}

/// [09AA-3] Núcleo compartido de Regenerar (uno y todo): normaliza el
/// excerpt, genera directo a la IA (bypass, gesto explícito) y reemplaza
/// la fila si es `ia`.
/// [09AA-4] Borra PRIMERO las filas no-corregidas del hilo y NO conserva
/// nada si la IA cae: 09AA-3 conservaba el viejo en `reserva` y el panel
/// seguía mostrando el texto viejo al abrir (reporte de ella 2026-10-09).
/// Las correcciones de la dueña (`corregida`) jamás se tocan.
/// Sin validar schema: las filas masivas ya se validaron al ingresar.
async fn regenerar_uno(
    pool: &sqlx::PgPool,
    mut r: BorradorRequest,
    clave: Option<&String>,
) -> Result<BorradorResponse, AppError> {
    /* [09AA-20] Fuente compartida con `borrador`: estructurada (firma v2 +
     * excerpt renderizado) o texto plano con su limpieza de siempre.
     * [08AA-16/18] Con contexto del hilo y `clave_hilo()` (ver `fuente_v1`).
     * [08AA-21] Crudo capturado antes de limpiar (ver `fuente_v1`).
     * [09AA-22] F3: la llave se verifica contra hint+firma (la masiva pasa
     * `None`: sus filas son plano sin conversacion y no hay nada que atar). */
    let fuente = resolver_fuente(&mut r)?;
    verificar_idempotencia_conversacion(&r, &fuente, clave)?;
    let FuenteBorrador {
        firma_cache,
        firma_version,
        crudo,
        firma_legacy: _legacy,
    } = fuente;
    let titulo_fb = aviso_fb_de_thread(r.thread_id.trim());
    let (seguro, precio_hash, catalog_hash, conocido) =
        claves_cache(pool, r.aviso_id.as_deref(), titulo_fb.as_deref()).await?;
    /* [09AA-4] Borrar primero, generar después: cada excerpt nuevo es una
     * firma nueva y las filas viejas del hilo viven 90 días; sin esto el
     * panel lista lo viejo junto a lo fresco y parece «cacheado».
     * `clave_hilo()` es idempotente (ver `guardar_cache`), así que vale
     * tanto el `thread_id` crudo del flotante como el ya guardado que
     * trae `regenerar_todo`. Las correcciones de la dueña quedan. */
    let borradas = borrar_hilo_no_corregidas(pool, &clave_hilo(r.thread_id.trim())).await?;
    /* Bypass: directo a la IA, sin vuelo (Regenerar es gesto explícito; si
     * dos llegan juntas, la última que escribe gana por `reemplazar`). */
    let gen = generar_borrador(&r, seguro.as_ref(), pool).await;
    /* [09AA-5] Evento para la tab de Logs: si esto dice `ia` y el panel
     * sigue mostrando lo viejo, el problema está del otro lado (caché del
     * front o el puente local). */
    mp_log(
        LogNivel::Info,
        "regenerar",
        gen.fuente.as_str(),
        format!(
            "regeneración directa: {borradas} filas viejas borradas (aviso conocido: {conocido})"
        ),
        &[
            ("hilo", serde_json::json!(hilo8(r.thread_id.trim()))),
            ("borradas", serde_json::json!(borradas)),
            ("aviso_conocido", serde_json::json!(conocido)),
        ],
    );
    if gen.fuente == "ia" {
        let foto = FotoHilo {
            thread_id: r.thread_id.trim(),
            excerpt: &r.excerpt.texto,
            excerpt_crudo: &crudo,
        };
        reemplazar_cache(
            pool,
            &firma_cache,
            &precio_hash,
            &catalog_hash,
            &gen.texto,
            &foto,
        )
        .await?;
    }
    Ok(BorradorResponse {
        borrador: gen.texto,
        fuente: gen.fuente,
        aviso_conocido: conocido,
        firma_version,
        corregida: false,
    })
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegenerarTodoFila {
    pub thread_id: String,
    /// `ia` (reemplazado), `reserva` (la IA cayó y NO quedó borrador fresco:
    /// lo viejo se borró antes de generar, ver `regenerar_uno`) u `omitido`
    /// (sin excerpt con que regenerar).
    pub fuente: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegenerarTodoResponse {
    pub candidatos: i64,
    pub regenerados: i64,
    pub en_reserva: i64,
    pub omitidos: i64,
    pub detalle: Vec<RegenerarTodoFila>,
}

/// Hora actual de Caracas en RFC3339 `-04:00` (el schema la exige así):
/// UTC-4 fijo, sin tzdata. El saludo del borrador queda fechado al momento
/// de la regeneración masiva, no al del mensaje original.
fn hora_caracas_actual() -> String {
    let ahora = chrono::Utc::now() - chrono::Duration::hours(4);
    format!("{}-04:00", ahora.format("%Y-%m-%dT%H:%M:%S"))
}

/// [09AA-3] Regeneración masiva del panel (botón «Regenerar todo»): una
/// pasada EN SERIE por cada fila con borrador (ver `filas_para_regenerar`:
/// salta correcciones de la dueña y filas solo-foto). En serie, no en
/// paralelo: la misma carga que N clics manuales, sin picos contra la IA
/// (cada uno tarda ~15s). Solo JWT admin. Responde el resumen por hilo.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/regenerar-todo",
    responses(
        (status = 200, description = "Resumen de la regeneración masiva", body = RegenerarTodoResponse),
    )
)]
pub async fn regenerar_todo(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Response, AppError> {
    let filas = filas_para_regenerar(&state.pool).await?;
    let hora = hora_caracas_actual();
    let mut detalle = Vec::with_capacity(filas.len());
    for f in &filas {
        if f.excerpt_texto.trim().is_empty() {
            detalle.push(RegenerarTodoFila {
                thread_id: f.thread_id.clone(),
                fuente: "omitido".to_string(),
            });
            continue;
        }
        /* La firma HMAC real viaja en la fila (es la llave de caché); el
         * resto se reconstruye: `remitente_hash` nunca sale del proceso
         * (la validación de ingreso ya pasó) y la hora es la actual. */
        let req = BorradorRequest {
            thread_id: f.thread_id.clone(),
            firma: f.firma.clone(),
            firma_version: "firma-v1".to_string(),
            lang: "es".to_string(),
            excerpt: ExcerptIn {
                remitente_hash: sha_hex("regenerar-todo"),
                texto: f.excerpt_texto.clone(),
                hora: hora.clone(),
                leido: false,
            },
            aviso_id: None,
            extras: None,
            /* [09AA-20] Masiva siempre por texto plano: las filas guardan
             * excerpt ya normalizado, no burbujas. */
            conversacion: None,
        };
        let fuente = regenerar_uno(&state.pool, req, None).await?.fuente;
        detalle.push(RegenerarTodoFila {
            thread_id: f.thread_id.clone(),
            fuente,
        });
    }
    /* `i64` sin cast: el conteo cabe siempre; si algún día no cupiera,
     * se satura en vez de envolver. */
    let cuenta = |fuente: &str| {
        i64::try_from(detalle.iter().filter(|d| d.fuente == fuente).count()).unwrap_or(i64::MAX)
    };
    let respuesta = RegenerarTodoResponse {
        candidatos: i64::try_from(detalle.len()).unwrap_or(i64::MAX),
        regenerados: cuenta("ia"),
        en_reserva: cuenta("reserva"),
        omitidos: cuenta("omitido"),
        detalle,
    };
    /* [09AA-5] Resumen a la tab de Logs. */
    mp_log(
        LogNivel::Info,
        "regenerar-todo",
        "ok",
        format!(
            "masiva: {} regenerados de {} ({} sin fresco, {} omitidos)",
            respuesta.regenerados, respuesta.candidatos, respuesta.en_reserva, respuesta.omitidos
        ),
        &[
            ("candidatos", serde_json::json!(respuesta.candidatos)),
            ("regenerados", serde_json::json!(respuesta.regenerados)),
            ("en_reserva", serde_json::json!(respuesta.en_reserva)),
            ("omitidos", serde_json::json!(respuesta.omitidos)),
        ],
    );
    Ok((StatusCode::OK, Json(respuesta)).into_response())
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReleerRequest {
    #[serde(rename = "threadId")]
    pub thread_id: String,
    /* Texto plano (no `ExcerptIn`: Releer no necesita remitente ni hora,
     * solo refresca la foto del hilo). */
    #[serde(default)]
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReleerResponse {
    pub actualizado: bool,
    /// [08AA-28] `true` si no había fila y se creó solo-foto (sin borrador).
    pub creado: bool,
}

/// Releer explícito de la dueña (botón separado de Regenerar, 08AA-9):
/// refresca la foto del hilo (`excerpt_texto`) sin generar ni tocar el
/// borrador guardado. Sin IA, sin invalidar caché: `UPDATE` por
/// `thread_id` (todas las firmas del hilo comparten la foto nueva).
/// [08AA-28] Sin fila del hilo → se crea solo-foto (`respuesta` vacía,
/// `creado:true`) para que el chat aparezca en el panel sin inventar
/// borrador. Mismo tope barato del borrador.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/releer",
    request_body = ReleerRequest,
    responses(
        (status = 200, description = "Foto del hilo refrescada o creada solo-foto", body = ReleerResponse),
        (status = 422, description = "Schema inválido", body = crate::errors::ErrorResponse),
        (status = 429, description = "Tope por sub", body = crate::errors::ErrorResponse)
    )
)]
pub async fn releer(
    State(state): State<AppState>,
    auth: MpAuth,
    r: Result<Json<ReleerRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    if !sub_exento(&auth.sub)
        && !consumir_minuto(
            &state.pool,
            &format!("releer:{}", auth.sub),
            TOPE_BORRADOR_MINUTO,
        )
        .await?
    {
        return Ok(limite(60));
    }
    let mut r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    if r.thread_id.trim().is_empty() {
        return Err(AppError::Validation("threadId requerido".to_string()));
    }
    let n = r.excerpt.chars().count();
    if n == 0 || n > 2000 {
        return Err(AppError::Validation(
            "excerpt 1..2000 caracteres".to_string(),
        ));
    }
    /* [08AA-5] Igual que en `borrador`: lo que se guarda es el excerpt
     * limpio, nunca el ruido crudo del DOM. [08AA-16] Con contexto.
     * [08AA-18] Guarda y busca con `clave_hilo()`: la cifra inyectada
     * por el puente (07AA-11) parpadea entre el `/borrador` y el
     * `/releer` y el `thread_id` literal no empareja (`actualizado=false`
     * en silencio).
     * [08AA-21] El crudo también se guarda (`excerpt_crudo`), para
     * calibrar el filtro (08AA-8): lo limpio al panel, lo crudo al
     * diagnóstico. */
    let hilo = clave_hilo(r.thread_id.trim());
    let crudo = r.excerpt.clone();
    let limpio = normalizar_excerpt_hilo(&hilo, &r.excerpt);
    if limpio.is_empty() {
        return Err(AppError::Validation(
            "excerpt sin contenido aprovechable".to_string(),
        ));
    }
    r.excerpt = limpio;
    // [08AA-28] Upsert atómico en el servicio: si no hay fila la crea
    // solo-foto para que el chat aparezca en el panel (caché borrada).
    let (actualizado, creado) = releer_foto(&state.pool, &hilo, &r.excerpt, &crudo).await?;
    Ok((
        StatusCode::OK,
        Json(ReleerResponse {
            actualizado,
            creado,
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
    if r.firma_version != "firma-v1" && r.firma_version != FIRMA_VERSION_V2 {
        return Err(AppError::Validation(
            "firma_version debe ser firma-v1 o firma-v2".to_string(),
        ));
    }
    let (_, precio_hash, catalog_hash, _) =
        claves_cache(&state.pool, r.aviso_id.as_deref(), None).await?;
    corregir_cache(&state.pool, &r.firma, &precio_hash, &catalog_hash, &r.texto).await?;
    Ok((StatusCode::OK, Json(CorregirResponse { corregida: true })).into_response())
}

/* [08AA-37] Saludo + regla de nombre fuera de `generar_borrador` (clippy
 * `too_many_lines` 108/100): el nombre del hilo es el único válido. */
fn saludo_y_regla(thread_id: &str) -> (String, String) {
    let nombre_hilo = nombre_de_thread(thread_id.trim());
    let saludo = match nombre_hilo.as_deref() {
        Some(n) => format!("salúdalo por su nombre («Hola, {n}, ...»)"),
        None => "salúdalo sin nombre (solo «Hola, ...»)".to_string(),
    };
    let regla = match nombre_hilo.as_deref() {
        Some(n) => format!(
            "El cliente se llama «{n}»: es el único nombre permitido para \
            dirigirte a él; ignora cualquier otro nombre, apellido o lugar \
            que aparezca en la conversación (p. ej. «Ordaz» de Puerto Ordaz): \
            jamás saludes con un nombre distinto ni inventes apellidos"
        ),
        None => "No sabes su nombre: saluda solo con «Hola» y jamás uses \
            ningún nombre propio para dirigirte a él"
            .to_string(),
    };
    (saludo, regla)
}

async fn generar_borrador(
    r: &BorradorRequest,
    seguro: Option<&crate::services::marketplace::PromptSeguro>,
    pool: &sqlx::PgPool,
) -> crate::services::marketplace::Generado {
    use crate::services::marketplace::{
        aviso_fb_de_thread, hilo_previo, precio_del_aviso, Generado, CONTACTO_TEL, CONTACTO_WA,
    };
    /* [07AA-8] El aviso de Facebook viaja en el hilo (`comprador|aviso`):
     * contexto aproximado para abrir con la ficha breve en el piloto. */
    let aviso = aviso_fb_de_thread(r.thread_id.trim()).unwrap_or_else(|| "desconocido".to_string());
    /* [07AA-9] En el piloto no hay ficha, pero el título del aviso sí puede
     * traer el precio publicado (`125.000$`): se extrae y se entrega como
     * dato conocido para que la IA lo dé directo en vez del fallback. */
    let precio_aviso = precio_del_aviso(&aviso);
    let datos = match (&seguro, &precio_aviso) {
        (Some(s), _) => serde_json::to_string(s).unwrap_or_else(|_| "SIN FICHA".to_string()),
        (None, Some(p)) => format!(
            "Precio publicado en el aviso: {p}. Sin ficha: no afirmes medidas, ubicación exacta ni otros datos; el precio del aviso sí dalo directo."
        ),
        (None, None) => {
            "SIN FICHA: no conoces el inmueble; no afirmes precio ni medidas.".to_string()
        }
    };
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
    /* [07AA-10] Saludo primero y por su nombre: el hilo trae `nombre|aviso`.
     * El precio según operación ([08AA-25]: un alquiler presentado como
     * venta rompe la confianza — testigo Townhouse Arivana, hilo cristo);
     * el cierre invita a contar qué busca (conocer intención, no solo
     * coordinar visita).
     * [08AA-31] Sin promesa de visita (hilo angelv: el borrador decía
     * "Sí, puedes visitarla y te coordinamos" sin saber la
     * disponibilidad real de la dueña — decir poco es mejor en el primer
     * mensaje): disponible ≠ visitable; la visita se confirma con ella.
     * [08AA-11] Párrafos separados por línea en blanco, no líneas sueltas:
     * el borrador se copia a WhatsApp y los saltos sueltos se ven rotos.
     * [08AA-15] Breve por pedido de ella: 3 párrafos cortos como máximo,
     * nombre corto del inmueble (tipo + residencia, sin dirección ni zona
     * duplicada) y sin párrafo de relleno ("sigue disponible y con gusto…"
     * ya va dicho en la apertura). */
    /* [08AA-36] Sin anuncio de confirmación con la dueña (mensaje de ella
     * 2026-10-08: el borrador decía «lo confirmo con la dueña» dos veces —
     * el prompt lo ORDENABA («di solo que está disponible y que lo
     * confirmas con ella», testigo fila olear). Ahora: el dato se da una
     * sola vez, prohibido «confirmo», «te confirmo su estatus» o cualquier
     * meta-comentario de coordinación; sin precio solo vale el FALLBACK
     * exacto.
     * [08AA-37] Nombre autoritativo + sin re-afirmar (mensajes de ella
     * 2026-10-08: saludó «Ordaz» con hilo `lidia|...` — tomó el apellido/
     * lugar del excerpt en vez del nombre del hilo; y el 2º párrafo repetía
     * lo del 1º: «Sí, se mantiene publicada en venta al momento»).
     * Ahora: el nombre del hilo es el único válido y el 2º párrafo jamás
     * reafirma disponibilidad/precio ni usa jerga interna.
     * [08AA-38] Regla estructural del 2º párrafo (mensaje de ella 2026-10-08:
     * «Sí, la publicación sigue vigente» burló la lista de frases de 08AA-37
     * con un sinónimo — fila sicilia v1, borrada en la limpieza 20:44).
     * Ahora: lógica condicional (pregunta ya respondida arriba → avanzar,
     * no responder) + veto por PALABRAS, no por frases.
     * [09AA-2] Raíz del bucle (4 subagentes 2026-10-09): el prompt mismo
     * ORDENABA el relleno («avanza la conversación: ofrece fotos o pregunta
     * qué busca») y esa orden positiva siempre le ganó al veto; además
     * «máximo 3» + 3 roles obligatorios se leía como «exactamente 3», y los
     * comentarios [08AA-*] nunca viajan al modelo. Ahora: formato de DOS
     * bloques + excepción de una línea, y la invariante la impone
     * `imponer_forma_borrador` en Rust aunque el modelo desobedezca. */
    let (saludo, regla_nombre) = saludo_y_regla(&r.thread_id);
    let sistema = format!(
        "Eres el asistente de MN Inmobiliaria respondiendo en Marketplace. \
         Tono {tono}, BREVE: el mensaje son DOS párrafos (primero + final) \
         y, solo si aplica la excepción de abajo, UNA línea intermedia; \
         cada bloque va en su párrafo separado por una línea en blanco \
         (nada de líneas sueltas). \
         Datos del inmueble: {datos}. \
         Aviso en Facebook: {aviso}. \
         Hora del mensaje: {hora}: saluda con buenos días, buenas tardes o \
         buenas noches según corresponda. \
         La conversación trae marcas: `Cliente:` es el comprador, `Dueña:` \
          es la dueña (tú no eres la dueña: no repitas lo que ella ya dijo). \
           Formato obligatorio, en este orden exacto: primer párrafo = el \
          saludo, {saludo} ({regla_nombre}), más el nombre corto del inmueble (solo tipo + \
          residencia), más si está disponible (sin prometer visitas ni \
          coordinación ni anunciar que confirmas nada con la dueña), más el \
          precio exacto de los datos o del aviso («venta» → cifra seguida de \
          «negociable»; «alquiler» → canon mensual, jamás venta ni \
          «negociable»); segundo bloque = este texto literal, sin cambiar ni \
          una palabra: «Cuéntame qué estás buscando y con gusto te ayudo. \
          Cualquier cosa escríbeme al {CONTACTO_TEL}» y en línea aparte {CONTACTO_WA}. \
          Excepción: un párrafo intermedio de UNA línea (máximo 140 \
          caracteres, sin ? ni ¿) SOLO si el Cliente pide un dato concreto \
          no dicho arriba (baños, habitaciones, m2, ubicación). Sin pregunta \
          concreta, OMITE el párrafo: nada de transiciones, fotos, preguntas \
          ni reafirmaciones con ninguna palabra. Prohibido fuera del bloque \
          final: ? ¿ fotos disponible precio visitas cifras contacto propio; \
          sin precio en datos ni aviso, no lo inventes; \
         si preguntan precio y no hay precio en los datos ni en el aviso, responde exactamente: {FALLBACK_BORRADOR} \
         (el sistema agrega el contacto y el enlace al final). \
          Ya le dijiste (no lo repitas igual): {ya_dicho}",
        hora = r.excerpt.hora,
        regla_nombre = regla_nombre
    );
    /* [09AA-4] Sesión estable por hilo (hash, jamás PII en claro): el relay
     * exige `x-opencode-session` y premia la estabilidad con ruteo afin y
     * prompt caching. */
    let sesion_hilo = sha_hex(&clave_hilo(r.thread_id.trim()));
    /* [09AA-15] Vía rápida sin razonamiento (pedido de ella por los 75 s):
     * si no trae texto cae sola a la estándar; la forma la sigue
     * imponiendo `imponer_forma_borrador` abajo. */
    let texto = match crate::handlers::ia::completar_opencode_rapido(
        &sistema,
        &r.excerpt.texto,
        &[],
        &sesion_hilo,
    )
    .await
    {
        Ok((t, _)) => t,
        Err(e) => {
            tracing::warn!("borrador mp: IA caída ({e}), va fallback");
            return Generado {
                texto: formatear_parrafos(&crate::services::marketplace::asegurar_contacto(
                    FALLBACK_BORRADOR,
                )),
                fuente: "reserva".to_string(),
            };
        }
    };
    /* [09AA-2] La invariante de forma la impone Rust: el texto de la IA
     * pasa por `imponer_forma_borrador` (poda de relleno + final canónico)
     * antes de garantizar el contacto. */
    Generado {
        texto: formatear_parrafos(&crate::services::marketplace::asegurar_contacto(
            &crate::services::marketplace::imponer_forma_borrador(&formatear_parrafos(&texto)),
        )),
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
     * Postgres; el secreto viaja solo en el parámetro dentro del servidor.
     * [08AA-18] HMAC sobre `clave_hilo()`: la misma auditoría aunque la
     * cifra inyectada (07AA-11) parpadee entre llamadas. */
    sqlx::query(
        "INSERT INTO mp_auditoria (hilo_hmac, ts_hora, evento) \
         SELECT encode(sha256(($1 || $2)::bytea), 'hex'), date_trunc('hour', now()), $3",
    )
    .bind(&state.jwt_secret)
    .bind(clave_hilo(r.thread_id.trim()))
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
        .route("/marketplace/regenerar-todo", post(regenerar_todo))
        .route("/marketplace/releer", post(releer))
        .route("/marketplace/corregir", post(corregir))
        .route("/marketplace/audit", post(audit))
        .route("/marketplace/uso", get(uso))
        .route("/marketplace/chats", get(chats).delete(borrar_todo))
        .route("/marketplace/chats/:thread", get(chat_detalle))
        /* [09AA-5] Tab de Logs: eventos del puente, recientes-primero. */
        .route("/marketplace/logs", get(super::mp_logs::logs))
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

/// [08AA-39] Limpieza total del panel: borra toda la caché de borradores.
/// Solo JWT admin. Responde cuántas filas cayeron.
#[utoipa::path(
    delete,
    path = "/api/admin/marketplace/chats",
    responses(
        (status = 200, description = "Caché limpiada")
    )
)]
pub async fn borrar_todo(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Response, AppError> {
    let n = borrar_todo_cache(&state.pool).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({"borrados": n}))).into_response())
}

/* [09AA-21] Matriz de `claves_cache`: ID exacto válido (ficha + conocido),
 * ID exacto inexistente (sin ficha, sin fallback al título), sin ID con
 * título que empareja (ficha por título) y UUID legacy intacto (ficha por
 * UUID). Humo contra la BD real de rama (`DATABASE_URL`); sin ella se omite.
 * El borrador jamás se bloquea: los casos negativos dan `SIN_FICHA`. */
#[cfg(test)]
mod pruebas_claves_cache_mp_id {
    use super::*;
    use crate::models::CreateInmuebleRequest;
    use crate::services::InmuebleService;

    fn pool_si_hay() -> Option<sqlx::PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    fn crear_humo(titulo: &str, marketplace_id: Option<&str>) -> CreateInmuebleRequest {
        CreateInmuebleRequest {
            titulo: titulo.to_string(),
            descripcion: String::new(),
            ubicacion: String::new(),
            puestos: 0,
            residencia: String::new(),
            precio: 0.0,
            tipo: "apartamento".to_string(),
            operacion: "venta".to_string(),
            habitaciones: 0,
            banos: 0,
            metros: 0.0,
            metros_terreno: 0.0,
            estado: "disponible".to_string(),
            marketplace_id: marketplace_id.map(str::to_string),
            alias_titulos: Vec::new(),
            copy: None,
        }
    }

    #[tokio::test]
    async fn matriz_id_valido_inexistente_titulo_y_uuid_legacy() {
        let Some(pool) = pool_si_hay() else { return };
        let aviso = "123456789066666";
        let titulo = "Casa clavescache mp-id 09AA-21 en Riberas del Caroní Norte";
        let creado = InmuebleService::create(&pool, crear_humo(titulo, Some(aviso)))
            .await
            .unwrap();
        InmuebleService::set_publicado(&pool, creado.id, true)
            .await
            .unwrap();

        let (ficha, precio, catalogo, conocido) =
            claves_cache(&pool, Some(aviso), Some("título que no empareja nada"))
                .await
                .unwrap();
        assert!(ficha.is_some() && conocido, "ID válido da ficha exacta");
        assert_ne!(precio, SIN_FICHA);
        assert_ne!(catalogo, SIN_FICHA);

        let (ficha, precio, _, conocido) =
            claves_cache(&pool, Some("999999999066666"), Some(titulo))
                .await
                .unwrap();
        assert!(
            ficha.is_none() && !conocido,
            "ID inexistente no cita otra ficha"
        );
        assert_eq!(precio, SIN_FICHA);

        let (ficha, _, _, conocido) = claves_cache(&pool, None, Some(titulo)).await.unwrap();
        assert!(ficha.is_some() && conocido, "sin ID el título empareja");

        let uuid = creado.id.to_string();
        let (ficha, _, _, conocido) = claves_cache(&pool, Some(&uuid), None).await.unwrap();
        assert!(ficha.is_some() && conocido, "UUID legacy sigue resolviendo");

        let (ficha, _, _, conocido) = claves_cache(&pool, Some(&Uuid::new_v4().to_string()), None)
            .await
            .unwrap();
        assert!(
            ficha.is_none() && !conocido,
            "UUID inexistente es sin ficha"
        );

        InmuebleService::delete(&pool, std::path::Path::new("."), creado.id)
            .await
            .unwrap();
    }
}
