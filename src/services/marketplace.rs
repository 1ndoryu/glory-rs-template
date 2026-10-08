/* [03AA-3 M3] Lógica pura del asistente Marketplace (sin HTTP): strip de ficha
 * por allowlist, validación del schema M3 v1, matriz negativa versionada,
 * claims del JWT mp y cubo de tasa por minuto. Verificable sin BD. */

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::InmuebleRow;
use crate::repositories::InmuebleRepository;

/* [08AA-7] Singleflight vive en su dominio (`marketplace_vuelo`); se
 * re-exporta para no mover sus usos externos (`lib.rs`, handlers, sombra). */
pub use super::marketplace_vuelo::{Generado, Singleflight};

/* [08AA-8] Texto puro (schema, excerpt, precio) vive en su dominio
 * (`marketplace_texto`); se re-exporta para no mover sus usos externos
 * (handlers, utoipa, sombra, tests). */
pub use super::marketplace_texto::{
    normalizar_excerpt, normalizar_excerpt_con_hilo, precio_del_aviso, precio_publico,
    validar_borrador, BorradorRequest, ExcerptIn, ExtrasIn, Largo, Tono,
};
/* Solo tests (`super::es_hex64` en `pruebas`): fuera de `cfg(test)` sería
 * import sin uso y rompería `clippy -D warnings` (mismo patrón que `ia.rs`). */
#[cfg(test)]
use super::marketplace_texto::es_hex64;

/// Versión del strip aceptada (`strip_vN` del plan: hoy solo v1).
pub const STRIP_VERSION: &str = "v1";
/// Fallback exacto cuando no hay ficha o falla la IA.
pub const FALLBACK_BORRADOR: &str = "Lo reviso y te confirmo precio/entrega por aquí";
/// [07AA-8] Contacto fijo de los borradores (decisión de ella 2026-10-07):
/// la IA no lo inventa, el prompt lo exige literal y `asegurar_contacto`
/// lo agrega si falta. [08AA-14] Sin matriz negativa por decisión de ella
/// 2026-10-08: el texto (propio o de la IA) pasa tal cual.
pub const CONTACTO_TEL: &str = "0424 9208855";
pub const CONTACTO_WA: &str = "https://wa.me/584249208855";

/// [07AA-8] Garantía determinista del formato: si el texto generado no trae
/// el teléfono o el enlace, se agregan (teléfono como penúltima línea,
/// enlace cerrando). Solo se aplica al borrador de IA, nunca al texto
/// manual de la dueña.
#[must_use]
pub fn asegurar_contacto(texto: &str) -> String {
    use std::fmt::Write as _;
    let mut t = texto.trim_end().to_string();
    if !t.contains(CONTACTO_TEL) {
        let _ = write!(t, "\nCualquier cosa escríbeme al {CONTACTO_TEL}.");
    }
    if !t.contains(CONTACTO_WA) {
        t.push('\n');
        t.push_str(CONTACTO_WA);
    }
    t
}

/// [08AA-11] Párrafos coherentes para el borrador que se copia a
/// `WhatsApp`: la IA a veces devuelve líneas sueltas (venía de pedirle
/// "máximo 6 líneas") o pega el contacto al final de la frase anterior.
/// Reglas deterministas: cada salto simple dentro de un párrafo se vuelve
/// espacio; los bloques se separan con una línea en blanco; la invitación
/// de contacto y el enlace wa.me siempre abren su propio párrafo; los
/// ítems de lista (`1. `, `- `, `• `) quedan en párrafo propio.
#[must_use]
pub fn formatear_parrafos(texto: &str) -> String {
    let plano = texto.replace("\r\n", "\n").replace('\r', "\n");
    let marca_contacto = format!("Cualquier cosa escríbeme al {CONTACTO_TEL}");
    let con_contacto = partir_pegado(&plano, &marca_contacto);
    let con_wa = partir_pegado(&con_contacto, CONTACTO_WA);
    let mut parrafos: Vec<String> = Vec::new();
    let mut actual = String::new();
    for linea in con_wa.lines() {
        let l = linea.trim();
        if l.is_empty() {
            vaciar_parrafo(&mut actual, &mut parrafos);
            continue;
        }
        if es_item_lista(l) {
            vaciar_parrafo(&mut actual, &mut parrafos);
            parrafos.push(l.to_string());
            continue;
        }
        if !actual.is_empty() {
            actual.push(' ');
        }
        actual.push_str(l);
    }
    vaciar_parrafo(&mut actual, &mut parrafos);
    parrafos.join("\n\n")
}

/// Vacía el párrafo en curso a la lista (sin el cierre no se puede usar
/// cierre + `push` directo: doble préstamo mutable del acumulador).
fn vaciar_parrafo(actual: &mut String, parrafos: &mut Vec<String>) {
    if !actual.trim().is_empty() {
        parrafos.push(actual.trim().to_string());
        actual.clear();
    }
}

/// Corta `marca` a su propia línea cuando viene pegada a texto previo
/// (con espacio simple). Si ya abre línea se deja intacta.
fn partir_pegado(texto: &str, marca: &str) -> String {
    let mut fuera = String::with_capacity(texto.len() + 8);
    let mut resto = texto;
    while let Some(pos) = resto.find(marca) {
        let antes = &resto[..pos];
        fuera.push_str(antes);
        if !(antes.is_empty() || antes.ends_with('\n')) {
            fuera.push_str("\n\n");
        }
        fuera.push_str(marca);
        resto = &resto[pos + marca.len()..];
    }
    fuera.push_str(resto);
    fuera
}

/// Ítem de lista al inicio de la línea: `1. `, `2) `, `- ` o `• `.
/// Siempre por `chars` (nunca por bytes: `•` es multibyte).
fn es_item_lista(linea: &str) -> bool {
    let mut letras = linea.chars();
    match letras.next() {
        Some('-' | '•') => letras.next() == Some(' '),
        Some(c) if c.is_ascii_digit() => {
            let mut cola = linea.chars();
            cola.next();
            let mut cola = cola.peekable();
            while cola.peek().is_some_and(char::is_ascii_digit) {
                cola.next();
            }
            match cola.next() {
                Some('.' | ')') => matches!(cola.next(), Some(' ') | None),
                _ => false,
            }
        }
        _ => false,
    }
}

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

/* [08AA-8] `precio_publico`, schema M3 (`BorradorRequest`…),
 * `validar_borrador` y `normalizar_excerpt` viven en `marketplace_texto.rs`
 * (re-export arriba para usos externos e internos). */

/* [08AA-8] `normalizar_excerpt*` vive en
 * `marketplace_texto.rs` (re-export arriba). */

/// [07AA-10] Nombre del cliente desde el hilo (`alejandro|casa en venta...`
/// → `Alejandro`): el borrador lo saluda por su nombre. `sin-hilo` o sin
/// `nombre|` → `None` (saludo sin nombre).
#[must_use]
pub fn nombre_de_thread(thread_id: &str) -> Option<String> {
    let (nombre, _) = thread_id.split_once('|')?;
    if nombre.trim().is_empty() || nombre.trim().eq_ignore_ascii_case("sin-hilo") {
        return None;
    }
    Some(
        nombre
            .split_whitespace()
            .map(|p| {
                let mut c = p.chars();
                match c.next() {
                    Some(i) => i.to_uppercase().to_string() + c.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/* [08AA-8] `precio_del_aviso` vive en `marketplace_texto.rs`
 * (re-export arriba). */

/// [07AA-8] Título del aviso desde el `thread_id` del puente
/// (`comprador|aviso`, minúsculas, tope 120): contexto aproximado para abrir
/// el borrador con la ficha breve cuando no hay `avisoId` (piloto: siempre).
pub fn aviso_fb_de_thread(thread_id: &str) -> Option<String> {
    let aviso = thread_id
        .split('|')
        .nth(1)
        .map(str::trim)
        .unwrap_or_default();
    if aviso.is_empty() {
        return None;
    }
    Some(aviso.to_string())
}

/// [08AA-18] Clave canónica del hilo para guardar y buscar en caché: el
/// puente inyecta la cifra del DOM tras el `|` (`tina|$43.000 vef0
/// casa...`, 07AA-11) y esa cifra parpadea entre llamadas, así que el
/// `thread_id` literal no sirve de clave (el `/borrador` guarda con una
/// forma y el `/releer` busca con otra → `actualizado=false` en
/// silencio). Se deshace un `$CIFRA ` inicial con dígitos (`$43.000`,
/// `US$ 43.000`); sin `|`, sin `$` inicial, sin dígitos en la cifra o
/// si no quedaría aviso, el hilo queda intacto. La cifra sigue viva en
/// el hilo crudo que ven el prompt y `precio_del_aviso`: aquí solo se
/// estabiliza la llave de la BD.
#[must_use]
pub fn clave_hilo(thread_id: &str) -> String {
    let texto = thread_id.trim();
    let Some((nombre, aviso)) = texto.split_once('|') else {
        return texto.to_string();
    };
    /* Moneda inicial (`$`, `US$`, `RD$`): solo letras y `$`, sin
     * espacios ni dígitos. Sin `$` no es inyección del puente
     * (`Casa en venta`, `Casa 3 habs` quedan intactos). */
    let aviso = aviso.trim();
    let tras_moneda = aviso.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == '$');
    let prefijo = &aviso[..aviso.len() - tras_moneda.len()];
    if !prefijo.contains('$') {
        return texto.to_string();
    }
    /* Cifra: dígitos con `.`/`,` tras un blanco opcional. Sin
     * dígitos no es cifra (`$negociable casa` intacto). */
    let tras_blanco = tras_moneda.trim_start();
    let tras_cifra =
        tras_blanco.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ',');
    if tras_cifra.len() == tras_blanco.len() || tras_cifra.trim_start().is_empty() {
        return texto.to_string();
    }
    format!("{}|{}", nombre.trim(), tras_cifra.trim_start())
}

/// [08AA-16] Normaliza el excerpt con contexto del hilo: nombre del
/// comprador y título del aviso salen del `thread_id`
/// (`comprador|aviso`). Lo usan `borrador`, `regenerar` y `releer` para
/// que el backend guarde la foto limpia del hilo, no el chrome del visor.
#[must_use]
pub fn normalizar_excerpt_hilo(thread_id: &str, texto: &str) -> String {
    normalizar_excerpt_con_hilo(
        texto,
        nombre_de_thread(thread_id).as_deref(),
        aviso_fb_de_thread(thread_id).as_deref(),
    )
}

/// [08AA-10] El piloto no trae `avisoId`, pero el título del hilo sí nombra
/// el aviso y el catálogo tiene la ficha con el precio real: se empareja
/// en el backend (fuente de verdad) en vez de fiarse del DOM. Sin
/// `regex` en el árbol: normalización manual (caja, tildes, ruido).
#[must_use]
pub fn normalizar_titulo(s: &str) -> String {
    let mut fuera = String::with_capacity(s.len());
    for c in s.to_lowercase().chars() {
        if c.is_alphanumeric() {
            fuera.push(quitar_tilde(c));
        } else if !fuera.ends_with(' ') {
            fuera.push(' ');
        }
    }
    fuera.trim().to_string()
}

/// Minúsculas ya aplicadas por quien llama.
fn quitar_tilde(c: char) -> char {
    match c {
        'á' | 'à' | 'ä' | 'â' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        _ => c,
    }
}

/// Palabras que no identifican un aviso (operación, tipología, zonas
/// grandes, conectores): dos hilos distintos las comparten.
const PALABRAS_GENERICAS_TITULO: &[&str] = &[
    "casa",
    "venta",
    "alquiler",
    "alquilo",
    "vendo",
    "vende",
    "apto",
    "apartamento",
    "terreno",
    "local",
    "galpon",
    "oficina",
    "town",
    "house",
    "townhouse",
    "quinta",
    "villa",
    "edificio",
    "piso",
    "anexo",
    "habitacion",
    "habitaciones",
    "urb",
    "urbanizacion",
    "residencia",
    "residencias",
    "conjunto",
    "sector",
    "zona",
    "centro",
    "norte",
    "sur",
    "este",
    "oeste",
    "puerto",
    "ordaz",
    "ciudad",
    "guayana",
    "bolivar",
    "san",
    "felix",
    "en",
    "de",
    "del",
    "la",
    "el",
    "los",
    "las",
    "con",
    "por",
    "para",
    "negociable",
];

/// Palabra sin valor identificativo: corta, cifra, precio (`vef0`,
/// `usd120000`) o genérica del negocio.
fn es_generica(palabra: &str) -> bool {
    palabra.len() <= 2
        || palabra.chars().all(|c| c.is_ascii_digit())
        || palabra.starts_with("vef")
        || palabra.starts_with("usd")
        || palabra.starts_with('$')
        || PALABRAS_GENERICAS_TITULO.contains(&palabra)
}

/// `(directo, solape, distintivo)`: `directo` si un título normalizado
/// contiene al otro (títulos de ≥12 caracteres: un "apto" suelto no vale);
/// si no, conteo de palabras compartidas y cuántas son distintivas.
#[must_use]
pub fn puntaje_titulo(fb: &str, titulo: &str) -> (bool, usize, usize) {
    let fb_n = normalizar_titulo(fb);
    let titulo_n = normalizar_titulo(titulo);
    if fb_n.is_empty() || titulo_n.is_empty() {
        return (false, 0, 0);
    }
    let directo = (fb_n.contains(&titulo_n) || titulo_n.contains(&fb_n))
        && fb_n.len() >= 12
        && titulo_n.len() >= 12;
    let en_fb: std::collections::HashSet<&str> = fb_n.split(' ').collect();
    let mut solape = 0;
    let mut distintivo = 0;
    for p in titulo_n.split(' ') {
        if p.len() > 2 && en_fb.contains(p) {
            solape += 1;
            if !es_generica(p) {
                distintivo += 1;
            }
        }
    }
    (directo, solape, distintivo)
}

/// Ficha publicada cuyo título mejor empareja con el del hilo: directo, o
/// solape ≥3 con ≥1 palabra distintiva. Empate entre dos avisos o BD
/// caída = `None` (nunca se cita un precio dudoso; quien llama decide si
/// lo registra: el borrador jamás se bloquea por esto).
pub async fn ficha_por_titulo(
    pool: &sqlx::PgPool,
    titulo_fb: &str,
) -> Result<Option<InmuebleRow>, AppError> {
    let candidatos = InmuebleRepository::titulos_publicados(pool).await?;
    let mut mejor: Option<(uuid::Uuid, usize, usize)> = None;
    let mut empate = false;
    for (id, titulo) in &candidatos {
        let (directo, solape, distintivo) = puntaje_titulo(titulo_fb, titulo);
        if !(directo || (solape >= 3 && distintivo >= 1)) {
            continue;
        }
        let clave = (distintivo, solape);
        match mejor {
            Some((_, md, ms)) if (md, ms) == clave => empate = true,
            Some((_, md, ms)) if (md, ms) > clave => {}
            _ => {
                mejor = Some((*id, distintivo, solape));
                empate = false;
            }
        }
    }
    match (mejor, empate) {
        (Some((id, _, _)), false) => Ok(InmuebleRepository::find_by_id(pool, id).await?),
        _ => Ok(None),
    }
}

/// [07AA-8] Últimos borradores del hilo (máx 3, recientes primero): contexto
/// "ya dicho" para que la IA avance la conversación en vez de repetir.
/// [08AA-18] Lee con `clave_hilo()`: la cifra inyectada por el puente
/// (07AA-11) parpadea entre llamadas y el `thread_id` literal no empareja.
pub async fn hilo_previo(pool: &sqlx::PgPool, thread: &str) -> Result<Vec<String>, AppError> {
    let filas: Vec<String> = sqlx::query_scalar(
        "SELECT respuesta FROM mp_respuestas_cache \
         WHERE thread_id = $1 ORDER BY valida_hasta DESC LIMIT 3",
    )
    .bind(clave_hilo(thread))
    .fetch_all(pool)
    .await?;
    Ok(filas)
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

/* [08AA-20] Vida del token CLI configurable: `MP_CLI_MINUTOS` manda
 * (local: 43200 = 30d en `.env`, gitignored); ausente/inválido → 480
 * (8h, lo que sigue viendo producción). El panel queda fijo en 15min.
 * Por qué env y no quitar la expiración: `jti`+revocación y binding a
 * máquina siguen valiendo; solo se estira el `exp`. */
#[must_use]
pub fn minutos_para_cli(es_cli: bool) -> i64 {
    if !es_cli {
        return 15;
    }
    std::env::var("MP_CLI_MINUTOS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|&m| m > 0)
        .unwrap_or(480)
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

/// [07AA-7] Panel por chat: un chat = un `thread_id` (= clave de ventana
/// del puente, trae nombre+aviso: PII solo-admin por decisión de ella
/// 2026-10-07, misma retención 90d + purga).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatResumen {
    pub thread_id: String,
    pub borradores: i64,
    pub usos: i64,
    pub corregidas: i64,
    pub ultimo: String,
}

/// [07AA-7] Una fila del chat: foto de la conversación + texto guardado.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatFila {
    pub excerpt_texto: String,
    pub respuesta: String,
    pub usos: i64,
    pub corregida: bool,
    pub valida_hasta: String,
}

/// Chats con borradores, ordenados por el más reciente. Una sola consulta.
pub async fn resumen_chats(pool: &sqlx::PgPool) -> Result<Vec<ChatResumen>, AppError> {
    let filas: Vec<(String, i64, i64, i64, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT thread_id, COUNT(*)::BIGINT, COALESCE(SUM(usos), 0)::BIGINT, \
         SUM(CASE WHEN corregida THEN 1 ELSE 0 END)::BIGINT, MAX(valida_hasta) \
         FROM mp_respuestas_cache GROUP BY thread_id ORDER BY MAX(valida_hasta) DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(filas
        .into_iter()
        .map(
            |(thread_id, borradores, usos, corregidas, ultimo)| ChatResumen {
                thread_id,
                borradores,
                usos,
                corregidas,
                ultimo: ultimo.to_rfc3339(),
            },
        )
        .collect())
}

/// Filas de un chat (tope 200, recientes primero).
/// [08AA-18] Busca con `clave_hilo()` (ver `hilo_previo`).
pub async fn detalle_chat(pool: &sqlx::PgPool, thread: &str) -> Result<Vec<ChatFila>, AppError> {
    let filas: Vec<(String, String, i64, bool, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT excerpt_texto, respuesta, usos::BIGINT, corregida, valida_hasta \
         FROM mp_respuestas_cache WHERE thread_id = $1 \
         ORDER BY valida_hasta DESC LIMIT 200",
    )
    .bind(clave_hilo(thread))
    .fetch_all(pool)
    .await?;
    Ok(filas
        .into_iter()
        .map(
            |(excerpt_texto, respuesta, usos, corregida, valida_hasta)| ChatFila {
                excerpt_texto,
                respuesta,
                usos,
                corregida,
                valida_hasta: valida_hasta.to_rfc3339(),
            },
        )
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

/// [08AA-21] Foto del hilo que acompaña a cada fila de caché: clave de la
/// ventana + excerpt limpio (panel) + excerpt crudo tal como llegó del
/// puente (diagnóstico del filtro, 08AA-8). Viaja junta para no engordar
/// la firma de `guardar_cache`/`reemplazar_cache` (clippy: máx 7 args).
pub struct FotoHilo<'a> {
    pub thread_id: &'a str,
    pub excerpt: &'a str,
    pub excerpt_crudo: &'a str,
}

/// Guarda una generación fresca; si la dueña ya corrigió esa clave, su texto
/// gana (`DO NOTHING`: la corrección humana no se pisa en silencio).
/// [07AA-7] Anota `thread_id` + `excerpt_texto` para el panel por chat.
/// [08AA-18] Guarda con `clave_hilo()`: la cifra inyectada por el puente
/// (07AA-11) parpadea entre llamadas y el `thread_id` literal no empareja
/// al releer.
/// [08AA-21] Guarda también `excerpt_crudo`: el texto tal como llegó del
/// puente, antes de `normalizar_excerpt`. El filtro por líneas no se puede
/// calibrar a ciegas (el puente aplana el DOM y lo pegado no se ve);
/// con el crudo a la vista se corrige el filtro (08AA-8).
pub async fn guardar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
    respuesta: &str,
    foto: &FotoHilo<'_>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, thread_id, excerpt_texto, excerpt_crudo) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .bind(respuesta)
    .bind(clave_hilo(foto.thread_id))
    .bind(foto.excerpt)
    .bind(foto.excerpt_crudo)
    .execute(pool)
    .await?;
    Ok(())
}

/// Pisa la fila (Regenerar explícito de la dueña): texto nuevo, vigencia
/// renovada, `corregida=FALSE`, contador a cero (nueva versión).
/// [07AA-7] Refresca también `thread_id` + `excerpt_texto` (foto actual).
/// [08AA-18] Guarda con `clave_hilo()` (ver `guardar_cache`).
/// [08AA-21] Refresca también `excerpt_crudo` (ver `guardar_cache`).
pub async fn reemplazar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
    respuesta: &str,
    foto: &FotoHilo<'_>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, thread_id, excerpt_texto, excerpt_crudo) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (firma, precio_hash, catalog_hash) DO UPDATE SET \
         respuesta = EXCLUDED.respuesta, valida_hasta = now() + INTERVAL '90 days', \
         corregida = FALSE, usos = 0, thread_id = EXCLUDED.thread_id, \
         excerpt_texto = EXCLUDED.excerpt_texto, excerpt_crudo = EXCLUDED.excerpt_crudo",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .bind(respuesta)
    .bind(clave_hilo(foto.thread_id))
    .bind(foto.excerpt)
    .bind(foto.excerpt_crudo)
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

/// Guarda la corrección de la dueña (su texto manda tal cual: lo revisa
/// ella a mano al enviar). Vigencia renovada; la fila misma es el registro
/// (sin audit separada: `corregida=TRUE` + `usos` ya lo cuentan).
/// [08AA-14] Sin matriz negativa por decisión de ella 2026-10-08.
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
        assert_eq!(precio_publico(1_250_000.0), "$1.250.000");
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
    fn nombre_de_thread_saluda_por_nombre() {
        assert_eq!(
            nombre_de_thread("alejandro|casa en venta en riberas"),
            Some("Alejandro".to_string())
        );
        assert_eq!(
            nombre_de_thread("jean carlos|apto amoblado"),
            Some("Jean Carlos".to_string())
        );
        assert_eq!(nombre_de_thread("sin-hilo"), None);
        assert_eq!(nombre_de_thread("solo-sin-barra"), None);
        assert_eq!(nombre_de_thread("|aviso sin nombre"), None);
    }

    #[test]
    fn normalizar_excerpt_quita_ruido_y_duplicados_conservando_roles() {
        /* Literales del HTML real de ella (hilo Riberas del Caroní,
         * `Agente/documentacion/usuario/conversacion-html-facebook.md`):
         * cada mensaje sale dos veces (visible + aria-label) y FB inyecta
         * inicio de chat, tip de seguridad y chrome. */
        let crudo = "Casa en venta en Riberas del Caroní, Puerto Ordaz\n\
            Jorge inició este chat.\n\
            Cliente: Hola. ¿Sigue estando disponible?\n\
            Cliente: Hola. ¿Sigue estando disponible?\n\
            Cliente: Si te vas a reunir con alguien en persona, cuéntales a familiares y amigos adónde vas.\n\
            Dueña: Sí, sigue disponible en $43.000 negociable.\n\
            View buyer\n\
            Sí. ¿Te interesa?\n\
            Cliente: Precio..??";
        assert_eq!(
            normalizar_excerpt(crudo),
            "Casa en venta en Riberas del Caroní, Puerto Ordaz\n\
            Cliente: Hola. ¿Sigue estando disponible?\n\
            Dueña: Sí, sigue disponible en $43.000 negociable.\n\
            Cliente: Precio..??"
        );
    }

    #[test]
    fn normalizar_excerpt_conserva_rapida_si_la_escribe_el_cliente() {
        let crudo = "Cliente: Sí. ¿Te interesa?\nDueña: Sí, dime qué buscas.";
        assert_eq!(normalizar_excerpt(crudo), crudo);
    }

    #[test]
    fn normalizar_excerpt_vacio_si_todo_es_ruido() {
        assert!(normalizar_excerpt("View buyer\nMore options\nAa").is_empty());
        assert!(normalizar_excerpt("   \n  ").is_empty());
    }

    #[test]
    fn normalizar_hilo_kerley_deja_solo_la_pregunta() {
        /* [08AA-16] Testigo exacto en BD
         * (`kerley|VEF0 apartamento residencias rio aro plaza puerto
         * ordaz`): el visor repite cabeceras (eco del título recortado,
         * `Mensajes`, `Kerley · Apartamento ...`, `Kerley` suelto) e
         * inyecta la instrucción + las dos sugeridas ES. Solo la
         * pregunta del cliente sobrevive. */
        let hilo = "kerley|VEF0 apartamento residencias rio aro plaza puerto ordaz";
        let crudo = "amento Residencias Rio Aro Plaza Puerto Ordaz\n\
            Mensajes\n\
            Kerley · Apartamento Residencias Rio Aro Plaza Puerto Ordaz\n\
            Kerley\n\
            ¿Sigue disponible?\n\
            Toca una respuesta para enviársela al comprador.\n\
            Lo estoy mirando. Te avisaré.\n\
            Lo siento, no está disponible.";
        assert_eq!(normalizar_excerpt_hilo(hilo, crudo), "¿Sigue disponible?");
    }

    #[test]
    fn normalizar_hilo_tina_limpia_cola_y_marcas() {
        /* [08AA-16] Testigo exacto en BD (`tina|VEF0 casa en venta en
         * riberas del caroní, puerto ordaz`): el float cortó a mitad de
         * palabra (`ponible?`), el visor mete hora (`2:43 am`) y marca
         * de enviado. El resto (aunque sea mensaje propio sin marca)
         * se conserva como contexto. */
        let hilo = "tina|VEF0 casa en venta en riberas del caroní, puerto ordaz";
        let crudo = "ponible?\n\
            2:43 am\n\
            Hola, disponible.\n\
            $43.000 negociable\n\
            04249208855\n\
            Enviado";
        assert_eq!(
            normalizar_excerpt_hilo(hilo, crudo),
            "Hola, disponible.\n$43.000 negociable\n04249208855"
        );
    }

    #[test]
    fn normalizar_hilo_conserva_mensaje_corto_con_mayuscula() {
        /* La cola truncada no se come saludos completos: empiezan en
         * mayúscula aunque vayan en primera línea y sin espacios. */
        let hilo = "tina|VEF0 casa en venta en riberas del caroní, puerto ordaz";
        assert_eq!(normalizar_excerpt_hilo(hilo, "Hola"), "Hola");
        assert_eq!(normalizar_excerpt_hilo(hilo, "Sí"), "Sí");
    }

    #[test]
    fn normalizar_hilo_tina_cargando_devuelve_vacio() {
        /* [08AA-17] Reporte de ella 2026-10-08 (panel, hilo Tina): el
         * hilo aún cargaba (`Cargando...`) y el visor repetía cabeceras
         * (`Tina · Casa ...`, `Marketplace`, `VEF0 - Casa ...` con guion,
         * `Escribe en Tina · ...`) sin ningún mensaje real. Todo es
         * ruido → vacío (el handler conserva el original en ese caso). */
        let hilo = "tina|VEF0 casa en venta en riberas del caroní, puerto ordaz";
        let crudo = "Tina · Casa en venta en Riberas del Caroní, Puerto Ordaz\n\
            Marketplace\n\
            VEF0 - Casa en venta en Riberas del Caroní, Puerto Ordaz\n\
            View buyer\n\
            More options\n\
            Mensajes\n\
            Cargando...\n\
            Escribir mensaje\n\
            Escribe en Tina · Casa en venta en Riberas del Caroní, Puerto Ordaz\n\
            Aa";
        assert!(normalizar_excerpt_hilo(hilo, crudo).is_empty());
    }

    #[test]
    fn clave_hilo_deshace_precio_inyectado_y_respeta_lo_demas() {
        /* [08AA-18] El puente (07AA-11) manda `tina|$43.000 vef0...` pero
         * la cifra parpadea entre llamadas: la BD solo ve la forma
         * canónica para que guardar y buscar emparejen siempre. */
        assert_eq!(
            clave_hilo("tina|$43.000 vef0 casa en venta"),
            "tina|vef0 casa en venta"
        );
        assert_eq!(
            clave_hilo("tina|US$ 43.000 vef0 casa en venta"),
            "tina|vef0 casa en venta"
        );
        assert_eq!(clave_hilo("  tina|$43.000 vef0 casa  "), "tina|vef0 casa");
        assert_eq!(
            clave_hilo("tina|vef0 casa en venta"),
            "tina|vef0 casa en venta"
        );
        assert_eq!(clave_hilo("sin-hilo"), "sin-hilo");
        assert_eq!(clave_hilo(""), "");
        /* Sin dígitos no es cifra; sin resto no hay aviso; `$` en otro
         * sitio no es inyección: todo queda intacto. */
        assert_eq!(clave_hilo("ana|$negociable casa"), "ana|$negociable casa");
        assert_eq!(clave_hilo("ana|$50"), "ana|$50");
        assert_eq!(clave_hilo("ana|casa $50 mil"), "ana|casa $50 mil");
    }

    #[test]
    fn precio_del_aviso_extrae_moneda_antes_o_despues() {
        assert_eq!(
            precio_del_aviso("town house en venta en las peonías 125.000$"),
            Some("125.000$".to_string())
        );
        assert_eq!(
            precio_del_aviso("Casa $95.000 en Riberas"),
            Some("$ 95.000".to_string())
        );
        assert_eq!(
            precio_del_aviso("APTO USD 120.000"),
            Some("usd 120.000".to_string())
        );
        assert_eq!(precio_del_aviso("casa en venta, 3 habitaciones"), None);
        assert_eq!(precio_del_aviso("piso 2, año 2024"), None);
    }

    #[test]
    fn asegurar_contacto_agrega_lo_que_falta_y_respeta_lo_presente() {
        let sin_nada = asegurar_contacto("Casa en Riberas.\nSí, aceptamos visita.");
        assert!(sin_nada.contains(CONTACTO_TEL));
        assert!(sin_nada.ends_with(CONTACTO_WA));
        let completo = asegurar_contacto(&format!(
            "Casa.\nCualquier cosa escríbeme al {CONTACTO_TEL}.\n{CONTACTO_WA}"
        ));
        assert_eq!(completo.matches(CONTACTO_TEL).count(), 1);
        assert_eq!(completo.matches(CONTACTO_WA).count(), 1);
    }

    #[test]
    fn aviso_fb_sale_del_hilo() {
        assert_eq!(
            aviso_fb_de_thread("javier|casa en venta en riberas del caroní, p..."),
            Some("casa en venta en riberas del caroní, p...".to_string())
        );
        assert_eq!(aviso_fb_de_thread("sin-hilo"), None);
        assert_eq!(aviso_fb_de_thread("solo|"), None);
    }

    #[test]
    fn titulo_normaliza_tildes_caja_y_ruido() {
        assert_eq!(
            normalizar_titulo("VEF0 Casa en venta en Riberas del Caroní, Puerto Ordaz"),
            "vef0 casa en venta en riberas del caroni puerto ordaz"
        );
        assert_eq!(normalizar_titulo("  "), "");
    }

    #[test]
    fn puntaje_titulo_directo_con_prefijo_de_precio() {
        /* Caso andreina 08AA-10: el título trae `VEF0` (precio 0 en
         * Facebook) y aun así empareja con la ficha del catálogo. */
        let (directo, _, _) = puntaje_titulo(
            "VEF0 casa en venta en riberas del caroní, puerto ordaz",
            "Casa en venta en Riberas del Caroní",
        );
        assert!(directo);
    }

    #[test]
    fn puntaje_titulo_no_confunde_avisos_genericos() {
        /* Mismo negocio, distinta zona: sin palabra distintiva no hay
         * emparejamiento (un precio ajeno es peor que el dodge). */
        let (directo, solape, distintivo) = puntaje_titulo(
            "casa en venta en arivana",
            "Casa en venta en Riberas del Caroní",
        );
        assert!(!directo);
        assert!(solape < 3 || distintivo < 1);
        /* Título suelto de 1 palabra jamás es directo. */
        assert_eq!(puntaje_titulo("apto precioso apTO", "apto").0, false);
    }

    #[test]
    fn formatear_parrafos_une_saltos_sueltos_y_separa_bloques() {
        let entrado = "Hola, Andreina, buenas noches.\nTe escribo por la casa.\nSí, sigue disponible.\nCuéntame qué estás buscando y con gusto te ayudo, cualquier cosa escríbeme al 0424 9208855 https://wa.me/584249208855";
        let salido = formatear_parrafos(&entrado);
        assert_eq!(
            salido,
            "Hola, Andreina, buenas noches. Te escribo por la casa. Sí, sigue disponible. Cuéntame qué estás buscando y con gusto te ayudo, cualquier cosa escríbeme al 0424 9208855\n\nhttps://wa.me/584249208855"
        );
    }

    #[test]
    fn formatear_parrafos_respeta_lista_y_no_duplica() {
        let entrado = "Tiene:\n1. Piscina\n2. Planta eléctrica\n\nhttps://wa.me/584249208855";
        let salido = formatear_parrafos(&entrado);
        assert!(salido.contains("Tiene:\n\n1. Piscina\n\n2. Planta eléctrica"));
        assert_eq!(salido.matches(CONTACTO_WA).count(), 1);
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

    /* [08AA-20] Por defecto CLI 8h/panel 15min; `MP_CLI_MINUTOS`
     * manda cuando es entero positivo; lo inválido cae al default. */
    #[test]
    fn cli_vive_8h_y_panel_15min() {
        std::env::remove_var("MP_CLI_MINUTOS");
        assert_eq!(minutos_para_cli(true), 480);
        assert_eq!(minutos_para_cli(false), 15);
        std::env::set_var("MP_CLI_MINUTOS", "43200");
        assert_eq!(minutos_para_cli(true), 43200);
        assert_eq!(minutos_para_cli(false), 15);
        std::env::set_var("MP_CLI_MINUTOS", "basura");
        assert_eq!(minutos_para_cli(true), 480);
        std::env::remove_var("MP_CLI_MINUTOS");
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

    /* [08AA-21] Atajo para la foto del hilo en pruebas de caché. */
    fn foto_prueba(
        hilo: &'static str,
        limpio: &'static str,
        crudo: &'static str,
    ) -> FotoHilo<'static> {
        FotoHilo {
            thread_id: hilo,
            excerpt: limpio,
            excerpt_crudo: crudo,
        }
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
        guardar_cache(
            &pool,
            &firma,
            &ph,
            &ch,
            "texto-ia",
            &foto_prueba("hilo-1", "Dueña: hola", "CRUDO Dueña: hola"),
        )
        .await
        .expect("guarda");
        let hit = buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca");
        assert_eq!(hit, Some(("texto-ia".to_string(), false)));
        /* [07AA-7] El panel agrupa por chat: hilo + foto guardados.
         * [08AA-21] El crudo viaja junto al limpio. */
        let hilo: (String, String, Option<String>) = sqlx::query_as(
            "SELECT thread_id, excerpt_texto, excerpt_crudo FROM mp_respuestas_cache \
             WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3",
        )
        .bind(&firma)
        .bind(&ph)
        .bind(&ch)
        .fetch_one(&pool)
        .await
        .expect("lee hilo");
        assert_eq!(
            hilo,
            (
                "hilo-1".to_string(),
                "Dueña: hola".to_string(),
                Some("CRUDO Dueña: hola".to_string())
            )
        );
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
        guardar_cache(
            &pool,
            &firma,
            &ph,
            &ch,
            "texto-ia",
            &foto_prueba("hilo-1", "x", "crudo-x"),
        )
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
        guardar_cache(
            &pool,
            &firma,
            &ph,
            &ch,
            "viejo",
            &foto_prueba("hilo-1", "x", "crudo-x"),
        )
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
        guardar_cache(
            &pool,
            &firma,
            &ph,
            &ch,
            "texto-ia",
            &foto_prueba("hilo-1", "x", "crudo-x"),
        )
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
        guardar_cache(
            &pool,
            &firma,
            &ph,
            &ch,
            "texto-ia-2",
            &foto_prueba("hilo-1", "x", "crudo-x"),
        )
        .await
        .expect("guarda x2");
        assert_eq!(
            buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca"),
            Some(("texto de la dueña".to_string(), true))
        );
        /* Regenerar explícito sí pisa y resetea versión. */
        reemplazar_cache(
            &pool,
            &firma,
            &ph,
            &ch,
            "nueva-ia",
            &foto_prueba(
                "hilo-2",
                "Dueña: sigue disponible?",
                "CRUDO Dueña: sigue disponible?",
            ),
        )
        .await
        .expect("reemplaza");
        assert_eq!(
            buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca"),
            Some(("nueva-ia".to_string(), false))
        );
        /* [07AA-7] Regenerar refresca la foto del chat (+ crudo [08AA-21]). */
        let hilo2: (String, String, Option<String>) = sqlx::query_as(
            "SELECT thread_id, excerpt_texto, excerpt_crudo FROM mp_respuestas_cache \
             WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3",
        )
        .bind(&firma)
        .bind(&ph)
        .bind(&ch)
        .fetch_one(&pool)
        .await
        .expect("lee hilo");
        assert_eq!(
            hilo2,
            (
                "hilo-2".to_string(),
                "Dueña: sigue disponible?".to_string(),
                Some("CRUDO Dueña: sigue disponible?".to_string())
            )
        );
        borrar_cache(&pool, &firma, &ph, &ch).await.expect("limpia");
    }

    #[tokio::test]
    async fn corregir_rechaza_vacio_y_acepta_contacto() {
        /* [08AA-14] Sin matriz negativa por decisión de ella: el texto de
         * la dueña (incluido su contacto) pasa tal cual; solo el vacío
         * se rechaza. (Antes este test exigía rechazar el contacto.) */
        let Some(pool) = pool_si_hay() else { return };
        let (firma, ph, ch) = (clave_azar(), clave_azar(), clave_azar());
        assert!(corregir_cache(&pool, &firma, &ph, &ch, "").await.is_err());
        assert!(
            corregir_cache(&pool, &firma, &ph, &ch, "llámame al 0412 1234567")
                .await
                .is_ok()
        );
        assert_eq!(
            buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca"),
            Some(("llámame al 0412 1234567".to_string(), true))
        );
        borrar_cache(&pool, &firma, &ph, &ch).await.expect("limpia");
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
