# Plan 279A-4 — Migrar queries runtime a macros sqlx (2026-09-27)

## Objetivo
Eliminar 72 findings `sqlx-query(-as)-sin-macro` del WM convirtiendo queries
runtime a `query!`/`query_as!`/`query_scalar!` con verificación en compilación.

## Causa raíz (verificada, no supuesta)
Los sitios flagged tocan tablas que NO existen en la BD local de compilación
(`migrate info`: ~30 pendientes, ej. `20260405000000_chat`,
`20260723100000_chat_alert_system`, `20260727100000_seo_settings`,
`20260515000000_google_auth`). Los headers de `chat_alert.rs`, `email_log.rs`
y `notification.rs::create_tx` ya documentan el runtime como intencional.
Migrar a macros sin BD al día rompería `cargo check`.

## Fases
1. **Fase 1 — BD local al día:** `cargo sqlx migrate run` (solo localhost,
   reversible con down-files). Éxito = 0 pending en `migrate info`.
   Fallback si falla: aceptar patrón runtime, cerrar 279A-4 documentado.
2. **Fase 2 — Migración (59 sitios, 12 archivos):** `query_as::<_, T>(...)`
   + `.bind()` → `query_as!(T, r#"..."#, args)`; `query()` → `query!`;
   `query_scalar` → `query_scalar!`. Excluido: `refund.rs` (13 sitios con
   `format!` dinámico — no migrable a macro).
3. **Fase 3 — Cierre:** `cargo sqlx prepare` (regenera `.sqlx`), gate
   `sentinel check 279A-4 --stages scripts/quality/stages-rust.json`
   (detached; cargo directo bloqueado por guard exit 78), archivar en
   `Agente/completados/tareas-2026-09-27.md`, commit + push.

## Estado
- Fase 1: HECHA. `nakomi_dev` creada (aislada; `glory_backend_kamples` es BD
  compartida con RESTAURANTE: 74 applied ajenos, colisiones de versión).
  `migrate run` aplicó todo (0 pending real; el único match es la palabra
  "pending" en la descripción `remove pending payment order`).
- Fase 2: HECHA. 59 sitios en 12 archivos → `query!`/`query_as!`/`query_scalar!`;
  `refund.rs` (13) excluido por `format!` dinámico (único `sqlx-query-as-sin-macro`
  restante en gate, aceptado). Gotchas compile-time: `query_as!` no acepta tuplas
  (→ `query!` + map); `EXISTS/COUNT/SUMA/COALESCE` infieren `Option` (→ alias
  `AS "x!"` dentro de `r#"..."#); `ORDER BY` debe citar el alias con `!`
  (`ORDER BY "bandwidth_used_gb!"`); `$n` tras `CONCAT`/aritmética necesita
  `CAST` explícito; `ON CONFLICT DO NOTHING` sin arbiter evita inferencia de índice.
- Fase 3: HECHA. `cargo sqlx prepare -D nakomi_dev` exit 0 (`.sqlx` 187→249
  ficheros); gate `task-check.mjs 279A-4` PASS full 106 archivos, 0 errores,
  política v2 enforce (`a4f70948…`).

## Próximo paso verificable
CERRADO 2026-09-27. Siguiente: 279A-5 `claseHuerfana` (129).
