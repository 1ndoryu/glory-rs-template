//! [08AA-8] Texto puro del asistente Marketplace (sin HTTP ni BD).
//!
//! Extraído de `marketplace.rs` (límite 700): schema M3 v1
//! (`BorradorRequest` y tipos), validación (`validar_borrador`), limpieza
//! del excerpt (`normalizar_excerpt`), precio del aviso (`precio_del_aviso`)
//! y formato de precio (`precio_publico`). `marketplace.rs` conserva strip de ficha, contacto,
//! párrafos, títulos, caché y tokens, y re-exporta estos nombres para no
//! mover sus usos externos (handlers, utoipa, sombra).

use chrono::DateTime;
use serde::Deserialize;
use utoipa::ToSchema;

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

/* `pub(super)`: lo usan los tests de `marketplace` vía `super::*`
 * (el `use` en el padre va con `cfg(test)` para no romper clippy). */
pub(super) fn es_hex64(s: &str) -> bool {
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
    DateTime::parse_from_rfc3339(hora).is_ok_and(|f| f.offset().local_minus_utc() == -4 * 3600)
}

/// [08AA-5] Limpieza del excerpt del puente antes de guardarlo y de pasarlo
/// a la IA. El DOM de Messenger repite cada mensaje en dos nodos (texto
/// visible + `aria-label`: por eso la conversación salía dos veces) e
/// inyecta ruido: tips de seguridad, aviso de Meta, `X inició este chat`,
/// chrome (`View buyer`, `More options`), composer y respuestas rápidas.
/// Literales calibrados con el HTML real de ella
/// (`Agente/documentacion/usuario/conversacion-html-facebook.md`, hilo
/// Riberas del Caroní). Se conserva el orden y las marcas
/// `Cliente:`/`Dueña:` que el prompt necesita; si solo había ruido se
/// devuelve vacío y el handler conserva el original (nunca se guarda vacío).
/// Sin contexto del hilo equivale a `normalizar_excerpt_con_hilo` con
/// `(None, None)`; los handlers pasan nombre y aviso del `thread_id`.
#[must_use]
pub fn normalizar_excerpt(texto: &str) -> String {
    normalizar_excerpt_con_hilo(texto, None, None)
}

/// [08AA-16] Variante con contexto del hilo: `nombre` (comprador, de
/// `nombre_de_thread`) filtra las cabeceras que el visor repite (`Kerley`,
/// `Kerley · Apartamento ...`); `aviso` (título FB del hilo) filtra el eco
/// del título (a veces recortado por la izquierda por el corte del float:
/// `amento Residencias Rio Aro ...`). Además quita marcas de tiempo
/// (`2:43 am`) y la cola truncada de la primera línea (`ponible?` de
/// `¿Sigue disponible?`: el float corta por carácter, no por línea).
#[must_use]
pub fn normalizar_excerpt_con_hilo(
    texto: &str,
    nombre: Option<&str>,
    aviso: Option<&str>,
) -> String {
    let mut fuera: Vec<&str> = Vec::new();
    let mut primera = true;
    for linea in texto.lines() {
        let t = linea.trim();
        let duplicada = fuera.last().is_some_and(|&u| u == t);
        if t.is_empty() || duplicada {
            continue;
        }
        if primera {
            primera = false;
            if es_cola_truncada(t) {
                continue;
            }
        }
        if es_ruido_excerpt(t) || es_marca_tiempo_fb(t) {
            continue;
        }
        if es_cabecera_hilo(t, nombre, aviso) {
            continue;
        }
        fuera.push(t);
    }
    fuera.join("\n")
}

/// [08AA-16] Cola de un mensaje cortado a mitad de palabra en la primera
/// línea (testigo: `ponible?`). Heurística estrecha: sin espacios, empieza
/// en minúscula, termina en `?`/`!` y ≤15 caracteres. Un mensaje completo
/// corto (`Hola`, `Sí`, `Gracias`) empieza en mayúscula y se conserva.
/// Fix canónico pendiente: que el float corte por línea, no por carácter.
fn es_cola_truncada(linea: &str) -> bool {
    let n = linea.chars().count();
    n > 0
        && n <= 15
        && !linea.contains(' ')
        && linea.starts_with(|c: char| c.is_lowercase())
        && (linea.ends_with('?') || linea.ends_with('!'))
}

/// [08AA-16] Marcas de tiempo del visor (`2:43 am`, `11:30 pm`): separan
/// mensajes, no son contenido. Formato exacto `H:MM am|pm` con hora 1-12
/// y minutos 00-59 (sin `regex` en el árbol: escaneo manual como
/// `precio_del_aviso`).
fn es_marca_tiempo_fb(linea: &str) -> bool {
    let t = linea.trim().to_lowercase();
    let hora = t
        .strip_suffix("am")
        .or_else(|| t.strip_suffix("pm"))
        .map_or("", str::trim_end);
    let mut partes = hora.split(':');
    match (partes.next(), partes.next(), partes.next()) {
        (Some(h), Some(m), None) => {
            h.len() <= 2
                && m.len() == 2
                && h.bytes().all(|b| b.is_ascii_digit())
                && m.bytes().all(|b| b.is_ascii_digit())
                && h.parse::<u32>().is_ok_and(|h| (1..=12).contains(&h))
                && m.parse::<u32>().is_ok_and(|m| m <= 59)
        }
        _ => false,
    }
}

/// [08AA-16] Cabeceras del visor que repiten metadatos del hilo, nunca
/// contenido: el nombre del comprador solo (`Kerley`, insensible a
/// caja y tildes) o como prefijo con `·` (`Kerley · Apartamento ...`),
/// y el eco del título del aviso (`amento Residencias Rio Aro ...`).
/// El eco del título solo vale sin marca de rol: una línea atribuida
/// (`Cliente:`/`Dueña:`) es mensaje real aunque cite el título.
fn es_cabecera_hilo(linea: &str, nombre: Option<&str>, aviso: Option<&str>) -> bool {
    if let Some(n) = nombre.map(str::trim).filter(|n| !n.is_empty()) {
        let canon = sin_tilde_min(n);
        if sin_tilde_min(linea) == canon {
            return true;
        }
        let prefijo = format!("{n} · ");
        if linea
            .get(..prefijo.len())
            .is_some_and(|h| sin_tilde_min(h) == sin_tilde_min(&prefijo))
        {
            return true;
        }
    }
    if let Some(a) = aviso.map(str::trim).filter(|a| !a.is_empty()) {
        let cuerpo = cuerpo_sin_marca(linea);
        if cuerpo.len() == linea.len() && cuerpo.chars().count() >= 12 {
            let a_min = a.to_lowercase();
            if a_min.contains(&cuerpo.to_lowercase()) {
                return true;
            }
        }
    }
    false
}

/// Minúsculas sin tildes para comparar cabeceras (`Kerley`/`kerley`,
/// `Andréina`/`Andreina`): el `thread_id` viaja sin tildes y el visor
/// puede traerlas. Réplica local de `quitar_tilde` (privada de
/// `marketplace.rs`) para no cruzar módulos por una comparación.
fn sin_tilde_min(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            otro => otro,
        })
        .collect()
}

/// Prefijos literales de ruido de Facebook (ES + EN).
const RUIDO_EXCERPT_PREFIJOS: &[&str] = &[
    "Si te vas a reunir con alguien",
    "If you're meeting someone",
    "If you are meeting someone",
    "Meta podría usar tecnología",
    "Meta may use technology",
    "Escribe en ",
    "Write to ",
    "Presionar Enter",
    "Press Enter",
    "Mensaje enviado",
    "Message sent",
];

/// Líneas completas del chrome del visor (comparación exacta).
/// [08AA-16] +`Mensajes` (cabecera de la columna), la instrucción de las
/// respuestas rápidas y `Enviado` (marca de mensaje propio enviado).
const RUIDO_EXCERPT_EXACTO: &[&str] = &[
    "View buyer",
    "More options",
    "Ver perfil",
    "Mensajes",
    "Enviar mensaje",
    "Escribir mensaje",
    "Aa",
    "Ver más consejos de seguridad",
    "See more safety tips",
    "Toca una respuesta",
    "Toca una respuesta para enviársela al comprador.",
    "Tap a reply",
    "Envía una respuesta rápida",
    "Send a quick reply",
    "Enviado",
    "Visto",
    "Seen",
];

/// Respuestas rápidas sugeridas por Facebook: solo se filtran sin marca de
/// rol (el chip centrado no trae `Cliente:`/`Dueña:`). Si el cliente las
/// escribe de verdad, llevan marca y se conservan.
/// [08AA-16] +las dos sugeridas ES del hilo Kerley (testigo en BD).
const RESPUESTAS_RAPIDAS_FB: &[&str] = &[
    "Sí. ¿Te interesa?",
    "Sí. ¿Sigue disponible?",
    "¿Cuál es el precio?",
    "Lo estoy mirando. Te avisaré.",
    "Lo siento, no está disponible.",
    "Yes. Are you interested?",
    "Yes. Is this still available?",
    "What is the price?",
];

/// Cuerpo de la línea sin la marca de rol (`Cliente:`/`Dueña:`), si la trae.
fn cuerpo_sin_marca(linea: &str) -> &str {
    linea
        .strip_prefix("Cliente:")
        .or_else(|| linea.strip_prefix("Dueña:"))
        .map_or(linea, str::trim_start)
}

fn es_ruido_excerpt(linea: &str) -> bool {
    let cuerpo = cuerpo_sin_marca(linea);
    if cuerpo.contains("inició este chat") || cuerpo.contains("started this chat") {
        return true;
    }
    if RUIDO_EXCERPT_PREFIJOS.iter().any(|p| cuerpo.starts_with(p)) {
        return true;
    }
    if RUIDO_EXCERPT_EXACTO.contains(&cuerpo) {
        return true;
    }
    let sin_marca = cuerpo.len() == linea.len();
    sin_marca && RESPUESTAS_RAPIDAS_FB.contains(&cuerpo)
}

/// [07AA-9] Precio publicado en el título del aviso (`125.000$`, `$95.000`,
/// `USD 120.000`): en el piloto no hay ficha, pero el título de Facebook sí
/// trae el precio y la IA debe darlo directo en vez del fallback. Sin `regex`
/// en el árbol: escaneo manual, moneda antes o después del número.
#[must_use]
pub fn precio_del_aviso(aviso: &str) -> Option<String> {
    let lower = aviso.to_lowercase();
    let bytes = lower.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        /* Avance por bytes: jamás se trocea a mitad de un carácter
         * multibyte (p. ej. la `í` de "peonías"). */
        if !lower.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let marca_len = if bytes[i] == b'$' {
            1
        } else if lower[i..].starts_with("usd") || lower[i..].starts_with("vef") {
            3
        } else {
            i += 1;
            continue;
        };
        let fin_marca = i + marca_len;
        if let Some(n) =
            numero_cercano(&lower, i, true).or_else(|| numero_cercano(&lower, fin_marca, false))
        {
            let moneda = if marca_len == 1 {
                "$"
            } else {
                &lower[i..fin_marca]
            };
            return Some(if numero_antes(&lower, i) {
                format!("{n}{moneda}")
            } else {
                format!("{moneda} {n}")
            });
        }
        i = fin_marca;
    }
    None
}

/// Número pegado a la marca: hacia atrás (`hacia_atras`) o hacia adelante,
/// permitiendo espacios y separadores de miles. Mínimo 4 dígitos (evita
/// "casa 2" o pisos sueltos).
fn numero_cercano(texto: &str, pos: usize, hacia_atras: bool) -> Option<String> {
    let mut j = pos;
    let bytes = texto.as_bytes();
    if hacia_atras {
        while j > 0 && (bytes[j - 1] == b' ' || bytes[j - 1] == b'\t') {
            j -= 1;
        }
        let mut k = j;
        while k > 0
            && (bytes[k - 1].is_ascii_digit() || bytes[k - 1] == b'.' || bytes[k - 1] == b',')
        {
            k -= 1;
        }
        let num = texto[k..j].trim_matches(['.', ',']);
        numero_valido(num).then(|| num.to_string())
    } else {
        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }
        let mut k = j;
        while k < bytes.len() && (bytes[k].is_ascii_digit() || bytes[k] == b'.' || bytes[k] == b',')
        {
            k += 1;
        }
        let num = texto[j..k].trim_matches(['.', ',']);
        numero_valido(num).then(|| num.to_string())
    }
}

fn numero_valido(num: &str) -> bool {
    !num.is_empty()
        && num
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
        && num.chars().filter(char::is_ascii_digit).count() >= 4
}

/// ¿El número está a la izquierda de la marca (`125.000$`) o a la derecha
/// (`$ 125.000`)? Decide el orden del literal devuelto.
fn numero_antes(texto: &str, pos_marca: usize) -> bool {
    numero_cercano(texto, pos_marca, true).is_some()
}
