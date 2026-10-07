# Plan 07AA — Barrido consola NAKOMI (73 → accionables 0)

Origen: consola área `problemas (73)` NAKOMI 2026-10-07 (cache `analisisNAKOMI`,
auto-timer; vigencia declarada por el endpoint). El gate propio está en
16W con waivers (ver roadmap §Gate 01AA); la consola cuenta en crudo
info/hints + waivers. Este plan los recorre por frentes.

## Fases

- [x] **F1 07AA-1 — varsense x4 (claseHuerfana): FP confirmado, sin cambio.**
  `uplot`/`u-legend` = clases runtime de uPlot (`ResourceUsageChart.tsx:2-3`
  importa `uplot` + su CSS dist); `tiptap`/`is-editor-empty` = clases runtime
  de TipTap (`RichTextEditor.tsx:9,174` `EditorContent`). Familia 4 de
  `Agente/prevencion/prevencion-claseHuerfana-falsos-positivos-2026-09-28.md`
  (gap sin allowlist pendiente en la herramienta, fuera de alcance aquí).
- [ ] **F2 07AA-2 — handler-accede-bd-rs x4 → repositorios. BLOQUEADO
  (árbol HEAD no compila; no mezclar deuda ajena).** Código F2 terminado
  (DIP verbatim, 4 upserts en repos, `sentinel-disable-file` retirado del
  handler); las 4 queries nuevas están verificadas live contra la BD de rama
  (entradas `.sqlx/query-6a9b3c…/99d029…/d3feb9…/f2231e…` con `describe` OK).
  Pero `sqlx prepare` no completa porque el árbol en HEAD `8f9a6fab` no
  compila: `order/meta.rs:20-21` usa `s.name` (la tabla `services` solo tiene
  `title`), faltan módulos (`ai_chat/ai_providers`, `ai_prompts`,
  `ai_tools*`, `ai_tools_misc/types`, `order_slugs`), `deployments` privado
  en 4 handlers, rutas utoipa inexistentes y `axum::extract::Query` sin
  importar (~25 errores E0432/E0603/E0599 en ~12 archivos ajenos a F2).
  Propuesta: nueva tarea 07AA-7 (reparar árbol) como prerrequisito; F2 se
  commitea tras ella. Sin commit a medias sobre árbol roto.
- [ ] **F3 07AA-3 — sqlite-carga-N-consultas x8: triage join! vs FP.**
  Sitios: `fixtures.rs:128`, `rest_messages.rs:292`, `resolve.rs:216`,
  `continuation_token.rs:78,140`, `ai_tools_orders.rs:85,222`,
  `ai_tools_reports.rs:83`, `chat_timing_escalation.rs:143`.
  Precedente 01AA-3 (11→2 reales + 9 FP documentados).
- [ ] **F4 07AA-4 — parametros-excesivos-rs x13 → structs de params.**
  `chat_alert.rs:25` (9), `email_log.rs:64` (9), `coolify.rs:465,803,1220`
  (9/9/9), `email_admin.rs` x6 (9-13), `email_orders.rs` x3 (9-10),
  `hosting_runtime_lifecycle.rs:44` (10), `checkout.rs:229` (9).
  Convención existente: `CreateHostingParams<'_>` & co.
- [ ] **F5 07AA-5 — god-object-rs x3: re-evaluar waiver F3g/regla14.**
  `handlers/mod.rs` (643), `handlers/vps.rs` (610), `services/coolify.rs`
  (613). Mantener waiver solo con justificación vigente; si se parte,
  por dominio como en 259A-4/01AA-4-F3.
- [ ] **F6 07AA-6 — large-interface-isp x37 (info): decisión de alcance.**
  Roadmap declara fuera de alcance "sin autorización"; la orden del usuario
  2026-10-07 ("resolver todo") la suple. DTOs de API (`frontend/src/api/*`)
  vs props de componente (FaseCard 19, OrdenDetalle 17, etc.): partir solo
  donde aporte ISP real, sin churn de contratos por silenciar el linter
  (regla 3 zero-patches).
- [ ] **F7 — re-verificar:** `quality:check` por bloque, `POST
  /api/gate/analizar {clave:NAKOMI, forzar:true}`, commit+push por bloque
  (regla 12), completadas, releer roadmap (regla 16).

## No alcance

Deploys/producción (restricción operativa roadmap); WIPs ajenos; puertos
8787/5174/5175; `../.quality-tools-harness/*`.
