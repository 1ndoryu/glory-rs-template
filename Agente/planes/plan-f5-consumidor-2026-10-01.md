# 011A-5 — F5 consumidor delgado MN (activo 2026-10-01)

Objetivo: MN como primer consumidor delgado de `glory-agent@8560269`
(núcleo F1–F4 con gate PASS), sin romper el WhatsApp que hoy funciona.
Diseño base: `glory-agent/Agente/planes/plan-extraccion-whatsapp-2026-10-01.md` §7b.

## Alcance / no alcance

- Sí: pin bump, `idempotency_key` en `agent_outbox` (espejo de la mitad
  outbox de `0004_canal.sql` del núcleo), sombra con diff real contra el
  núcleo, corte sticky por canal con rollback, pruebas de todo.
- No: mitad `agent_messages` de 0004 (el canal vive en `canal_sesiones`,
  no se toca), F11 import, gateway real, deploy, front.

## Fases

### 0. Pin bump (desbloquea todo lo demás)

- `Cargo.toml:56` rev `e4ad5d7…` → `856026948d18bc17d6b68ebc4cfdddbc9e90ffca`;
  `Dockerfile.rust:28` `ARG GLORY_AGENT_REF` igual; `cargo check` verde.
- Seguro: `8560269` desciende de `e4ad5d7` (incluye 289A-5/6), cambios
  F10 aditivos (`channels`, `handoff`).

### 1. Outbox idempotente (migración `20261001000019`)

- Espejo del DDL del núcleo que el compilador exige (`query!` del core
  verificados contra esta BD): `0004_canal.sql` COMPLETO (mitad
  `agent_messages` + mitad outbox) y `0005_handoff.sql` (tabla
  `agent_eventos`). El plan original excluía la mitad de mensajes;
  el compilador manda (6 errores sin ella). El canal de MN sigue en
  `canal_sesiones`: estas columnas son del núcleo, no se usan aquí.
- TTL 7d: el worker (`vigilar`, cada poll 15 s) borra
  `sent/failed` con `created_at < NOW() - INTERVAL '7 days'`
  (usa `idx_agent_outbox_status`, barato).
- Clave = `sha256("{session_id}:{motivo}:{texto}")` hex. Excepción
  documentada: hash de contenido, no HMAC (los 7 puntos de encolado
  solo tienen `pool`; sin secreto disponible). Suficiente para
  dedup de reintentos en ventana 7d.
- Nuevo wrapper `encolar_outbox_idempotente(pool, kind, payload, key)`:
  `INSERT ... ON CONFLICT (idempotency_key) DO NOTHING`, devuelve si
  insertó. Con clave: `ia, tarjeta, ia_foto, acuse (+secuencia),
  fallback, asesor, aviso_humano, tope`. Sin clave (excepción): `manual`
  (cada clic staff es intención; con clave se tragaría el doble-clic).
- `programar_acuse` y `atender_resultado_turno` reciben `secuencia`
  para acotar la clave al turno.

### 2. Sombra completa (usa el pin nuevo)

- `HuellaSombra` suma `nucleo {responde_ia: Option<bool>,
  via_permitida: bool, partes_igual: bool} + coincide: bool`.
- Se construye `AdapterConfig` desde `agent_config`
  (sesiones `wa_a` responde_IA=true, `wa_b`=false por decisión 289A-1
  solo-A-es-IA; `via_permitidas=[wa_a,wa_b]`) y se compara:
  reparto MN vs sesión declarada, `via` del hilo vs allowlist,
  `partir_respuesta` MN vs núcleo (mismo MAX3).
- Solo lectura + tras `GLORY_SHADOW=1`, como hoy.

### 3. Corte sticky `corte_whatsapp` (`agent_config`)

- Valores `apagado|wa_b|total` (default `apagado`). Sticky = solo
  cambio manual en consola (auditable); nunca auto-revierte.
- El flag cubre el camino idempotente por canal: `wa_b` primero
  (número de pruebas), luego `total`. Rollback = volver a `apagado`
  o `wa_b`: misma tabla, nada varado; requeue `failed→pending`
  seguro por las claves.
- Orden: drenar `pending` antes de subir de nivel (el humo lo verifica).

### 4. Probar todo (Definition of Done)

- `npm run self-check` verde + `cargo test` con los nuevos tests:
  doble enqueue misma clave = 1 fila; claves distintas = 2 filas;
  TTL borra `sent` viejo; sombra `coincide:true` con `GLORY_SHADOW=1`;
  corte `wa_b` no toca `wa_a`.
- Humo HTTP contra `:3000`: webhook simulado `wa_b` dos veces
  (mismo texto) = 1 `client` + outbox sin duplicados; sombra 200 con
  `coincide:true`; corte en `wa_b`, outbox drenado, volver a `apagado`.
- Datos de prueba borrados; BD de rama usada para no tocar `main`.

## Estado

- [x] Preflight (doctor sin policy en MN: vale wrapper+self-check)
- [ ] Fase 0 pin bump
- [ ] Fase 1 outbox idempotente
- [ ] Fase 2 sombra completa
- [ ] Fase 3 corte sticky
- [ ] Fase 4 pruebas + humo
- [ ] Cierre (roadmap, completada, commit, push)

## Decisiones

- Sin HMAC: hash de contenido (ver Fase 1). Si aparece secreto
  disponible en los puntos de encolado, migrar a HMAC (tarea nueva).
- `manual` sin clave (ver Fase 1).
- Mitad `agent_messages` de 0004 fuera (canal ya vive en
  `canal_sesiones`; no duplicar esquema).
