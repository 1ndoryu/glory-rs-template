use axum::extract::State;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::whatsapp;
use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::repositories::ClienteRepository;
use crate::AppState;

/* [011A-2] Sombra F5-Paso1: huella de lo que MN resolvería para una entrada
 * WhatsApp (canal/modo por reparto, sesión ya existente, estima de uso),
 * SIN escribir nada: la sombra solo compara. Vive tras `GLORY_SHADOW=1`
 * (sin la flag el endpoint es 404); el núcleo `channels` aún no está
 * publicado en este pin, así que esta huella es la línea base contra la que
 * se difuminará el adapter cuando aterrice. Solo SELECTs + puro: nunca crea
 * cliente/sesión/mensaje ni encola outbox. */

/// Entrada a comparar: la misma forma que recibe el webhook.
#[derive(Debug, Deserialize)]
pub struct EntradaSombra {
    pub numero_destino: String,
    pub remitente: String,
    #[serde(default)]
    pub texto: String,
}

/// Huella serializable para el diff futuro contra el núcleo.
#[derive(Debug, Serialize, PartialEq)]
pub struct HuellaSombra {
    pub remitente: String,
    pub canal: Option<String>,
    pub modo: Option<String>,
    pub session_id: Option<Uuid>,
    pub tokens_est: i32,
}

/// La sombra solo existe con `GLORY_SHADOW=1` explícito.
#[must_use]
pub fn sombra_activa_valor(flag: Option<&str>) -> bool {
    flag.is_some_and(|v| v == "1")
}

fn sombra_activa() -> bool {
    sombra_activa_valor(std::env::var("GLORY_SHADOW").ok().as_deref())
}

/// Estima de uso con la misma fórmula del trigger `registrar_uso_estimado`
/// (`GREATEST(1,(char_length+3)/4)`): si el trigger cambia, el diff canta.
#[must_use]
pub fn estima_uso(texto: &str) -> i32 {
    let cuartos = texto.chars().count().saturating_add(3) / 4;
    i32::try_from(cuartos).unwrap_or(i32::MAX).max(1)
}

/// Resuelve la huella con solo lecturas: reparto puro + `clientes` /
/// `canal_sesiones` por SELECT. `None` ante cualquier miss o error de BD
/// (fail-open: la sombra nunca rompe ni inventa sesión).
pub async fn huella(
    pool: &sqlx::PgPool,
    numero_a: &str,
    numero_b: &str,
    numero_destino: &str,
    remitente: &str,
    texto: &str,
) -> HuellaSombra {
    let remitente_norm = ClienteRepository::normalizar_telefono(remitente);
    let (canal, modo) = match whatsapp::reparto(numero_a, numero_b, numero_destino) {
        Some((c, m)) => (Some(c.to_string()), Some(m.to_string())),
        None => (None, None),
    };
    let mut session_id = None;
    if let Some(ref c) = canal {
        if let Ok(Some(cliente_id)) =
            ClienteRepository::id_por_telefono(pool, &remitente_norm).await
        {
            session_id = ClienteRepository::buscar_sesion_por_cliente_canal(pool, cliente_id, c)
                .await
                .ok()
                .flatten();
        }
    }
    HuellaSombra {
        remitente: remitente_norm,
        canal,
        modo,
        session_id,
        tokens_est: estima_uso(texto),
    }
}

/// Compara una entrada sin mutar nada. 404 sin `GLORY_SHADOW=1`.
pub(crate) async fn comparar(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(entrada): Json<EntradaSombra>,
) -> Result<Json<HuellaSombra>, AppError> {
    if !sombra_activa() {
        return Err(AppError::NotFound("sombra apagada".to_string()));
    }
    let (numero_a, numero_b) = whatsapp::numeros_configurados(&state.pool).await;
    Ok(Json(
        huella(
            &state.pool,
            &numero_a,
            &numero_b,
            &entrada.numero_destino,
            &entrada.remitente,
            &entrada.texto,
        )
        .await,
    ))
}

pub fn sombra_routes() -> Router<AppState> {
    Router::new().route("/agent/whatsapp/sombra", axum::routing::post(comparar))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn pool_si_hay() -> Option<sqlx::PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    /* [011A-2] La sombra solo existe con `GLORY_SHADOW=1` exacto. */
    #[test]
    fn sombra_solo_con_flag_exacta() {
        assert!(!sombra_activa_valor(None));
        assert!(!sombra_activa_valor(Some("")));
        assert!(!sombra_activa_valor(Some("0")));
        assert!(!sombra_activa_valor(Some("2")));
        assert!(!sombra_activa_valor(Some("true")));
        assert!(sombra_activa_valor(Some("1")));
    }

    /* [011A-2] Misma fórmula que el trigger: `GREATEST(1,(len+3)/4)` en
     * caracteres (no bytes: `ñ` cuenta 1). */
    #[test]
    fn estima_igual_que_el_trigger() {
        assert_eq!(estima_uso(""), 1);
        assert_eq!(estima_uso("ok"), 1);
        assert_eq!(estima_uso("Hola, busco apartamento"), 6);
        assert_eq!(estima_uso("ññññ"), 1);
    }

    /* [011A-2] Sin cliente vinculado no hay sesión, pero canal/modo/estima
     * sí se resuelven. Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn huella_sin_vinculo_no_inventa_sesion() {
        let Some(pool) = pool_si_hay() else { return };
        let h = huella(
            &pool,
            "584120825234",
            "584249208855",
            "0412 0825234",
            "+34609123456",
            "hola",
        )
        .await;
        assert_eq!(h.remitente, "34609123456");
        assert_eq!(h.canal.as_deref(), Some("wa_a"));
        assert_eq!(h.modo.as_deref(), Some("completo"));
        assert!(h.session_id.is_none());
        assert_eq!(h.tokens_est, 1);
        let h2 = huella(
            &pool,
            "584120825234",
            "584249208855",
            "04120000000",
            "+34609123456",
            "hola",
        )
        .await;
        assert!(h2.canal.is_none());
        assert!(h2.modo.is_none());
        assert!(h2.session_id.is_none());
    }

    /* [011A-2] Con cliente+sesión vinculados la huella la encuentra (solo
     * lectura: el test monta y limpia; la huella no escribe). Sin
     * `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn huella_encuentra_sesion_existente() {
        let Some(pool) = pool_si_hay() else { return };
        let tel = "34609555111";
        let cliente = ClienteRepository::registrar(&pool, Some("Sombra"), tel)
            .await
            .unwrap();
        let sid = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sid)
            .await
            .unwrap();
        ClienteRepository::vincular_canal(&pool, sid, cliente.id, tel, "wa_a", "completo")
            .await
            .unwrap();
        let h = huella(
            &pool,
            "584120825234",
            "584249208855",
            "584120825234",
            tel,
            "hola, busco piso",
        )
        .await;
        assert_eq!(h.session_id, Some(sid));
        assert_eq!(h.canal.as_deref(), Some("wa_a"));
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sid)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sid)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sid)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM clientes WHERE id = $1")
            .bind(cliente.id)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [011A-2] El endpoint es 404 sin la flag (no revela nada) y 200 con
     * ella. Solo este test toca `GLORY_SHADOW` real; ningún otro test del
     * binario la lee, así que no hay carrera. Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn endpoint_404_sin_flag_y_200_con_flag() {
        let Some(pool) = pool_si_hay() else { return };
        let estado = AppState {
            pool,
            jwt_secret: "sombra-test".to_string(),
            upload_dir: std::path::PathBuf::from(r"C:\tmp\sombra-test"),
            static_dir: None,
            hub: glory_agent::session::ChatHub::new(),
        };
        let entrada = || {
            Json(EntradaSombra {
                numero_destino: "04120000000".to_string(),
                remitente: "+34609000000".to_string(),
                texto: "hola".to_string(),
            })
        };
        let auth = || AuthUser {
            user_id: Uuid::new_v4(),
        };
        std::env::remove_var("GLORY_SHADOW");
        let r = comparar(auth(), State(estado.clone()), entrada()).await;
        assert!(matches!(r, Err(AppError::NotFound(_))));
        std::env::set_var("GLORY_SHADOW", "1");
        let r2 = comparar(auth(), State(estado), entrada()).await;
        std::env::remove_var("GLORY_SHADOW");
        let h = r2.unwrap().0;
        assert!(h.canal.is_none());
        assert_eq!(h.tokens_est, 1);
    }
}
