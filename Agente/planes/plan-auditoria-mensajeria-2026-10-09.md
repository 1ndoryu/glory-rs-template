# Plan 09AA-6 — Auditoría de lógica + SOLID en mensajería (2026-10-09)

> Estado: ACTIVO (Fase A pendiente de lanzamiento). Tarea `09AA-6` en roadmap.
> Precedente: 09AA-2 usó 4 subagentes para confirmar raíz; este plan lo
> sistematiza en 6 zonas sin solaparse.

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

6 × `code-analyst` (solo lectura, Flash por regla 22), uno por zona, en UN
mensaje con 6 llamadas `Task` en paralelo. Son independientes: cada uno recibe
sus archivos explícitos y la orden de NO leer fuera de su zona salvo imports
directos.

Prompt base (igual para los 6, cambiando zona/archivos/riesgos):

> Eres auditor de solo lectura. NO edites, NO ejecutes comandos de escritura,
> NO reinicies procesos. Zona: {Z}. Archivos: {lista}. Archivos prohibidos
> fuera de la zona salvo imports directos. Devuelve SOLO: (1) flujo
> entrada→salida con `ruta:línea` por paso; (2) invariantes que el código
> asume pero no verifica; (3) violaciones S/O/L/I/D con `ruta:línea`,
> severidad (alta/media/baja) y por qué; (4) carreras/N+1/reintentos sin
> tope; (5) top-3 riesgos con testigo concreto (línea que lo demuestra).
> Si algo "no existe", declara método y cobertura exacta, nunca en absoluto.
> Máximo 40 líneas.

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

## 8. Plan de arreglo (se escribe tras la Fase B)

Pendiente de síntesis.
