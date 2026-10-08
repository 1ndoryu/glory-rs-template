//! [08AA-8] Texto puro del asistente Marketplace (sin HTTP ni BD).
//!
//! Extraído de `marketplace.rs` (límite 700): schema M3 v1
//! (`BorradorRequest` y tipos), validación (`validar_borrador`), limpieza
//! del excerpt (`normalizar_excerpt`), matriz negativa (`matriz_negativa`),
//! precio del aviso (`precio_del_aviso`) y formato de precio
//! (`precio_publico`). `marketplace.rs` conserva strip de ficha, contacto,
//! párrafos, títulos, caché y tokens, y re-exporta estos nombres para no
//! mover sus usos externos (handlers, utoipa, sombra).

use chrono::DateTime;
use serde::Deserialize;
use utoipa::ToSchema;

use super::marketplace::{CONTACTO_TEL, CONTACTO_WA};

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
#[must_use]
pub fn normalizar_excerpt(texto: &str) -> String {
    let mut fuera: Vec<&str> = Vec::new();
    for linea in texto.lines() {
        let t = linea.trim();
        let duplicada = fuera.last().is_some_and(|&u| u == t);
        if !t.is_empty() && !es_ruido_excerpt(t) && !duplicada {
            fuera.push(t);
        }
    }
    fuera.join("\n")
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
const RUIDO_EXCERPT_EXACTO: &[&str] = &[
    "View buyer",
    "More options",
    "Ver perfil",
    "Enviar mensaje",
    "Escribir mensaje",
    "Aa",
    "Ver más consejos de seguridad",
    "See more safety tips",
    "Toca una respuesta",
    "Tap a reply",
    "Envía una respuesta rápida",
    "Send a quick reply",
    "Visto",
    "Seen",
];

/// Respuestas rápidas sugeridas por Facebook: solo se filtran sin marca de
/// rol (el chip centrado no trae `Cliente:`/`Dueña:`). Si el cliente las
/// escribe de verdad, llevan marca y se conservan.
const RESPUESTAS_RAPIDAS_FB: &[&str] = &[
    "Sí. ¿Te interesa?",
    "Sí. ¿Sigue disponible?",
    "¿Cuál es el precio?",
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

/// Matriz negativa v2 sobre el borrador generado: teléfono (7+ dígitos),
/// email o URL → la IA no entrega contacto salvo el fijo de [07AA-8]
/// (`CONTACTO_TEL` + `CONTACTO_WA` literales; lo demás sigue bloqueado).
/// Con `precio` conocido (v2, [07AA-9]): el literal citado y sus dígitos no
/// cuentan — si no, "125.000$" + "3 habitaciones" sumaría 7 y caería como
/// "teléfono". Compromiso: un teléfono alucinado que contenga los dígitos
/// exactos del precio pasaría; el resto sigue bloqueado.
/// Devuelve el motivo o `None` si pasa.
#[must_use]
pub fn matriz_negativa(texto: &str) -> Option<&'static str> {
    matriz_negativa_con_precio(texto, None)
}

/// Variante con precio del aviso eximido (ver `matriz_negativa`).
#[must_use]
pub fn matriz_negativa_con_precio(texto: &str, precio: Option<&str>) -> Option<&'static str> {
    let min = texto.to_lowercase();
    let mut limpio = min
        .replace(&CONTACTO_WA.to_lowercase(), "")
        .replace(&CONTACTO_TEL.to_lowercase(), "");
    let mut digitos_conocidos = String::new();
    if let Some(p) = precio {
        let pl = p.to_lowercase();
        if limpio.contains(&pl) {
            limpio = limpio.replace(&pl, "");
        }
        digitos_conocidos = pl.chars().filter(char::is_ascii_digit).collect();
    }
    if limpio.contains('@') {
        return Some("email");
    }
    if limpio.contains("http") || limpio.contains("wa.me") || limpio.contains("www.") {
        return Some("url");
    }
    let mut digitos: String = limpio.chars().filter(char::is_ascii_digit).collect();
    if !digitos_conocidos.is_empty() {
        digitos = digitos.replace(&digitos_conocidos, "");
    }
    if digitos.len() >= 7 {
        return Some("telefono");
    }
    None
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
