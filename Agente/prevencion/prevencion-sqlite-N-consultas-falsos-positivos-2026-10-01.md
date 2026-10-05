# Prevención: falsos positivos de `sqlite-carga-N-consultas` Sentinel

**Fecha:** 2026-10-01 · **Origen:** auditoría 01AA-3 (NAKOMI, 11 sitios en 8 archivos: 2 reales con `join!`, 9 FP).
**Re-verificación:** 2026-10-05 (01AA-4-f3s): los 9 sitios actuales son las mismas 5 familias
(líneas desplazadas por los splits F3a–F3r: `main.rs` → `bootstrap/fixtures.rs`,
`middleware/prerender.rs` → `middleware/prerender/resolve.rs`). Ninguno paralelizable.

## Familias de FP verificadas contra código real

1. **Ramas `match` mutuamente excluyentes** (solo una ejecuta, no hay nada que paralelizar):
   `query_orders_for_scope` (`ai_tools_orders.rs:85`), `query_payments_for_scope`
   (`ai_tools_orders.rs:222`), `query_reports_for_scope` (`ai_tools_reports.rs:83`).
2. **Ramas `if`/`else-if` excluyentes** (un solo slug matchea):
   `dynamic_seo_for_path` (`middleware/prerender/resolve.rs:216`, servicios/proyectos/blog).
3. **Transacción explícita** (misma `tx`, orden secuencial por diseño):
   `mark_connected` (`repositories/continuation_token.rs:78`),
   `schedule_after_disconnect` (`repositories/continuation_token.rs:140`, el INSERT usa la fila del SELECT).
4. **Cadena de datos** (cada await consume el resultado del anterior):
   background summary (`services/chat_timing_escalation.rs:143`,
   summary → perfil → update), `send_message` (`handlers/chat/rest_messages.rs:292`,
   msg → sesión → IA).
5. **Orden impuesto por FK** (hijos antes que padres, no paralelizable):
   limpieza legacy (`bootstrap/fixtures.rs:128`, cascade chat → orders → hosting).

## Casos reales (los que sí se paralelizaron con `join!`)

- `exec_admin_operational_summary` (`ai_tools_reports.rs:289`): `query_admin_report_stats` +
  `query_hosting_status_counts` independientes con fallback `unwrap_or` (61W→59W).
- `process_email` (`chat_alert_worker.rs:292`): en cada brazo de error,
  `revoke_for_session` + `mark_retry` fire-and-forget sobre tablas distintas.

## Detección esperada

- No contar awaits en brazos `match`/`if-else` excluyentes como si fueran secuenciales.
- No contar awaits sobre la misma `&mut tx` ni cuando un await consume bindings de otro.
- Reconocer `tokio::join!` ya presente para no re-reportar.
- Sin automatizar: los FP quedan como warnings aceptados (`severityCounts.error = 0`,
  el gate cierra en PASS con warnings).

## Referencia

Roadmap Gate 01AA → `Agente/completados/tareas-2026-10-01.md` (01AA-3).
Commits `e830db7e` (DIP orders) + `93a94b0e` (join! x2).
