//! [08AA-3 B4] Rate-limit global por IP en el punto único de entrada.
//!
//! Cubre las 20 rutas POST (auth, chat, IA, inmuebles, marketplace, notas,
//! solicitud, sombra, usuarios, whatsapp...) y cualquier POST futuro sin
//! parchear 13 ficheros de handlers: un solo middleware en `create_router`.
//! Ventana deslizante en memoria (60 s): las escrituras
//! (POST/PUT/PATCH/DELETE) tienen tope estricto y las lecturas uno amplio
//! para no romper el polling del panel. Sin dependencias nuevas.
//!
//! La IP sale de la primera entrada válida de `X-Forwarded-For` (detrás del
//! proxy de Coolify el par TCP siempre es el proxy) y cae al par de
//! `ConnectInfo` si no hay cabecera. El exceso responde 429 con
//! `Retry-After`; nunca se silencia (WARN con la IP y la ruta).

use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{header, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// Ventana deslizante común a lecturas y escrituras.
const VENTANA: Duration = Duration::from_secs(60);
/// Escrituras por minuto e IP (sobra para uso humano del panel/staff).
const TOPE_ESCRITURA: usize = 120;
/* Lecturas por minuto e IP: el panel hace polling del chat/WS-history y el
 * catálogo público recibe crawlers; amplio para no dar falsos 429. */
const TOPE_LECTURA: usize = 1200;
/// Techo de IPs con estado antes de podar las vencidas.
const TOPE_IPS: usize = 20_000;

/// Ventana deslizante por IP compartida entre conexiones (`Clone` barato).
#[derive(Clone, Default)]
pub struct LimitadorTasa {
    interno: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

fn ip_cliente(cabeceras: &axum::http::HeaderMap, par: Option<ConnectInfo<SocketAddr>>) -> String {
    let xff = cabeceras
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok());
    if let Some(ip) = ip_de_cabecera(xff) {
        return ip;
    }
    par.map_or("desconocida".to_string(), |c| c.0.ip().to_string())
}

/// Middleware de `create_router`: deja pasar o responde 429 con `Retry-After`.
pub async fn capa_limite(
    State(limite): State<LimitadorTasa>,
    par: Option<ConnectInfo<SocketAddr>>,
    req: Request,
    siguiente: Next,
) -> Response {
    let escritura = matches!(
        *req.method(),
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    );
    let tope = if escritura {
        TOPE_ESCRITURA
    } else {
        TOPE_LECTURA
    };
    let clave = format!(
        "{}:{}",
        ip_cliente(req.headers(), par),
        if escritura { "e" } else { "l" }
    );
    let ahora = Instant::now();
    let espera_segs = {
        let mut mapa = limite
            .interno
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if mapa.len() > TOPE_IPS {
            mapa.retain(|_, tiempos| {
                tiempos
                    .back()
                    .is_some_and(|t| ahora.duration_since(*t) < VENTANA)
            });
        }
        let tiempos = mapa.entry(clave).or_default();
        while tiempos
            .front()
            .is_some_and(|t| ahora.duration_since(*t) >= VENTANA)
        {
            tiempos.pop_front();
        }
        if excede(tope, tiempos.len()) {
            tiempos.front().map(|t| {
                VENTANA
                    .as_secs()
                    .saturating_sub(ahora.duration_since(*t).as_secs())
                    .max(1)
            })
        } else {
            tiempos.push_back(ahora);
            None
        }
    };
    match espera_segs {
        Some(segs) => {
            tracing::warn!(
                "rate-limit: 429 a {} en {} (tope {tope}/min)",
                ip_cliente(req.headers(), None),
                req.uri().path()
            );
            (
                StatusCode::TOO_MANY_REQUESTS,
                [(header::RETRY_AFTER, segs.to_string())],
                Json(serde_json::json!({
                    "error": "demasiadas peticiones, espera unos segundos"
                })),
            )
                .into_response()
        }
        None => siguiente.run(req).await,
    }
}

/// Pura para testear: ¿el conteo en ventana ya llena el `tope`?
#[must_use]
pub fn excede(tope: usize, intentos: usize) -> bool {
    intentos >= tope
}

/// Pura para testear: la IP es la primera entrada válida de `X-Forwarded-For`.
#[must_use]
pub fn ip_de_cabecera(xff: Option<&str>) -> Option<String> {
    xff.and_then(|h| {
        h.split(',')
            .next()
            .map(str::trim)
            .filter(|s| s.parse::<IpAddr>().is_ok())
            .map(str::to_string)
    })
}

#[cfg(test)]
mod pruebas {
    use super::{excede, ip_de_cabecera, TOPE_ESCRITURA, TOPE_LECTURA};

    #[test]
    fn topes_coherentes() {
        assert!(TOPE_ESCRITURA > 0 && TOPE_LECTURA >= TOPE_ESCRITURA);
        assert!(excede(TOPE_ESCRITURA, TOPE_ESCRITURA));
        assert!(!excede(TOPE_ESCRITURA, TOPE_ESCRITURA - 1));
    }

    #[test]
    fn xff_toma_la_primera_valida() {
        assert_eq!(
            ip_de_cabecera(Some("203.0.113.7, 10.0.0.1")),
            Some("203.0.113.7".to_string())
        );
        assert_eq!(ip_de_cabecera(Some("basura")), None);
        assert_eq!(ip_de_cabecera(None), None);
    }
}
