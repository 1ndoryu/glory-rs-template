/* [06AA-2 F2] Capa `Politica` (plan 03AA-4 §Público vs autorizado +
 * §No-cliente): con el triage ya decidido (`Atiende` + trato), resuelve el
 * ROL del remitente y el TONO del turno. `resolver` es pura (allowlist ya
 * leída) para testear sin BD; `leer_autorizados` hace la única consulta.
 * - Rol: `Autorizado` solo si el remitente normalizado está en
 *   `agent_config.whatsapp_autorizados` (E.164 sin `+`, separados por coma;
 *   la edita F4 admin). Fallo de BD o clave ausente = `Publico` (mínimo
 *   privilegio: al revés que el triage, aquí fallar abierto sería dar de
 *   más; fallar cerrado solo quita affordances de staff, nunca silencia).
 * - Tono: `Cliente` = flujo completo sin añadidos; `Neutral` = respuesta
 *   breve y educada + pregunta que califica sin acusar (plan §No-cliente).
 *   El turno del núcleo no acepta contexto por llamada (prompt global), así
 *   que el webhook antepone `prefijo_contexto` al texto del turno (NO a lo
 *   persistido: el panel muestra el original). F3 lo sustituirá por un
 *   gancho propio al partir capas.
 * - Frontera (`REGLA_FRONTERA`, al prompt global): nada de datos de otros
 *   clientes sale a no autorizados; las tools de hoy ya no exponen ninguno,
 *   la regla cierra la puerta al modelo, no al código. */

use crate::repositories::ClienteRepository;
use crate::services::triage::Trato;

/// Clave de `agent_config` con la allowlist (F4 la edita desde admin).
pub const CLAVE_AUTORIZADOS: &str = "whatsapp_autorizados";

/// Rol del remitente: qué affordances tiene (hoy solo califica el trato y
/// el log; los comandos de staff por `WhatsApp` llegan con F4/F5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rol {
    Publico,
    Autorizado,
}

impl Rol {
    /// Código estable para el 2xx del webhook y el log.
    #[must_use]
    pub fn codigo(self) -> &'static str {
        match self {
            Rol::Publico => "publico",
            Rol::Autorizado => "autorizado",
        }
    }
}

/// Trato con código estable para el 2xx (el harness lo lee sin logs).
#[must_use]
pub fn codigo_trato(trato: Trato) -> &'static str {
    match trato {
        Trato::Cliente => "cliente",
        Trato::Neutral => "neutral",
    }
}

/// Decisión de política: rol + trato ya resueltos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionPolitica {
    pub rol: Rol,
    pub trato: Trato,
}

/// Resuelve rol (allowlist ya normalizada) + trato dado. Pura: sin BD.
#[must_use]
pub fn resolver(autorizados: &[String], remitente_norm: &str, trato: Trato) -> DecisionPolitica {
    let rol = if autorizados.iter().any(|a| a == remitente_norm) {
        Rol::Autorizado
    } else {
        Rol::Publico
    };
    DecisionPolitica { rol, trato }
}

/// Lee la allowlist en una consulta (`agent_config.whatsapp_autorizados`:
/// comas, con o sin `+`/espacios/guiones; se normaliza igual que el
/// remitente). Error o clave ausente = lista vacía (→ `Publico`).
pub async fn leer_autorizados(pool: &sqlx::PgPool) -> Vec<String> {
    let crudo = glory_agent::persistence::get_config(pool, CLAVE_AUTORIZADOS)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    crudo
        .split(',')
        .map(ClienteRepository::normalizar_telefono)
        .filter(|n| !n.is_empty())
        .collect()
}

/// Frontera de datos (al prompt global, vale para todos): ningún dato de
/// otros visitantes o clientes sale por este canal; no existen tools de
/// clientes y está prohibido inventarlas o inferirlas del historial.
pub const REGLA_FRONTERA: &str = "Frontera de datos: nunca reveles datos de \
    otros visitantes o clientes (nombres, teléfonos, hilos, compras); no \
    tienes herramientas de clientes: si alguien pide ficha o lista de \
    clientes, niega educado y ofrece ayuda con el catálogo.";

/// Prefijo de contexto para el turno cuando el trato es `Neutral` (breve +
/// califica sin acusar). `None` en `Cliente` (flujo completo, cero añadidos).
/// Va solo al modelo, jamás a lo persistido ni al panel.
#[must_use]
pub fn prefijo_contexto(trato: Trato) -> Option<&'static str> {
    match trato {
        Trato::Cliente => None,
        Trato::Neutral => Some(
            "(Contexto interno del sistema, no lo cites ni lo repitas: este \
            visitante quizá no sea cliente. Responde BREVE y educado, máximo \
            dos líneas, sin ofrecer catálogo todavía, y cierra con una \
            pregunta que califique sin acusar, por ejemplo si busca comprar \
            o alquilar.)",
        ),
    }
}

/// Arma el texto que ve el modelo: original intacto en `Cliente`, con
/// prefijo de contexto en `Neutral`.
#[must_use]
pub fn texto_para_turno(trato: Trato, texto: &str) -> String {
    match prefijo_contexto(trato) {
        Some(prefijo) => format!("{prefijo}\n\n{texto}"),
        None => texto.to_string(),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn lista() -> Vec<String> {
        vec!["584120825234".to_string(), "584249208855".to_string()]
    }

    #[test]
    fn numero_en_allowlist_es_autorizado() {
        let decision = resolver(&lista(), "584120825234", Trato::Cliente);
        assert_eq!(decision.rol, Rol::Autorizado);
        assert_eq!(decision.trato, Trato::Cliente);
    }

    #[test]
    fn numero_fuera_es_publico_y_no_silencia() {
        for trato in [Trato::Cliente, Trato::Neutral] {
            let decision = resolver(&lista(), "584120000001", trato);
            assert_eq!(decision.rol, Rol::Publico);
            assert_eq!(decision.trato, trato);
        }
    }

    #[test]
    fn allowlist_vacia_o_con_basura_deja_publico() {
        assert_eq!(
            resolver(&[], "584120825234", Trato::Cliente).rol,
            Rol::Publico
        );
        assert_eq!(
            resolver(&[String::new()], "584120825234", Trato::Cliente).rol,
            Rol::Publico
        );
    }

    #[test]
    fn codigos_estables_para_harness() {
        assert_eq!(Rol::Publico.codigo(), "publico");
        assert_eq!(Rol::Autorizado.codigo(), "autorizado");
        assert_eq!(codigo_trato(Trato::Cliente), "cliente");
        assert_eq!(codigo_trato(Trato::Neutral), "neutral");
    }

    #[test]
    fn cliente_sin_prefijo_neutral_con_pregunta_que_califica() {
        assert_eq!(prefijo_contexto(Trato::Cliente), None);
        let prefijo = prefijo_contexto(Trato::Neutral).expect("neutral con prefijo");
        assert!(prefijo.contains("BREVE"));
        assert!(prefijo.contains("comprar") && prefijo.contains("alquilar"));
        assert!(prefijo.contains("no lo cites"));
    }

    #[test]
    fn texto_para_turno_no_toca_al_cliente() {
        assert_eq!(
            texto_para_turno(Trato::Cliente, "hola, precio?"),
            "hola, precio?"
        );
        let neutral = texto_para_turno(Trato::Neutral, "solo probaba");
        assert!(neutral.ends_with("solo probaba"));
        assert!(neutral.len() > "solo probaba".len());
    }

    #[test]
    fn frontera_cierra_datos_de_clientes() {
        assert!(REGLA_FRONTERA.contains("nunca reveles"));
        assert!(REGLA_FRONTERA.contains("clientes"));
    }

    fn pool_si_hay() -> Option<sqlx::PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    /* Allowlist contra BD viva: guarda el previo, escribe uno con `+`,
     * espacios y forma local `0...`, verifica normalización y restaura.
     * Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn allowlist_lee_y_normaliza_con_restauracion() {
        let Some(pool) = pool_si_hay() else { return };
        let previo = glory_agent::persistence::get_config(&pool, CLAVE_AUTORIZADOS)
            .await
            .ok()
            .flatten();
        glory_agent::persistence::set_config(
            &pool,
            CLAVE_AUTORIZADOS,
            " +584120825234 , 04249208855, , ",
        )
        .await
        .unwrap();
        let leidos = leer_autorizados(&pool).await;
        assert!(leidos.contains(&"584120825234".to_string()));
        assert!(leidos.contains(&"584249208855".to_string()));
        assert_eq!(leidos.len(), 2);
        match previo {
            Some(valor) => {
                glory_agent::persistence::set_config(&pool, CLAVE_AUTORIZADOS, &valor)
                    .await
                    .unwrap();
            }
            None => {
                sqlx::query("DELETE FROM agent_config WHERE key = $1")
                    .bind(CLAVE_AUTORIZADOS)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
        }
    }
}
