# Plan 09AA-6 — Auditoría de lógica + SOLID en mensajería (2026-10-09)

> Estado: ACTIVO (Fase A rehecha con supervisor-review). Tarea `09AA-6` en roadmap.
> Precedente: 09AA-2 usó 4 subagentes para confirmar raíz; este plan lo
> sistematiza en 6 zonas sin solaparse.
> Corrección 2026-10-09 (regla 22 AGENTS.md): la primera pasada usó
> `code-analyst` por error — explica pero no juzga. Se relanza con
> `supervisor-review`, que sí dictamina defectos con severidad. Los informes
> de la primera pasada se conservan como Anexo A.

## 1. Objetivo

Auditar lógica y SOLID en el pipeline de mensajes (marketplace + WhatsApp +
frontend chat + conexión con opencode) con subagentes de solo lectura, reunir
sus informes en una síntesis única y, solo después, emitir el plan de arreglo.
Este archivo define CÓMO se sueltan los subagentes; el plan de arreglo sale de
la Fase B, no antes.

## 2. Alcance / no alcance

- Sí: `src/handlers/marketplace*.rs`, `chat*.rs`, `whatsapp.rs`, `ia*.rs`;
  `src/services/marketplace*.rs`, `transporte.rs`, `canal_resolver.rs`,
  `sesion.rs`, `triage.rs`, `tope_uso.rs`, `alerta_whatsapp.rs`,
  `outbox_idempotency.rs`; `frontend/src/features/chat/*`,
  `frontend/src/hooks/chat/*`, `frontend/src/data/chat/*`,
  `frontend/src/platform/*`; conexión opencode (provider `opencode-go`,
  `x-opencode-session`, espejo schema M3 ↔ `plugins-opencode`, float en
  `opencode-propio`, verificación en pestaña Navegador).
- No: inmuebles/fotos/ask/pagos (otros frentes); editar código en Fase A/B;
  reiniciar el backend vivo; tocar `opencode-propio` (puerto 5174), perfiles
  de navegador o `%APPDATA%`; deploy prod (decisión de ella).

## 3. Mapa estratégico (6 zonas, sin solapamiento)

- **Z1 Pipeline borrador marketplace (backend):** `handlers/marketplace.rs`
  (`generar_borrador`, `regenerar_uno/todo`, `releer`, `clave_hilo`,
  `borrar_todo_cache`) + `services/marketplace.rs` (`imponer_forma_borrador`,
  `combinar_foto_hilo`, `filas_para_regenerar`, `asegurar_contacto`).
  Riesgos conocidos: dualidad `thread_id` literal vs `clave_hilo()` (08AA-30
  abierto), conserva-vs-reserva (09AA-3/4), ramas IA/fallback/plantilla.
- **Z2 Texto/excerpt del hilo:** `services/marketplace_texto.rs`
  (`eco_propio`, `retirar_bloque_duplicado`, `canon_eco`, `combinar_foto_hilo`,
  etiquetado Tú/Cliente, schema M3 que espeja
  `plugins-opencode/src/nucleo/schema.ts`). Riesgo: el espejo M3 diverge en
  silencio entre repos.
- **Z3 WhatsApp y transporte:** `handlers/whatsapp.rs`,
  `services/transporte.rs`, `canal_resolver.rs`, `sesion.rs`,
  `alerta_whatsapp.rs`, `gateway/media/*` (runtime, no auditar contenido).
  Riesgos: estados de sesión/QR, reintentos, acoplamiento transporte↔dominio.
- **Z4 Proveedores IA y sesión estable:** `handlers/ia.rs`,
  `ia_proveedores.rs` (`completar_opencode`, reintento ante vacío, tope 8000,
  `sesion=sha_hex(clave_hilo)` vs `"centro-ia"`/`"fotos"`, diagnóstico sin PII).
  Riesgos: afinidad de sesión, 400 `MissingSessionID`, vacíos 200-sin-texto.
- **Z5 Frontend chat:** `features/chat/*` (panel, hilo, sesiones, bandeja,
  uso-auditoria), `hooks/chat/*` (los 5 de 08AA-32), `data/chat/*`,
  `platform/descarga|ventana|dialogos.ts`. Riesgos: polling vs eventos,
  estados duplicados front/back, errores visibles vs silenciados (regla 6).
- **Z6 Frontera opencode:** cómo el navegador (pestaña Navegador) y el float
  de `opencode-propio` consumen este backend: rutas admin con JWT vs token mp
  (401 documentado), `chat_staff*.rs` (el navegador nunca habla con el
  gateway directo), `chat_tools*.rs` + detector captación, `outbox_idempotency`,
  `triage.rs`, `tope_uso.rs`, `ask.rs`. Riesgos: auth por ruta, idempotencia
  de envíos, topes globales vs por-hilo.

## 4. Despliegue de subagentes (Fase A — un solo bloque paralelo)

6 × `supervisor-review` (juzgar código y dictaminar defectos con severidad;
corrección 2026-10-09: `code-analyst` solo explica, no sirve para auditar),
mensaje con 6 llamadas `Task` en paralelo. Son independientes: cada uno recibe
sus archivos explícitos y la orden de NO leer fuera de su zona salvo imports
directos.

Prompt base (igual para los 6, cambiando zona/archivos/riesgos):

> Eres revisor de código: REVISA la zona, JUZGA cada problema y DICTAMINA
> veredicto (defecto-real / mejora-opcional / falso-positivo) con severidad
> (alta/media/baja). NO edites, NO ejecutes comandos de escritura. Zona: {Z}.
> Archivos: {lista}. Prohibido leer fuera de la zona salvo imports directos.
> Devuelve SOLO: (1) veredicto por hallazgo con `ruta:línea` y testigo; (2)
> invariantes que el código asume pero no verifica; (3) violaciones S/O/L/I/D
> con severidad y por qué; (4) carreras/N+1/reintentos sin tope; (5) top-3
> defectos-reales con línea que lo demuestra. Si algo "no existe", declara
> método y cobertura exacta, nunca en absoluto. Máximo 40 líneas.

Asignación:

| Agente | Zona | Foco SOLID |
|---|---|---|
| A1 | Z1 | SRP (`generar_borrador` hace prompt+DB+fallback), OCP (ramas por provider) |
| A2 | Z2 | ISP/DIP (puras vs acopladas), divergencia espejo M3 |
| A3 | Z3 | DIP (transporte abstracto), SRP sesión vs envío |
| A4 | Z4 | OCP (orden de intento gloryapi/opencode-go), SRP diagnóstico |
| A5 | Z5 | SRP componentes/hooks (regla 8), errores visibles (regla 6) |
| A6 | Z6 | Auth por frontera, idempotencia, topes (DIP/LSP en guards) |

## 5. Fase B — Síntesis (agente principal, sin subagentes)

1. Tabla consolidada por severidad con `zona:ruta:línea` y testigo.
2. Resolver contradicciones entre agentes (si A1 y A4 discrepan sobre la
   misma rama, releer el código y decidir con evidencia).
3. Reabrir negativos calibrados: todo "no encontrado" sin testigo se marca
   no-verificado, no ausente.
4. Salida: ranking top-10 + mapa qué-fix-toca-qué (evitar 2 fixes al mismo
   invariante).

## 6. Fase C — Plan de arreglo (solo tras B)

Un bloque por hallazgo alta/media: fix mínimo, archivos, verificación
(funcional real, no solo type-check), gate sin nuevos. Las bajas van a
deuda en roadmap, no se mezclan. Antes de implementar, 1 ×
`supervisor-thinking` desafía el plan; al cerrar, 1 × `supervisor-review`.

## 7. Definition of Done (de la auditoría)

Fase A: 6 informes con el formato del §4. Fase B: ranking + mapa. Fase C:
plan de arreglo escrito aquí (§8) con severidad/verificación por item.
Nada de esto commitea código; el commit es docs (plan + roadmap).

## 8. Plan de arreglo (final 2026-10-09: veredictos supervisor-review + orden supervisor-thinker)

Veredictos que cambian el pre-plan: ping-afinidad REFUTADO (one-shot, solo
etiqueta fija); `canon_eco` ends_with FALSO-POSITIVO (exige `==` :369);
alerta-`failed` NO reintenta (terminal; solo `pending`-sin-admin eterno,
baja); tope kill-switch FALSO-POSITIVO (es diseño); `canal_resolver` SÍ usa
trait `Resolver`. Nuevos: `responder` sin transacción (6 writes, media);
`retirar_bloque` invade `Cliente:` (media); GET `media_url` sin tope →
SSRF/OOM que amplifica el webhook abierto (alta); sesión fija
`centro-ia`/`fotos` contradice afinidad; TOPE 8000 sin medición; STT 9
claves ×120s sin backoff; `get(uso)` rama muerta; triage dedup volátil.
Z5 quedó parcial (cuota: QR y polling NO verificados).

Orden (no codificar sin el prerrequisito indicado):
1. **09AA-7 — Épica invariante persistencia (fusiona F1+F4+`pending`):**
   prerrequisito elegir clave canónica (`clave_hilo` vs `thread_id`,
   08AA-30) + backfill legacy; luego agrupar-por-hilo y borrar-1-vez
   (`marketplace.rs:424-438` + `:337`, SELECT sin DISTINCT :940-941),
   conservar-ante-fallo-IA (`:357-372` solo persiste si `fuente==ia`,
   testigo :385-387), `responder` con clave idempotente + transacción
   (`chat_staff.rs:168-233`, `manual` excluido `outbox:53-57`), contador +
   dead-letter para `pending`-sin-admin. Invariante:
   `clave_idem UNIQUE + queued→sent/failed-terminal + delete solo
   terminal`. Verificación: tests 2-filas-mismo-hilo, fallo-IA-conserva,
   doble-POST→1 envío, gateway caído.
2. **09AA-8 — Webhook fail-closed + tope media:** `transporte.rs:95-99`
   a fail-closed con `ALLOW_EMPTY_IN_DEV` (o rompe local);
   `sesion.rs:124,147` tope previo a `bytes()` + solo-esquema ya en
   :305-315. Verificación: 401 sin secreto, descarga gigante cortada.
3. **09AA-9 — Auth con rol:** `_auth` ignorado (`chat_staff.rs:88,136,169`,
   `auth.rs:12-14` sin `role`, `ask.rs:30,60` igual) es defecto-real alta;
   NO codificar sin verificar contrato del float opencode-propio
   (emisor/rol que usa hoy) o se rompe la integración viva.
   Verificación: 403 no-admin + float operando.
4. **09AA-10 — Dedup + espejo M3:** estrechar `retirar_bloque`
   (`:354-376` invade `Cliente:`), `es_cola_truncada` (`disponible?`
   legítimo), cita bare (mejora); alinear hex mayús + allowlist `extras`
   fijando lado canónico (¿rs o ts manda?). Verificación: batería
   excerpt + test comparativo espejo.
5. **09AA-11 — Z4 (tras 09AA-4):** partir en F6a (dispatch `if id==` →
   abstracción mínima, `ia.rs:60,186,236,301,396`) y F6b (reintento glory
   coordinado con 09AA-4, no duplicar); medir TOPE 8000; quitar rama
   muerta `get(uso)`; backoff STT; sesión fija vs afinidad.
   Verificación: tests + replay vacío.
6. **09AA-12 — Z5 (independiente, puede ir en paralelo):** sub-hook
   regeneración (5 useState, `use-chats-marketplace.ts:15-17,52-53`),
   feedback visible para `pedirInfo` null, re-verificar QR y polling
   (quedaron fuera de cobertura). Verificación: `tsc` + sentinel 0 nuevos.
Deuda (no tocar aún): triage dedup volátil → BD; trait
transporte/sesión; `archivar_media` best-effort documentado;
check-then-set `ia_tope_alertado`. No-goals: backfill masivo legacy,
refactor relay, F10 arquitectura.

## Anexo A — Primera pasada con code-analyst (2026-10-09, informativa, sin veredicto)

A1/Z1: `generar_borrador` SRP-alta (639-785: prompt+DB+fallback); `releer`
SRP-media (504-558); sin `thread_id` literal en escrituras actuales (solo
legacy 08AA-03); `regenerar_uno`/:320 y `todo`/:408-438 borran-lo-fresco
(defecto-real alta); `asegurar_contacto` O-media (contacto implícito);
`PROMPT_BORRADOR` DIP-baja; `consejo` LSP-baja; invariante
`thread_id=nombre|aviso`; carreras al regenerar concurrente; sin tope IA
(09AA-4 lo cubre).
A2/Z2: `con_hilo` SRP-media (:149-234); `Media` ISP-media (`MediaDb` mezcla
foto+docs); `retirar_bloque_duplicado` `canon_eco` ends_with falso-positivo
(:381) + `es_cola_truncada` borra "disponible?" (:573) +
`es_cabecera` contains borra citas (:644-646); espejo M3 diverge en 3
testigos (mayús hex, extras sin allowlist, offset hora); sin carreras/N+1.
A3/Z3: `VEREDICTO: VIABLE CON RESERVAS`; DIP-alta: transporte/sesión/alerta
usan tipos concretos, sin trait (impide mock/test); invariantes: estados
`sesion.rs:282-342`, `canon` asume formato, `canal` sin normalizar;
`WA_WEBHOOK_SECRETO` vacío acepta todo (`transporte.rs:96-98` defecto-real
alta); reintento infinito sin backoff en `alerta_whatsapp.rs` (alta);
DIP-media `canal_resolver.rs:23-29`; `archivar_media` sin transacción
(`sesion.rs:234-275`); `canon_telefono` asume dígitos (`sesion.rs:118,243`).
A4/Z4: OCP-alta: añadir proveedor toca 5 puntos (`ia.rs:60,396-423`,
`:301`, `:270`, `ia_proveedores.rs:186`); LSP-media: `200-sin-texto`
reintenta opencode pero NO glory (`:144-168` vs `:151-153`); sin `trait`;
invariante afinidad `sesion=sha_hex(clave_hilo)` vs `"centro-ia"`/`"fotos"`;
ping usa uuid fresco y rompe afinidad (`ia_proveedores.rs:68` vs `:222`);
tope 8000 documentado; diagnóstico sin PII OK.
A5/Z5: `use-chats-marketplace.ts:15-17,52-53` S-alta (5×useState);
`pedirInfo` S-media (null silencioso); polling 20s OK / 5s bandeja sin
evidencia; QR 404→'' (L-media); resto hooks 08AA-32 S-media (dentro de
tolerancia); cobertura parcial (cuota: sin leer `usar-*.ts`,
`descarga|ventana|dialogos.ts`).
A6/Z6: `chat_staff.rs:24-55` S-alta (auth+proxy+reglas); `_auth` ignorado
L-alta (`:88,136,169` cualquier JWT=admin); `tope_uso.rs:10-14` solo
warn/encola, nunca `ai_enabled=false` (S-alta); responder sin clave
idempotente (`:207-219` + `outbox:22,56` defecto-real alta); 401-token-mp y
enlace staff→gateway NO verificados (cuota consultar).

## Anexo B — Desafío de supervisor-thinker al pre-plan (2026-10-09)

`VEREDICTO: VIABLE CON RESERVAS` — orden F1→F9, no. F2 primero (fail-closed,
con `ALLOW_EMPTY_IN_DEV` o rompe local); F3 no codificar sin verificar
contrato float opencode-propio; F8 antes de F1 (predicados contaminan tests);
F1+F4+F5 fusionar en épica `outbox/invariante`
(`clave_idem UNIQUE + queued→sent/failed-terminal + delete solo terminal`);
F6 partir en F6a/F6b, F6b coordinar con 09AA-4 (no duplicar reintento);
F7 en paralelo tardío fijando lado canónico; F9 último/independiente.
Riesgos que añadió: agrupar-F1 ¿por `thread_id` o `clave_hilo`? (08AA-30
abierto: elegir canónica + backfill legacy); colisión con 09AA-5 (mismo
pipeline, backend vivo); backoff sin métricas es ciego. Siguiente acción:
reescribir plan como épica + contrato auth + clave canónica. A la espera
de los veredictos de supervisor-review (Fase A rehecha).
