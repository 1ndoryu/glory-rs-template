# Prevención: falsos positivos de `sqlite-carga-N-consultas` Sentinel

**Fecha:** 2026-10-01 · **Origen:** auditoría 01AA-3 (NAKOMI, 11 sitios en 8 archivos: 2 reales con `join!`, 9 FP).

## Familias de FP verificadas contra código real

1. **Ramas `match` mutuamente excluyentes** (solo una ejecuta, no hay nada que paralelizar):
   `query_orders_for_scope` (`ai_tools_orders.rs:63`), `query_payments_for_scope`
   (`ai_tools_orders.rs:200`), `query_reports_for_scope` (`ai_tools_reports.rs:61`).
2. **Ramas `if`/`else-if` excluyentes** (un solo slug matchea):
   `dynamic_seo_for_path` (`middleware/prerender.rs:331`, servicios/proyectos/blog).
3. **Transacción explícita** (misma `tx`, orden secuencial por diseño):
   `mark_connected` (`repositories/continuation_token.rs:58`),
   `schedule_after_disconnect` (`repositories/continuation_token.rs:109`, el INSERT usa la fila del SELECT).
4. **Cadena de datos** (cada await consume el resultado del anterior):
   `check_and_update_timing` (`services/chat_timing_escalation.rs:110`,
   summary → perfil → update), `create_message` (`handlers/chat/rest_messages.rs:230`,
   msg → sesión → IA).
5. **Orden impuesto por FK** (hijos antes que padres, no paralelizable):
   limpieza legacy (`main.rs:565`, cascade chat → orders → hosting).

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
