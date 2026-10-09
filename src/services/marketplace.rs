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

/* [09AA-20] Burbujas estructuradas F0: tipos+validador+firma-v2 viven en su
 * dominio (`marketplace_burbujas`); se re-exporta para no mover sus usos
 * externos (handlers, utoipa, tests). */
pub use super::marketplace_burbujas::{
    estructuradas_apagadas, llave_esperada, texto_para_prompt, validar_conversacion,
    validar_idempotency_key, BurbujaIn, BurbujaUtil, ConversacionEstructurada,
    ConversacionValidada, ErrorEstructurado, Lado, LadoUtil, CODIGO_ESQUEMA, CODIGO_IDEMPOTENCIA,
    CODIGO_PAYLOAD_GIGANTE, CODIGO_REINTENTO_FOREGROUND, CODIGO_VERSION_DESCONOCIDA,
    ENV_KILL_SWITCH, FIRMA_VERSION_V2, MAX_BURBUJAS, MAX_HINT_CARACTERES, MAX_IDEMPOTENCY_CHARS,
    MAX_POR_BURBUJA, MAX_TOTAL_CARACTERES, VERSION_ESTRUCTURADA,
};
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

/// Versión del strip aceptada (`strip_vN` del plan).
/// [08AA-25] v2 suma `operacion` al allowlist: sin ella la IA presentaba
/// los alquileres como ventas (testigo: Townhouse Arivana, hilo cristo).
pub const STRIP_VERSION: &str = "v2";
/// Fallback exacto cuando no hay ficha o falla la IA.
/// [09AA-2] Sin «confirmo»: el prompt prohíbe anunciar confirmaciones y el
/// texto anterior («te confirmo precio/entrega») minaba ese veto.
pub const FALLBACK_BORRADOR: &str = "Lo reviso y te escribo el precio por aquí";
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

/// [09AA-2] Fin del bucle del párrafo de relleno (08AA-36/37/38 burlados con
/// sinónimos): el prompt mismo ORDENABA el relleno («avanza la conversación:
/// ofrece fotos o pregunta qué busca») y esa orden positiva siempre le ganó
/// al veto. Ahora la invariante la impone Rust, no el wording: el borrador
/// de IA sale como P1 + [una línea de dato útil] + CTA canónico + wa.
/// Cualquier párrafo intermedio con pregunta, oferta de fotos o reafirmación
/// se poda; el final se reconstruye literal (nunca se conserva el de la IA).
/// No toca texto manual de la dueña ni la rama `reserva`.
pub const CTA_FIJO: &str = "Cuéntame qué estás buscando y con gusto te ayudo.";

/// Garantía determinista de forma sobre el texto crudo de la IA.
#[must_use]
pub fn imponer_forma_borrador(ia: &str) -> String {
    let norm = formatear_parrafos(ia);
    let ps: Vec<String> = norm
        .split("\n\n")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if ps.is_empty() {
        return borrador_minimo();
    }
    /* Ancla final: último párrafo con contacto o CTA (lo que la IA haya
     * puesto ahí se descarta igual; el final se reconstruye canónico). */
    let fin = ps
        .iter()
        .rposition(|p| {
            p.contains(CONTACTO_WA) || p.contains(CONTACTO_TEL) || p.contains("qué estás buscando")
        })
        .unwrap_or(ps.len() - 1);
    let p1 = sanear_p1(&ps[0]);
    if !p1_valido(&p1) {
        return borrador_minimo();
    }
    /* Medios = párrafos entre P1 y el ancla; se conserva como máximo UNA
     * frase que sea dato útil (respuesta a pregunta concreta no cubierta). */
    let mut medio: Option<String> = None;
    for m in ps.iter().skip(1).take(fin.saturating_sub(1)) {
        for f in partir_frases(m) {
            if es_dato_util(&f) {
                medio = Some(f);
                break;
            }
        }
        if medio.is_some() {
            break;
        }
    }
    let mut out = vec![p1];
    if let Some(m) = medio {
        out.push(m);
    }
    out.push(format!(
        "{CTA_FIJO} Cualquier cosa escríbeme al {CONTACTO_TEL}."
    ));
    out.push(CONTACTO_WA.to_string());
    formatear_parrafos(&out.join("\n\n"))
}

/// P1 trae disponibilidad o precio (o el fallback): si la IA alucinó otro
/// texto, se descarta todo y va el mínimo.
fn p1_valido(p1: &str) -> bool {
    p1.contains("disponible") || p1.contains('$') || p1.contains(&FALLBACK_BORRADOR[..10])
}

/// Sanea el primer párrafo a nivel frase: fuera interrogativas (ofertas de
/// fotos / preguntas pegadas) y frases de relleno que no aporten el dato
/// central (el cual trae `$`, «disponible» o el fallback y por eso sobrevive).
fn sanear_p1(p1: &str) -> String {
    let frases: Vec<String> = partir_frases(p1)
        .into_iter()
        .filter(|f| {
            f.contains("wa.me")
                || (!f.contains('?')
                    && !f.contains('¿')
                    && (!es_relleno(f)
                        || f.contains('$')
                        || f.contains("disponible")
                        || f.contains(&FALLBACK_BORRADOR[..10])))
        })
        .collect();
    let unido = frases.join(" ");
    unido.chars().take(400).collect()
}

/// Una frase sobrevive en el medio solo si es dato concreto (dígito o
/// sustantivo de ficha), sin preguntas, sin contacto y sin relleno.
fn es_dato_util(frase: &str) -> bool {
    let t = frase.trim();
    (3..=140).contains(&t.chars().count())
        && !t.contains('?')
        && !t.contains('¿')
        && !t.contains("wa.me")
        && !t.contains(CONTACTO_TEL)
        && !es_relleno(t)
        && tiene_dato_concreto(t)
}

/// Relleno por intención (actos de habla), no por frases: ofertas, preguntas,
/// reafirmaciones de estado/precio y meta-coordinación. En minúsculas.
fn es_relleno(frase: &str) -> bool {
    const RELLENO: &[&str] = &[
        "foto",
        "compart",
        "enví",
        "envi",
        "interesa",
        "dispon",
        "vige",
        "publica",
        "precio",
        "$",
        "mensual",
        "canon",
        "cuesta",
        "vale",
        "estatus",
        "ficha",
        "negociable",
        "visita",
        "coordin",
        "confirm",
        "busca",
        "ayudo",
        "gusto",
        "encanta",
        "oferta",
        "descuento",
        "oportunidad",
        "aprovecha",
        "anímate",
        "animate",
        "escríbeme",
        "escribeme",
        "llámame",
        "llamame",
        "contáctame",
        "contactame",
        "dueña",
        "duena",
    ];
    let min = frase.to_lowercase();
    RELLENO.iter().any(|r| min.contains(r))
}

/// Dato concreto = dígito o sustantivo de ficha (baños, m2, ubicación...).
fn tiene_dato_concreto(frase: &str) -> bool {
    const DATOS: &[&str] = &[
        "bañ",
        "habit",
        "dormitorio",
        "m2",
        "m²",
        "metro",
        "terreno",
        "puesto",
        "amobl",
        "ubic",
        "financ",
        "cuota",
        "cocina",
        "estaciona",
        "piscina",
        "pozo",
        "planta",
        "sala",
        "comedor",
        "vista",
        "colegio",
        "centro",
        "cerca",
        "villa",
        "residenc",
    ];
    if frase.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    let min = frase.to_lowercase();
    DATOS.iter().any(|d| min.contains(d))
}

/// Parte en frases por `.`/`?`/`!`/salto; el punto entre dígitos ($43.000)
/// no parte para no romper cifras.
fn partir_frases(t: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut act = String::new();
    let mut it = t.chars().peekable();
    while let Some(c) = it.next() {
        act.push(c);
        let corta = match c {
            '?' | '!' | '\n' => true,
            '.' => !matches!(it.peek(), Some(n) if n.is_ascii_digit()),
            _ => false,
        };
        if corta {
            let f = act.trim().to_string();
            if !f.is_empty() {
                out.push(f);
            }
            act.clear();
        }
    }
    let f = act.trim().to_string();
    if !f.is_empty() {
        out.push(f);
    }
    out
}

/// Mínimo servible cuando la IA devolvió basura: fallback + contacto + wa.
fn borrador_minimo() -> String {
    formatear_parrafos(&format!(
        "{FALLBACK_BORRADOR}\nCualquier cosa escríbeme al {CONTACTO_TEL}.\n{CONTACTO_WA}"
    ))
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

/// Prompt seguro: solo los 7 campos del allowlist. La frase canónica de la
/// ficha vive en la tabla `inmuebles`; el plugin jamás ve el resto.
/// [08AA-25] `operacion` (`venta`|`alquiler`, tal cual en la fila): sin
/// ella el prompt hablaba siempre en lenguaje de venta.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PromptSeguro {
    pub titulo: String,
    pub precio_publico: String,
    pub operacion: String,
    pub zona: String,
    pub m2: f64,
    pub habitaciones: i32,
    pub descripcion_corta: String,
}

/// Recorta la ficha al allowlist. `strip` distinto de la versión vigente se
/// rechaza para que un despliegue viejo no cuele campos nuevos en silencio.
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
        operacion: ficha.operacion.clone(),
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

/// [08AA-39] Limpieza total del panel: borra TODAS las filas de caché de una
/// vez (la dueña lo pidió como botón al lado de «Recargar» para no depender
/// de limpiezas manuales por SQL). Devuelve cuántas filas cayeron.
pub async fn borrar_todo_cache(pool: &sqlx::PgPool) -> Result<u64, AppError> {
    let r = sqlx::query("DELETE FROM mp_respuestas_cache")
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}

/// [09AA-4] Borrado previo a regenerar: elimina las filas del hilo que NO
/// son correcciones de la dueña (borradores viejos de excerpts anteriores).
/// Las correcciones (`corregida`, puestas por ella con el lápiz) jamás se
/// tocan: son su texto, no caché. Devuelve cuántas filas cayeron.
/// Sin esto, cada excerpt nuevo es una firma nueva y sus filas viejas viven
/// 90 días: el panel lista lo viejo junto a lo fresco y parece «cacheado».
pub async fn borrar_hilo_no_corregidas(
    pool: &sqlx::PgPool,
    thread_clave: &str,
) -> Result<u64, AppError> {
    let r = sqlx::query("DELETE FROM mp_respuestas_cache WHERE thread_id = $1 AND NOT corregida")
        .bind(thread_clave)
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
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
/// [09AA-21] `aviso_conocido`: el aviso del hilo empareja con una ficha
/// (ID exacto o título); el front lo usa para la vista de huérfanos.
/// [09AA-23] `inmueble_vinculado`: título de la ficha emparejada (`None` =
/// huérfano); el front lo muestra como «Vinculado: X»/«Sin ficha» en chats.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatResumen {
    pub thread_id: String,
    pub borradores: i64,
    pub usos: i64,
    pub corregidas: i64,
    pub ultimo: String,
    pub aviso_conocido: bool,
    pub inmueble_vinculado: Option<String>,
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

/// Chats con borradores, ordenados por el más reciente. Una sola consulta
/// para el agregado + dos para el vínculo (títulos e IDs publicados, una
/// vez, sin N+1): cada hilo resuelve en memoria si su aviso es conocido.
/// Si las auxiliares fallan, todo queda `false` (el panel jamás se bloquea).
pub async fn resumen_chats(pool: &sqlx::PgPool) -> Result<Vec<ChatResumen>, AppError> {
    let filas: Vec<(String, i64, i64, i64, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT thread_id, COUNT(*)::BIGINT, COALESCE(SUM(usos), 0)::BIGINT, \
         SUM(CASE WHEN corregida THEN 1 ELSE 0 END)::BIGINT, MAX(valida_hasta) \
         FROM mp_respuestas_cache GROUP BY thread_id ORDER BY MAX(valida_hasta) DESC",
    )
    .fetch_all(pool)
    .await?;
    let candidatos: Vec<(uuid::Uuid, String)> =
        match InmuebleRepository::titulos_publicados(pool).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("resumen_chats: sin títulos publicados ({e}), avisos no conocidos");
                Vec::new()
            }
        };
    let vinculos: std::collections::HashMap<String, String> =
        match InmuebleRepository::vinculos_publicados(pool).await {
            Ok(v) => v.into_iter().collect(),
            Err(e) => {
                tracing::warn!("resumen_chats: sin vínculos de aviso ({e}), solo título");
                std::collections::HashMap::new()
            }
        };
    Ok(filas
        .into_iter()
        .map(|(thread_id, borradores, usos, corregidas, ultimo)| {
            let vinculado = titulo_vinculado_del_hilo(&thread_id, &candidatos, &vinculos);
            ChatResumen {
                thread_id,
                borradores,
                usos,
                corregidas,
                ultimo: ultimo.to_rfc3339(),
                aviso_conocido: vinculado.is_some(),
                inmueble_vinculado: vinculado,
            }
        })
        .collect())
}

/* [09AA-21] ¿El aviso del hilo empareja con una ficha? Rama exacta primero:
 * el texto tras `|` son dígitos 5–32 vinculados; si no, emparejado por título
 * (misma regla que `ficha_por_titulo`: directo o solape ≥3 con distintiva,
 * empate = no conocido). Pura en memoria (sin BD).
 * [09AA-23] Devuelve el título emparejado (rama exacta: el título del
 * vínculo; rama título: el título candidato). `resumen_chats` deriva
 * `aviso_conocido` como `vinculado.is_some()`. */
fn titulo_vinculado_del_hilo(
    thread_id: &str,
    candidatos: &[(uuid::Uuid, String)],
    vinculos: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let aviso = aviso_fb_de_thread(thread_id).unwrap_or_default();
    let recortado = aviso.trim();
    if recortado.is_empty() {
        return None;
    }
    if recortado.chars().all(|c| c.is_ascii_digit()) && (5..=32).contains(&recortado.len()) {
        return vinculos.get(recortado).cloned();
    }
    let mut mejor: Option<(usize, usize)> = None;
    let mut titulo_mejor: Option<String> = None;
    let mut empate = false;
    for (_, titulo) in candidatos {
        let (directo, solape, distintivo) = puntaje_titulo(recortado, titulo);
        if !(directo || (solape >= 3 && distintivo >= 1)) {
            continue;
        }
        let clave = (distintivo, solape);
        match mejor {
            Some(m) if m == clave => empate = true,
            Some(m) if m > clave => {}
            _ => {
                mejor = Some(clave);
                titulo_mejor = Some(titulo.clone());
                empate = false;
            }
        }
    }
    if mejor.is_some() && !empate {
        titulo_mejor
    } else {
        None
    }
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

/// [09AA-3] Candidata a regeneración masiva: hilos con borrador real.
/// Salta correcciones de la dueña (`corregida`: su texto manda) y filas
/// solo-foto (`respuesta=''`: sin borrador que refrescar). Una fila por
/// clave de caché (un hilo puede traer varias: cada una se regenera).
#[derive(Debug, Clone)]
pub struct FilaRegen {
    pub thread_id: String,
    pub firma: String,
    pub excerpt_texto: String,
}

pub async fn filas_para_regenerar(pool: &sqlx::PgPool) -> Result<Vec<FilaRegen>, AppError> {
    let filas: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT thread_id, firma, excerpt_texto FROM mp_respuestas_cache \
         WHERE respuesta <> '' AND NOT corregida ORDER BY valida_hasta DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(filas
        .into_iter()
        .map(|(thread_id, firma, excerpt_texto)| FilaRegen {
            thread_id,
            firma,
            excerpt_texto,
        })
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
/// prompt (los 7 campos del strip + `estado`, que condiciona disponibilidad),
/// calculada sobre la fila recién leída y antes de stripeear. Si la ficha
/// cambia en algo que la respuesta cita → hash distinto → miss → regenera.
#[must_use]
pub fn hash_ficha(ficha: &InmuebleRow) -> String {
    let canon = match strip_ficha_para_prompt(ficha, STRIP_VERSION) {
        Ok(s) => serde_json::json!({
            "titulo": s.titulo,
            "precio": s.precio_publico,
            "operacion": s.operacion,
            "zona": s.zona,
            "m2": s.m2,
            "hab": s.habitaciones,
            "desc": s.descripcion_corta,
            "estado": ficha.estado,
        }),
        /* Inalcanzable con v2 (el handler lo rechazaría antes); clave
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

/// [08AA-31] Foto combinada del hilo (pura, sin BD): une la foto vieja con
/// el `limpio` nuevo, línea a línea, con dedup exacto y tope de 30 líneas
/// (las últimas). Sin esto, `releer_foto` pisaba todas las filas con el
/// último snapshot y si ese solo traía lo propio (`Tú:`), el mensaje del
/// cliente desaparecía del panel (hilo angelv). Las líneas del cliente
/// viajan sin etiqueta y las propias como `Tú:` (08AA-29): el dedup es por
/// línea exacta, así que ambos lados conviven.
#[must_use]
pub fn combinar_foto_hilo(vieja: &str, nueva: &str) -> String {
    use std::collections::HashSet as Conjunto;
    let mut vistas: Conjunto<String> = Conjunto::new();
    let mut lineas: Vec<&str> = Vec::new();
    for linea in vieja.lines().chain(nueva.lines()) {
        let t = linea.trim();
        if t.is_empty() || vistas.contains(t) {
            continue;
        }
        vistas.insert(t.to_string());
        lineas.push(t);
    }
    let desde = lineas.len().saturating_sub(30);
    lineas[desde..].join("\n")
}

/// [08AA-28] Releer con creación: refresca la foto del hilo; si no hay
/// fila (caché borrada o hilo nuevo sin borrador), la crea solo con la
/// foto y `respuesta` vacía para que el chat aparezca en el panel sin
/// inventar borrador. La PK sintética (`firma=sha("releer-sin-borrador|hilo")`,
/// `precio/catalog="releer"`) nunca choca con firmas HMAC reales, así un
/// borrador posterior inserta su propia fila y gana por `valida_hasta`.
/// `ON CONFLICT DO UPDATE` lo hace idempotente (doble clic o releers
/// concurrentes convergen; regla 6: upsert atómico, no buscar-crear).
/// [08AA-31] Fusiona con `combinar_foto_hilo` en vez de pisar: la foto
/// vieja aporta las líneas que el último snapshot ya no trae (el mensaje
/// del cliente cuando el eco propio es lo único nuevo).
/// Devuelve `(actualizado, creado)`.
pub async fn releer_foto(
    pool: &sqlx::PgPool,
    hilo: &str,
    limpio: &str,
    crudo: &str,
) -> Result<(bool, bool), AppError> {
    let vieja: Option<(String,)> = sqlx::query_as(
        "SELECT excerpt_texto FROM mp_respuestas_cache WHERE thread_id = $1 \
         ORDER BY valida_hasta DESC LIMIT 1",
    )
    .bind(clave_hilo(hilo))
    .fetch_optional(pool)
    .await?;
    let combinada = match &vieja {
        Some((v,)) => combinar_foto_hilo(v, limpio),
        None => limpio.to_string(),
    };
    let tocadas = sqlx::query(
        "UPDATE mp_respuestas_cache SET excerpt_texto = $1, excerpt_crudo = $2 \
         WHERE thread_id = $3",
    )
    .bind(&combinada)
    .bind(crudo)
    .bind(clave_hilo(hilo))
    .execute(pool)
    .await?
    .rows_affected();
    if tocadas > 0 {
        return Ok((true, false));
    }
    let firma = sha_hex(&format!("releer-sin-borrador|{hilo}"));
    sqlx::query(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, thread_id, excerpt_texto, excerpt_crudo) \
         VALUES ($1, 'releer', 'releer', '', $2, $3, $4) \
         ON CONFLICT (firma, precio_hash, catalog_hash) DO UPDATE SET \
         excerpt_texto = EXCLUDED.excerpt_texto, excerpt_crudo = EXCLUDED.excerpt_crudo",
    )
    .bind(firma)
    .bind(clave_hilo(hilo))
    .bind(&combinada)
    .bind(crudo)
    .execute(pool)
    .await?;
    Ok((true, true))
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
            marketplace_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /* El strip deja pasar exactamente los 7 campos del allowlist: ni el
     * slug, ni el estado interno, ni la receta viajan al prompt.
     * (`serde_json::Map` ordena claves: se compara ordenado.)
     * [08AA-25] `operacion` viaja (v2): sin ella la IA vendía alquileres. */
    #[test]
    fn strip_solo_allowlist_siete_campos() {
        let s = strip_ficha_para_prompt(&ficha(), STRIP_VERSION).unwrap();
        let v = serde_json::to_value(&s).unwrap();
        let mut claves: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        claves.sort_unstable();
        assert_eq!(
            claves,
            vec![
                "descripcion_corta",
                "habitaciones",
                "m2",
                "operacion",
                "precio_publico",
                "titulo",
                "zona"
            ]
        );
        assert_eq!(s.precio_publico, "$43.000");
        assert_eq!(s.operacion, "venta");
        assert_eq!(s.zona, "Puerto Ordaz, Riberas del Caroní");
    }

    #[test]
    fn strip_version_desconocida_se_rechaza() {
        assert!(strip_ficha_para_prompt(&ficha(), "v9").is_err());
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
    fn normalizar_hilo_edickson_pela_chrome_nuevo() {
        /* [09AA-16] Testigo exacto en BD (`edickson|VEF0 casa en venta en
         * riberas del caroní, puerto ordaz`, `length(excerpt_texto)=137`):
         * chrome nuevo del visor — eco del título con cabeza cortada
         * (`n · Casa...`), `Se unió a Facebook en 2010`, cola huérfana
         * `del comprador` y etiqueta `Comprador`. Solo la pregunta del
         * cliente sobrevive. */
        let hilo = "edickson|VEF0 casa en venta en riberas del caroní, puerto ordaz";
        let crudo = "n · Casa en venta en Riberas del Caroní, Puerto Ordaz\n\
            Se unió a Facebook en 2010\n\
            del comprador\n\
            Comprador\n\
            Hola. ¿Sigue estando disponible?";
        assert_eq!(
            normalizar_excerpt_hilo(hilo, crudo),
            "Hola. ¿Sigue estando disponible?"
        );
    }

    #[test]
    fn normalizar_hilo_edgarluis_pela_eco_propio_y_despega_url() {
        /* [09AA-17] Testigo exacto en BD (`edgarluis|VEF0 casa en venta en
         * urbanización villa icabarú, puerto ordaz`,
         * `length(excerpt_texto)=353`): la burbuja de las 12:59am trae
         * `por Edgarluis:` + tip + NUESTRO borrador sin marca (el pelado
         * de la atribución lo dejaba como Cliente: CTA + teléfono + wa
         * con `wa.me` pegado por el aria `...855wa.mewa.me`), y el eco
         * `por Tú:` repite el borrador completo. Sobreviven el fragmento
         * de corte (`nión.`, irrecuperable) y el saludo propio etiquetado;
         * los cierres caen en ambas copias (son boilerplate que
         * `imponer_forma` re-agrega). */
        let hilo = "edgarluis|VEF0 casa en venta en urbanización villa icabarú, puerto ordaz";
        let crudo = "nión. Ver más consejos de seguridadPresionar Enter, Mensaje enviado 12:59 am por Edgarluis: Si te vas a reunir con alguien en persona, cuéntales a familiares y amigos adónde vas. Usa la función de compartir la ubicación en tiempo real directamente con un amigo o familiar durante la reunión.Hola, Edgarluis, buenas noches, la Casa en Villa Icabarú está disponible en $90.000 negociable.\n\nCuéntame qué estás buscando y con gusto te ayudo.\n\nCualquier cosa escríbeme al 0424 9208855.\n\nhttps://wa.me/584249208855wa.mewa.meEnviado hace 3 hPresionar Enter, Mensaje enviado 1:10 am por Tú: Hola, Edgarluis, buenas noches, la Casa en Villa Icabarú está disponible en $90.000 negociable.\n\nCuéntame qué estás buscando y con gusto te ayudo.\n\nCualquier cosa escríbeme al 0424 9208855.\n\nhttps://wa.me/584249208855Meta podría usar tecnología para revisar los mensajes de Marketplace con el fin de detectar y reducir las estafas y el fraude.Presionar Enter, Mensaje enviado 1:10 am por Tú: Meta podría usar tecnología para revisar los mensajes de Marketplace con el fin de detectar y reducir las estafas y el fraude.Escribir mensajeEscribe en Edgarluis · Casa en venta en Urbanización Villa Icabarú, Puerto Ordaz.Aa";
        assert_eq!(
            normalizar_excerpt_hilo(hilo, crudo),
            "nión.\nTú: Hola, Edgarluis, buenas noches, la Casa en Villa Icabarú está disponible en $90.000 negociable."
        );
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
         * `Escribe en Tina · ...`) sin ningún mensaje real: es
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
    fn normalizar_hilo_wilmery_pegado_deja_solo_la_pregunta() {
        /* [08AA-8] Testigo exacto en BD
         * (`wilmery|apartamento amoblado 3 hab. en vista hermosa, puerto
         * ordaz.`, `length(excerpt_texto)=554`): el puente aplana el DOM a
         * una sola línea pegada (`OrdazDetalles`, `WilmeryHola.`,
         * `disponible?Presionar`, `mensajeEscribe`) y el filtro por líneas
         * no tocaba nada. Solo la pregunta del cliente sobrevive (la
         * duplicada se colapsa). */
        let hilo = "wilmery|apartamento amoblado 3 hab. en vista hermosa, puerto ordaz.";
        let crudo = "También es miembro de CASAS y APARTAMENTOS en Puerto OrdazDetalles del compradorPresionar Enter, Mensaje enviado: 3:18 pm por: WilmeryHola. ¿Sigue estando disponible?Presionar Enter, Mensaje enviado 3:18 pm por Wilmery: Hola. ¿Sigue estando disponible?Envía una respuesta rápidaToca una respuesta para enviársela al comprador.Sí. ¿Te interesa?Lo estoy mirando. Te avisaré.Lo siento, no está disponible.Presionar Enter, Mensaje enviado: 3:18 pm por: WilmeryEscribir mensajeEscribe en Wilmery · Apartamento amoblado 3 hab. en Vista Hermosa, Puerto Ordaz.Aa";
        assert_eq!(
            normalizar_excerpt_hilo(hilo, crudo),
            "Hola. ¿Sigue estando disponible?"
        );
    }

    #[test]
    fn normalizar_hilo_cristo_dia_y_truncado_deja_solo_preguntas() {
        /* [08AA-24] Crudo exacto del hilo cristo (`excerpt_crudo` 1200,
         * con saltos): día de semana ante la hora (`lunes 22:48 por
         * Cristo:`), duplicado con dos puntos (`lunes 22:48 por:
         * Cristo`), inicio truncado por el `slice(-1200)` del float
         * (`sionar Enter,`) y marca `3:53 pm` separando mensajes. Solo
         * las 2 preguntas sobreviven. */
        let hilo = "cristo|VEF0 alquiler townhouse 2 niveles en arivana, puerto ordaz.";
        let crudo = "sionar Enter, Mensaje enviado lunes 22:48 por Cristo: Hola. ¿Sigue estando disponible?\nEnvía una respuesta rápida\nToca una respuesta para enviársela al comprador.\nSí. ¿Te interesa?\nLo estoy mirando. Te avisaré.\nLo siento, no está disponible.\nPresionar Enter, Mensaje enviado: lunes 22:48 por: Cristo\nSi te vas a reunir con alguien en persona, cuéntales a familiares y amigos adónde vas. Usa la función de compartir la ubicación en tiempo real directamente con un amigo o familiar durante la reunión. Ver más consejos de seguridad\nPresionar Enter, Mensaje enviado lunes 22:48 por Cristo: Si te vas a reunir con alguien en persona, cuéntales a familiares y amigos adónde vas. Usa la función de compartir la ubicación en tiempo real directamente con un amigo o familiar durante la reunión.\n3:53 pm\nCristo\n¿Sigue disponible?\nPresionar Enter, Mensaje enviado 3:53 pm por Cristo: ¿Sigue disponible?\nEnvía una respuesta rápida\nToca una respuesta para enviársela al comprador.\nSí. ¿Te interesa?\nLo estoy mirando. Te avisaré.\nLo siento, no está disponible.\nPresionar Enter, Mensaje enviado: 3:53 pm por: Cristo\nEscribir mensaje\nEscribe en Cristo · Alquiler Townhouse 2 niveles en Arivana, Puerto Ordaz.\n\n\n\n\nAa";
        assert_eq!(
            normalizar_excerpt_hilo(hilo, crudo),
            "Hola. ¿Sigue estando disponible?\n¿Sigue disponible?"
        );
    }

    #[test]
    fn normalizar_hilo_yusmelis_etiqueta_lado_propio_y_pela_chrome() {
        /* [08AA-29] Crudo exacto del hilo yusmelis (`excerpt_crudo` 1200
         * en BD): el aviso de seguridad de Meta llega como texto suelto
         * (`fin de detectar...`) y como eco propio, la burbuja propia se
         * duplica en su eco (`por Tú:`), y el chrome trae `wa.me`,
         * `En medio de la conversación` y `Enviado hace 1 min`. Lo de
         * ella se etiqueta (`Tú:`) para separarlo del cliente, el eco
         * repetido no duplica y el ruido no sobrevive. */
        let hilo = "yusmelis|VEF0 casa en venta en urbanización villa icabarú, puerto ordaz";
        let crudo = "fin de detectar y reducir las estafas y el fraude.\n\
            Presionar Enter, Mensaje enviado 5:28 pm por Tú: Meta podría usar tecnología para revisar los mensajes y así garantizar la seguridad de todas las personas.\n\
            Yusmelis\n\
            Buenas tardes Mayerlin, gracias por la información y no ofrece algún plan de financiamiento para el pago de la casa?\n\
            Presionar Enter, Mensaje enviado 5:28 pm por Yusmelis: Buenas tardes Mayerlin, gracias por la información y no ofrece algún plan de financiamiento para el pago de la casa?\n\
            Estoy interesada en una casa en puerto Ordaz que esté en una zona céntrica sí tienes otras opciones que no superen los 60 mil $ me podrías informar por favor\n\
            Mensajes\n\
            Hola, por favor,\n\
            dejame un numero\n\
            para guardarte y pasarte\n\
            la información.\n\
            Presionar Enter, Mensaje enviado 5:51 pm por Tú: Hola, por favor, dejame un numero para guardarte y pasarte la información.\n\
            Escribir mensaje\n\
            Escribe en Yusmelis · Casa en venta en Urbanización Villa Icabarú, Puerto Ordaz.\n\
            wa.me\n\
            En medio de la conversación\n\
            Enviado hace 1 min\n\
            Aa\n\
            Presionar Enter, Mensaje enviado 5:51 pm por Tú: Hola, por favor, dejame un numero para guardarte y pasarte la información.";
        assert_eq!(
            normalizar_excerpt_hilo(hilo, crudo),
            "Buenas tardes Mayerlin, gracias por la información y no ofrece algún plan de financiamiento para el pago de la casa?\n\
            Estoy interesada en una casa en puerto Ordaz que esté en una zona céntrica sí tienes otras opciones que no superen los 60 mil $ me podrías informar por favor\n\
            Tú: Hola, por favor, dejame un numero para guardarte y pasarte la información."
        );
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
         * sitio no es inyección: queda intacto. */
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
        assert!(!puntaje_titulo("apto precioso apTO", "apto").0);
    }

    /* [09AA-21] `aviso_conocido` del panel: ID exacto vinculado, título que
     * empareja, huérfano que no empareja, y empate entre dos fichas que no
     * reclama a ninguna (mismo criterio que `ficha_por_titulo`).
     * [09AA-23] Los vínculos ahora son mapa ID→título y se verifica además
     * que el título devuelto es el de la ficha emparejada. */
    #[test]
    fn aviso_conocido_id_titulo_huerfano_y_empate() {
        use std::collections::HashMap;
        let conocido =
            |hilo: &str, candidatos: &[(Uuid, String)], vinculos: &HashMap<String, String>| {
                titulo_vinculado_del_hilo(hilo, candidatos, vinculos).is_some()
            };
        let id = Uuid::new_v4();
        let titulo_riberas = "Casa en venta en Riberas del Caroní".to_string();
        let candidatos = vec![(id, titulo_riberas.clone())];
        let vinculos: HashMap<String, String> =
            [("123456789012345".to_string(), titulo_riberas.clone())]
                .into_iter()
                .collect();
        assert_eq!(
            titulo_vinculado_del_hilo("tina|123456789012345", &candidatos, &vinculos),
            Some(titulo_riberas.clone())
        );
        assert!(conocido("tina|123456789012345", &candidatos, &vinculos));
        assert!(!conocido("tina|999999999999999", &candidatos, &vinculos));
        assert_eq!(
            titulo_vinculado_del_hilo(
                "tina|VEF0 casa en venta en riberas del caroní, puerto ordaz",
                &candidatos,
                &vinculos
            ),
            Some(titulo_riberas.clone())
        );
        assert!(!conocido(
            "tina|casa en venta en arivana",
            &candidatos,
            &vinculos
        ));
        assert!(!conocido("sin-separador", &candidatos, &vinculos));
        let empatados = vec![
            (
                Uuid::new_v4(),
                "Casa en venta en Riberas del Caroní Norte".to_string(),
            ),
            (
                Uuid::new_v4(),
                "Casa en venta en Riberas del Caroní Sur".to_string(),
            ),
        ];
        assert!(!conocido(
            "tina|casa en venta en riberas del caroní",
            &empatados,
            &HashMap::new()
        ));
        /* Huérfano real de hoy: Río Aro no está en el catálogo → None. */
        assert_eq!(
            titulo_vinculado_del_hilo(
                "salazar|VEF0 apartamento residencias rio aro plaza puerto ordaz",
                &candidatos,
                &vinculos
            ),
            None
        );
    }

    #[test]
    fn formatear_parrafos_une_saltos_sueltos_y_separa_bloques() {
        let entrado = "Hola, Andreina, buenas noches.\nTe escribo por la casa.\nSí, sigue disponible.\nCuéntame qué estás buscando y con gusto te ayudo, cualquier cosa escríbeme al 0424 9208855 https://wa.me/584249208855";
        let salido = formatear_parrafos(entrado);
        assert_eq!(
            salido,
            "Hola, Andreina, buenas noches. Te escribo por la casa. Sí, sigue disponible. Cuéntame qué estás buscando y con gusto te ayudo, cualquier cosa escríbeme al 0424 9208855\n\nhttps://wa.me/584249208855"
        );
    }

    #[test]
    fn formatear_parrafos_respeta_lista_y_no_duplica() {
        let entrado = "Tiene:\n1. Piscina\n2. Planta eléctrica\n\nhttps://wa.me/584249208855";
        let salido = formatear_parrafos(entrado);
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
            marketplace_id: None,
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
        /* [08AA-25] La operación condiciona el lenguaje del borrador
         * (venta vs canon mensual): cambiarla invalida la caché. */
        f = fila_prueba(43_000.0);
        f.operacion = "alquiler".to_string();
        assert_ne!(hash_ficha(&f), base, "operación distinta debe invalidar");
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
        let a = precio_hash_seguro(
            &strip_ficha_para_prompt(&fila_prueba(43_000.0), STRIP_VERSION).unwrap(),
        );
        let b = precio_hash_seguro(
            &strip_ficha_para_prompt(&fila_prueba(45_000.0), STRIP_VERSION).unwrap(),
        );
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

    /* [09AA-3] Candidatos de la regeneración masiva: entra el borrador
     * normal; quedan fuera la corrección de la dueña y la fila vacía. */
    #[tokio::test]
    async fn regen_solo_borradores_no_corregidos() {
        let Some(pool) = pool_si_hay() else { return };
        let (fa, pa, ca) = (clave_azar(), clave_azar(), clave_azar());
        guardar_cache(
            &pool,
            &fa,
            &pa,
            &ca,
            "borrador-a",
            &foto_prueba("hilo-1", "x", "crudo-x"),
        )
        .await
        .expect("guarda a");
        let (fb, pb, cb) = (clave_azar(), clave_azar(), clave_azar());
        guardar_cache(
            &pool,
            &fb,
            &pb,
            &cb,
            "borrador-b",
            &foto_prueba("hilo-1", "y", "crudo-y"),
        )
        .await
        .expect("guarda b");
        corregir_cache(&pool, &fb, &pb, &cb, "texto duena")
            .await
            .expect("corrige b");
        let (fc, pc, cc) = (clave_azar(), clave_azar(), clave_azar());
        guardar_cache(
            &pool,
            &fc,
            &pc,
            &cc,
            "",
            &foto_prueba("hilo-1", "z", "crudo-z"),
        )
        .await
        .expect("guarda c");
        let filas = filas_para_regenerar(&pool).await.expect("lista");
        let firmas: Vec<&str> = filas.iter().map(|f| f.firma.as_str()).collect();
        assert!(firmas.contains(&fa.as_str()), "a entra: {firmas:?}");
        assert!(
            !firmas.contains(&fb.as_str()),
            "b fuera (corregida): {firmas:?}"
        );
        assert!(
            !firmas.contains(&fc.as_str()),
            "c fuera (vacía): {firmas:?}"
        );
        borrar_cache(&pool, &fa, &pa, &ca).await.expect("limpia a");
        borrar_cache(&pool, &fb, &pb, &cb).await.expect("limpia b");
        borrar_cache(&pool, &fc, &pc, &cc).await.expect("limpia c");
    }

    /* [09AA-4] Borrado previo a regenerar: caen los borradores viejos del
     * hilo, queda la corrección de la dueña y no se toca otro hilo. */
    #[tokio::test]
    async fn borrar_hilo_respeta_correccion_y_otro_hilo() {
        let Some(pool) = pool_si_hay() else { return };
        let (f1, p1, c1) = (clave_azar(), clave_azar(), clave_azar());
        guardar_cache(
            &pool,
            &f1,
            &p1,
            &c1,
            "viejo-1",
            &foto_prueba("hilo-r", "x", "crudo-x"),
        )
        .await
        .expect("guarda 1");
        let (f2, p2, c2) = (clave_azar(), clave_azar(), clave_azar());
        guardar_cache(
            &pool,
            &f2,
            &p2,
            &c2,
            "viejo-2",
            &foto_prueba("hilo-r", "y", "crudo-y"),
        )
        .await
        .expect("guarda 2");
        corregir_cache(&pool, &f2, &p2, &c2, "texto duena")
            .await
            .expect("corrige 2");
        let (f3, p3, c3) = (clave_azar(), clave_azar(), clave_azar());
        guardar_cache(
            &pool,
            &f3,
            &p3,
            &c3,
            "otro-hilo",
            &foto_prueba("hilo-otro", "z", "crudo-z"),
        )
        .await
        .expect("guarda 3");
        let n = borrar_hilo_no_corregidas(&pool, &clave_hilo("hilo-r"))
            .await
            .expect("borra");
        assert_eq!(n, 1, "solo cae el borrador viejo del hilo");
        assert!(
            buscar_cache(&pool, &f2, &p2, &c2)
                .await
                .expect("busca 2")
                .is_some(),
            "la corrección de la dueña queda"
        );
        assert!(
            buscar_cache(&pool, &f3, &p3, &c3)
                .await
                .expect("busca 3")
                .is_some(),
            "el otro hilo no se toca"
        );
        borrar_cache(&pool, &f1, &p1, &c1).await.expect("limpia 1");
        borrar_cache(&pool, &f2, &p2, &c2).await.expect("limpia 2");
        borrar_cache(&pool, &f3, &p3, &c3).await.expect("limpia 3");
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
    /* [08AA-31] La foto fusiona sin perder al cliente: el snapshot nuevo
     * solo trae lo propio (`Tú:`) y la foto vieja aporta la pregunta;
     * el dedup exacto evita duplicar lo que ya estaba. */
    #[test]
    fn combinar_foto_hilo_conserva_cliente_ante_eco_propio() {
        let vieja = "¿Sigue disponible?\nTú: Hola, por favor, déjame un número.";
        let nueva = "Tú: Hola, por favor, déjame un número.";
        assert_eq!(combinar_foto_hilo(vieja, nueva), vieja);
        assert_eq!(combinar_foto_hilo("", nueva), nueva);
        assert_eq!(combinar_foto_hilo(vieja, vieja), vieja);
    }

    /* [08AA-28] Releer crea la fila solo-foto si falta (sin inventar
     * borrador) y la segunda vez solo refresca. Vivo con `pool_si_hay`;
     * sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn releer_crea_fila_si_falta() {
        let Some(pool) = pool_si_hay() else { return };
        let hilo = "releer-test|hilo sintético 08AA-28";
        let limpia = || async {
            sqlx::query("DELETE FROM mp_respuestas_cache WHERE thread_id = $1")
                .bind(clave_hilo(hilo))
                .execute(&pool)
                .await
                .expect("limpia")
        };
        limpia().await;
        let (actualizado, creado) = releer_foto(&pool, hilo, "Hola. ¿Sigue disponible?", "crudo")
            .await
            .expect("releer crea");
        assert!(actualizado && creado);
        let (actualizado2, creado2) =
            releer_foto(&pool, hilo, "Hola. ¿Sigue disponible?", "crudo2")
                .await
                .expect("releer refresca");
        assert!(actualizado2 && !creado2);
        let filas = detalle_chat(&pool, hilo).await.expect("detalle");
        assert_eq!(filas.len(), 1);
        assert_eq!(filas[0].respuesta, "");
        let crudo: (String,) =
            sqlx::query_as("SELECT excerpt_crudo FROM mp_respuestas_cache WHERE thread_id = $1")
                .bind(clave_hilo(hilo))
                .fetch_one(&pool)
                .await
                .expect("lee crudo");
        assert_eq!(crudo.0, "crudo2");
        limpia().await;
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

    /* [09AA-2] La forma del borrador la impone Rust: batería de regresión
     * con los testigos reales (fotos, «sigue vigente», baños). */
    fn ia_fabio(medio: &str) -> String {
        format!(
            "Hola, Fabio, buenas noches, la Casa en Riberas del Caroní está disponible en $43.000 negociable.\n\n{medio}\n\nCuéntame qué estás buscando y con gusto te ayudo, cualquier cosa escríbeme al 0424 9208855\nhttps://wa.me/584249208855"
        )
    }

    #[test]
    fn forma_poda_oferta_de_fotos() {
        let r =
            imponer_forma_borrador(&ia_fabio("¿Te comparto fotos para que la veas por dentro?"));
        assert!(!r.contains("fotos"), "{r}");
        assert!(!r.contains('?'), "{r}");
        assert!(r.contains("$43.000"), "{r}");
        assert!(r.contains(CTA_FIJO), "{r}");
        assert!(r.trim_end().ends_with(CONTACTO_WA), "{r}");
    }

    #[test]
    fn forma_poda_sinonimo_vigente() {
        let r = imponer_forma_borrador(&ia_fabio("Sí, la publicación sigue vigente."));
        assert!(!r.contains("vigente"), "{r}");
        assert!(!r.contains("publicación"), "{r}");
        /* P1 + CTA + teléfono + wa (el teléfono va en bloque propio). */
        assert_eq!(r.split("\n\n").count(), 4, "{r}");
    }

    #[test]
    fn forma_conserva_dato_banos() {
        let r = imponer_forma_borrador(&ia_fabio("Tiene 2 baños y 3 habitaciones."));
        assert!(r.contains("Tiene 2 baños y 3 habitaciones."), "{r}");
        /* P1 + dato + CTA + teléfono + wa. */
        assert_eq!(r.split("\n\n").count(), 5, "{r}");
    }

    #[test]
    fn forma_sanea_pregunta_pegada_en_p1() {
        let ia = "Hola, Fabio, ¿Te comparto fotos? La Casa en Riberas del Caroní está disponible en $43.000 negociable.\n\nCuéntame qué estás buscando y con gusto te ayudo.";
        let r = imponer_forma_borrador(ia);
        assert!(!r.contains('?'), "{r}");
        assert!(r.contains("$43.000"), "{r}");
    }

    #[test]
    fn forma_p1_invalido_da_minimo() {
        let r = imponer_forma_borrador("¿Te comparto fotos para que la veas?");
        assert!(r.contains(&FALLBACK_BORRADOR[..10]), "{r}");
        assert!(r.trim_end().ends_with(CONTACTO_WA), "{r}");
    }

    #[test]
    fn forma_final_siempre_canonico() {
        let r = imponer_forma_borrador(&ia_fabio("Tiene 2 baños."));
        assert!(
            r.contains("Cualquier cosa escríbeme al 0424 9208855"),
            "{r}"
        );
        assert!(r.trim_end().ends_with(CONTACTO_WA), "{r}");
    }

    #[test]
    fn forma_partir_no_rompe_cifras() {
        let f = partir_frases("Cuesta $43.000 negociable. Tiene 2 baños.");
        assert!(f.iter().any(|x| x.contains("$43.000")), "{f:?}");
    }
}
