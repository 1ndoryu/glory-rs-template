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
    /* [C1-lab 2026-10-07] default: el puente del piloto no lo manda y no se
     * usa en ningun calculo; exigirlo rompia la integracion con 422. */
    #[serde(default)]
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
/// `mid` (E3, solo CLI): hash hex64 de la máquina atada; `None` = token de
/// panel sin binding. `default` para que los tokens de panel en vuelo (sin
/// `mid`) sigan decodificando.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub scope: String,
    pub exp: usize,
    pub jti: String,
    #[serde(default)]
    pub mid: Option<String>,
}

/// Minutos de vida por alcance: panel 15min, CLI 8h (E3, con binding).
#[must_use]
pub const fn minutos_para_cli(es_cli: bool) -> i64 {
    if es_cli {
        480
    } else {
        15
    }
}

/// Hash de máquina válido: 64 hex (igual que `firma`; nunca el id en claro).
#[must_use]
pub fn maquina_valida(mid: &str) -> bool {
    mid.len() == 64 && mid.chars().all(|c| c.is_ascii_hexdigit())
}

/// Binding E3: con `mid` en el token, la petición debe traer la misma máquina
/// en `X-MP-Maquina`; sin `mid` (panel) no se exige nada.
#[must_use]
pub fn maquina_autorizada(claims_mid: Option<&str>, cabecera: Option<&str>) -> bool {
    match (claims_mid, cabecera) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(a), Some(b)) => a == b,
    }
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

/// Fila del dashboard M2: conteos por día y evento. Sin PII: el HMAC del hilo
/// jamás sale, solo día + evento + conteo.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UsoDia {
    pub dia: String,
    pub hit: i64,
    pub miss: i64,
    pub copiar: i64,
    pub regenerar: i64,
    pub emision: i64,
}

/// Agrega `mp_auditoria` en una sola consulta (`GROUP BY` día+evento, sin
/// N+1). `dias` se acota a 1..=90; la ventana es día calendario local del
/// servidor (misma base que `ts_hora` truncada a la hora).
pub async fn resumen_uso(pool: &sqlx::PgPool, dias: i32) -> Result<Vec<UsoDia>, AppError> {
    use sqlx::Row as _;
    let dias = dias.clamp(1, 90);
    let filas = sqlx::query(
        "SELECT ts_hora::date AS dia, evento, COUNT(*) AS n \
         FROM mp_auditoria \
         WHERE ts_hora >= date_trunc('day', now()) - make_interval(days => $1) \
         GROUP BY dia, evento ORDER BY dia",
    )
    .bind(dias)
    .fetch_all(pool)
    .await?;
    let mut orden: Vec<String> = Vec::new();
    let mut por_dia: std::collections::HashMap<String, UsoDia> = std::collections::HashMap::new();
    for f in &filas {
        let dia: chrono::NaiveDate = f.try_get("dia")?;
        let evento: String = f.try_get("evento")?;
        let n: i64 = f.try_get("n")?;
        let clave = dia.format("%Y-%m-%d").to_string();
        let entrada = por_dia.entry(clave.clone()).or_insert_with(|| {
            orden.push(clave.clone());
            UsoDia {
                dia: String::new(),
                hit: 0,
                miss: 0,
                copiar: 0,
                regenerar: 0,
                emision: 0,
            }
        });
        entrada.dia.clone_from(&clave);
        match evento.as_str() {
            "hit" => entrada.hit = n,
            "miss" => entrada.miss = n,
            "copiar" => entrada.copiar = n,
            "regenerar" => entrada.regenerar = n,
            "emision" => entrada.emision = n,
            otro => tracing::warn!("resumen_uso: evento desconocido {otro}"),
        }
    }
    Ok(orden
        .into_iter()
        .filter_map(|d| por_dia.remove(&d))
        .collect())
}

/* [03AA-3 M4] Caché de respuestas (`mp_respuestas_cache`): la clave es
 * (firma, precio_hash, catalog_hash). `precio_hash` ata la respuesta al
 * precio citado (si cambia el precio, miss y se regenera: jamás se sirve un
 * precio viejo). `catalog_hash` sale de `hash_ficha` — hash de los bytes que
 * alimentan el prompt (campos del strip + estado) calculados tras el fetch
 * y antes del strip; cualquier cambio ahí invalida. Campos ajenos al prompt
 * (copy, receta, extras) NO invalidan a propósito: no cambian la respuesta.
 * Solo se cachea `fuente=ia`; el fallback nunca (con la IA caída, cachearlo
 * envenenaría 90 días). La corrección humana (`corregida`) gana sobre
 * generaciones futuras (`guardar` usa DO NOTHING; solo `reemplazar`, vía
 * Regenerar explícito, la pisa). Sin ficha: claves literales "sin-ficha"
 * (la `firma` ya diferencia cada excerpt). */

/// Marca sin ficha para `precio_hash`/`catalog_hash` (la firma diferencia).
pub const SIN_FICHA: &str = "sin-ficha";

/// SHA-256 hex con `sha2` (ya dependencia directa). Solo hashes, sin PII.
#[must_use]
pub fn sha_hex(canon: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(canon.as_bytes()))
}

/// Hash del catálogo: serialización canónica de exactamente lo que entra al
/// prompt (los 6 campos del strip + `estado`, que condiciona disponibilidad),
/// calculada sobre la fila recién leída y antes de stripeear. Si la ficha
/// cambia en algo que la respuesta cita → hash distinto → miss → regenera.
#[must_use]
pub fn hash_ficha(ficha: &InmuebleRow) -> String {
    let canon = match strip_ficha_para_prompt(ficha, STRIP_VERSION) {
        Ok(s) => serde_json::json!({
            "titulo": s.titulo,
            "precio": s.precio_publico,
            "zona": s.zona,
            "m2": s.m2,
            "hab": s.habitaciones,
            "desc": s.descripcion_corta,
            "estado": ficha.estado,
        }),
        /* Inalcanzable con v1 (el handler lo rechazaría antes); clave
         * estable para no romper el flujo si el strip evoluciona. */
        Err(_) => serde_json::json!({"strip": "error"}),
    };
    sha_hex(&canon.to_string())
}

/// Hash del precio citado: el `precio_publico` ya formateado que ve la IA.
#[must_use]
pub fn precio_hash_seguro(seguro: &PromptSeguro) -> String {
    sha_hex(&seguro.precio_publico)
}

/// Hit de caché: el texto listo + si es corrección de la dueña. El `UPDATE`
/// atómico cuenta el uso en la misma sentencia (sin roundtrip ni carrera).
pub async fn buscar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
) -> Result<Option<(String, bool)>, AppError> {
    let fila: Option<(String, bool)> = sqlx::query_as(
        "UPDATE mp_respuestas_cache SET usos = usos + 1 \
         WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3 \
         AND valida_hasta > now() \
         RETURNING respuesta, corregida",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .fetch_optional(pool)
    .await?;
    Ok(fila)
}

/// Guarda una generación fresca; si la dueña ya corrigió esa clave, su texto
/// gana (`DO NOTHING`: la corrección humana no se pisa en silencio).
pub async fn guardar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
    respuesta: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta) \
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .bind(respuesta)
    .execute(pool)
    .await?;
    Ok(())
}

/// Pisa la fila (Regenerar explícito de la dueña): texto nuevo, vigencia
/// renovada, `corregida=FALSE`, contador a cero (nueva versión).
pub async fn reemplazar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
    respuesta: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (firma, precio_hash, catalog_hash) DO UPDATE SET \
         respuesta = EXCLUDED.respuesta, valida_hasta = now() + INTERVAL '90 days', \
         corregida = FALSE, usos = 0",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .bind(respuesta)
    .execute(pool)
    .await?;
    Ok(())
}

/// Borra la fila (primer paso de Regenerar: la siguiente lectura es miss).
pub async fn borrar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM mp_respuestas_cache \
         WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .execute(pool)
    .await?;
    Ok(())
}

/// Guarda la corrección de la dueña: la matriz negativa también vale para su
/// texto (las respuestas jamás llevan contacto; lo añade ella a mano fuera
/// del borrador). Vigencia renovada; la fila misma es el registro (sin audit
/// separada: `corregida=TRUE` + `usos` ya lo cuentan).
pub async fn corregir_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
    texto: &str,
) -> Result<(), AppError> {
    let n = texto.chars().count();
    if n == 0 || n > 2000 {
        return Err(AppError::Validation("texto 1..2000 caracteres".to_string()));
    }
    if let Some(motivo) = matriz_negativa(texto) {
        return Err(AppError::Validation(format!(
            "la corrección no puede traer {motivo} (lo añades a mano al enviar)"
        )));
    }
    sqlx::query(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, corregida) \
         VALUES ($1, $2, $3, $4, TRUE) \
         ON CONFLICT (firma, precio_hash, catalog_hash) DO UPDATE SET \
         respuesta = EXCLUDED.respuesta, valida_hasta = now() + INTERVAL '90 days', \
         corregida = TRUE, usos = 0",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .bind(texto)
    .execute(pool)
    .await?;
    Ok(())
}

/// Purga vencidas; devuelve cuántas cayeron. Se corre al arrancar (siempre) y
/// a diario vía `pg_cron` (solo `DB_24H=true`).
pub async fn purgar_cache(pool: &sqlx::PgPool) -> Result<u64, sqlx::Error> {
    let r = sqlx::query("DELETE FROM mp_respuestas_cache WHERE valida_hasta <= now()")
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}

/// Programa la purga diaria en `pg_cron` (07:00 UTC = 03:00 Caracas, sin horario
/// de verano). Idempotente (reemplaza el job si existe). Falla si no hay
/// `pg_cron` en el servidor: el llamador lo deja en `warn` y sigue (la purga al
/// arrancar ya cubre; fail-open documentado, nunca tumba el boot).
pub async fn programar_purga_diaria(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_cron")
        .execute(pool)
        .await?;
    sqlx::query(
        "DO $purga$ BEGIN \
           IF EXISTS (SELECT 1 FROM cron.job WHERE jobname = 'mp-purga-diaria') THEN \
             PERFORM cron.unschedule('mp-purga-diaria'); \
           END IF; \
           PERFORM cron.schedule('mp-purga-diaria', '0 7 * * *', \
             'DELETE FROM mp_respuestas_cache WHERE valida_hasta <= now()'); \
         END $purga$",
    )
    .execute(pool)
    .await?;
    Ok(())
}

/* Singleflight single-process: N peticiones concurrentes con la misma clave
 * (doble clic en Regenerar, reintento + original en vuelo) comparten UNA
 * generación de IA en vez de gastar N. El líder computa y difunde; los
 * seguidores esperan el mismo `Arc`. Ventana residual de microsegundos entre
 * `send` y `remove` (el que llegue ahí recomputa): best-effort honesto para
 * doble clic humano, no barrera distribuida; el `UNIQUE` + `DO NOTHING` de
 * la tabla respalda duplicados. Solo vive en memoria del proceso. */

/// Generación compartible en vuelo: texto + fuente (`ia`, nunca `reserva` —
/// el fallback no entra al vuelo: cada miss reintenta la IA).
#[derive(Debug, Clone)]
pub struct Generado {
    pub texto: String,
    pub fuente: String,
}

#[derive(Debug, Default)]
pub struct Singleflight {
    vuelo: tokio::sync::Mutex<
        std::collections::HashMap<String, tokio::sync::broadcast::Sender<std::sync::Arc<Generado>>>,
    >,
}

impl Singleflight {
    /// Ejecuta `f` si nadie vuela con `clave`; si no, espera el resultado
    /// ajeno. Sin `Send` en `f`: se sondea inline, sin `spawn`.
    pub async fn ejecutar<F, Fut>(&self, clave: &str, f: F) -> std::sync::Arc<Generado>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Generado>,
    {
        let seguidor = {
            let mut mapa = self.vuelo.lock().await;
            if let Some(tx) = mapa.get(clave) {
                Some(tx.subscribe())
            } else {
                let (tx, _rx) = tokio::sync::broadcast::channel(1);
                mapa.insert(clave.to_string(), tx);
                None
            }
        };
        /* Seguidor: el líder difunde; si el canal murió (líder caído),
         * se degrada a fallback en vez de colgar (fail-open). */
        if let Some(mut rx) = seguidor {
            rx.recv().await.unwrap_or_else(|_| {
                std::sync::Arc::new(Generado {
                    texto: FALLBACK_BORRADOR.to_string(),
                    fuente: "reserva".to_string(),
                })
            })
        } else {
            let gen = std::sync::Arc::new(f().await);
            let mut mapa = self.vuelo.lock().await;
            if let Some(tx) = mapa.remove(clave) {
                let _ = tx.send(std::sync::Arc::clone(&gen));
            }
            gen
        }
    }
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

    fn pool_si_hay() -> Option<sqlx::PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    /* Contra BD viva: 2 hit + 1 copiar hoy se agregan en la fila del día;
     * sin `DATABASE_URL` se omite. Solo lee conteos, sin PII. */

    #[test]
    fn cli_vive_8h_y_panel_15min() {
        assert_eq!(minutos_para_cli(true), 480);
        assert_eq!(minutos_para_cli(false), 15);
    }

    #[test]
    fn maquina_solo_hex64() {
        assert!(maquina_valida(&"a".repeat(64)));
        assert!(maquina_valida(&"A1".repeat(32)));
        assert!(!maquina_valida("corto"));
        assert!(!maquina_valida(&"z".repeat(64)));
        assert!(!maquina_valida(""));
    }

    #[test]
    fn binding_solo_cuando_hay_mid() {
        assert!(maquina_autorizada(None, None));
        assert!(maquina_autorizada(None, Some("x")));
        assert!(!maquina_autorizada(Some("a"), None));
        assert!(!maquina_autorizada(Some("a"), Some("b")));
        assert!(maquina_autorizada(Some("a"), Some("a")));
    }

    /* Tests M4: hashes estables, invalidación honesta y ciclo de la caché.
     * Los vivos usan `pool_si_hay` (sin `DATABASE_URL` se omiten). */

    fn fila_prueba(precio: f64) -> InmuebleRow {
        InmuebleRow {
            id: uuid::Uuid::new_v4(),
            titulo: "Apartamento en Los Palos Grandes".to_string(),
            descripcion: "Lindo apartamento con vista".to_string(),
            ubicacion: "Chacao".to_string(),
            puestos: 1,
            residencia: "Edif. Los Pinos".to_string(),
            precio,
            tipo: "apartamento".to_string(),
            operacion: "venta".to_string(),
            habitaciones: 2,
            banos: 2,
            metros: 85.0,
            metros_terreno: 0.0,
            estado: "disponible".to_string(),
            publicado: true,
            slug: "apt-test".to_string(),
            copy_corta: None,
            copy_larga: None,
            copy_modelo: None,
            copy_actualizada_en: None,
            receta: None,
            extras: sqlx::types::Json(serde_json::json!({})),
            precio_minimo: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn clave_azar() -> String {
        format!(
            "{:x}{:x}",
            uuid::Uuid::new_v4().as_simple(),
            uuid::Uuid::new_v4().as_simple()
        )
    }

    #[test]
    fn hash_ficha_estable_y_hex64() {
        let f = fila_prueba(43_000.0);
        let a = hash_ficha(&f);
        let b = hash_ficha(&fila_prueba(43_000.0));
        assert_eq!(a, b);
        assert!(es_hex64(&a));
    }

    #[test]
    fn hash_ficha_invalida_si_cambia_lo_que_cita() {
        let base = hash_ficha(&fila_prueba(43_000.0));
        let mut f = fila_prueba(45_000.0);
        assert_ne!(hash_ficha(&f), base, "precio distinto debe invalidar");
        f = fila_prueba(43_000.0);
        f.titulo = "Otro título".to_string();
        assert_ne!(hash_ficha(&f), base, "título distinto debe invalidar");
        f = fila_prueba(43_000.0);
        f.estado = "vendido".to_string();
        assert_ne!(hash_ficha(&f), base, "estado distinto debe invalidar");
    }

    #[test]
    fn hash_ficha_ignora_lo_que_no_entra_al_prompt() {
        let base = hash_ficha(&fila_prueba(43_000.0));
        let mut f = fila_prueba(43_000.0);
        f.copy_corta = Some("Copy marketing".to_string());
        f.extras = sqlx::types::Json(serde_json::json!({"piso": "3"}));
        assert_eq!(hash_ficha(&f), base, "copy/extras no cambian la respuesta");
    }

    #[test]
    fn precio_hash_ata_al_precio_citado() {
        let a = precio_hash_seguro(&strip_ficha_para_prompt(&fila_prueba(43_000.0), "v1").unwrap());
        let b = precio_hash_seguro(&strip_ficha_para_prompt(&fila_prueba(45_000.0), "v1").unwrap());
        assert!(es_hex64(&a));
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn cache_guarda_hit_y_cuenta_usos() {
        let Some(pool) = pool_si_hay() else { return };
        let (firma, ph, ch) = (clave_azar(), clave_azar(), clave_azar());
        guardar_cache(&pool, &firma, &ph, &ch, "texto-ia")
            .await
            .expect("guarda");
        let hit = buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca");
        assert_eq!(hit, Some(("texto-ia".to_string(), false)));
        buscar_cache(&pool, &firma, &ph, &ch)
            .await
            .expect("busca x2");
        let usos: i64 = sqlx::query_scalar(
            "SELECT usos::BIGINT FROM mp_respuestas_cache WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3",
        )
        .bind(&firma).bind(&ph).bind(&ch)
        .fetch_one(&pool).await.expect("lee usos");
        assert_eq!(usos, 2);
        borrar_cache(&pool, &firma, &ph, &ch).await.expect("limpia");
    }

    #[tokio::test]
    async fn cache_miss_si_cambia_precio_o_catalogo() {
        let Some(pool) = pool_si_hay() else { return };
        let (firma, ph, ch) = (clave_azar(), clave_azar(), clave_azar());
        guardar_cache(&pool, &firma, &ph, &ch, "texto-ia")
            .await
            .expect("guarda");
        assert!(buscar_cache(&pool, &firma, &clave_azar(), &ch)
            .await
            .expect("busca")
            .is_none());
        assert!(buscar_cache(&pool, &firma, &ph, &clave_azar())
            .await
            .expect("busca")
            .is_none());
        borrar_cache(&pool, &firma, &ph, &ch).await.expect("limpia");
    }

    #[tokio::test]
    async fn cache_vencida_no_devuelve_y_purga_limpia() {
        let Some(pool) = pool_si_hay() else { return };
        let (firma, ph, ch) = (clave_azar(), clave_azar(), clave_azar());
        guardar_cache(&pool, &firma, &ph, &ch, "viejo")
            .await
            .expect("guarda");
        sqlx::query(
            "UPDATE mp_respuestas_cache SET valida_hasta = now() - INTERVAL '1 day' \
             WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3",
        )
        .bind(&firma)
        .bind(&ph)
        .bind(&ch)
        .execute(&pool)
        .await
        .expect("envejece");
        assert!(buscar_cache(&pool, &firma, &ph, &ch)
            .await
            .expect("busca")
            .is_none());
        let n = purgar_cache(&pool).await.expect("purga");
        assert!(n >= 1, "purga={n}");
        let queda: i64 =
            sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM mp_respuestas_cache WHERE firma = $1")
                .bind(&firma)
                .fetch_one(&pool)
                .await
                .expect("cuenta");
        assert_eq!(queda, 0);
    }

    #[tokio::test]
    async fn corregir_marca_y_guardar_no_pisa_correccion() {
        let Some(pool) = pool_si_hay() else { return };
        let (firma, ph, ch) = (clave_azar(), clave_azar(), clave_azar());
        guardar_cache(&pool, &firma, &ph, &ch, "texto-ia")
            .await
            .expect("guarda");
        corregir_cache(&pool, &firma, &ph, &ch, "texto de la dueña")
            .await
            .expect("corrige");
        assert_eq!(
            buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca"),
            Some(("texto de la dueña".to_string(), true))
        );
        /* Generación posterior no pisa la corrección (DO NOTHING). */
        guardar_cache(&pool, &firma, &ph, &ch, "texto-ia-2")
            .await
            .expect("guarda x2");
        assert_eq!(
            buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca"),
            Some(("texto de la dueña".to_string(), true))
        );
        /* Regenerar explícito sí pisa y resetea versión. */
        reemplazar_cache(&pool, &firma, &ph, &ch, "nueva-ia")
            .await
            .expect("reemplaza");
        assert_eq!(
            buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca"),
            Some(("nueva-ia".to_string(), false))
        );
        borrar_cache(&pool, &firma, &ph, &ch).await.expect("limpia");
    }

    #[tokio::test]
    async fn corregir_rechaza_vacio_y_contacto() {
        let Some(pool) = pool_si_hay() else { return };
        let (firma, ph, ch) = (clave_azar(), clave_azar(), clave_azar());
        assert!(corregir_cache(&pool, &firma, &ph, &ch, "").await.is_err());
        assert!(
            corregir_cache(&pool, &firma, &ph, &ch, "llámame al 0412 1234567")
                .await
                .is_err()
        );
        assert!(
            corregir_cache(&pool, &firma, &ph, &ch, "escríbeme a a@b.com")
                .await
                .is_err()
        );
        assert!(buscar_cache(&pool, &firma, &ph, &ch)
            .await
            .expect("busca")
            .is_none());
    }

    /* Singleflight (assert del plan): 10 concurrentes con la misma clave =
     * UNA sola ejecución y el mismo `Arc` para todos. */
    #[tokio::test]
    async fn vuelo_comparte_una_generacion() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let vuelo = Singleflight::default();
        let vuelo = Arc::new(vuelo);
        let contador = Arc::new(AtomicUsize::new(0));
        let mut tareas = Vec::new();
        for _ in 0..10 {
            let v = Arc::clone(&vuelo);
            let c = Arc::clone(&contador);
            tareas.push(tokio::spawn(async move {
                v.ejecutar("clave-x", || async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    Generado {
                        texto: "hola".to_string(),
                        fuente: "ia".to_string(),
                    }
                })
                .await
            }));
        }
        let mut resultados = Vec::new();
        for t in tareas {
            resultados.push(t.await.expect("tarea"));
        }
        assert_eq!(contador.load(Ordering::SeqCst), 1);
        for r in &resultados {
            assert_eq!(r.texto, "hola");
            assert!(Arc::ptr_eq(&resultados[0], r), "mismo Arc para todos");
        }
    }

    /* Expiración (DoD E3): un token con `exp` pasado no decodifica — la misma
     * `decode`+`Validation` que usa `MpAuth`, así que el rechazo queda
     * probado a nivel JWT (el chequeo DB `expira_en > now()` es redundante). */
    #[test]
    fn decode_rechaza_expirado() {
        use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
        let pasado = usize::try_from(chrono::Utc::now().timestamp() - 60).unwrap_or(0);
        let claims = MpClaims {
            iss: "mn-backend".to_string(),
            sub: "s".to_string(),
            aud: "mp".to_string(),
            scope: "mp:borrador".to_string(),
            exp: pasado,
            jti: "j".to_string(),
            mid: Some("a".repeat(64)),
        };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(b"x")).unwrap();
        let r = decode::<MpClaims>(
            &token,
            &DecodingKey::from_secret(b"x"),
            &Validation::new(jsonwebtoken::Algorithm::HS256),
        );
        assert!(r.is_err());
    }
    #[tokio::test]
    async fn uso_agrega_por_dia_y_evento() {
        let Some(pool) = pool_si_hay() else { return };
        let base = uuid::Uuid::new_v4().to_string().replace('-', "");
        for (sufijo, evento) in [("a", "hit"), ("b", "hit"), ("c", "copiar")] {
            sqlx::query(
                "INSERT INTO mp_auditoria (hilo_hmac, ts_hora, evento) \
                 VALUES ($1, date_trunc('hour', now()), $2)",
            )
            .bind(format!("{base}{sufijo}"))
            .bind(evento)
            .execute(&pool)
            .await
            .expect("inserta auditoria");
        }
        let filas = resumen_uso(&pool, 7).await.expect("resume uso");
        let hoy = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let fila = filas.iter().find(|f| f.dia == hoy).expect("fila de hoy");
        assert!(fila.hit >= 2, "hit={}", fila.hit);
        assert!(fila.copiar >= 1, "copiar={}", fila.copiar);
    }
}
