# Plan 03AA-3 — Asistente Marketplace: repo propio + respuestas con IA
> 4º reto 2026-10-05 (veredicto: REPLANTEAR por inejecutabilidad) aplicado
> entero: E0 reescrito como checklist ejecutable, dueño de ficha canónico,
> schema M3 publicado, rollback por repo, PII fuera de git, números con
> origen+método, orden E1→E2→M3→M2→E3→M4→M1 (contrato antes que panel).
> Regenerar sin tope (decisión usuaria); 429 anti-abuso protege endpoint.

## Objetivo
Sacar el asistente de respuestas de Messenger de `opencode-propio` a un repo
nuevo `plugins-opencode`, y pasar sus borradores de plantillas a IA vía
backend MN con caché (firma conocida = 0 tokens). El envío siempre es humano.

## Alcance / no alcance
- Sí (en orden): E0 inventario; E1 núcleo+contrato M3 congelado; E2
  adaptador con flag; M3 endpoint; M2 panel; E3 CLI; M4 caché; M1 strip.
- No: auto-envío (prohibido siempre), credenciales/sesión de Meta (las pone
  ella), API oficial de Messenger, auto-pegado al composer (solo gesto humano).

## Doctrina Meta (común con 03AA-5)
Solo-lectura del DOM de hilos que ella abrió; red solo vía `background` al
backend local; **cero clipboard automático** (botón "Copiar" manual,
Regenerar nunca auto-pega). Ritmo humano = límite operativo; 429 anti-abuso
no es tope para ella. Métrica `borradores/día` visible.

## Frase canónica de ficha (cierra contradicción E1/M3)
La ficha vive en el backend MN (tabla `inmuebles`); el plugin jamás la ve
(solo manda excerpt+avisoId). `stripFichaParaPrompt(ficha: Ficha) ->
PromptSeguro` vive en `src/handlers/marketplace.rs` del backend y es la
única puerta hacia la IA.

## Dependencias y contrato
- `opencode-propio` (no git, otro frente): solo se toca el adaptador;
  nadie modifica `marketplace-*` sin avisar; el adaptador verifica el
  sha256 del tarball pineado (`npm pack` + hash en config del adaptador).
- Backend MN local con `OPENCODE_GO_API_KEY`; modelo generador pineado
  `chat-MN vX.Y` con assert en CI (nunca `jev`).
- Remoto del repo (lo crea ella o local hasta su aviso); sin remoto no hay E1.

## E0 — Checklist ejecutable (SOLO lectura, timebox 3 días, owner: agente)
1. Acceso (día 1): pedir al frente de opencode-propio o muestra a la usuaria.
2. Freeze: `Compress-Archive -Path <fork>\* -DestinationPath
   C:\tmp\mp-freeze-2026-10-XX.zip` + `Get-FileHash -Algorithm SHA256`
   (ruta y hash quedan en el inventario).
3. Inventario: tabla `path:línea` por cada `marketplace-*.ts` (qué hace cada
   uno + funciones clave). Ejemplo: `marketplace-watch.ts:40 observarHilo`.
4. `excerpt` = 5 campos (remitente, texto, hora, aviso enlazado si hay,
   estado leído); `firma normalizada` = `sha256(minúsculas sin tildes del
   texto + avisoId)` (algoritmo versionado `firma-v1`).
5. Anonimización con `scripts/anonimizar-mjs` (nombres→`[NOMBRE]`,
   teléfonos→`[TEL]`, fotos→solo hash) + `scripts/verificar-anonimizacion`
   (falla si encuentra `\d{7,}` o nombres fuera de allowlist).
6. Store: `C:\Users\Owner\.mp-privado\` cifrado con 7z+clave (la clave vive
   en el gestor de claves del SO, nunca en repo ni notas); en el repo solo
   hash del corpus + 1 fixture sintética. Retención 90d (ciclo acordado con
   usuaria, revisable), borrado a petición <72h (corpus+caché+logs).
7. Fallback día 3 sin acceso: corpus sintético que NO desbloquea E1.
   Recalibrar en vivo = 1 hilo real con usuaria 30min + checklist
   (owner agente, fecha E0+2d). Sin eso el plan queda aparcado con este
   paso como único pendiente.
8. DoD: freeze (ruta+hash) + inventario + corpus o declaración de aparcado.

## Fases (secuenciales)
- **E1 — Repo + núcleo + contrato congelado (bloqueado hasta E0 real):**
  crear repo, bootstrap Sentinel (lo instala el agente con la herramienta
  del proyecto en E1, no "día 1" mágico), `lector`+`firmas` como librería,
  schema M3 publicado aquí (antes que el panel). DoD: tests verde, 0
  imports Electron (`madge`+`depcheck` en CI), 5/5 campos en corpus.
- **E2 — Adaptador con flag (tras E1):** flag `mp_nucleo=off` por defecto,
  lógica vieja intacta, tarball verificado por sha256. Paridad sobre corpus:
  10 casos (3 hilos × variaciones; origen del N), timeout 20s + estado de
  carga definidos. DoD: paridad 10/10 en corpus (la viva es de M1).
- **M3 — Endpoint `POST /api/admin/marketplace/borrador`:** JWT
  (`iss mn-backend`, `aud mp`, scope `mp:borrador`, `exp 15min` —origen:
  uso puntual CLI/panel—, secreto en env/keyring); `429 30/min` por `sub`
  JWT + `Retry-After` (en localhost la clave es `sub`, no IP). Emisión para
  E3: `POST /api/admin/marketplace/token` (scope `mp:borrador`, documentado).
  Schema cerrado: `{excerpt:{remitente,texto,hora ISO8601,leido},
  avisoId, extras?:{tono enum [corto,amable,formal], largo enum [s,m,l]}}`;
  `avisoId` canónico fuera, prohibido dentro (mismatch=422); allowlist con
  regex normalizada (minúsculas, sin separadores: `precio_minimo|margen|
  precioMinimo|minprice|floor|cost` + matriz negativa versionada en repo).
  Frontera: fetch ficha → `stripFichaParaPrompt()` → IA; fallback sin cifra:
  `Lo reviso y te confirmo precio/entrega por aquí`. Test: matriz en IA +
  fallback + `SELECT respuesta` (audit nunca guarda texto). p95 <8s,
  N=50 misses seguidos en local, `scripts/measure-mjs` (tokens a cuenta
  del proyecto; origen del N y método escritos).
- **M2 — Panel (usa contrato E1):** `selectores.json v1` versionado + test
  semanal (Meta cambia el DOM); ancla composer + F1 + F2 nombrados ahí;
  key `mp_borrador:{threadIdDeURL}` + TTL 24h; arbitraje
  `registerMode()`: prioridad asistente>radar + mutex (un float visible).
  Nota CSP/ManifestV3 + riesgo de revisión por scraping (se declara, no se
  esconde); DOM móvil distinto = fixture 390px solo desktop-estrecho.
  Borrador pendiente en `sessionStorage` por hilo. DoD: fixture con/sin
  composer + 1280/390 + teclado/contraste.
- **E3 — CLI (tras M3):** usa endpoint con token de `.../marketplace/token`.
  DoD: sin Electron, fuga 0.
- **M4 — Caché:** migración + DOWN (`DROP TABLE`);
  `UNIQUE(firma,precio_hash,catalog_hash)`; `hashFicha()` la llama el
  backend tras fetch y antes de `strip`; lookup por terna; Regenerar =
  `DELETE` de la fila + bypass (sin ventana de race; test de doble
  Regenerar simultáneo); `corregida` la marca el admin al corregir (lector:
  endpoint aprobar); singleflight en memoria (válido: backend local es un
  solo proceso); purga al arrancar el backend + pg_cron diario si hay DB
  24h (timezone America/Caracas); `valida_hasta` default 90d (mismo ciclo
  que corpus, revisable). Contador `GET /api/admin/marketplace/uso-hoy`
  (scope `mp:lectura`, mismo `iss`). DoD: hit-rate medido, precio 10/10
  tras cambio de ficha, hit con ficha vieja = 0.
- **M1 — Strip (SOLO si E2+M2+M3 verdes en viva):** tag `pre-strip-vX` en
  `plugins-opencode` + backup zip fechado de `opencode-propio`; recién ahí
  se borra watch/strip UI + i18n. Rollback = runbook probado (flag off +
  uninstall + restore; pasos 1..N temporizados una vez, sin prometer minutos).

## Observabilidad
`POST /api/admin/marketplace/audit` + tabla audit
`(hash_hilo,ts,evento:hit/miss/copiar/regenerar)` sin texto ni cifras.
Dashboard: se construye en M2 (ruta admin `marketplace/uso` con uso-hoy,
p95, hit-rate). Alerta fuga: log + contador + umbral 1 (cualquier fuga
dispara revisión). Viva final grabada con consentimiento, en store cifrado,
nunca en repo.

## IA y coste
Genera el modelo del chat MN pineado (nunca `jev`, nunca en el plugin).
Caché antes que IA; Regenerar sin tope (decisión usuaria), contador de métrica.

## Estado
E0 en curso (checklist de arriba, 3 días). E1 bloqueado hasta E0 real.
E2+ espera tus puntos restantes.

## Gate y cierre
Gate separado por repo: plugin TS (`tsc` 0 + tests + Sentinel si aplica) y
backend Rust (fmt/check/clippy/test). Cierre con viva grabada; evidencia en
`MN-Inmobiliaria/Agente/completados/` + nota en docs del plugin.
