/* [01AA-4-F3r] Extraído de `main.rs`: logger del heartbeat del watchdog y
 * volcado de stacks del kernel para diagnóstico de deadlocks. Sin cambio de
 * comportamiento. */

use std::time::Duration;

use glory_rs::runtime::RuntimeHeartbeat;

pub(crate) fn spawn_runtime_heartbeat_logger(heartbeat: RuntimeHeartbeat) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("hb-logger".into())
        .spawn(move || {
            let mut last_sequence = 0;
            let mut last_progress = std::time::Instant::now();
            std::thread::sleep(Duration::from_secs(15));
            loop {
                let current_sequence = heartbeat.sequence();
                if current_sequence != last_sequence {
                    last_sequence = current_sequence;
                    last_progress = std::time::Instant::now();
                }
                let status = if current_sequence == 0 {
                    "starting".to_string()
                } else {
                    format!("healthy stalled_for={}s", last_progress.elapsed().as_secs())
                };
                eprintln!("[hb-logger] sequence={current_sequence} status={status}");
                std::thread::sleep(Duration::from_secs(15));
            }
        })?;
    Ok(())
}

/* Volcar stacks del kernel de todos los threads via /proc/self/task/TID/stack.
 * Funciona dentro de Docker en Linux. No requiere gdb ni herramientas externas.
 * Los stacks del kernel muestran si un thread está bloqueado en futex (mutex),
 * esperando I/O, o en estado running. Muy útil para diagnosticar deadlocks.
 * [096A-11] En Docker, /proc/self/task/TID/stack puede estar vacío. Como fallback,
 * listar threads con su estado y nombre para diagnóstico mínimo. */
pub(crate) fn dump_kernel_stacks() {
    let Ok(tasks) = std::fs::read_dir("/proc/self/task") else {
        eprintln!("[rt-watchdog] No se pudo leer /proc/self/task");
        return;
    };
    let mut found_any = false;
    for entry in tasks.flatten() {
        let tid = entry.file_name();
        let tid_str = tid.to_string_lossy();
        let stack_path = format!("/proc/self/task/{tid_str}/stack");
        let status_path = format!("/proc/self/task/{tid_str}/status");
        let name = std::fs::read_to_string(&status_path)
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("Name:"))
                    .map(|l| l.trim_start_matches("Name:").trim().to_string())
            })
            .unwrap_or_default();
        let state = std::fs::read_to_string(&status_path)
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("State:"))
                    .map(|l| l.trim_start_matches("State:").trim().to_string())
            })
            .unwrap_or_default();
        if let Ok(stack) = std::fs::read_to_string(&stack_path) {
            let trimmed = stack.trim();
            if !trimmed.is_empty() && trimmed != "(empty)" {
                eprintln!("--- Thread {tid_str} ({name}) [{state}] ---\n{trimmed}\n");
                found_any = true;
            }
        }
        /* [096A-11] Fallback: si stacks vacíos, listar threads con estado */
        if !found_any {
            eprintln!("[rt-watchdog] Thread {tid_str}: name={name} state={state}");
        }
    }
}
