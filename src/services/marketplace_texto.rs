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
    /* [08AA-8] El puente aplana el DOM a texto pegado sin saltos (testigo
     * wilmery 554: una sola línea): el filtro por líneas no veía nada.
     * Se segmenta antes de filtrar (un `\n` antes de cada prefijo conocido
     * y aislando las líneas exactas largas + rápidas) y se pela la
     * atribución `Mensaje enviado ... por [:] Nombre` pegada al mensaje
     * (con el `nombre` del hilo se separa `WilmeryHola.` sin separador). */
    let troceado = segmentar_pegado(texto);
    let mut fuera: Vec<String> = Vec::new();
    let mut primera = true;
    let mut marca_pendiente: Option<&str> = None;
    for linea in troceado.lines() {
        let t = linea
            .trim()
            .trim_start_matches([',', '.', ':', ';', '·'])
            .trim();
        /* La segmentación puede dejar la marca de rol colgada
         * (`Cliente:` + rápida en la línea siguiente): se reata al
         * segmento siguiente para que el filtro la vea con su marca y la
         * conserve como mensaje real. */
        if t == "Cliente:" || t == "Dueña:" {
            marca_pendiente = Some(t);
            continue;
        }
        let compuesto;
        let mut t = t;
        if let Some(marca) = marca_pendiente.take() {
            compuesto = format!("{marca} {t}");
            t = compuesto.as_str();
        }
        if primera {
            primera = false;
            /* [08AA-24] El `slice(-1200)` del float corta por carácter y el
             * crudo puede empezar a mitad de `Presionar Enter,` (testigo
             * cristo: `sionar Enter,`): se pela el fragmento ANTES de
             * quitar el prefijo, porque el pelado expone el `Mensaje
             * enviado ...` que hay que pelar después. */
            t = pelar_enter_truncado(t);
            if es_cola_truncada(t) {
                continue;
            }
        }
        let t = quitar_prefijo_enviado(t, nombre).trim();
        /* [08AA-24] El pelado (cola truncada, `sionar Enter,`) puede dejar
         * la línea vacía: no es mensaje, se salta antes del split. */
        if t.is_empty() {
            continue;
        }
        let duplicada = fuera.last().is_some_and(|u| u == t);
        if duplicada {
            continue;
        }
        if es_ruido_excerpt(t) || es_marca_tiempo_fb(t) {
            continue;
        }
        if es_cabecera_hilo(t, nombre, aviso) {
            continue;
        }
        fuera.push(t.to_string());
    }
    fuera.join("\n")
}

/// [08AA-8] Parte el texto pegado del puente para que el filtro por líneas
/// lo vea: `\n` antes de cada prefijo de ruido (conserva la atribución
/// `Mensaje enviado ...` pegada al mensaje para pelarla después) y aísla
/// las líneas exactas largas + respuestas rápidas. Las exactas cortas
/// (`Aa`, `Visto`, `Enviado`) no se tocan: partirlas rompería mensajes
/// reales (`He visto...`); `Toca una respuesta` / `Tap a reply` tampoco:
/// son prefijo de la instrucción completa y partirlas dejaría la cola
/// (`para enviársela al comprador.`) como supuesto mensaje.
fn segmentar_pegado(texto: &str) -> String {
    let mut fuera = texto.to_string();
    let mut marcadores: Vec<(&&str, bool)> = RUIDO_EXCERPT_PREFIJOS
        .iter()
        .map(|m| (m, true))
        .chain(
            RUIDO_EXCERPT_EXACTO
                .iter()
                .filter(|m| {
                    m.chars().count() >= 8 && **m != "Toca una respuesta" && **m != "Tap a reply"
                })
                .map(|m| (m, false)),
        )
        .chain(RESPUESTAS_RAPIDAS_FB.iter().map(|m| (m, false)))
        .collect();
    marcadores.sort_by_key(|a| std::cmp::Reverse(a.0.len()));
    for (m, es_prefijo) in marcadores {
        if fuera.contains(*m) {
            let corte = if es_prefijo {
                format!("\n{m}")
            } else {
                format!("\n{m}\n")
            };
            fuera = fuera.replace(*m, &corte);
        }
    }
    fuera
}

/// [08AA-8] Pela la atribución del visor al inicio del segmento
/// (`Mensaje enviado: 3:18 pm por: Wilmery`, `Message sent ... by ...`):
/// devuelve el mensaje que trae pegado o vacío si era solo atribución.
/// Sin `regex`: escaneo manual como `precio_del_aviso`.
fn quitar_prefijo_enviado<'a>(linea: &'a str, nombre: Option<&str>) -> &'a str {
    let lower = linea.to_lowercase();
    let marca = if lower.starts_with("mensaje enviado") {
        "mensaje enviado".len()
    } else if lower.starts_with("message sent") {
        "message sent".len()
    } else {
        return linea;
    };
    let mut resto = linea[marca..].trim_start_matches([' ', ':', ',', '.', '\u{a0}', '\u{202f}']);
    resto = saltar_hora_fb(resto);
    resto = resto.trim_start_matches([' ', ':', ',', '.']).trim_start();
    let palabra = |w: &str| {
        let l = resto.to_lowercase();
        l.starts_with(w)
            && l[w.len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_alphabetic())
    };
    let con_por = palabra("por") || palabra("by");
    if con_por {
        let n = if resto.to_lowercase().starts_with("por") {
            3
        } else {
            2
        };
        resto = resto[n..]
            .trim_start_matches([' ', ':', ',', '.'])
            .trim_start();
        if let Some(nom) = nombre.map(str::trim).filter(|n| !n.is_empty()) {
            if let Some(r) = cortar_nombre(resto, nom) {
                return r.trim_start_matches([' ', ':', ',', '.']).trim_start();
            }
        }
        resto = saltar_palabra(resto)
            .trim_start_matches([' ', ':', ',', '.'])
            .trim_start();
    }
    resto
}

/// Salta la hora de la atribución (`3:18 pm`, `lunes 22:48`) sin comerse
/// el `por`/`by` que viene después (la `p` colisiona con `pm`).
/// [08AA-24] El visor antepone el día de la semana a la hora
/// (`Mensaje enviado lunes 22:48 por Cristo:`): se salta igual.
fn saltar_hora_fb(s: &str) -> &str {
    let mut resto = s;
    while !resto.is_empty() {
        let l = resto.to_lowercase();
        if l.starts_with("por")
            && l["por".len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_alphabetic())
        {
            break;
        }
        if l.starts_with("by")
            && l["by".len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_alphabetic())
        {
            break;
        }
        if largo_dia_semana(&l) > 0 {
            resto = &resto[largo_dia_semana(&l)..];
            continue;
        }
        let c = resto.chars().next().unwrap_or(' ');
        if c.is_ascii_digit() || " :,./-\u{a0}\u{202f}apmAPM".contains(c) {
            resto = &resto[c.len_utf8()..];
        } else {
            break;
        }
    }
    resto
}

/// Longitud en bytes del día de semana al inicio (ES + EN, con y sin
/// tildes: el corte del float a veces las pela), o 0 si no hay día.
/// El día debe cerrar con no-letra (`lunes 22:48`, no `lunes bueno`...
/// bueno, `lunes` + espacio + `bueno` también casaría, pero esto solo se
/// usa tras `Mensaje enviado`, donde día = fecha, nunca mensaje).
fn largo_dia_semana(min: &str) -> usize {
    const DIAS: &[&str] = &[
        "lunes",
        "martes",
        "miercoles",
        "miércoles",
        "jueves",
        "viernes",
        "sabado",
        "sábado",
        "domingo",
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ];
    DIAS.iter()
        .find_map(|d| {
            min.strip_prefix(d)
                .filter(|r| r.chars().next().is_none_or(|c| !c.is_alphabetic()))
                .map(|_| d.len())
        })
        .unwrap_or(0)
}

/// Corta el `nombre` del hilo al inicio de `resto` (insensible a caja y
/// tildes: el `thread_id` viaja sin tildes y el visor puede traerlas).
fn cortar_nombre<'a>(resto: &'a str, nombre: &str) -> Option<&'a str> {
    let canon = sin_tilde_min(nombre);
    let n_chars = nombre.chars().count();
    let toma: String = resto.chars().take(n_chars).collect();
    if sin_tilde_min(&toma) == canon {
        let idx = resto
            .char_indices()
            .nth(n_chars)
            .map_or(resto.len(), |(i, _)| i);
        Some(resto[idx..].trim_start())
    } else {
        None
    }
}

/// Salta una palabra (el nombre cuando el hilo no lo dio): solo letras,
// tildes y espacios, tope 30 caracteres.
fn saltar_palabra(s: &str) -> &str {
    let mut bytes = 0;
    for (n, (i, c)) in s.char_indices().enumerate() {
        if n >= 30 || !(c.is_alphabetic() || c == ' ') {
            break;
        }
        bytes = i + c.len_utf8();
    }
    s[bytes..].trim_start()
}

/// [08AA-24] Cola del `Presionar Enter,` inicial cortada por el
/// `slice(-1200)` del float (testigo cristo: el crudo empieza en `sionar
/// Enter,`): el corte cae dentro de `pre|sionar` y el filtro no reconoce
/// el resto. Se pela el fragmento `...r enter,` (0-8 letras: cualquier
/// truncado de `presionar`) solo en la primera línea; un mensaje real
/// jamás empieza así (`enterarme` no casa por la coma obligatoria).
fn pelar_enter_truncado(linea: &str) -> &str {
    let lower = linea.to_lowercase();
    if let Some(pos) = lower.find("r enter,") {
        let cabeza = &lower[..pos];
        if cabeza.len() <= 8 && cabeza.bytes().all(|b| b.is_ascii_alphabetic()) {
            return linea[pos + "r enter,".len()..].trim_start();
        }
    }
    linea
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

/// [08AA-16] Marcas de tiempo del visor (`2:43 am`, `11:30 pm`,
/// [08AA-24] `lunes 22:48`, `22:48`): separan mensajes, no son contenido.
/// Formato exacto `H:MM` + opcional `am|pm` y opcional día de semana
/// delante; con `am|pm` la hora es 1-12, sin es 0-23 (formato 24h del
/// visor ES). Sin `regex` en el árbol: escaneo manual como
/// `precio_del_aviso`.
fn es_marca_tiempo_fb(linea: &str) -> bool {
    let bajado = linea.trim().to_lowercase();
    let sin_dia: &str = bajado[largo_dia_semana(&bajado)..].trim_start();
    let (hora, con_ampm) = if let Some(h) = sin_dia
        .strip_suffix("am")
        .or_else(|| sin_dia.strip_suffix("pm"))
        .or_else(|| sin_dia.strip_suffix("a.m."))
        .or_else(|| sin_dia.strip_suffix("p.m."))
    {
        (h.trim_end(), true)
    } else {
        (sin_dia, false)
    };
    let mut partes = hora.split(':');
    match (partes.next(), partes.next(), partes.next()) {
        (Some(h), Some(m), None) => {
            h.len() <= 2
                && m.len() == 2
                && h.bytes().all(|b| b.is_ascii_digit())
                && m.bytes().all(|b| b.is_ascii_digit())
                && m.parse::<u32>().is_ok_and(|m| m <= 59)
                && h.parse::<u32>().is_ok_and(|h| {
                    if con_ampm {
                        (1..=12).contains(&h)
                    } else {
                        h <= 23
                    }
                })
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
            let a_canon = canon_separadores(&a.to_lowercase());
            let c_canon = canon_separadores(&cuerpo.to_lowercase());
            if a_canon.contains(&c_canon) {
                return true;
            }
        }
    }
    false
}

/// [08AA-17] El visor separa con ` - ` o ` · ` donde el título trae un
/// espacio (`VEF0 - Casa en venta...` vs aviso `vef0 casa en venta...`):
/// se canonizan a un espacio en ambos lados antes del `contains`.
fn canon_separadores(s: &str) -> String {
    s.replace(" - ", " ").replace(" · ", " ")
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
/// [08AA-8] +cabeceras ES del hilo wilmery (testigo 554 en BD, texto
/// pegado): `También es miembro de...`, `Detalles del comprador` /
/// `Detalles de la conversación` y `Ver perfil...` nunca son contenido.
const RUIDO_EXCERPT_PREFIJOS: &[&str] = &[
    "Si te vas a reunir con alguien",
    "If you're meeting someone",
    "If you are meeting someone",
    "Meta podría usar tecnología",
    "Meta may use technology",
    "También es miembro de",
    "Also a member of",
    "Detalles de",
    "Detalles del",
    "Details of",
    "Ver perfil",
    "View profile",
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
/// [08AA-17] +`Marketplace` suelto, `Cargando...`/`Loading...`
/// (placeholder de hilo aún cargando) del reporte Tina.
const RUIDO_EXCERPT_EXACTO: &[&str] = &[
    "Marketplace",
    "Cargando...",
    "Loading...",
    "View buyer",
    "More options",
    "Ver perfil",
    "Ver perfil del comprador",
    "Detalles del comprador",
    "Detalles de la conversación",
    "Buyer details",
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
