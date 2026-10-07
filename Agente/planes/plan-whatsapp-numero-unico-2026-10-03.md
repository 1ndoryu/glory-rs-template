# Plan 03AA-4 — WhatsApp un solo número + triage + config + escenarios

## Objetivo
Un solo asistente general en el **0412 0825234** (canónico; el `wa_b` temporal
se jubila), el mismo de la página, centralizado. Multi-número queda como
capacidad por configuración, no como modos distintos.

## Decisiones base
- Se elimina el reparto por `numero_destino` (completo/inicial): el número es
  solo el enchufe; el que manda es el **remitente** (público vs autorizado).
- `wa_b`/B temporal: se apaga (`responde_ia=0` ya) y se retira del flujo; el
  código multicanal se conserva genérico por si se suma otro número real.

## Arquitectura (revisión SOLID)
Capas con traits, cada una una responsabilidad:
`Transporte` (webhook→evento normalizado; hoy Baileys, mañana Messenger) →
`Triage` (atiende / no atiende + motivo) → `Sesion` (hilo por cliente×canal) →
`Politica` (público vs autorizado) → `Agente` (turno glory-agent + tools) →
`Envio` (outbox + espejo + E-fluido). Deuda actual: `whatsapp.rs` mezcla
transporte+media+secreto y `chat.rs` turno+política; se parten sin cambiar
conducta (tests vivos antes y después).
- Repos: `glory-agent` = núcleo agnóstico (turno, tools, historial, usage,
  STT); MN = negocio (tools inmobiliarias, prompt, catálogo, política,
  transportes); `plugins-opencode` = consumidores externos (float Messenger
  contra M3).

## Triage: qué se atiende y qué no
Atiende (público): texto/media de número externo, hilo nuevo o `activa`, IA
habilitada en hilo y global.
No atiende + motivo registrado: eco propio que vuelve, hilo `delegada` o
`escalada` sin devolver, IA pausada (global/hilo), duplicado
(`client_seq`/eco TTL), mensaje de sistema (presencia, receipts), remitente =
número propio del negocio, vacío sin media, retrasado (>X min del gateway).
Regla de oro: **por defecto se atiende**; ignorar solo cae en la lista
explícita de arriba (cada `no` deja `motivo` en log/auditoría).

## No-cliente sin falsos positivos
Sospechar que no es cliente **nunca silencia**: cambia el trato, no el
triage. Tres tratos: `cliente` (flujo completo), `neutral` (respuesta breve
y educada + pregunta que califica sin acusar: "¿buscas comprar o alquilar?"),
`no-atender` (solo la lista de arriba). Señales (heurística + IA, ninguna
decide sola): número equivocado, vendedor/spam, troleo, mensajes de prueba.
Anti-falsos-positivos: 1 señal débil no mueve el trato (mínimo 2
independientes o 1 explícita); aperturas típicas (`hola`, `precio`, `sí`)
siempre = cliente; cada mensaje re-evalúa (reversible); la duda reincidente
se marca a humano, jamás se bloquea solo.

## Público vs autorizado
- Público (cualquiera): buscar, detalle, fotos, precio/disponibilidad,
  coordinar visita (deriva a asesor), registrarse como interesado, dudas.
- Autorizado (`numeros_autorizados[]` + admin web): ficha/lista de clientes,
  tomar/soltar IA por hilo, devolver a IA, pausa global, estado/uso,
  responder manual. Nada de clientes sale a no autorizados (frontera en
  `Politica`, no en el prompt).

## Config todo-controlable (admin)
`agent_config`: `numero_canonico`, `canales[]` (numero, alias, responde_ia),
`numeros_autorizados[]`, `ia_activo`, `ia_tope_tokens_dia`, tono (saludo,
acuse, fallback), `ventana_retraso_min`, alertas (`whatsapp_admin`,
`GLORY_ALERT_GATEWAY_URL`). Pestaña Config existente: un bloque por grupo,
sin claves secretas en UI (solo en `.env`).

## Escenarios automatizados (sin WhatsApp)
Harness `scripts/harness-conversacion.mjs`: inyecta turnos directo al
pipeline (sin Baileys), guiones multi-paso en fixtures versionados, asserts
por paso (intención, tool, outbox, estado del hilo, texto contiene/no
contiene). Reporte tabla: escenario × paso × esperado × obtenido × PASS/FAIL.
Dos modos: `stub` (respuestas IA fijas, rápido, para gate) y `vivo`
(OpenCode Go real, validación previa a cierre). Escenarios base: saludo→
búsqueda→detalle→foto→precio→visita; fuera de tema; cliente que pide humano;
autorizado pide ficha; no autorizado pide clientes (debe negar); duplicados;
hilo delegado que no debe responder; audio; foto.
Suite anti-falsos-positivos (todos deben quedar `cliente`): `hola` seco,
`precio?`, `sí` solo, apertura con errores de tipeo, cliente que pregunta por
tercero, curioso que pide fotos sin más datos. Suite no-cliente (todos
`neutral`, ninguno silenciado): vendedor de servicios, número equivocado
admitido, troleo leve, "solo probaba". Cada caso con señales esperadas.

## Fases
F1 triage + tests; F2 política público/autorizado + allowlist; F3 partir
capas (sin cambiar conducta); F4 config admin; F5 retirar `wa_b` del flujo;
F6 harness + 8 escenarios base en `stub`; F7 pasada `vivo` + cierre.

## Estado
Plan escrito 2026-10-03. Base limpia (chats a cero). F1–F5 hechas
(2026-10-07). Próximo: F6.

## Gate y DoD
Backend: fmt/check/clippy/test + harness `stub` verde en el gate; `vivo`
antes de cerrar. Un número atiende, el resto calla; lo no atendido siempre
con motivo; admin lo controla todo sin tocar código.
