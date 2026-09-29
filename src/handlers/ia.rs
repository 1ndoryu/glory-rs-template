/* [199A-1] Centro de IA de texto (redaccion de fichas + copy; la mejora
 * de fotos va por su propio servicio). Dos proveedores: GloryAPI (chat
 * completions OpenAI-compatible, `model:auto`) y OpenCode Go (Responses API
 * via `glory-agent`). El activo se elige en Configuracion; si falla, se
 * reintenta con el otro. Las claves viven solo en `.env` y nunca se loguean
 * ni se devuelven (regla del provider de glory-agent). */

use axum::extract::{DefaultBodyLimit, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::AppState;

const ID_GLORY: &str = "gloryapi";
const ID_OPENCODE: &str = "opencode-go";

const CLAVE_ACTIVO: &str = "ia_activo";
const CLAVE_HAB_GLORY: &str = "ia_hab_gloryapi";
const CLAVE_HAB_OPENCODE: &str = "ia_hab_opencode_go";

const GLORY_DEFECTO_URL: &str = "http://127.0.0.1:3101";
const MAX_FOTOS: usize = 4;
/* 4 dataURL JPEG 1280px caben de sobra; coincide con el tope del front. */
const MAX_CUERPO_FOTOS: usize = 12_000_000;
const MAX_TEXTO: usize = 8000;

fn fail(e: &glory_agent::errors::AgentError) -> AppError {
    AppError::Internal(e.to_string())
}

fn leer_env(nombre: &str) -> String {
    std::env::var(nombre).unwrap_or_default().trim().to_string()
}

fn glory_base() -> String {
    let base = leer_env("GLORY_API_URL");
    let base = if base.is_empty() {
        GLORY_DEFECTO_URL.to_string()
    } else {
        base
    };
    base.trim_end_matches('/').to_string()
}

fn ordenar_intento(activo: &str) -> [&'static str; 2] {
    if activo == ID_OPENCODE {
        [ID_OPENCODE, ID_GLORY]
    } else {
        [ID_GLORY, ID_OPENCODE]
    }
}

async fn leer_clave(pool: &sqlx::PgPool, clave: &str, defecto: &str) -> Result<String, AppError> {
    Ok(glory_agent::persistence::get_config(pool, clave)
        .await
        .map_err(|e| fail(&e))?
        .unwrap_or_else(|| defecto.to_string()))
}

fn habilitado(valor: &str) -> bool {
    valor != "off"
}

/// Diagnostico guardado `ok|<epoch>|<ms>|<modelo>` o `error|<epoch>|<motivo>`.
#[derive(Debug, Default)]
struct Diagnostico {
    estado: String,
    comprobado_en: Option<i64>,
    latencia_ms: Option<u64>,
    ultimo_modelo: Option<String>,
    ultimo_error: Option<String>,
}

fn parsear_diagnostico(crudo: Option<String>) -> Diagnostico {
    let Some(texto) = crudo else {
        return Diagnostico {
            estado: "no-probado".to_string(),
            ..Diagnostico::default()
        };
    };
    let partes: Vec<&str> = texto.splitn(4, '|').collect();
    let (kind, resto) = (partes.first(), &partes[1..]);
    match (kind, resto) {
        (Some(&"ok"), [epoch, ms, modelo]) => Diagnostico {
            estado: "ok".to_string(),
            comprobado_en: epoch.parse().ok(),
            latencia_ms: ms.parse().ok(),
            ultimo_modelo: Some((*modelo).to_string()),
            ..Diagnostico::default()
        },
        (Some(&"error"), [epoch, motivo]) => Diagnostico {
            estado: "error".to_string(),
            comprobado_en: epoch.parse().ok(),
            ultimo_error: Some((*motivo).to_string()),
            ..Diagnostico::default()
        },
        _ => Diagnostico {
            estado: "no-probado".to_string(),
            ..Diagnostico::default()
        },
    }
}

#[derive(Debug, Serialize)]
struct EstadoProveedor {
    id: String,
    nombre: String,
    modelo: String,
    habilitado: bool,
    configurado: bool,
    estado: String,
    comprobado_en: Option<i64>,
    latencia_ms: Option<u64>,
    ultimo_modelo: Option<String>,
    ultimo_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct EstadoIA {
    activo: String,
    proveedores: Vec<EstadoProveedor>,
}

async fn estado_proveedor(
    pool: &sqlx::PgPool,
    id: &'static str,
    nombre: &str,
    modelo: String,
    clave_hab: &str,
    clave_check: &str,
    api_key: &str,
) -> Result<EstadoProveedor, AppError> {
    let hab = habilitado(&leer_clave(pool, clave_hab, "on").await?);
    let configurado = !api_key.trim().is_empty();
    let diag = parsear_diagnostico(
        glory_agent::persistence::get_config(pool, clave_check)
            .await
            .map_err(|e| fail(&e))?,
    );
    let estado = if !hab {
        "deshabilitado".to_string()
    } else if !configurado {
        "sin-clave".to_string()
    } else {
        diag.estado.clone()
    };
    Ok(EstadoProveedor {
        id: id.to_string(),
        nombre: nombre.to_string(),
        modelo,
        habilitado: hab,
        configurado,
        estado,
        comprobado_en: diag.comprobado_en,
        latencia_ms: diag.latencia_ms,
        ultimo_modelo: diag.ultimo_modelo,
        ultimo_error: diag.ultimo_error,
    })
}

async fn leer_estado(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<EstadoIA>, AppError> {
    let activo = leer_clave(&state.pool, CLAVE_ACTIVO, ID_GLORY).await?;
    let activo = if activo == ID_OPENCODE {
        ID_OPENCODE.to_string()
    } else {
        ID_GLORY.to_string()
    };
    let proveedores = vec![
        estado_proveedor(
            &state.pool,
            ID_GLORY,
            "GloryAPI",
            "auto".to_string(),
            CLAVE_HAB_GLORY,
            "ia_check_gloryapi",
            &leer_env("GLORY_API_KEY"),
        )
        .await?,
        estado_proveedor(
            &state.pool,
            ID_OPENCODE,
            "OpenCode Go",
            "muse-spark-1.3-contributor".to_string(),
            CLAVE_HAB_OPENCODE,
            "ia_check_opencode_go",
            &leer_env("OPENCODE_GO_API_KEY"),
        )
        .await?,
    ];
    Ok(Json(EstadoIA {
        activo,
        proveedores,
    }))
}

#[derive(Debug, Deserialize)]
struct ConfigIAEntrada {
    activo: String,
    gloryapi_habilitado: bool,
    opencode_go_habilitado: bool,
}

async fn guardar_config_ia(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<ConfigIAEntrada>,
) -> Result<Json<serde_json::Value>, AppError> {
    if input.activo != ID_GLORY && input.activo != ID_OPENCODE {
        return Err(AppError::BadRequest(
            "activo debe ser gloryapi|opencode-go".to_string(),
        ));
    }
    let ConfigIAEntrada {
        activo,
        gloryapi_habilitado,
        opencode_go_habilitado,
    } = input;
    for (clave, valor) in [
        (CLAVE_ACTIVO, activo),
        (
            CLAVE_HAB_GLORY,
            if gloryapi_habilitado { "on" } else { "off" }.to_string(),
        ),
        (
            CLAVE_HAB_OPENCODE,
            if opencode_go_habilitado { "on" } else { "off" }.to_string(),
        ),
    ] {
        glory_agent::persistence::set_config(&state.pool, clave, &valor)
            .await
            .map_err(|e| fail(&e))?;
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

#[derive(Debug, Deserialize)]
struct ProbarEntrada {
    proveedor: String,
}

#[derive(Debug, Serialize)]
struct ProbarSalida {
    ok: bool,
    latencia_ms: u64,
    modelo: Option<String>,
    error: Option<String>,
}

fn cliente_http(segs: u64) -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(segs))
        .build()
        .map_err(|e| AppError::Internal(format!("AI http: {e}")))
}

async fn ping_glory(base: &str, key: &str) -> Result<String, String> {
    let cliente = cliente_http(30).map_err(|e| e.to_string())?;
    let resp = cliente
        .post(format!("{base}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "model": "auto",
            "stream": false,
            "max_tokens": 16,
            "messages": [
                {"role": "system", "content": "Responde exactamente: OK"},
                {"role": "user", "content": "ping"},
            ],
        }))
        .send()
        .await
        .map_err(|e| format!("GloryAPI red: {e}"))?;
    if resp.status() == 401 || resp.status() == 403 {
        return Err("GloryAPI rechazo la clave (revisa GLORY_API_KEY)".to_string());
    }
    if !resp.status().is_success() {
        return Err(format!("GloryAPI HTTP {}", resp.status()));
    }
    let cuerpo: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("GloryAPI respuesta no JSON: {e}"))?;
    Ok(cuerpo
        .pointer("/model")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("auto")
        .to_string())
}

async fn ping_opencode(key: &str) -> Result<String, String> {
    let cliente = cliente_http(30).map_err(|e| e.to_string())?;
    let config = glory_agent::providers::ProviderConfig::opencode_go(key.to_string());
    let cuerpo = glory_agent::providers::build_responses_body(
        &config.model,
        &[serde_json::json!({"role": "user", "content": "Responde exactamente: OK"})],
        None,
        glory_agent::providers::ChatApiOptions::terse(16),
    );
    let resp = cliente
        .post(config.responses_url())
        .header("Authorization", format!("Bearer {key}"))
        .header("x-opencode-session", uuid::Uuid::new_v4().to_string())
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| format!("OpenCode Go red: {e}"))?;
    if resp.status() == 401 || resp.status() == 403 {
        return Err("OpenCode Go rechazo la clave (revisa OPENCODE_GO_API_KEY)".to_string());
    }
    if !resp.status().is_success() {
        let trozo: String = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect();
        return Err(format!("OpenCode Go HTTP: {trozo}"));
    }
    Ok(config.model.clone())
}

async fn guardar_diagnostico(
    pool: &sqlx::PgPool,
    id: &str,
    resultado: &Result<String, String>,
    latencia_ms: u64,
) -> Result<(), AppError> {
    let clave = if id == ID_OPENCODE {
        "ia_check_opencode_go"
    } else {
        "ia_check_gloryapi"
    };
    let epoch = chrono::Utc::now().timestamp();
    let valor = match resultado {
        Ok(modelo) => format!("ok|{epoch}|{latencia_ms}|{modelo}"),
        Err(motivo) => {
            let corto: String = motivo.chars().take(120).collect();
            format!("error|{epoch}|{corto}")
        }
    };
    glory_agent::persistence::set_config(pool, clave, &valor)
        .await
        .map_err(|e| fail(&e))?;
    Ok(())
}

async fn probar(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<ProbarEntrada>,
) -> Result<Json<ProbarSalida>, AppError> {
    let inicio = Instant::now();
    let resultado = match input.proveedor.as_str() {
        ID_GLORY => {
            let key = leer_env("GLORY_API_KEY");
            if key.is_empty() {
                Err("Sin GLORY_API_KEY en .env".to_string())
            } else {
                ping_glory(&glory_base(), &key).await
            }
        }
        ID_OPENCODE => {
            let key = leer_env("OPENCODE_GO_API_KEY");
            if key.is_empty() {
                Err("Sin OPENCODE_GO_API_KEY en .env".to_string())
            } else {
                ping_opencode(&key).await
            }
        }
        otro => {
            return Err(AppError::BadRequest(format!(
                "proveedor desconocido: {otro}"
            )));
        }
    };
    let latencia_ms = u64::try_from(inicio.elapsed().as_millis()).unwrap_or(u64::MAX);
    guardar_diagnostico(&state.pool, &input.proveedor, &resultado, latencia_ms).await?;
    Ok(Json(match resultado {
        Ok(modelo) => ProbarSalida {
            ok: true,
            latencia_ms,
            modelo: Some(modelo),
            error: None,
        },
        Err(motivo) => ProbarSalida {
            ok: false,
            latencia_ms,
            modelo: None,
            error: Some(motivo),
        },
    }))
}

#[derive(Debug, Clone, Deserialize)]
struct CompletarEntrada {
    system: String,
    texto: String,
    #[serde(default)]
    fotos: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CompletarSalida {
    ok: bool,
    texto: Option<String>,
    proveedor: Option<String>,
    modelo: Option<String>,
    motivos: Vec<String>,
}

fn validar_entrada(input: &CompletarEntrada) -> Result<(), AppError> {
    if input.system.trim().is_empty() || input.system.len() > MAX_TEXTO {
        return Err(AppError::Validation(
            "system requerido (max 8000 caracteres)".to_string(),
        ));
    }
    if input.texto.len() > MAX_TEXTO {
        return Err(AppError::Validation(
            "texto supera 8000 caracteres".to_string(),
        ));
    }
    if input.fotos.len() > MAX_FOTOS {
        return Err(AppError::BadRequest(format!(
            "maximo {MAX_FOTOS} fotos por llamada"
        )));
    }
    if input.texto.trim().is_empty() && input.fotos.is_empty() {
        return Err(AppError::BadRequest("texto o fotos requeridos".to_string()));
    }
    let total: usize = input.fotos.iter().map(String::len).sum();
    if total > MAX_CUERPO_FOTOS {
        return Err(AppError::PayloadMuyGrande);
    }
    Ok(())
}

fn texto_glory(cuerpo: &serde_json::Value) -> Option<String> {
    let contenido = cuerpo.pointer("/choices/0/message/content")?;
    let texto = contenido.as_str()?;
    let texto = texto.trim();
    if texto.is_empty() {
        None
    } else {
        Some(texto.to_string())
    }
}

async fn completar_glory(
    system: &str,
    texto: &str,
    fotos: &[String],
) -> Result<(String, String), String> {
    let key = leer_env("GLORY_API_KEY");
    if key.is_empty() {
        return Err("Sin GLORY_API_KEY en .env".to_string());
    }
    let base = glory_base();
    let mut partes = vec![serde_json::json!({"type": "text", "text": texto})];
    for foto in fotos.iter().take(MAX_FOTOS) {
        partes.push(serde_json::json!({"type": "image_url", "image_url": {"url": foto}}));
    }
    let cliente = cliente_http(120).map_err(|e| e.to_string())?;
    let resp = cliente
        .post(format!("{base}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "model": "auto",
            "stream": false,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": partes},
            ],
        }))
        .send()
        .await
        .map_err(|e| format!("GloryAPI red: {e}"))?;
    if resp.status() == 401 || resp.status() == 403 {
        return Err("GloryAPI rechazo la clave (revisa GLORY_API_KEY)".to_string());
    }
    if !resp.status().is_success() {
        let trozo: String = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect();
        return Err(format!("GloryAPI HTTP: {trozo}"));
    }
    let cuerpo: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("GloryAPI respuesta no JSON: {e}"))?;
    let modelo = cuerpo
        .pointer("/model")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("auto")
        .to_string();
    texto_glory(&cuerpo)
        .map(|t| (t, modelo))
        .ok_or_else(|| "GloryAPI devolvio una respuesta sin texto".to_string())
}

async fn completar_opencode(
    system: &str,
    texto: &str,
    fotos: &[String],
) -> Result<(String, String), String> {
    let key = leer_env("OPENCODE_GO_API_KEY");
    if key.is_empty() {
        return Err("Sin OPENCODE_GO_API_KEY en .env".to_string());
    }
    let config = glory_agent::providers::ProviderConfig::opencode_go(key.clone());
    let mut contenido: Vec<serde_json::Value> =
        vec![serde_json::json!({"type": "input_text", "text": texto})];
    for foto in fotos.iter().take(MAX_FOTOS) {
        contenido.push(serde_json::json!({"type": "input_image", "image_url": foto}));
    }
    let entrada = vec![
        serde_json::json!({"role": "system", "content": system}),
        serde_json::json!({"role": "user", "content": contenido}),
    ];
    let cliente = cliente_http(120).map_err(|e| e.to_string())?;
    let respuesta = glory_agent::providers::call_provider(
        &config,
        &entrada,
        None,
        glory_agent::providers::ChatApiOptions {
            max_output_tokens: 2500,
            timeout_secs: 120,
        },
        Some(&uuid::Uuid::new_v4().to_string()),
        &cliente,
    )
    .await
    .map_err(|e| format!("OpenCode Go: {e}"))?;
    glory_agent::providers::extract_first_text(&respuesta)
        .map(|t| (t, config.model.clone()))
        .ok_or_else(|| "OpenCode Go devolvio una respuesta sin texto".to_string())
}

/// [299A-1 E11] La IA del turno de `WhatsApp` es texto puro (el transporte de
/// `glory-agent` arma `input` Responses solo con strings, sin `input_image`):
/// para que "vea" la foto entrante se describe aquí con visión real
/// (`completar_opencode`, probado con `input_image`) y el llamador anexa el
/// texto al mensaje `[foto]` ANTES del turno. Best-effort: cualquier `Err` y
/// el llamador conserva el `[foto]` pelado + el prompt pide que la describan.
pub(crate) async fn describir_foto(data_url: &str, pie: &str) -> Result<String, String> {
    let pie = pie.trim();
    let texto = if pie.is_empty() || pie == "(foto sin pie)" {
        "Describe lo que se ve en esta foto en 2-3 frases: qué tipo de ambiente \
         o lugar es, su estado y los detalles visibles relevantes para alguien \
         que busca inmueble."
            .to_string()
    } else {
        format!(
            "El visitante envió esta foto con el pie: \"{pie}\". Describe lo que \
             se ve en 2-3 frases (ambiente, estado, detalles visibles para \
             alguien que busca inmueble)."
        )
    };
    let (texto, _) = completar_opencode(
        "Eres el ojo del Asistente de IA de MN Inmobiliaria. Respondes solo con \
         la descripción de la foto, texto plano, sin adornos.",
        &texto,
        &[data_url.to_string()],
    )
    .await?;
    Ok(texto.chars().take(1200).collect())
}

async fn completar(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CompletarEntrada>,
) -> Result<Json<CompletarSalida>, AppError> {
    validar_entrada(&input)?;
    let activo = leer_clave(&state.pool, CLAVE_ACTIVO, ID_GLORY).await?;
    let mut motivos = Vec::new();
    for id in ordenar_intento(&activo) {
        let (clave_hab, tiene_clave) = if id == ID_OPENCODE {
            (
                CLAVE_HAB_OPENCODE,
                !leer_env("OPENCODE_GO_API_KEY").is_empty(),
            )
        } else {
            (CLAVE_HAB_GLORY, !leer_env("GLORY_API_KEY").is_empty())
        };
        if !habilitado(&leer_clave(&state.pool, clave_hab, "on").await?) {
            motivos.push(format!("{id}: deshabilitado en Configuracion"));
            continue;
        }
        if !tiene_clave {
            motivos.push(format!("{id}: sin clave en .env"));
            continue;
        }
        let texto_fallback = input.texto.trim().to_string();
        let texto_enviar = if texto_fallback.is_empty() {
            "Ordena la ficha de este inmueble a partir de las fotos.".to_string()
        } else {
            texto_fallback
        };
        let intento = if id == ID_OPENCODE {
            completar_opencode(&input.system, &texto_enviar, &input.fotos).await
        } else {
            completar_glory(&input.system, &texto_enviar, &input.fotos).await
        };
        match intento {
            Ok((texto, modelo)) => {
                return Ok(Json(CompletarSalida {
                    ok: true,
                    texto: Some(texto),
                    proveedor: Some(id.to_string()),
                    modelo: Some(modelo),
                    motivos,
                }));
            }
            Err(motivo) => {
                tracing::warn!("IA {id} fallo: {motivo}");
                motivos.push(format!("{id}: {motivo}"));
            }
        }
    }
    Ok(Json(CompletarSalida {
        ok: false,
        texto: None,
        proveedor: None,
        modelo: None,
        motivos,
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ia/estado", get(leer_estado))
        .route("/ia/config", put(guardar_config_ia))
        .route("/ia/probar", post(probar))
        /* Sin el limite Json de 2 MiB de axum: las fotos base64 lo superan;
         * `validar_entrada` aplica el tope real (413 si excede). */
        .route(
            "/ia/completar",
            post(completar).route_layer(DefaultBodyLimit::disable()),
        )
}

#[cfg(test)]
mod pruebas {
    use super::{ordenar_intento, parsear_diagnostico, validar_entrada, CompletarEntrada};

    /* [199A-1] Orden de intento: el activo primero, el otro de respaldo. */
    #[test]
    fn activo_primero_y_respaldo_despues() {
        assert_eq!(ordenar_intento("gloryapi"), ["gloryapi", "opencode-go"]);
        assert_eq!(ordenar_intento("opencode-go"), ["opencode-go", "gloryapi"]);
        assert_eq!(ordenar_intento("otro"), ["gloryapi", "opencode-go"]);
    }

    /* [199A-1] Validacion en el boundary: texto, conteo y peso de fotos. */
    #[test]
    fn validar_entrada_rechaza_excesos() {
        let base = CompletarEntrada {
            system: "s".to_string(),
            texto: "t".to_string(),
            fotos: vec![],
        };
        assert!(validar_entrada(&base).is_ok());
        let larga = CompletarEntrada {
            texto: "x".repeat(8001),
            ..base.clone()
        };
        assert!(validar_entrada(&larga).is_err());
        let muchas = CompletarEntrada {
            fotos: vec!["f".to_string(); 5],
            ..base.clone()
        };
        assert!(validar_entrada(&muchas).is_err());
    }

    /* [199A-1] El diagnostico `ok|epoch|ms|modelo` se rehidrata; el error no. */
    #[test]
    fn parsear_diagnostico_redondea() {
        let d = parsear_diagnostico(Some("ok|1726233600|420|muse-spark-1.3".to_string()));
        assert_eq!(d.estado, "ok");
        assert_eq!(d.comprobado_en, Some(1_726_233_600));
        assert_eq!(d.latencia_ms, Some(420));
        assert_eq!(d.ultimo_modelo.as_deref(), Some("muse-spark-1.3"));
        let e = parsear_diagnostico(Some("error|1726233600|sin clave".to_string()));
        assert_eq!(e.estado, "error");
        assert_eq!(
            e.ultimo_error.as_deref(),
            Some("sin clave"),
            "el motivo se conserva para Configuracion"
        );
        assert!(e.ultimo_modelo.is_none());
        assert_eq!(parsear_diagnostico(None).estado, "no-probado");
    }
}
