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

## E0 — Inventario + corpus (pre-requisito, SOLO lectura, timebox 3 días)
1. Freeze: zip/hash fechado del fork versionado (el fork vive y otro frente
   lo toca; sin freeze la "firma" se pudre). Inventario por cada
   `marketplace-*.ts`: líneas y qué hace, con `path:línea` de funciones clave.
2. Corpus: 3 hilos Messenger anonimizados (precio / disponibilidad / visita)
   + definición exacta de `excerpt`, los 5 campos (remitente, texto, hora,
   aviso enlazado si hay, estado leído) y `firma normalizada`. Regla de
   anonimización: sin nombres reales, sin teléfonos, retención 90 días y
   borrado a petición.
3. Fallback: si en 3 días no hay acceso, corpus sintético con markup
   equivalente (marcado `sintetico`, a recalibrar en vivo).
4. DoD: freeze + inventario + corpus versionados; sin esto no hay E1
   (E1 queda bloqueado hasta E0 real, sin "desbloqueados" de palabra).

## Fases verificables
- **E1 — Repo + núcleo recortado (bloqueado hasta E0 real):**
  crear repo, Sentinel día 1, solo `lector` (observa excerpt) + `firmas`
  (normaliza). Nada de `catalog/ficha/precio` (producto MN, vive en el
  plugin). Dueño del host futuro: el núcleo monta el overlay; los plugins
  solo registran modos. DoD: tests verde, 0 imports Electron (grep directo
  y transitivo en CI), extracción 5/5 campos E0 en corpus.
- **E2 — Adaptador en paralelo con flag:** `opencode-propio` consume el
  paquete tras flag (`mp_nucleo=off` por defecto); lógica vieja intacta.
  Paridad ciega: 10 casos del corpus (3 hilos × variaciones) mismo excerpt
  → mismo borrador salvo plantilla declarada. DoD: paridad 10/10 + viva
  con precio correcto.
- **E3 — CLI `generarBorrador` (tras M3, usa su endpoint):** excerpt →
  borrador por terminal con JWT admin. DoD: funciona sin Electron, fuga 0.
- **M1 — Fuera el strip (solo si E2+M2+M3 verdes en vivo):** recién ahí se
  elimina watch/strip UI + claves i18n. Nunca delete sin red.
- **M2 — Panel (dueño: núcleo):** un content-script + ShadowDOM montado por
  el núcleo; `asistente` y `Radar` registran modos (no dos floats). Ancla
  primario al composer (`div[contenteditable="true"]` en panel chat);
  fallbacks nombrados: F1 `div[role="textbox"]`, F2 caja del hilo activo por
  `aria-label`; sin composer se oculta y el borrador pendiente queda en
  memoria del modo (no se pierde, no flota suelto). Minimiza a burbuja.
  DoD: fixture con/sin composer + 1280px y 390px.
- **M3 — Endpoint `POST /api/admin/marketplace/borrador` (auth JWT admin):**
  in: excerpt + id aviso + extras con **allowlist cerrada** (solo claves
  listadas; si trae `precio_minimo`/`margen` → 422). Frontera: a la IA entra
  ficha comercial SIN mínimo; el generador pineado es el modelo del chat MN
  (nunca `jev`, que en 03AA-5 sí ve precio completo). Del endpoint no sale
  jamás una cifra de mínimo (ni IA, ni fallback, ni logs). Test negativo con
  matriz: cifra exacta, `85k`, `$85.000`, en letras, y caso `margen=true` +
  scrub del writer de caché y logs antes de INSERT. Presupuesto: latencia p95
  <8s medida aquí (no en E2).
- **M4 — Caché con spec:** migración `mp_respuestas_cache(firma,
  precio_hash, catalog_hash, respuesta, valida_hasta DEFAULT +90d, usos,
  corregida; UNIQUE(firma,precio_hash,catalog_hash))`; lookup por la terna
  (ficha cambiante no colisiona, historia por filas); writer nombrado
  `hashFicha()` calcula los hash al guardar; purga de vencidas programada.
  Firma conocida y válida = 0 tokens; Regenerar = bypass lectura + invalida
  esa firma, sin tope (decisión usuaria 2026-10-05; el loop lo frena el ritmo
  humano, no un contador). Contador visible en admin
  (`GET /mp/respuestas/uso-hoy`) como métrica. DoD: hit-rate medido, precio
  correcto 10/10 tras cambio de ficha, hit con ficha vieja = 0, contador
  visible.

## IA y coste (unificado con 03AA-5)
Texto lo genera el modelo del chat MN vía backend (nunca en el plugin ni en
`jev`, que solo decide en 03AA-5). Latencia p95 <8s medida en M3. Caché manda
antes que IA; Regenerar sin tope (decisión usuaria), contador admin de
métrica.

## Estado
E0 en curso (solo lectura, timebox 3 días, fallback sintético). E1 bloqueado
hasta E0 real. E2+ espera tus puntos restantes.

## Próximo paso
E0: freeze hash del fork + inventario + 3 hilos anonimizados (pide acceso al
frente de opencode-propio o muestra de la usuaria).

## Gate y DoD global
Sentinel PASS en plugin + backend MN (fmt/check/clippy/test) + `tsc` 0;
paridad 10/10; fuga del mínimo 0/N; precio correcto 10/10; cierre con viva
(panel con precio + Regenerar + Copiar manual) y evidencia en
`Agente/completados/`.
