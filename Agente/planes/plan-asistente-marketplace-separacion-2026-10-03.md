# Plan 03AA-3 — Asistente Marketplace: repo propio + respuestas con IA
> Replanteado 2026-10-03, 2º reto 2026-10-05 (allowlist, caché por terna,
> host dueño núcleo), 3er reto 2026-10-05 (P0 seguridad/PII/rollback/
> concurrentes + 8 contradicciones cerradas). Regenerar sin tope por
> decisión usuaria; el freno es el ritmo humano.

## Objetivo
Sacar el asistente de respuestas de Messenger de `opencode-propio` a un repo
nuevo `plugins-opencode`, y pasar sus borradores de plantillas a IA vía
backend MN con caché (firma conocida = 0 tokens). El envío siempre es humano.

## Alcance / no alcance
- Sí (en orden): E0 inventario; E1 núcleo lector; E2 adaptador con flag;
  M2 panel; M3 endpoint; E3 CLI (usa M3); M4 caché; M1 strip al final.
- No: auto-envío (prohibido siempre), credenciales/sesión de Meta (las pone
  ella), API oficial de Messenger, auto-pegado al composer (solo gesto humano).

## Doctrina Meta (común con 03AA-5)
Solo-lectura del DOM de hilos que ella abrió; red solo vía `background` al
backend local; **cero clipboard automático** (botón "Copiar" manual,
Regenerar nunca auto-pega). Ritmo humano = límite operativo (sin throttles
artificiales); el endpoint se protege con 429 anti-abuso (no es tope para
ella). Métrica `borradores/día` visible.

## Dependencias y contrato
- `opencode-propio` no es git y lo trabaja otro frente: solo se toca el
  adaptador; contrato: nadie modifica `marketplace-*` sin avisar; el
  adaptador verifica hash del paquete pineado; rollback = flag off +
  uninstall + restore tag (probado <15min, ver P0-4).
- Backend MN local con `OPENCODE_GO_API_KEY` (en uso diario en el chat MN).
- Remoto del repo (lo crea ella o local hasta su aviso).

## E0 — Inventario + corpus (SOLO lectura, timebox 3 días)
1. Freeze: zip/hash fechado del fork versionado. Inventario por cada
   `marketplace-*.ts`: líneas y qué hace, con `path:línea` de funciones clave.
2. Corpus: 3 hilos Messenger anonimizados (precio / disponibilidad / visita)
   + definición exacta de `excerpt`, los 5 campos (remitente, texto, hora,
   aviso enlazado si hay, estado leído) y `firma normalizada`. Anonimización
   ejecutable: nombres→`[NOMBRE]`, teléfonos→`[TEL]`, fotos→solo hash;
   verificación por script; retención 90 días, borrado a petición.
3. PII fuera de git: corpus real en store cifrado local; en el repo solo
   hash + 1 fixture sintética. Lo mismo para caché y logs (P0-2).
4. Fallback: si en 3 días no hay acceso, corpus sintético marcado
   `sintetico`, que NO desbloquea E1 (antes hay que recalibrar en vivo).
5. DoD: freeze + inventario + corpus versionados; E1 bloqueado hasta E0 real.

## Fases verificables (secuenciales, nada en paralelo real)
- **E1 — Repo + núcleo recortado (bloqueado hasta E0 real):** crear repo,
  Sentinel día 1, solo `lector` (observa excerpt) + `firmas` (normaliza) como
  librería. Nada de `catalog/ficha/precio` (producto MN, vive en el plugin).
  DoD: tests verde, 0 imports Electron (verificado con `madge`+`depcheck`
  en CI), extracción 5/5 campos E0 en corpus.
- **E2 — Adaptador con flag (tras E1):** `opencode-propio` consume el paquete
  pineado tras flag (`mp_nucleo=off` por defecto); lógica vieja intacta.
  Paridad sobre corpus: 10 casos (3 hilos × variaciones), mismo excerpt →
  mismo borrador salvo plantilla declarada; timeout y estado de carga
  definidos. DoD: paridad 10/10 + precio correcto en corpus (la prueba viva
  con usuaria pertenece a M1, no a E2).
- **M2 — Panel (lo construye el núcleo aquí):** un content-script +
  ShadowDOM montado por el núcleo; `asistente` y `Radar` (03AA-5) registran
  modos vía contrato `registerMode()` (arbitraje: un solo float visible).
  Selectores versionados: ancla primario al composer
  (`div[contenteditable="true"]` en panel chat); F1 `div[role="textbox"]`,
  F2 caja del hilo activo por `aria-label`. Sin composer se oculta y el
  borrador pendiente persiste en `sessionStorage` por hilo (sobrevive
  reload). Minimiza a burbuja. DoD: fixture con/sin composer + 1280px y
  390px + test teclado/contraste.
- **M3 — Endpoint `POST /api/admin/marketplace/borrador`:** auth JWT con
  scope mínimo `mp:borrador` (no admin pleno), exp corta, secreto en
  env/keyring nunca en repo; 429 por IP/usuario anti-abuso; schema cerrado
  publicado: `{excerpt:{remitente,texto,hora,avisoId,leido}, avisoId,
  extras?:{tono?,largo?}}` — cualquier clave fuera de allowlist (incluye
  `precio_minimo`/`margen`/`precioMinimo`) → 422. Frontera: el backend
  fetcha la ficha completa por `avisoId` y la pasa por `stripFichaParaPrompt()`
  (función nombrada que quita mínimo/margen) antes de llamar a la IA;
  generador pineado al modelo del chat MN `vX.Y` con assert en CI (nunca
  `jev`, que en 03AA-5 sí ve precio completo). Fallback genérico sin cifra:
  `Lo reviso y te confirmo precio/entrega por aquí`. Test negativo con
  matriz versionada: cifra exacta, `85k`, `$85.000`, en letras,
  `margen=true`, en IA + fallback + `SELECT respuesta,logs`. Latencia p95
  <8s con N≥50 medida aquí.
- **E3 — CLI `generarBorrador` (tras M3, usa su endpoint):** excerpt →
  borrador por terminal con JWT de scope `mp:borrador`. DoD: funciona sin
  Electron, fuga 0.
- **M4 — Caché con spec:** migración `mp_respuestas_cache(firma,
  precio_hash, catalog_hash, respuesta, valida_hasta DEFAULT +90d, usos,
  corregida; UNIQUE(firma,precio_hash,catalog_hash))` + migración DOWN;
  lookup por la terna; writer `hashFicha()` calcula hash al guardar; purga
  de vencidas con schedule nombrado (diaria 03:00). Firma conocida y válida
  = 0 tokens; Regenerar = bypass lectura + invalida esa firma
  (`UPDATE valida_hasta=now(), corregida=true`), sin tope (decisión
  usuaria). Concurrentes: `INSERT ... ON CONFLICT DO NOTHING + SELECT` y
  singleflight por firma (un miss simultáneo = 1 llamada IA). Contador en
  admin `GET /api/admin/marketplace/uso-hoy` (scope `mp:lectura`). DoD:
  hit-rate medido, precio correcto 10/10 tras cambio de ficha, hit con
  ficha vieja = 0, contador visible.
- **M1 — Fuera el strip (SOLO si E2+M2+M3 verdes en viva con usuaria):**
  tag pre-strip del estado anterior, recién ahí se elimina watch/strip UI +
  claves i18n. Rollback probado: `flag off + uninstall + restore tag` <15min.

## Observabilidad y auditoría
Audit log sin PII: hash(hilo)+timestamp+hit/miss/copiar/regenerar (jamás
texto del excerpt, respuesta ni cifras). Dashboard admin: `uso-hoy`,
p95 (N≥50), hit-rate; alerta ante fuga del mínimo. Prueba viva final
(panel con precio + Regenerar + Copiar manual) grabada como evidencia.

## IA y coste (unificado con 03AA-5)
Texto lo genera el modelo del chat MN vía backend (nunca en el plugin ni en
`jev`, que solo decide en 03AA-5). Caché manda antes que IA; Regenerar sin
tope (decisión usuaria), contador admin de métrica.

## Estado
E0 en curso (solo lectura, timebox 3 días, sintético no desbloquea).
E1 bloqueado hasta E0 real; E2+ espera tus puntos restantes.

## Próximo paso
E0: freeze hash del fork + inventario + 3 hilos anonimizados (pide acceso al
frente de opencode-propio o muestra de la usuaria).

## Gate y DoD global
Sentinel PASS en plugin + backend MN (fmt/check/clippy/test) + `tsc` 0;
paridad 10/10; fuga del mínimo 0/N; precio correcto 10/10; rollback probado;
cierre con viva grabada y evidencia en `Agente/completados/`.
