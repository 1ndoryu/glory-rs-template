/* [06AA-3 F3] Capa `Transporte` (plan 03AA-4 §Arquitectura): el evento crudo
 * del gateway normalizado a decisión de encaminamiento, sin BD de negocio
 * (solo lee `agent_config` para los números A/B). Lo que era prefijo de
 * `handlers/whatsapp.rs` (DTO + secreto + números + reparto puro) vive aquí;
 * el webhook solo orquesta. Sin cambios de conducta: movimientos verbatim. */

use serde::Deserialize;

use crate::repositories::ClienteRepository;
use crate::services::modo_por_canal;

#[derive(Debug, Deserialize)]
pub struct EntradaWhatsapp {
    pub(crate) numero_destino: String,
    pub(crate) remitente: String,
    pub(crate) texto: String,
    pub(crate) nombre: Option<String>,
    pub(crate) media_url: Option<String>,
    /* [06AA-1] Campos del transporte real (F-transporte): el simulado no los
     * manda (`None` = dato ausente = el triage atiende, regla de oro). */
    #[serde(default)]
    pub(crate) ts_ms: Option<i64>,
    #[serde(default)]
    pub(crate) from_me: Option<bool>,
    #[serde(default)]
    pub(crate) es_sistema: Option<bool>,
    #[serde(default)]
    pub(crate) client_seq: Option<String>,
}

/// Números MN por defecto (E.164 sin `+`; se normalizan igual que el resto).
/// El B es el vivo del negocio: no usar hasta el final (plan §Estado).
fn numero_a_defecto() -> String {
    std::env::var("WA_NUMERO_A").unwrap_or_else(|_| "584120825234".to_string())
}

fn numero_b_defecto() -> String {
    std::env::var("WA_NUMERO_B").unwrap_or_else(|_| "584249208855".to_string())
}

/// Números configurados: `agent_config` (`wa_numero_a`/`wa_numero_b`, editables
/// en consola F5) con fallback a env (`WA_NUMERO_A`/`WA_NUMERO_B`).
pub(crate) async fn numeros_configurados(pool: &sqlx::PgPool) -> (String, String) {
    let a = glory_agent::persistence::get_config(pool, "wa_numero_a")
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(numero_a_defecto);
    let b = glory_agent::persistence::get_config(pool, "wa_numero_b")
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(numero_b_defecto);
    (
        ClienteRepository::normalizar_telefono(&a),
        ClienteRepository::normalizar_telefono(&b),
    )
}

/// Reparto puro por destino (testeable sin BD): `wa_a`/`completo`, `wa_b`/`inicial`.
/// El `modo` sale de `modo_por_canal` (fuente única, compartida con el resolutor).
#[must_use]
pub fn reparto(
    numero_a: &str,
    numero_b: &str,
    numero_destino: &str,
) -> Option<(&'static str, &'static str)> {
    let destino = ClienteRepository::normalizar_telefono(numero_destino);
    let canal = if destino == ClienteRepository::normalizar_telefono(numero_a) {
        "wa_a"
    } else if destino == ClienteRepository::normalizar_telefono(numero_b) {
        "wa_b"
    } else {
        return None;
    };
    Some((canal, modo_por_canal(canal)?))
}

/// Secreto compartido con el gateway Baileys (llega con F2 real): si
/// `WA_WEBHOOK_SECRETO` está definido, el webhook exige la cabecera
/// `X-Gateway-Secret` idéntica (401 si falta o difiere). Sin definir
/// (simulado/dev local) acepta todo: ni el simulado ni los tests mandan
/// cabecera. Comparación exacta, sin normalizar (un secreto con espacios
/// es válido y se respeta tal cual).
#[must_use]
pub fn secreto_valido(esperado: Option<&str>, recibido: Option<&str>) -> bool {
    let Some(sec) = esperado.filter(|s| !s.is_empty()) else {
        return true;
    };
    recibido.is_some_and(|r| r == sec)
}
