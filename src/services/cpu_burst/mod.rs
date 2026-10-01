/* [265A-1][265A-5] Politica dinamica de CPU para hostings Coolify.
 * `baseline_burst` mantiene el baseline comercial siempre y habilita burst con
 * holgura sostenida. `contention_throttle` deja el sitio sin cap mientras el
 * host esta sano y solo aplica un limite compartido cuando la VPS entra en
 * contencion real.
 * [01AA-4-F3j] Partido por capa (era god-object 700+): state = tipos + consts +
 * mapa; policy = decisión pura testeable; actuate = SSH + loop. */

mod actuate;
mod policy;
mod state;

pub use actuate::cpu_burst_loop;
