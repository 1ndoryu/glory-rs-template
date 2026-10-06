/* [03AA-3 M3] Lógica pura del asistente Marketplace (sin HTTP): strip de ficha
 * por allowlist, validación del schema M3 v1, matriz negativa versionada,
 * claims del JWT mp y cubo de tasa por minuto. Todo testeable sin BD. */

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::InmuebleRow;

/// Versión del strip aceptada (`strip_vN` del plan: hoy solo v1).
pub const STRIP_VERSION: &str = "v1";
/// Versión de la matriz negativa aplicada al borrador generado.
pub const MATRIZ_NEGATIVA_VERSION: u8 = 1;
/// Fallback exacto cuando no hay ficha, falla la IA o salta la matriz.
pub const FALLBACK_BORRADOR: &str = "Lo reviso y te confirmo precio/entrega por aquí";

/// Prompt seguro: solo los 6 campos del allowlist. La frase canónica de la
/// ficha vive en la tabla `inmuebles`; el plugin jamás ve el resto.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PromptSeguro {
    pub titulo: String,
    pub precio_publico: String,
    pub zona: String,
    pub m2: f64,
    pub habitaciones: i32,
    pub descripcion_corta: String,
}

/// Recorta la ficha al allowlist. `strip` distinto de v1 se rechaza para que
/// un despliegue viejo no cuele campos nuevos en silencio.
pub fn strip_ficha_para_prompt(ficha: &InmuebleRow, strip: &str) -> Result<PromptSeguro, AppError> {
    if strip != STRIP_VERSION {
        return Err(AppError::BadRequest(format!(
            "strip desconocido: {strip} (se esperaba {STRIP_VERSION})"
        )));
    }
    let zona = if ficha.residencia.trim().is_empty() {
        ficha.ubicacion.clone()
    } else {
        format!("{}, {}", ficha.ubicacion, ficha.residencia)
    };
    Ok(PromptSeguro {
        titulo: ficha.titulo.clone(),
        precio_publico: precio_publico(ficha.precio),
        zona,
        m2: ficha.metros,
        habitaciones: ficha.habitaciones,
        descripcion_corta: ficha.descripcion.chars().take(500).collect(),
    })
}

/// `$43.000`: miles con punto, sin decimales, solo con strings (sin casts).
#[must_use]
pub fn precio_publico(precio: f64) -> String {
    let digitos = format!("{precio:.0}");
    format!("${}", agrupar_miles(&digitos))
}

fn agrupar_miles(digitos: &str) -> String {
    let mut fuera = String::with_capacity(digitos.len() + digitos.len() / 3);
    for (i, c) in digitos.chars().enumerate() {
        let resto = digitos.len() - i;
        if i > 0 && resto.is_multiple_of(3) {
            fuera.push('.');
        }
        fuera.push(c);
    }
    fuera
}

/// Schema M3 v1 (espeja `plugins-opencode/src/nucleo/schema.ts`).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ExcerptIn {
    pub remitente_hash: String,
    pub texto: String,
    pub hora: String,
    pub leido: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Tono {
    Corto,
    Amable,
    Formal,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Largo {
    S,
    M,
    L,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ExtrasIn {
    pub tono: Tono,
    pub largo: Largo,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BorradorRequest {
    #[serde(rename = "threadId")]
    pub thread_id: String,
    pub firma: String,
    pub firma_version: String,
    pub lang: String,
    pub excerpt: ExcerptIn,
    #[serde(rename = "avisoId")]
    pub aviso_id: Option<String>,
    pub extras: Option<ExtrasIn>,
}

fn es_hex64(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Valida el schema y devuelve la lista de motivos (vacía = válido).
/// El 422 del handler sale de aquí; el parse JSON fallido sale de axum.
#[must_use]
pub fn validar_borrador(r: &BorradorRequest) -> Vec<String> {
    let mut errores = Vec::new();
    if r.thread_id.trim().is_empty() {
        errores.push("threadId requerido".to_string());
    }
    if !es_hex64(&r.firma) {
        errores.push("firma debe ser hex64".to_string());
    }
    if r.firma_version != "firma-v1" {
        errores.push("firma_version debe ser firma-v1".to_string());
    }
    if !(r.lang.len() == 2 && r.lang.chars().all(|c| c.is_ascii_lowercase())) {
        errores.push("lang ISO 2 letras minúsculas".to_string());
    }
    if !es_hex64(&r.excerpt.remitente_hash) {
        errores.push("excerpt.remitente_hash debe ser hex64".to_string());
    }
    let n = r.excerpt.texto.chars().count();
    if n == 0 || n > 2000 {
        errores.push("excerpt.texto 1..2000 caracteres".to_string());
    }
    if !es_hora_caracas(&r.excerpt.hora) {
        errores.push("excerpt.hora debe ser ISO8601 America/Caracas (-04:00)".to_string());
    }
    if r.aviso_id.as_deref().is_some_and(str::is_empty) {
        errores.push("avisoId null o string no vacío".to_string());
    }
    errores
}

/// RFC3339 con desplazamiento exactamente -04:00 (hora de Caracas).
fn es_hora_caracas(hora: &str) -> bool {
    DateTime::parse_from_rfc3339(hora).is_ok_and(|f| {
        f.offset()
            == &FixedOffset::west_opt(4 * 3600).unwrap_or_else(|| FixedOffset::east_opt(0).unwrap())
    })
}

/// Matriz negativa v1 sobre el borrador generado: teléfono (7+ dígitos),
/// email o URL → la IA no entrega contacto; lo añade la dueña a mano.
/// Devuelve el motivo o `None` si pasa.
#[must_use]
pub fn matriz_negativa(texto: &str) -> Option<&'static str> {
    let min = texto.to_lowercase();
    if min.contains('@') {
        return Some("email");
    }
    if min.contains("http") || min.contains("wa.me") || min.contains("www.") {
        return Some("url");
    }
    let digitos: String = min.chars().filter(char::is_ascii_digit).collect();
    if digitos.len() >= 7 {
        return Some("telefono");
    }
    None
}

/// Claims del JWT mp (`iss mn-backend`, `aud mp`, `scope mp:borrador`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub scope: String,
    pub exp: usize,
    pub jti: String,
}

/// `MP_SIN_LIMITE_SUB` (coma-separada): la dueña queda exenta del 429 del
/// borrador; el resto cae al tope de 30/min con `Retry-After`.
#[must_use]
pub fn sub_exento(sub: &str) -> bool {
    std::env::var("MP_SIN_LIMITE_SUB")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .any(|s| !s.is_empty() && s == sub)
}

/// Cubo por minuto atómico (`clave` + `date_trunc('minute')`): devuelve las
/// peticiones acumuladas incluyendo la actual.
pub async fn consumir_minuto(
    pool: &sqlx::PgPool,
    clave: &str,
    limite: i64,
) -> Result<bool, AppError> {
    let n: i64 = sqlx::query_scalar(
        "INSERT INTO mp_uso_minuto (clave, ventana, n) \
         VALUES ($1, date_trunc('minute', now()), 1) \
         ON CONFLICT (clave, ventana) DO UPDATE SET n = mp_uso_minuto.n + 1 \
         RETURNING n::BIGINT",
    )
    .bind(clave)
    .fetch_one(pool)
    .await?;
    Ok(n <= limite)
}

/// Emite un `jti` fresco y lo registra para poder revocarlo.
pub async fn registrar_token(
    pool: &sqlx::PgPool,
    sub: &str,
    expira_en: &DateTime<chrono::Utc>,
) -> Result<String, AppError> {
    let jti = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO mp_tokens_emitidos (jti, sub, expira_en) VALUES ($1, $2, $3)")
        .bind(&jti)
        .bind(sub)
        .bind(expira_en)
        .execute(pool)
        .await?;
    Ok(jti)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn ficha() -> InmuebleRow {
        InmuebleRow {
            id: Uuid::new_v4(),
            titulo: "Casa en Riberas".to_string(),
            descripcion: "Bonita casa".to_string(),
            ubicacion: "Puerto Ordaz".to_string(),
            puestos: 1,
            residencia: "Riberas del Caroní".to_string(),
            precio: 43000.0,
            tipo: "casa".to_string(),
            operacion: "venta".to_string(),
            habitaciones: 3,
            banos: 2,
            metros: 180.0,
            metros_terreno: 300.0,
            estado: "disponible".to_string(),
            publicado: true,
            slug: "casa-riberas".to_string(),
            copy_corta: None,
            copy_larga: None,
            copy_modelo: None,
            copy_actualizada_en: None,
            receta: None,
            extras: sqlx::types::Json(serde_json::json!({})),
            /* El mínimo privado jamás viaja al prompt: el test de claves lo
             * amarra junto al slug y al estado interno. */
            precio_minimo: Some(40000.0),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /* El strip deja pasar exactamente los 6 campos del allowlist: ni el
     * slug, ni el estado interno, ni la receta viajan al prompt.
     * (`serde_json::Map` ordena claves: se compara ordenado.) */
    #[test]
    fn strip_solo_allowlist_seis_campos() {
        let s = strip_ficha_para_prompt(&ficha(), "v1").unwrap();
        let v = serde_json::to_value(&s).unwrap();
        let mut claves: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        claves.sort_unstable();
        assert_eq!(
            claves,
            vec![
                "descripcion_corta",
                "habitaciones",
                "m2",
                "precio_publico",
                "titulo",
                "zona"
            ]
        );
        assert_eq!(s.precio_publico, "$43.000");
        assert_eq!(s.zona, "Puerto Ordaz, Riberas del Caroní");
    }

    #[test]
    fn strip_version_desconocida_se_rechaza() {
        assert!(strip_ficha_para_prompt(&ficha(), "v2").is_err());
    }

    #[test]
    fn precio_agrupa_miles_sin_casts() {
        assert_eq!(precio_publico(43000.0), "$43.000");
        assert_eq!(precio_publico(1250000.0), "$1.250.000");
        assert_eq!(precio_publico(900.0), "$900");
    }

    fn pedido() -> BorradorRequest {
        serde_json::from_value(serde_json::json!({
            "threadId": "hilo-sintetico-001",
            "firma": "ab".repeat(32),
            "firma_version": "firma-v1",
            "lang": "es",
            "excerpt": {
                "remitente_hash": "cd".repeat(32),
                "texto": "Hola, ¿sigue disponible?",
                "hora": "2026-10-05T18:00:00-04:00",
                "leido": true
            },
            "avisoId": null
        }))
        .unwrap()
    }

    #[test]
    fn schema_valido_pasa_y_cada_campo_malo_falla() {
        assert!(validar_borrador(&pedido()).is_empty());
        let mut malo = pedido();
        malo.thread_id.clear();
        assert!(!validar_borrador(&malo).is_empty());
        let mut malo = pedido();
        malo.firma = "xyz".to_string();
        assert!(!validar_borrador(&malo).is_empty());
        let mut malo = pedido();
        malo.firma_version = "otra".to_string();
        assert!(!validar_borrador(&malo).is_empty());
        let mut malo = pedido();
        malo.lang = "esp".to_string();
        assert!(!validar_borrador(&malo).is_empty());
        let mut malo = pedido();
        malo.excerpt.hora = "2026-10-05T18:00:00Z".to_string();
        assert!(!validar_borrador(&malo).is_empty());
        let mut malo = pedido();
        malo.aviso_id = Some(String::new());
        assert!(!validar_borrador(&malo).is_empty());
    }

    #[test]
    fn matriz_frena_contacto_y_deja_pasar_precio() {
        assert_eq!(matriz_negativa("Llama al 0412 1234567"), Some("telefono"));
        assert_eq!(matriz_negativa("Escríbeme a x@y.com"), Some("email"));
        assert_eq!(matriz_negativa("Mira wa.me/584121234567"), Some("url"));
        assert_eq!(matriz_negativa("Sí, sigue disponible en $43.000"), None);
        assert_eq!(MATRIZ_NEGATIVA_VERSION, 1);
    }

    #[test]
    fn sub_exento_lee_env() {
        std::env::set_var("MP_SIN_LIMITE_SUB", "ella,otro");
        assert!(sub_exento("ella"));
        assert!(!sub_exento("plugin"));
        std::env::remove_var("MP_SIN_LIMITE_SUB");
        assert!(!sub_exento("ella"));
    }
}
