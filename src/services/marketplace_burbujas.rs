//! Burbujas estructuradas del chat Marketplace — F0 (09AA-20).
//!
//! El flotante manda la conversación ya separada `[{lado, texto}]` y el
//! backend la guarda tal cual, sin adivinar desde texto plano. Fin de la
//! familia wilmery/edickson/edgarluis: cada burbuja trae su dueño y los
//! mensajes del sistema (tips de seguridad, aviso de Meta) viajan como
//! `sistema`, nunca camuflados como cliente o dueña.
//!
//! Contrato F0: `{v: 1, hilo_hint, burbujas: [{lado, texto}]}`.
//! - `v` viaja como `i64` para que una versión futura produzca el error
//!   tipado `unknown-version` (con fallback a texto plano) en vez de un
//!   422 genérico de deserialización.
//! - `hilo_hint` es opaco: solo se usa para logs sin PII, jamás entra en
//!   la firma ni en claves de caché/dedup.
//! - `lado: sistema` se descarta (ruido del DOM, no conversación);
//!   `desconocido` se conserva y se renderiza como `Desconocido:` para que
//!   la IA lo vea tal cual. Si más del 30% queda sin dueño, el payload se
//!   rechaza con `reintento-foreground` sin guardar nada.
//! - `firma-v2 = sha256` de las burbujas útiles normalizadas en orden
//!   cronológico; convive con `firma-v1` (texto plano legacy).
//! - UTF-8 lo garantiza axum/`serde_json` en el boundary (rechazan bytes
//!   inválidos antes de llegar aquí); este módulo solo chequea NUL.
//!
//! Pendiente (fuera de F0): explicar la marca `Desconocido:` en el prompt
//! del sistema (dueño 09AA-2) y traer `precio_hash`/`catalog_hash` reales
//! al vuelo estructurado (hoy se firman como `sin-ficha`, ver handler).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

/// Versión de esquema aceptada hoy.
pub const VERSION_ESTRUCTURADA: i64 = 1;
/// Firma calculada sobre burbujas normalizadas (convive con `firma-v1`).
pub const FIRMA_VERSION_V2: &str = "firma-v2";
/// Variable de entorno kill-switch: `off`/`0`/`no` = estructuradas apagadas.
pub const ENV_KILL_SWITCH: &str = "MP_BURBUJAS_ESTRUCTURADAS";
/// Topes anti-abuso de la F0.
pub const MAX_BURBUJAS: usize = 50;
pub const MAX_TOTAL_CARACTERES: usize = 20_000;
pub const MAX_POR_BURBUJA: usize = 2_000;
/// `hilo_hint` es corto y opaco (un hash del flotante, no un identificador).
pub const MAX_HINT_CARACTERES: usize = 500;
/// Cabecera de idempotencia: 1..=128 visibles ASCII sin espacios.
pub const MAX_IDEMPOTENCY_CHARS: usize = 128;
/// Códigos de error estables que consume el flotante.
pub const CODIGO_VERSION_DESCONOCIDA: &str = "unknown-version";
pub const CODIGO_REINTENTO_FOREGROUND: &str = "reintento-foreground";
pub const CODIGO_PAYLOAD_GIGANTE: &str = "payload-gigante";
pub const CODIGO_ESQUEMA: &str = "esquema-invalido";
pub const CODIGO_IDEMPOTENCIA: &str = "idempotency-key-invalida";

/// Dueño declarado de cada burbuja por el flotante.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Lado {
    Cliente,
    Duena,
    Sistema,
    Desconocido,
}

/// Dueños que sobreviven al validador (`sistema` se descarta: es ruido del
/// DOM, no conversación). Tipo separado para que el descarte sea
/// estructural y no una bandera que alguien pueda olvidar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LadoUtil {
    Cliente,
    Duena,
    Desconocido,
}

impl LadoUtil {
    /// Marca legible que ve la IA en el prompt (`Cliente:`/`Dueña:` son las
    /// que el prompt ya conoce; `Desconocido:` se explica en 09AA-2).
    #[must_use]
    pub fn marca(self) -> &'static str {
        match self {
            Self::Cliente => "Cliente",
            Self::Duena => "Dueña",
            Self::Desconocido => "Desconocido",
        }
    }

    /// Marca canónica ASCII para la firma (estable, sin tildes).
    #[must_use]
    pub fn canonica(self) -> &'static str {
        match self {
            Self::Cliente => "cliente",
            Self::Duena => "duena",
            Self::Desconocido => "desconocido",
        }
    }

    /// Convierte un lado ya filtrado. `Sistema` es inalcanzable aquí (el
    /// validador lo descarta antes); se mapea a `Desconocido` en defensa.
    #[must_use]
    pub fn de_util(lado: Lado) -> Self {
        match lado {
            Lado::Cliente => Self::Cliente,
            Lado::Duena => Self::Duena,
            Lado::Sistema | Lado::Desconocido => Self::Desconocido,
        }
    }
}

/// Una burbuja tal como la manda el flotante.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct BurbujaIn {
    pub lado: Lado,
    pub texto: String,
}

/// Conversación estructurada tal como la manda el flotante.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ConversacionEstructurada {
    pub v: i64,
    pub hilo_hint: String,
    pub burbujas: Vec<BurbujaIn>,
}

/// Burbuja validada: texto recortado, dueño útil.
#[derive(Debug, Clone)]
pub struct BurbujaUtil {
    pub lado: LadoUtil,
    pub texto: String,
}

/// Resultado del validador, listo para firmar y renderizar.
#[derive(Debug, Clone)]
pub struct ConversacionValidada {
    /// Burbujas útiles en orden cronológico (el de llegada).
    pub utiles: Vec<BurbujaUtil>,
    /// `firma-v2` hex64 sobre las útiles normalizadas.
    pub firma_v2: String,
    /// Total de burbujas recibidas (incluye las del sistema descartadas).
    pub total_burbujas: usize,
    pub descartadas_sistema: usize,
    pub desconocidas: usize,
}

/// Error tipado de la F0: `codigo` estable para el flotante + mensaje humano.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorEstructurado {
    pub codigo: &'static str,
    pub mensaje: String,
}

impl std::fmt::Display for ErrorEstructurado {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.codigo, self.mensaje)
    }
}

impl std::error::Error for ErrorEstructurado {}

impl From<ErrorEstructurado> for crate::errors::AppError {
    fn from(e: ErrorEstructurado) -> Self {
        Self::Validation(e.to_string())
    }
}

fn esquema(mensaje: String) -> ErrorEstructurado {
    ErrorEstructurado {
        codigo: CODIGO_ESQUEMA,
        mensaje,
    }
}

fn gigante(mensaje: String) -> ErrorEstructurado {
    ErrorEstructurado {
        codigo: CODIGO_PAYLOAD_GIGANTE,
        mensaje,
    }
}

fn reintento(mensaje: String) -> ErrorEstructurado {
    ErrorEstructurado {
        codigo: CODIGO_REINTENTO_FOREGROUND,
        mensaje,
    }
}

/// Kill-switch: `MP_BURBUJAS_ESTRUCTURADAS=off|0|no` desactiva la F0 y el
/// handler usa texto plano. Ausente o cualquier otro valor = activada.
#[must_use]
pub fn estructuradas_apagadas() -> bool {
    std::env::var(ENV_KILL_SWITCH).is_ok_and(|v| {
        let n = v.trim().to_ascii_lowercase();
        n == "off" || n == "0" || n == "no"
    })
}

/// Valida la conversación estructurada sin guardar nada. Orden de chequeos:
/// versión → hint → topes de cantidad → cada burbuja (NUL, vacío,
/// por-burbuja, total) → umbral de desconocidas → firma.
pub fn validar_conversacion(
    c: &ConversacionEstructurada,
) -> Result<ConversacionValidada, ErrorEstructurado> {
    if c.v != VERSION_ESTRUCTURADA {
        return Err(ErrorEstructurado {
            codigo: CODIGO_VERSION_DESCONOCIDA,
            mensaje: format!(
                "versión {} no soportada; reenviá la conversación como texto plano",
                c.v
            ),
        });
    }
    validar_hint(&c.hilo_hint)?;
    if c.burbujas.is_empty() {
        return Err(esquema("conversación vacía".to_string()));
    }
    if c.burbujas.len() > MAX_BURBUJAS {
        return Err(gigante(format!(
            "más de {MAX_BURBUJAS} burbujas; partí el hilo"
        )));
    }
    let mut utiles = Vec::new();
    let mut total = 0usize;
    let mut descartadas_sistema = 0usize;
    for b in &c.burbujas {
        if matches!(b.lado, Lado::Sistema) {
            descartadas_sistema += 1;
            continue;
        }
        let texto = b.texto.trim();
        if texto.contains('\0') {
            return Err(esquema("burbuja con NUL incrustado".to_string()));
        }
        if texto.is_empty() {
            return Err(esquema("burbuja con texto vacío".to_string()));
        }
        let n = texto.chars().count();
        if n > MAX_POR_BURBUJA {
            return Err(gigante(format!(
                "burbuja de {n} caracteres supera {MAX_POR_BURBUJA}"
            )));
        }
        total += n;
        if total > MAX_TOTAL_CARACTERES {
            return Err(gigante(format!(
                "conversación supera {MAX_TOTAL_CARACTERES} caracteres; partí el hilo"
            )));
        }
        utiles.push(BurbujaUtil {
            lado: LadoUtil::de_util(b.lado),
            texto: texto.to_owned(),
        });
    }
    if utiles.is_empty() {
        return Err(reintento(
            "solo trae mensajes del sistema; reenviá el texto visible del hilo".to_string(),
        ));
    }
    let desconocidas = utiles
        .iter()
        .filter(|u| u.lado == LadoUtil::Desconocido)
        .count();
    if desconocidas * 10 > utiles.len() * 3 {
        return Err(reintento(format!(
            "{desconocidas} de {} burbujas sin dueño (>30%); reenviá el texto visible del hilo",
            utiles.len()
        )));
    }
    let firma_v2 = firmar(&utiles);
    Ok(ConversacionValidada {
        utiles,
        firma_v2,
        total_burbujas: c.burbujas.len(),
        descartadas_sistema,
        desconocidas,
    })
}

/// `hilo_hint` opaco y corto; jamás entra en firma ni claves.
fn validar_hint(hint: &str) -> Result<(), ErrorEstructurado> {
    let h = hint.trim();
    if h.is_empty() {
        return Err(esquema("hilo_hint requerido".to_string()));
    }
    if h.contains('\0') {
        return Err(esquema("hilo_hint con NUL incrustado".to_string()));
    }
    if h.chars().count() > MAX_HINT_CARACTERES {
        return Err(esquema(format!(
            "hilo_hint supera {MAX_HINT_CARACTERES} caracteres"
        )));
    }
    Ok(())
}

/// Firma canónica: `v1` + una línea `lado:texto` por burbuja útil en orden
/// cronológico, unidas con `\n`. El hint queda fuera a propósito.
fn firmar(utiles: &[BurbujaUtil]) -> String {
    let mut canonica = String::from("v1");
    for u in utiles {
        canonica.push('\n');
        canonica.push_str(u.lado.canonica());
        canonica.push(':');
        canonica.push_str(&u.texto);
    }
    sha_hex(canonica.as_bytes())
}

/// `sha256` local: duplicado de 6 líneas a propósito para no crear ciclo
/// `marketplace_burbujas → marketplace` (`marketplace.rs` importa este
/// módulo; si este importara `sha_hex` de vuelta, clippy llora).
fn sha_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// Render `Marca: texto` por línea, en orden cronológico. Es lo que ve la IA
/// y lo que se guarda como excerpt normalizado.
#[must_use]
pub fn texto_para_prompt(v: &ConversacionValidada) -> String {
    v.utiles
        .iter()
        .map(|u| format!("{}: {}", u.lado.marca(), u.texto))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Valida el `Idempotency-Key` del flotante (opaco para el servidor: lo
/// devuelve tal cual en la respuesta y lo mezcla en la clave del vuelo).
/// Por contrato el flotante manda `sha_hex(hint + firma_v2)`; ver
/// `llave_esperada`.
pub fn validar_idempotency_key(valor: &str) -> Result<(), ErrorEstructurado> {
    let n = valor.chars().count();
    let visibles = valor.bytes().all(|b| b.is_ascii_graphic());
    if valor.is_empty() || n > MAX_IDEMPOTENCY_CHARS || !visibles {
        return Err(ErrorEstructurado {
            codigo: CODIGO_IDEMPOTENCIA,
            mensaje: format!(
                "Idempotency-Key de 1..={MAX_IDEMPOTENCY_CHARS} caracteres visibles sin espacios"
            ),
        });
    }
    Ok(())
}

/// Llave que el flotante debe mandar como `Idempotency-Key`: ata el hint
/// opaco con la firma para que reintentos del mismo hilo colapsen.
#[must_use]
pub fn llave_esperada(hint: &str, firma_v2: &str) -> String {
    sha_hex(format!("{hint}:{firma_v2}").as_bytes())
}

#[cfg(test)]
mod pruebas {
    use super::super::marketplace_texto::{validar_borrador, BorradorRequest};
    use super::*;

    const HINT: &str = "hilo-edgarluis-a1b2c3d4";
    const TIP_SEGURIDAD: &str = "Si te vas a reunir con el comprador, hazlo en un lugar público.";
    const AVISO_META: &str = "Meta podría usar esta conversación para mejorar sus sistemas.";

    fn lado_de(nombre: &str) -> Lado {
        match nombre {
            "cliente" => Lado::Cliente,
            "duena" => Lado::Duena,
            "sistema" => Lado::Sistema,
            _ => Lado::Desconocido,
        }
    }

    fn conv(v: i64, hint: &str, pares: &[(&str, &str)]) -> ConversacionEstructurada {
        ConversacionEstructurada {
            v,
            hilo_hint: hint.to_string(),
            burbujas: pares
                .iter()
                .map(|(lado, texto)| BurbujaIn {
                    lado: lado_de(lado),
                    texto: (*texto).to_string(),
                })
                .collect(),
        }
    }

    fn fixture_base() -> ConversacionEstructurada {
        conv(
            1,
            HINT,
            &[
                ("cliente", "Hola. ¿Sigue estando disponible?"),
                (
                    "duena",
                    "Hola, buenas noches, la Casa en Villa Icabarú está disponible en $90.000 negociable.",
                ),
                ("sistema", TIP_SEGURIDAD),
                ("sistema", AVISO_META),
            ],
        )
    }

    #[test]
    fn estructurada_v1_ok_descarta_sistema() {
        let val = validar_conversacion(&fixture_base()).expect("fixture válida");
        assert_eq!(val.utiles.len(), 2);
        assert_eq!(val.total_burbujas, 4);
        assert_eq!(val.descartadas_sistema, 2);
        assert_eq!(val.desconocidas, 0);
        assert_eq!(val.firma_v2.len(), 64);
        assert!(val.firma_v2.chars().all(|c| c.is_ascii_hexdigit()));
        let prompt = texto_para_prompt(&val);
        assert!(prompt.contains("Cliente: Hola. ¿Sigue estando disponible?"));
        assert!(prompt.contains("Dueña: Hola, buenas noches"));
        assert!(!prompt.contains(TIP_SEGURIDAD));
        assert!(!prompt.contains(AVISO_META));
    }

    #[test]
    fn firma_determinista_y_sensible_al_orden() {
        let a = validar_conversacion(&fixture_base()).expect("a");
        let b = validar_conversacion(&fixture_base()).expect("b");
        assert_eq!(a.firma_v2, b.firma_v2);
        let invertida = conv(
            1,
            HINT,
            &[
                (
                    "duena",
                    "Hola, buenas noches, la Casa en Villa Icabarú está disponible en $90.000 negociable.",
                ),
                ("cliente", "Hola. ¿Sigue estando disponible?"),
            ],
        );
        let c = validar_conversacion(&invertida).expect("c");
        assert_ne!(a.firma_v2, c.firma_v2);
    }

    #[test]
    fn hint_no_entra_en_firma() {
        let otra = conv(
            1,
            "hilo-distinto-ffffffff",
            &[
                ("cliente", "Hola. ¿Sigue estando disponible?"),
                (
                    "duena",
                    "Hola, buenas noches, la Casa en Villa Icabarú está disponible en $90.000 negociable.",
                ),
                ("sistema", TIP_SEGURIDAD),
                ("sistema", AVISO_META),
            ],
        );
        let a = validar_conversacion(&fixture_base()).expect("a");
        let b = validar_conversacion(&otra).expect("b");
        assert_eq!(a.firma_v2, b.firma_v2);
    }

    #[test]
    fn version_desconocida_es_error_tipiado() {
        for v in [0, 2, 99] {
            let err = validar_conversacion(&conv(v, HINT, &[("cliente", "Hola")]))
                .expect_err("versión futura debe fallar");
            assert_eq!(err.codigo, CODIGO_VERSION_DESCONOCIDA);
            assert!(err.mensaje.contains("texto plano"));
        }
    }

    #[test]
    fn todo_desconocido_o_solo_sistema_reintenta_foreground() {
        let sin_dueno = conv(
            1,
            HINT,
            &[
                ("desconocido", "Hola. ¿Sigue disponible?"),
                ("desconocido", "Sí, $90.000."),
                ("desconocido", "¿Cuándo puedo verla?"),
            ],
        );
        let err = validar_conversacion(&sin_dueno).expect_err("100% sin dueño");
        assert_eq!(err.codigo, CODIGO_REINTENTO_FOREGROUND);

        let solo_ruido = conv(
            1,
            HINT,
            &[("sistema", TIP_SEGURIDAD), ("sistema", AVISO_META)],
        );
        let err = validar_conversacion(&solo_ruido).expect_err("solo sistema");
        assert_eq!(err.codigo, CODIGO_REINTENTO_FOREGROUND);
    }

    #[test]
    fn umbral_30_por_ciento_de_desconocidas() {
        let muchas: Vec<(&str, &str)> = (0..7)
            .map(|_| ("cliente", "Sigue disponible?"))
            .chain((0..3).map(|_| ("desconocido", "bla")))
            .collect();
        assert_eq!(muchas.len(), 10);
        let val = validar_conversacion(&conv(1, HINT, &muchas)).expect("3/10 pasa");
        assert_eq!(val.desconocidas, 3);

        let pocas = [
            ("cliente", "Hola"),
            ("cliente", "Sigue?"),
            ("desconocido", "x"),
            ("desconocido", "y"),
        ];
        let err = validar_conversacion(&conv(1, HINT, &pocas)).expect_err("2/4 no pasa");
        assert_eq!(err.codigo, CODIGO_REINTENTO_FOREGROUND);
    }

    #[test]
    fn payload_gigante_se_rechaza() {
        let muchas: Vec<(&str, &str)> = (0..51).map(|_| ("cliente", "Hola")).collect();
        let err = validar_conversacion(&conv(1, HINT, &muchas)).expect_err("51 burbujas");
        assert_eq!(err.codigo, CODIGO_PAYLOAD_GIGANTE);

        let larga = "x".repeat(MAX_POR_BURBUJA + 1);
        let err = validar_conversacion(&conv(1, HINT, &[("cliente", larga.as_str())]))
            .expect_err("burbuja gigante");
        assert_eq!(err.codigo, CODIGO_PAYLOAD_GIGANTE);

        let medianas: Vec<String> = (0..11).map(|_| "y".repeat(1900)).collect();
        let pares: Vec<(&str, &str)> = medianas.iter().map(|s| ("cliente", s.as_str())).collect();
        let err = validar_conversacion(&conv(1, HINT, &pares)).expect_err("total gigante");
        assert_eq!(err.codigo, CODIGO_PAYLOAD_GIGANTE);
    }

    #[test]
    fn cliente_que_cita_tip_se_conserva_palabra_por_palabra() {
        let c = conv(
            1,
            HINT,
            &[
                ("cliente", TIP_SEGURIDAD),
                ("duena", "Claro, nos vemos en el centro comercial."),
            ],
        );
        let val = validar_conversacion(&c).expect("cita del cliente es conversación");
        assert_eq!(val.utiles.len(), 2);
        let prompt = texto_para_prompt(&val);
        assert!(prompt.contains(&format!("Cliente: {TIP_SEGURIDAD}")));
    }

    #[test]
    fn burbuja_vacia_nul_y_hint_vacio_son_esquema() {
        let vacia = conv(1, HINT, &[("cliente", "   ")]);
        assert_eq!(
            validar_conversacion(&vacia).expect_err("vacía").codigo,
            CODIGO_ESQUEMA
        );
        let nul = conv(1, HINT, &[("cliente", "hola\0mundo")]);
        assert_eq!(
            validar_conversacion(&nul).expect_err("nul").codigo,
            CODIGO_ESQUEMA
        );
        let sin_hint = conv(1, "  ", &[("cliente", "Hola")]);
        assert_eq!(
            validar_conversacion(&sin_hint)
                .expect_err("sin hint")
                .codigo,
            CODIGO_ESQUEMA
        );
        let sin_burbujas = conv(1, HINT, &[]);
        assert_eq!(
            validar_conversacion(&sin_burbujas)
                .expect_err("vacía total")
                .codigo,
            CODIGO_ESQUEMA
        );
    }

    #[test]
    fn idempotency_key_se_valida_y_llave_esperada_ata_hint_con_firma() {
        assert!(validar_idempotency_key("a1b2c3d4e5").is_ok());
        assert_eq!(
            validar_idempotency_key("").expect_err("vacía").codigo,
            CODIGO_IDEMPOTENCIA
        );
        assert_eq!(
            validar_idempotency_key("con espacios no")
                .expect_err("espacios")
                .codigo,
            CODIGO_IDEMPOTENCIA
        );
        let larga = "k".repeat(MAX_IDEMPOTENCY_CHARS + 1);
        assert_eq!(
            validar_idempotency_key(&larga).expect_err("larga").codigo,
            CODIGO_IDEMPOTENCIA
        );
        let a = llave_esperada(
            HINT,
            "firmaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        let b = llave_esperada(
            "otro-hint",
            "firmaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
    }

    #[test]
    fn texto_viejo_sin_conversacion_sigue_valido() {
        let crudo = serde_json::json!({
            "threadId": "hilo-edgarluis",
            "firma": "ab".repeat(32),
            "firma_version": "firma-v1",
            "lang": "es",
            "excerpt": {
                "remitente_hash": "cd".repeat(32),
                "texto": "Hola. ¿Sigue estando disponible?",
                "hora": "2026-10-09T12:59:00-04:00"
            }
        });
        let r: BorradorRequest = serde_json::from_value(crudo).expect("legacy parsea");
        assert!(r.conversacion.is_none());
        assert!(validar_borrador(&r).is_empty());
    }
}
