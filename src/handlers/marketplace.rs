/* [03AA-3 M3] HTTP del asistente Marketplace (token, borrador, audit).
 * [03AA-3 M2] suma `uso`: dashboard agregado (día+evento+conteo, sin PII)
 * con el JWT admin. [03AA-3 M4] suma caché (`regenerar`, `corregir`):
 * hit/miss por (firma, precio, catálogo), sin servir precio viejo.
 * La lógica pura vive en `services::marketplace`; aquí
 * solo boundary HTTP + 429 con `Retry-After`. */

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
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
    aviso_fb_de_thread, borrar_cache, buscar_cache, clave_hilo, consumir_minuto, corregir_cache,
    detalle_chat, formatear_parrafos, guardar_cache, hash_ficha, normalizar_excerpt_hilo,
    precio_hash_seguro, reemplazar_cache, releer_foto, resumen_chats, resumen_uso,
    strip_ficha_para_prompt, sub_exento, validar_borrador, BorradorRequest, FotoHilo,
    FALLBACK_BORRADOR, SIN_FICHA, STRIP_VERSION,
};
use crate::AppState;

/* [08AA-8] Token mp (extractor + emisión) vive en `marketplace_token.rs`
 * (split límite 500). Re-export `pub` para las rutas utoipa de `mod.rs`
 * (`marketplace::emitir_token`…); `MpAuth`/`limite` para este boundary. */
use super::marketplace_token::limite;
pub use super::marketplace_token::{
    emitir_token, emitir_token_cli, CliTokenRequest, MpAuth, TokenResponse,
};

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
            Err(_) => claves_por_titulo(pool, titulo_fb).await,
        },
        None => claves_por_titulo(pool, titulo_fb).await,
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
    let mut r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    let errores = validar_borrador(&r);
    if !errores.is_empty() {
        return Err(AppError::Validation(errores.join("; ")));
    }
    /* [08AA-5] El excerpt del puente trae cada mensaje dos veces + ruido
     * de Facebook: se limpia antes del prompt y del guardado. Si solo
     * había ruido se conserva el original (nunca se guarda vacío).
     * [08AA-16] Con contexto del hilo: fuera cabeceras del visor.
     * [08AA-18] Contexto con `clave_hilo()`: la cifra inyectada por el
     * puente (07AA-11) parpadea y el aviso con `$` no empareja el eco.
     * [08AA-21] El crudo se captura antes de limpiar: se guarda tal cual
     * llegó para calibrar el filtro (08AA-8). */
    let crudo = r.excerpt.texto.clone();
    let limpio = normalizar_excerpt_hilo(&clave_hilo(r.thread_id.trim()), &r.excerpt.texto);
    if !limpio.is_empty() {
        r.excerpt.texto = limpio;
    }
    let titulo_fb = aviso_fb_de_thread(r.thread_id.trim());
    let (seguro, precio_hash, catalog_hash, conocido) =
        claves_cache(&state.pool, r.aviso_id.as_deref(), titulo_fb.as_deref()).await?;
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
        let foto = FotoHilo {
            thread_id: r.thread_id.trim(),
            excerpt: &r.excerpt.texto,
            excerpt_crudo: &crudo,
        };
        guardar_cache(
            &state.pool,
            &r.firma,
            &precio_hash,
            &catalog_hash,
            &gen.texto,
            &foto,
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
            corregida: false,
        }),
    )
        .into_response())
}

/// Regenerar explícito de la dueña: `DELETE` + bypass de lectura (nueva IA
/// siempre) + reemplazo (pisa incluso correcciones: lo pidió ella).
/// Sin tope por minuto por decisión 2026-10-05 (freno = ritmo humano); el
/// resto del flujo (schema 422, reserva si cae la IA, no cachear fallback)
/// es idéntico al `borrador`. [08AA-14] Sin matriz negativa por decisión de
/// ella 2026-10-08: el texto de la IA pasa tal cual (el prompt conserva la
/// regla de no inventar contacto).
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
    let mut r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    let errores = validar_borrador(&r);
    if !errores.is_empty() {
        return Err(AppError::Validation(errores.join("; ")));
    }
    /* [08AA-5] Igual que en `borrador`: excerpt limpio al prompt y al reemplazo.
     * [08AA-16] Con contexto del hilo.
     * [08AA-18] Contexto con `clave_hilo()` (ver `borrador`).
     * [08AA-21] Crudo capturado antes de limpiar (ver `borrador`). */
    let crudo = r.excerpt.texto.clone();
    let limpio = normalizar_excerpt_hilo(&clave_hilo(r.thread_id.trim()), &r.excerpt.texto);
    if !limpio.is_empty() {
        r.excerpt.texto = limpio;
    }
    let titulo_fb = aviso_fb_de_thread(r.thread_id.trim());
    let (seguro, precio_hash, catalog_hash, conocido) =
        claves_cache(&state.pool, r.aviso_id.as_deref(), titulo_fb.as_deref()).await?;
    borrar_cache(&state.pool, &r.firma, &precio_hash, &catalog_hash).await?;
    /* Bypass: directo a la IA, sin vuelo (Regenerar es gesto explícito; si
     * dos llegan juntas, la última que escribe gana por `reemplazar`). */
    let gen = generar_borrador(&r, seguro.as_ref(), &state.pool).await;
    if gen.fuente == "ia" {
        let foto = FotoHilo {
            thread_id: r.thread_id.trim(),
            excerpt: &r.excerpt.texto,
            excerpt_crudo: &crudo,
        };
        reemplazar_cache(
            &state.pool,
            &r.firma,
            &precio_hash,
            &catalog_hash,
            &gen.texto,
            &foto,
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
            corregida: false,
        }),
    )
        .into_response())
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
    if r.firma_version != "firma-v1" {
        return Err(AppError::Validation(
            "firma_version debe ser firma-v1".to_string(),
        ));
    }
    let (_, precio_hash, catalog_hash, _) =
        claves_cache(&state.pool, r.aviso_id.as_deref(), None).await?;
    corregir_cache(&state.pool, &r.firma, &precio_hash, &catalog_hash, &r.texto).await?;
    Ok((StatusCode::OK, Json(CorregirResponse { corregida: true })).into_response())
}

async fn generar_borrador(
    r: &BorradorRequest,
    seguro: Option<&crate::services::marketplace::PromptSeguro>,
    pool: &sqlx::PgPool,
) -> crate::services::marketplace::Generado {
    use crate::services::marketplace::{
        aviso_fb_de_thread, hilo_previo, nombre_de_thread, precio_del_aviso, Generado,
        CONTACTO_TEL, CONTACTO_WA,
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
     * [08AA-11] Párrafos separados por línea en blanco, no líneas sueltas:
     * el borrador se copia a WhatsApp y los saltos sueltos se ven rotos.
     * [08AA-15] Breve por pedido de ella: 3 párrafos cortos como máximo,
     * nombre corto del inmueble (tipo + residencia, sin dirección ni zona
     * duplicada) y sin párrafo de relleno ("sigue disponible y con gusto…"
     * ya va dicho en la apertura). */
    let saludo = match nombre_de_thread(r.thread_id.trim()) {
        Some(n) => format!("salúdalo por su nombre («Hola, {n}, ...»)"),
        None => "salúdalo sin nombre (solo «Hola, ...»)".to_string(),
    };
    let sistema = format!(
        "Eres el asistente de MN Inmobiliaria respondiendo en Marketplace. \
         Tono {tono}, BREVE: máximo 3 párrafos cortos, cada uno en su \
         párrafo separado por una línea en blanco (nada de líneas sueltas). \
         Datos del inmueble: {datos}. \
         Aviso en Facebook: {aviso}. \
         Hora del mensaje: {hora}: saluda con buenos días, buenas tardes o \
         buenas noches según corresponda. \
         La conversación trae marcas: `Cliente:` es el comprador, `Dueña:` \
         es la dueña (tú no eres la dueña: no repitas lo que ella ya dijo). \
         Formato obligatorio, en este orden exacto: primer párrafo = el \
         saludo, {saludo}, más el nombre corto del inmueble (solo tipo + \
         residencia, sin dirección ni zona duplicada), más si está \
          disponible, más el precio con la cifra exacta de los datos o del \
          aviso (si los datos traen «operacion»:«alquiler» es un ALQUILER: \
          la cifra es el canon mensual —«$1.500 mensuales»—, jamás hables \
          de venta ni uses la palabra «negociable»; si trae «venta», la \
          cifra va seguida siempre de la palabra «negociable»); segundo párrafo \
         = responde la última pregunta del Cliente en una línea, con \
         coherencia y sin repetir lo ya dicho; tercer párrafo = invítalo a \
         contarte qué busca para ayudarlo (cálido, p. ej. \
         «Cuéntame qué estás buscando y con gusto te ayudo») e incluye \
         siempre «cualquier cosa escríbeme al {CONTACTO_TEL}»; cierra \
         siempre con {CONTACTO_WA}. \
         Reglas: jamás inventes teléfono, email, dirección ni cifras fuera \
         de los datos y el aviso; si no hay precio en los datos ni en el \
         aviso, no lo inventes: di que lo confirmas con la dueña; \
         si preguntan precio y no hay precio en los datos ni en el aviso, responde exactamente: {FALLBACK_BORRADOR} \
         (el sistema agrega el contacto y el enlace al final). \
         Ya le dijiste (no lo repitas igual): {ya_dicho}",
        hora = r.excerpt.hora
    );
    let texto = match crate::handlers::ia::completar_opencode(&sistema, &r.excerpt.texto, &[]).await
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
    Generado {
        texto: formatear_parrafos(&crate::services::marketplace::asegurar_contacto(&texto)),
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
        .route("/marketplace/releer", post(releer))
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
