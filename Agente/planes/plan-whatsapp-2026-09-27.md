# Plan: Agente WhatsApp MN (279A-2) — 2026-09-27

## Objetivo
Un solo agente (glory-agent) que atiende WhatsApp: envía, lee, muestra
bandeja en `/admin`, propone borradores y auto-responde FAQs con datos
reales del catálogo. Sin API de pago.

## Decisiones tomadas (con usuaria)
- Gateway: **Baileys** (open-source, QR una vez, sesión persistente).
  Evolution API queda como migración futura si hacen falta varios números o UI propia.
- Hospedaje gateway: **PC usuaria** (gratis; solo envía/lee con PC encendida).
  VPS descartado por ahora (más ops).
- Número: lo aporta la usuaria más tarde (nuevo o actual).
- Cerebro auto-respuesta: `opencode serve` local (v1.18.30 verificado),
  sesión por número de cliente.

## Requisitos aceptados
1. Conectado al catálogo: respuestas solo con datos de la BD (precio,
   hab/baños, link ficha). Prohibido inventar.
2. Enviar y recibir **imágenes** (con pie / guardadas y visibles).
3. **Audios**: transcripción local con Whisper (gratis, español), texto al
   cerebro, audio original guardado.
4. Todo visible en `/admin → Mensajes`, pestaña WhatsApp (PC y móvil).
5. Mismo agente: sesión WhatsApp = sesión glory-agent (historial unificado
   web + WhatsApp por cliente).

## Reparto glory-agent / inmobiliaria (regla 17)
- `glory-agent` (agnóstico): abstracción de canal en sesiones, payload
  `destino`+`texto`+`media` en outbox, manejo de media entrante.
- Este repo: tools inmobiliarias, PromptConfig, rutas staff, worker
  extendido, bandeja WhatsApp en front.
- Fuera de repos: gateway Baileys + Whisper en PC (servicio local con
  QR; documentado, versionado como script en este repo).

## Fases (cada una usable sola, con su verificación)
- **F1 Envío manual**: endpoint admin encolar + worker acepta
  `destino`/`texto` del payload (hoy solo `whatsapp_admin` fijo) +
  gateway Baileys `POST /send` + QR. Verificar: mensaje real al número
  de prueba, outbox `sent`.
- **F2 Lectura + bandeja**: webhook entrante → tabla mensajes WhatsApp →
  pestaña en `VistaMensajes` (lista + hilo + responder) + aviso de
  nuevos. Verificar: escribir desde un teléfono, verlo en `/admin`.
- **F3 Borradores sugeridos**: opencode propone respuesta, staff aprueba
  con un clic. Verificar: propuesta coherente con catálogo, envío tras
  aprobar.
- **F4 Auto-respuesta + media**: fotos con pie, recepción de imágenes,
  audios→Whisper→texto, guardarraíles (solo FAQs con datos BD, escala a
  humano lo demás, horario, firma como asistente). Verificar: matriz de
  casos (precio real, invento bloqueado, visita → humano).

## Estado
- **Bloqueado esperando usuaria**: número + 2 min para QR.
- Nada implementado; sin cambios de código hasta F1.

## Gate / DoD por fase
- Rust: `cargo fmt --check && cargo check && cargo clippy -- -D warnings && cargo test`.
- Front: `npx tsc --noEmit` (+ 2 resoluciones si hay UI).
- Funcional real en cada fase (mensaje/bandeja/respuesta de verdad,
  no solo compila). Deploy vía coolify-manager-rs si toca prod.

## Riesgos
- Automatización no oficial: solo 1:1, nada masivo (riesgo de restricción).
- Gateway en PC: requiere PC encendida; documentar arranque.
- Precios/visitas inventadas: mitigado por F3→F4 con guardarraíles y
  escalación a humano.
