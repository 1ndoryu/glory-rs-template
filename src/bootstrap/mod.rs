/* [01AA-4-F3r] Bootstrap del binario extraído de `main.rs` (766L, god-object 566):
 * `fixtures` = pool + fixtures + seed legacy; `background` = tareas de fondo;
 * `loops` = loops de chat; `server` = listener + hyper + watchdog;
 * `diagnostics` = heartbeat logger + volcado de stacks. `main.rs` queda como
 * entrypoint delgado (~20 efectivas). Misma firma pública; sin cambio de
 * comportamiento. */

pub mod background;
pub mod diagnostics;
pub mod fixtures;
pub mod loops;
pub mod server;
