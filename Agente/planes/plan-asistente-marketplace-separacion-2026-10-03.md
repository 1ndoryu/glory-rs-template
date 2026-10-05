# Plan 03AA-3 — Asistente Marketplace: repo propio + respuestas con IA
> Replanteado 2026-10-03 tras reto hostil (veredicto: no-viable-en-su-forma).
> Cambios: E0 inventario+corpus, strip con flag (nunca delete directo), doctrina
> Meta alineada con 03AA-5, caché con spec, frontera precio-mínimo con test de
> fuga, núcleo recortado, E1-mínimo desbloqueado, DoD con números.

## Objetivo
Sacar el asistente de respuestas de Messenger de `opencode-propio` a un repo
nuevo `plugins-opencode`, y pasar sus borradores de plantillas a IA vía
backend MN con caché (firma conocida = 0 tokens). El envío siempre es humano.

## Alcance / no alcance
- Sí: E0 inventario; E1-mínimo repo + núcleo lector; E2 adaptador con flag;
  E3 CLI; M1 strip solo tras paridad; M2 panel; M3 endpoint; M4 caché.
- No: auto-envío (prohibido siempre), credenciales/sesión de Meta (las pone
  ella), API oficial de Messenger, auto-pegado al composer (solo gesto humano).

## Doctrina Meta (alineada con 03AA-5, sin contradicciones)
Solo-lectura del DOM de hilos que ella abrió; red solo vía `background` al
backend local; **cero clipboard automático**: el borrador se muestra en el
float con botón "Copiar" (gesto humano explícito) y Regenerar nunca auto-pega.
Ritmo humano = límite (sin throttles); métrica `borradores/día` visible.

## Dependencias y contrato
- `opencode-propio` no es git y lo trabaja otro frente: solo se toca el
  adaptador; contrato escrito: nadie modifica `marketplace-*` sin avisar,
  versionado del IPC (`mp.ipc.v1`), rollback = flag off.
- Backend MN local con `OPENCODE_GO_API_KEY` (en uso diario en el chat MN).
- Remoto del repo (lo crea ella o local hasta su aviso).

## E0 — Inventario + corpus (pre-requisito, sin extraer nada)
1. Inventario firmado del fork: por cada `marketplace-*.ts`, líneas y qué
   hace (assistant/catalog/watch/cache/generador/float/service) con
   `path:línea` de funciones clave.
2. Corpus: 3 hilos Messenger anonimizados (pregunta precio / disponibilidad /
   visita) + definición exacta de `excerpt` y `firma normalizada`.
3. DoD: inventario + corpus versionados; sin esto no hay E1.

## Fases verificables
- **E1-mínimo — Repo + núcleo recortado (desbloqueado, no espera puntos):**
  crear repo, Sentinel día 1, solo `lector` (observa excerpt) + `firmas`
  (normaliza). Nada de `catalog/ficha/precio` (eso es producto MN, vive en
  el plugin, no en el núcleo). DoD: tests verde, 0 imports Electron,
  extracción 5/5 campos en corpus E0.
- **E2 — Adaptador en paralelo con flag:** `opencode-propio` consume el
  paquete tras flag (`mp_nucleo=off` por defecto); lógica vieja intacta.
  Paridad ciega: mismo excerpt → mismo borrador ±plantilla en 10/10 casos.
  DoD: paridad 10/10 + viva con precio correcto.
- **E3 — CLI `generarBorrador`:** excerpt → borrador por terminal. DoD:
  funciona sin Electron, fuga 0 (ver M3).
- **M1 — Fuera el strip (solo si E2+M2+M3 verdes en vivo):** recién ahí se
  elimina watch/strip UI + claves i18n. Nunca delete sin red.
- **M2 — Panel:** un solo overlay host (compartido con Radar 03AA-5, dos
  modos, no dos gemelos); ancla primario al composer (`contenteditable`) +
  2 fallbacks; sin composer se oculta (no flota suelto); minimiza a burbuja
  (término único). DoD: fixture con/sin composer + 2 resoluciones.
- **M3 — Endpoint `POST /api/admin/marketplace/borrador` (auth JWT admin):**
  in: excerpt + id aviso + extras; out: 1 borrador. Frontera escrita: a la IA
  entra ficha comercial SIN `precio_minimo`/margen; del endpoint no sale
  jamás una cifra de mínimo (ni en IA, ni en plantilla-fallback, ni en logs).
  Test negativo obligatorio "mínimo no aparece" (IA + fallback + logs).
- **M4 — Caché con spec:** migración `mp_respuestas_cache(firma UNIQUE,
  respuesta, precio_hash, catalog_hash, valida_hasta, usos, corregida)`;
  upsert atómico; cambio de ficha (`precio_hash`/`catalog_hash` distinto) =
  invalidar; firma conocida y válida = 0 tokens; Regenerar = bypass lectura
  + invalida esa firma (no "IA bruta" sin más). DoD: hit-rate medido,
  precio correcto 10/10 tras cambio de ficha, hit con ficha vieja = 0.

## IA y coste (unificado con 03AA-5)
Texto lo genera el modelo del chat MN vía backend (nunca en el plugin);
`jev` no genera texto, solo decide (03AA-5). Presupuesto por borrador a
medir en E2 (latencia p95 objetivo <8s). Caché manda antes que IA.

## Estado
E0 pendiente (inventario + corpus Messenger). E1-mínimo desbloqueado y no
espera los puntos restantes (solo E2+ los necesita).

## Próximo paso
E0: inventario firmado + 3 hilos anonimizados (pide acceso/lectura al frente
de opencode-propio o muestra de la usuaria).

## Gate y DoD global
Sentinel PASS en plugin + backend MN (fmt/check/clippy/test) + `tsc` 0;
paridad 10/10; fuga del mínimo 0/N; precio correcto 10/10; cierre con viva
(panel con precio + Regenerar + Copiar manual) y evidencia en
`Agente/completados/`.
