# Prevención: gate declarado no detecta árbol Rust que no compila (2026-10-07)

## Caso
`HEAD 8f9a6fab` (NAKOMI, rama `glory-rust-nakomi`) no compila: ~25 errores
E0432/E0603/E0599 en ~12 archivos (`order/meta.rs:20` usa `s.name`
inexistente —la tabla `services` solo tiene `title`—, módulos ausentes
`ai_chat/ai_providers`, `ai_prompts`, `ai_tools*`, `ai_tools_misc/types`,
`order_slugs`, `deployments` privado, rutas utoipa inexistentes). Pese a
ello pasó el gate declarado y se commiteó. Descubierto al intentar
`sqlx prepare` para 07AA-2 (falla re-verificando todo el workspace).

## Causa raíz
`npm run quality:check` → `task-check.mjs` lee `scripts/quality/stages.json`,
que contiene SOLO la etapa `sentinel` (análisis estático, sin compilar).
Existe `scripts/quality/stages-rust.json` (`fmt/clippy/test`) pero nada lo
invoca en el flujo declarado. El análisis estático no sustituye la
compilación (violación silenciosa de la regla VI del protocolo).

## Detección esperada
Regla/gate `compilacion-rota-rs`: el check de calidad de un proyecto Rust
debe incluir al menos `cargo check` (vía `scripts/quality/cargo-stage.ps1`,
nunca cargo directo por el guard). Si `stages.json` no declara etapa Rust
habiendo `Cargo.toml`, el gate debe fallar como "configuración incompleta".

## Referencia
Roadmap §Barrido consola 07AA (07AA-2 BLOQUEADO); plan
`Agente/planes/plan-barrido-consola-07AA-2026-10-07.md` F2.
