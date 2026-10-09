/* F0 estructuradas/idempotencia extraído de `marketplace.rs` (split
 * god-object: el boundary superaba 800 efectivas). Comportamiento idéntico:
 * solo se movió de archivo; `marketplace.rs` conserva el wiring (llamadas).
 * Sin PII en logs: solo conteos, códigos e `hilo8`. */
/* [09AA-22] F3 endurecido: `hilo_hint` solo como hash en logs (jamás
 * PK/firma/dedup), `FuenteBorrador` arrastra la firma legacy del body para
 * el dual-lookup v2→v1, y la `Idempotency-Key` se verifica contra
 * `llave_esperada(hint, firma_v2)` (422 si no corresponde). */

use axum::http::HeaderMap;
use axum::response::Response;

use crate::errors::AppError;
use crate::services::marketplace::{
    buscar_cache, clave_hilo, estructuradas_apagadas, llave_esperada, normalizar_excerpt_hilo,
    sha_hex, texto_para_prompt, validar_conversacion, validar_idempotency_key, BorradorRequest,
    ConversacionValidada, ErrorEstructurado, CODIGO_IDEMPOTENCIA, FIRMA_VERSION_V2,
};

use super::mp_logs::{hilo8, mp_log, LogNivel};

/* [09AA-20] F0: de qué conversación hablamos. La estructurada (`Some` +
 * kill-switch encendido) se valida, se firma v2 y se renderiza a
 * `excerpt.texto`; el resto del flujo (caché, vuelo, IA) no distingue.
 * `firma_cache` es la llave de caché/dedup; `crudo` lo que se guarda para
 * calibrar. Sin PII en logs: solo conteos y códigos. */
/* [09AA-22] F3: `firma_legacy` es la `firma` del body (compat del flotante),
 * ignorada como llave principal pero reutilizada como fallback de lectura
 * durante la transición v1→v2 (ver `buscar_cache_convivencia`). */
#[derive(Debug)]
pub(crate) struct FuenteBorrador {
    pub(crate) firma_cache: String,
    pub(crate) firma_version: String,
    pub(crate) crudo: String,
    pub(crate) firma_legacy: Option<String>,
}

/// `hilo_hint` opaco a hash-8 para logs: correlaciona sin exponer el hint
/// (el hint puede derivar de URL+aviso y viaja solo en logs, jamás en
/// PK/firma/dedup). Se hashea directo, sin `clave_hilo()` (ese normaliza
/// `thread_id`, no hints).
fn hint8(hint: &str) -> String {
    sha_hex(hint.trim()).chars().take(8).collect()
}

pub(crate) fn resolver_fuente(r: &mut BorradorRequest) -> Result<FuenteBorrador, AppError> {
    if let Some(c) = r.conversacion.as_ref() {
        if estructuradas_apagadas() {
            mp_log(
                LogNivel::Warn,
                "borrador.estructurada",
                "apagada",
                "kill-switch activo: va por texto plano".to_string(),
                &[
                    ("hilo", serde_json::json!(hilo8(r.thread_id.trim()))),
                    ("hint", serde_json::json!(hint8(&c.hilo_hint))),
                ],
            );
        } else {
            match validar_conversacion(c) {
                Ok(val) => {
                    mp_log(
                        LogNivel::Info,
                        "borrador.estructurada",
                        "ok",
                        format!(
                            "{} útiles ({} sistema fuera, {} sin dueño)",
                            val.utiles.len(),
                            val.descartadas_sistema,
                            val.desconocidas
                        ),
                        &[
                            ("hilo", serde_json::json!(hilo8(r.thread_id.trim()))),
                            ("hint", serde_json::json!(hint8(&c.hilo_hint))),
                            ("recibidas", serde_json::json!(val.total_burbujas)),
                            ("utiles", serde_json::json!(val.utiles.len())),
                            (
                                "descartadas_sistema",
                                serde_json::json!(val.descartadas_sistema),
                            ),
                            ("desconocidas", serde_json::json!(val.desconocidas)),
                        ],
                    );
                    let crudo =
                        serde_json::to_string(c).unwrap_or_else(|_| "{\"v\":1}".to_string());
                    /* La firma legacy del body viaja por compat; no manda,
                     * pero se arrastra para el fallback de lectura (F3). */
                    let legacy = r.firma.clone();
                    r.excerpt.texto = texto_para_prompt(&val);
                    return Ok(fuente_v2(&val, crudo, legacy));
                }
                Err(e) => {
                    mp_log(
                        LogNivel::Warn,
                        "borrador.estructurada",
                        "rechazada",
                        format!("{}: se espera texto plano", e.codigo),
                        &[
                            ("hilo", serde_json::json!(hilo8(r.thread_id.trim()))),
                            ("hint", serde_json::json!(hint8(&c.hilo_hint))),
                            ("codigo", serde_json::json!(e.codigo)),
                        ],
                    );
                    return Err(e.into());
                }
            }
        }
    }
    Ok(fuente_v1(r))
}

/// F0 aceptada: la firma v2 manda sobre la `firma` del body (el flotante la
/// manda igual por compat, pero la llave real es la calculada aquí).
/* [09AA-22] F3: la legacy se conserva aparte para `buscar_cache_convivencia`
 * (fallback de lectura, jamás llave principal ni parte de la firma). */
pub(crate) fn fuente_v2(
    val: &ConversacionValidada,
    crudo: String,
    legacy: String,
) -> FuenteBorrador {
    FuenteBorrador {
        firma_cache: val.firma_v2.clone(),
        firma_version: FIRMA_VERSION_V2.to_string(),
        crudo,
        firma_legacy: Some(legacy),
    }
}

/* Texto plano legacy: mismo comportamiento de siempre (limpia excerpt,
 * conserva el original si solo había ruido, crudo para calibrar). */
pub(crate) fn fuente_v1(r: &mut BorradorRequest) -> FuenteBorrador {
    let crudo = r.excerpt.texto.clone();
    let limpio = normalizar_excerpt_hilo(&clave_hilo(r.thread_id.trim()), &r.excerpt.texto);
    if !limpio.is_empty() {
        r.excerpt.texto = limpio;
    }
    FuenteBorrador {
        firma_cache: r.firma.clone(),
        firma_version: "firma-v1".to_string(),
        crudo,
        firma_legacy: None,
    }
}

/* [09AA-22] F3: la llave de idempotencia ata hint+firma
 * (`llave_esperada`). Si el flotante manda una llave que no corresponde a
 * este hilo+firma (reintento cruzado entre hilos), se rechaza con 422 antes
 * de tocar caché o IA. Sin llave o sin estructurada no hay nada que atar. */
pub(crate) fn verificar_idempotencia_conversacion(
    r: &BorradorRequest,
    fuente: &FuenteBorrador,
    clave: Option<&String>,
) -> Result<(), AppError> {
    if fuente.firma_version != FIRMA_VERSION_V2 {
        return Ok(());
    }
    let Some(c) = r.conversacion.as_ref() else {
        return Ok(());
    };
    if estructuradas_apagadas() {
        return Ok(());
    }
    let Some(k) = clave else {
        return Ok(());
    };
    let esperada = llave_esperada(c.hilo_hint.trim(), &fuente.firma_cache);
    if k != &esperada {
        mp_log(
            LogNivel::Warn,
            "borrador.idempotencia",
            "no-corresponde",
            "Idempotency-Key de otro hilo+firma; reenviá la llave de este hilo".to_string(),
            &[
                ("hilo", serde_json::json!(hilo8(r.thread_id.trim()))),
                ("hint", serde_json::json!(hint8(&c.hilo_hint))),
            ],
        );
        return Err(ErrorEstructurado {
            codigo: CODIGO_IDEMPOTENCIA,
            mensaje: "Idempotency-Key no corresponde a este hilo+firma".to_string(),
        }
        .into());
    }
    Ok(())
}

/* [09AA-22] F3: convivencia v1→v2 en lectura. La llave principal es la v2;
 * si falla y el body traía firma legacy distinta, se reintenta con ella
 * para no regenerar (y re-gastar IA) lo que ya se resolvió en texto plano.
 * La respuesta sigue marcada con la versión pedida (v2); el hit legacy
 * solo se anota en logs para medir la transición. */
pub(crate) async fn buscar_cache_convivencia(
    pool: &sqlx::PgPool,
    firma_v2: &str,
    firma_legacy: Option<&str>,
    precio_hash: &str,
    catalog_hash: &str,
) -> Result<Option<(String, bool)>, AppError> {
    if let Some(hit) = buscar_cache(pool, firma_v2, precio_hash, catalog_hash).await? {
        return Ok(Some(hit));
    }
    if let Some(legacy) = firma_legacy {
        if !legacy.is_empty() && legacy != firma_v2 {
            if let Some(hit) = buscar_cache(pool, legacy, precio_hash, catalog_hash).await? {
                mp_log(
                    LogNivel::Info,
                    "borrador.cache",
                    "legacy-hit",
                    "hit por firma-v1 de transición".to_string(),
                    &[("llave", serde_json::json!("v1"))],
                );
                return Ok(Some(hit));
            }
        }
    }
    Ok(None)
}

/// `Idempotency-Key` opcional: si viene se valida (422 si es basura) y se
/// devuelve tal cual en la respuesta; además entra al vuelo para que un
/// reintento colapse con el original en vez de disparar otra IA.
pub(crate) fn clave_idempotencia(headers: &HeaderMap) -> Result<Option<String>, AppError> {
    let Some(valor) = headers.get("idempotency-key") else {
        return Ok(None);
    };
    let texto = valor.to_str().map_err(|_| {
        AppError::Validation("Idempotency-Key con caracteres inválidos".to_string())
    })?;
    validar_idempotency_key(texto).map_err(AppError::from)?;
    Ok(Some(texto.to_string()))
}

/// Devuelve la llave en la respuesta para que el flotante correlacione.
pub(crate) fn con_idempotencia(mut resp: Response, clave: Option<&String>) -> Response {
    if let Some(k) = clave {
        if let Ok(v) = axum::http::HeaderValue::from_str(k) {
            resp.headers_mut()
                .insert(axum::http::HeaderName::from_static("idempotency-key"), v);
        }
    }
    resp
}

/* [09AA-22] Matriz F3 a nivel `resolver_fuente` (+ verificación de llave):
 * el caller (`borrador`/`regenerar_uno`) no guarda nada si esto devuelve
 * `Err`, así que "422 sin guardar" se fija aquí: excerpt intacto + código
 * estable en el mensaje. Asserts por contenido exacto y sha256, nunca solo
 * `length`. El candado serializa el módulo porque el test kill-switch toca
 * la env global del proceso. */
#[cfg(test)]
mod pruebas_f3 {
    use super::*;
    use crate::services::marketplace::{
        BurbujaIn, ConversacionEstructurada, ExcerptIn, Lado, CODIGO_PAYLOAD_GIGANTE,
        CODIGO_REINTENTO_FOREGROUND, CODIGO_VERSION_DESCONOCIDA, ENV_KILL_SWITCH,
    };
    use std::sync::{Mutex, OnceLock};

    static CANDADO_ENV: OnceLock<Mutex<()>> = OnceLock::new();
    /* [09AA-22] Guarda RAII: restaura la env al salir del test. Vive aquí
     * arriba (clippy `items_after_statements` prohíbe items tras código). */
    struct Quita;
    impl Drop for Quita {
        fn drop(&mut self) {
            std::env::remove_var(ENV_KILL_SWITCH);
        }
    }
    fn candado() -> std::sync::MutexGuard<'static, ()> {
        CANDADO_ENV
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    const HINT_A: &str = "hilo-edgarluis-a1b2c3d4";
    const HINT_B: &str = "hilo-wilmery-f9e8d7c6";
    const CLI: &str = "Hola. ¿Sigue estando disponible?";
    const DUE: &str =
        "Hola, buenas noches, la Casa en Villa Icabarú está disponible en $90.000 negociable.";
    const TIP: &str = "Si te vas a reunir con el comprador, hazlo en un lugar público.";

    fn burbuja(lado: Lado, texto: &str) -> BurbujaIn {
        BurbujaIn {
            lado,
            texto: texto.to_string(),
        }
    }

    fn conversacion(v: i64, hint: &str, burbujas: Vec<BurbujaIn>) -> ConversacionEstructurada {
        ConversacionEstructurada {
            v,
            hilo_hint: hint.to_string(),
            burbujas,
        }
    }

    fn base_ok(hint: &str) -> ConversacionEstructurada {
        conversacion(
            1,
            hint,
            vec![
                burbuja(Lado::Cliente, CLI),
                burbuja(Lado::Duena, DUE),
                burbuja(Lado::Sistema, TIP),
            ],
        )
    }

    fn peticion(conv: Option<ConversacionEstructurada>) -> BorradorRequest {
        BorradorRequest {
            thread_id: "edgarluis|Casa en Villa Icabarú".to_string(),
            firma: "ab".repeat(32),
            firma_version: if conv.is_some() {
                FIRMA_VERSION_V2.to_string()
            } else {
                "firma-v1".to_string()
            },
            lang: "es".to_string(),
            excerpt: ExcerptIn {
                remitente_hash: "cd".repeat(32),
                texto: "texto plano original".to_string(),
                hora: "2026-10-09T12:59:00-04:00".to_string(),
                leido: false,
            },
            aviso_id: None,
            extras: None,
            conversacion: conv,
        }
    }

    #[test]
    fn f3_estructurada_ok_marcas_orden_y_firma() {
        let _g = candado();
        let mut req = peticion(Some(base_ok(HINT_A)));
        let fuente = resolver_fuente(&mut req).expect("estructurada válida");
        assert_eq!(fuente.firma_version, FIRMA_VERSION_V2);
        /* Contenido exacto: marcas `Cliente:`/`Dueña:`, orden cronológico,
         * sistema descartado (el tip no aparece). */
        assert_eq!(req.excerpt.texto, format!("Cliente: {CLI}\nDueña: {DUE}"));
        assert!(!req.excerpt.texto.contains(TIP));
        /* Firma pinneada a la canónica (`v1` + `lado:texto` por línea):
         * sha256 exacto, no solo longitud. */
        let firma_esperada = sha_hex(&format!("v1\ncliente:{CLI}\nduena:{DUE}"));
        assert_eq!(fuente.firma_cache, firma_esperada);
        assert_eq!(fuente.firma_cache.len(), 64);
        assert!(fuente.firma_cache.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(
            fuente.firma_legacy.as_deref(),
            Some("ab".repeat(32).as_str())
        );
    }

    #[test]
    fn f3_texto_viejo_sin_conversacion_sigue_v1() {
        let _g = candado();
        let firma = "ef".repeat(32);
        let mut req = peticion(None);
        req.firma = firma.clone();
        let fuente = resolver_fuente(&mut req).expect("plano válido");
        assert_eq!(fuente.firma_version, "firma-v1");
        assert_eq!(fuente.firma_cache, firma);
        assert!(fuente.firma_legacy.is_none());
        assert_eq!(req.excerpt.texto, "texto plano original");
    }

    #[test]
    fn f3_version_desconocida_422_sin_guardar() {
        let _g = candado();
        let mut req = peticion(Some(conversacion(
            99,
            HINT_A,
            vec![burbuja(Lado::Cliente, CLI)],
        )));
        match resolver_fuente(&mut req) {
            Err(AppError::Validation(m)) => assert!(
                m.contains(CODIGO_VERSION_DESCONOCIDA),
                "esperaba {CODIGO_VERSION_DESCONOCIDA}, fue: {m}"
            ),
            otro => panic!("esperaba 422 unknown-version, fue: {otro:?}"),
        }
        /* Sin guardar: el excerpt queda intacto (el flotante reintenta en
         * texto plano ante este código). */
        assert_eq!(req.excerpt.texto, "texto plano original");
    }

    #[test]
    fn f3_todas_desconocidas_422_reintento() {
        let _g = candado();
        let mut req = peticion(Some(conversacion(
            1,
            HINT_A,
            vec![
                burbuja(Lado::Desconocido, "Hola. ¿Sigue disponible?"),
                burbuja(Lado::Desconocido, "Sí, $90.000."),
                burbuja(Lado::Desconocido, "¿Cuándo puedo verla?"),
            ],
        )));
        match resolver_fuente(&mut req) {
            Err(AppError::Validation(m)) => assert!(
                m.contains(CODIGO_REINTENTO_FOREGROUND),
                "esperaba {CODIGO_REINTENTO_FOREGROUND}, fue: {m}"
            ),
            otro => panic!("esperaba 422 reintento-foreground, fue: {otro:?}"),
        }
        assert_eq!(req.excerpt.texto, "texto plano original");
    }

    #[test]
    fn f3_gigante_422_payload() {
        let _g = candado();
        let muchas = vec![burbuja(Lado::Cliente, "Hola"); 51];
        let mut req = peticion(Some(conversacion(1, HINT_A, muchas)));
        match resolver_fuente(&mut req) {
            Err(AppError::Validation(m)) => assert!(
                m.contains(CODIGO_PAYLOAD_GIGANTE),
                "esperaba {CODIGO_PAYLOAD_GIGANTE}, fue: {m}"
            ),
            otro => panic!("esperaba 422 payload-gigante, fue: {otro:?}"),
        }
        assert_eq!(req.excerpt.texto, "texto plano original");
    }

    #[test]
    fn f3_kill_switch_off_ignora_estructurada() {
        let _g = candado();
        std::env::set_var(ENV_KILL_SWITCH, "off");
        let _quita = Quita;
        let mut req = peticion(Some(base_ok(HINT_A)));
        let fuente = resolver_fuente(&mut req).expect("con kill-switch va por plano");
        assert_eq!(fuente.firma_version, "firma-v1");
        assert_eq!(fuente.firma_cache, "ab".repeat(32));
        assert!(fuente.firma_legacy.is_none());
        assert!(
            !req.excerpt.texto.contains("Cliente:"),
            "la estructurada se ignora: {}",
            req.excerpt.texto
        );
    }

    #[test]
    fn f3_idempotencia_ata_hint_y_firma() {
        let _g = candado();
        let mut req = peticion(Some(base_ok(HINT_A)));
        let fuente = resolver_fuente(&mut req).expect("válida");
        let buena = llave_esperada(HINT_A, &fuente.firma_cache);
        assert_eq!(buena.len(), 64);
        assert!(buena.chars().all(|c| c.is_ascii_hexdigit()));
        verificar_idempotencia_conversacion(&req, &fuente, Some(&buena)).expect("llave propia ok");
        verificar_idempotencia_conversacion(&req, &fuente, None).expect("sin llave ok");
        /* Llave de otro hilo con la misma firma: 422 (test con 2 hilos). */
        let ajena = llave_esperada(HINT_B, &fuente.firma_cache);
        assert_ne!(buena, ajena);
        match verificar_idempotencia_conversacion(&req, &fuente, Some(&ajena)) {
            Err(AppError::Validation(m)) => assert!(
                m.contains(CODIGO_IDEMPOTENCIA),
                "esperaba {CODIGO_IDEMPOTENCIA}, fue: {m}"
            ),
            otro => panic!("esperaba 422 idempotency-key-invalida, fue: {otro:?}"),
        }
    }

    #[test]
    fn f3_hint_no_entra_en_firma() {
        let _g = candado();
        let mut ra = peticion(Some(base_ok(HINT_A)));
        let mut rb = peticion(Some(base_ok(HINT_B)));
        let fa = resolver_fuente(&mut ra).expect("a");
        let fb = resolver_fuente(&mut rb).expect("b");
        assert_eq!(fa.firma_cache, fb.firma_cache);
        assert_eq!(ra.excerpt.texto, rb.excerpt.texto);
        assert_ne!(
            llave_esperada(HINT_A, &fa.firma_cache),
            llave_esperada(HINT_B, &fb.firma_cache)
        );
    }
}
