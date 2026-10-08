# Prevención: `ruta-post-sin-rate-limit` no ve middleware global (2026-10-08)

## Caso mínimo
21 findings `ruta-post-sin-rate-limit` en `src/handlers/*.rs`
(`auth`, `chat`, `chat_staff`, `ia`, `inmuebles`, `marketplace`,
`notes`, `solicitud`, `sombra`, `suscriptor`, `users`, `whatsapp`)
pese a que TODAS las rutas pasan por el middleware global de [08AA-3 B4]:
`src/handlers/rate_limit.rs` (`LimitadorTasa`: escritura 120/min,
lectura 1200/min, 429 + `Retry-After` + WARN) cableado con
`from_fn_with_state` en `create_router` + `ConnectInfo` en `main.rs`.

## Capa responsable
Regla `ruta-post-sin-rate-limit` de Sentinel: heurística textual por fichero
(busca `governor`/`RateLimit` en el fichero o en `Cargo.toml`), ciega a
mitigaciones arquitectónicas transversales (middleware Tower global).

## Detección esperada
- Antes de añadir `governor` por fichero, comprobar `create_router` en
  `src/handlers/mod.rs`: si hay una capa de límite global con cubos por IP,
  los 21 findings son falsos positivos, no 21 rutas desprotegidas.
- El waiver por fichero (`sentinel-disable-file` × 21) sería churn invasivo;
  un solo middleware global es estrictamente mejor que 21 governors locales
  (cubre rutas futuras por defecto).

## Regla
No añadir rate-limit por fichero para callar la regla cuando existe
mitigación global. Documentar el falso positivo aquí y referenciarlo desde
el roadmap; si la regla evoluciona, que acepte como mitigación válida una
capa `from_fn_with_state`/`from_fn` con semántica de cubo + 429 en el
`create_router` del crate.

## Referencia
Roadmap 08AA-3 B4 (autorizaba "waiver o limite"; se implementó límite
global); `src/handlers/rate_limit.rs`, `src/handlers/mod.rs`
(`create_router`), `src/main.rs` (`ConnectInfo`).
