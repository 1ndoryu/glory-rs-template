# Plan 03AA-3 — Asistente Marketplace: repo propio + respuestas con glory-agent

## Objetivo
Sacar el asistente de respuestas de Messenger de `opencode-propio` a un repo
nuevo `plugins-opencode` (núcleo agnóstico, SOLID, con Sentinel), y pasar sus
borradores de plantillas a glory-agent con caché inteligente sin gastar tokens.

## Alcance / no alcance
- Sí: E1 repo nuevo + extracción del núcleo; E2 adaptador delgado en
  opencode-propio; E3 CLI del núcleo; M1 eliminar strip lateral; M2 panel
  anclado al composer + ocultar al minimizar/burbuja; M3 endpoint backend MN;
  M4 tabla de caché + Regenerar.
- No: auto-envío (prohibido, riesgo de cierre), credenciales/sesión de Meta
  (las pone la usuaria), API oficial de Messenger (solo Páginas).

## Dependencias
- `opencode-propio` no es repo git y lo trabaja otro frente: la E2 se coordina,
  no se invade (solo se toca el adaptador `marketplace-*`).
- Backend MN local con `OPENCODE_GO_API_KEY` (ya existe) para M3/M4.
- Remoto del repo nuevo (lo crea la usuaria o se trabaja local hasta su aviso).

## Fases verificables
- **E1 — Repo `plugins-opencode` + núcleo:** crear repo, Sentinel desde día 1,
  extraer `assistant` (intención/emparejado/plantillas), `catalog`,
  `watch` (lector), `cache` y `generador` (interfaz; plantillas = fallback).
  Tests del núcleo en verde + typecheck. Núcleo sin imports de Electron.
- **E2 — Adaptador delgado:** `opencode-propio` consume el paquete (float
  in-page + IPC + clipboard); se borra la lógica duplicada. Paridad viva F5.
- **E3 — CLI `generarBorrador`:** excerpt + catálogo → borrador por terminal
  (escalar por fuera sin Electron).
- **M1 — Fuera el strip:** eliminar watch/strip UI + claves i18n `watch.*`;
  solo queda el float in-page.
- **M2 — Panel anclado:** el float se ancla al composer con foco (ancestro con
  `contenteditable`), no fijo a la izquierda; se oculta al minimizar o pasar a
  burbuja (el MutationObserver ya detecta los estados).
- **M3 — Endpoint MN `POST /api/admin/marketplace/borrador`:** recibe
  extracto + aviso + extras de `/ask`, devuelve 1 borrador con ficha real
  (precio, negociable, margen sin cifras); si la IA falla, plantilla local.
- **M4 — Caché inteligente:** tabla `mp_respuestas` (firma normalizada →
  respuesta, usos, corregida); firma conocida = 0 tokens; cada respuesta
  usada/corregida alimenta la caché; Regenerar fuerza IA bruta.

## Estado
Plan escrito 2026-10-03 con 4 decisiones de la usuaria: repo nuevo,
caché en backend MN, panel que se oculta, strip eliminado. Falta que la
usuaria pase los puntos restantes que anunció.

## Próximo paso
Recibir puntos restantes → cerrar E1 (crear repo + Sentinel + núcleo).

## Gate y DoD
Repo nuevo: Sentinel PASS + tests + typecheck. MN: `tsc` + backend
(fmt/check/clippy/test). Cierre con verificación viva (panel con precio +
Regenerar) y envío siempre humano. Evidencia en `Agente/completados/`.
