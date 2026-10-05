# Plan 03AA-3 — Asistente Marketplace: repo propio + respuestas con IA
> 5º reto 2026-10-05 aplicado: 18 puntos (avisoId nullable, freeze cifrado,
> token endpoint con spec, HMAC en audit, restore con dueño, strip por
> allowlist, CLI 8h, scopes separados, 429 exime a ella, store en memoria,
> selectores con dueño, measure con mock, MP_PRIVADO, doctrina referenciada).
> Regenerar sin tope (decisión usuaria); retención 90d revisable.

## Objetivo
Sacar el asistente de respuestas de Messenger de `opencode-propio` a un repo
nuevo `plugins-opencode`, y pasar sus borradores de plantillas a IA vía
backend MN con caché (firma conocida = 0 tokens). El envío siempre es humano.

## Alcance / no alcance
- Sí (en orden): E0 inventario; E1 núcleo+contrato M3 congelado; E2
  adaptador con flag (contra stub M3 declarado); M3 endpoint; M2 panel;
  E3 CLI; M4 caché; M1 strip.
- No: auto-envío (prohibido siempre), credenciales/sesión de Meta (las pone
  ella), API oficial de Messenger, auto-pegado al composer, móvil real
  (fuera de alcance: solo desktop-estrecho 390px).

## Doctrina Meta
Fuente canónica: plan 03AA-5 (pin `doctrina-v1`); aquí solo ref + delta
marketplace: cero clipboard automático (botón "Copiar" manual), 429
anti-abuso que exime a su `sub` (ver M3). Si 03AA-5 cambia, este plan sigue
al pin, no a la copia.

## Frase canónica de ficha
La ficha vive en el backend MN (tabla `inmuebles`); el plugin jamás la ve.
`stripFichaParaPrompt(ficha: Ficha, strip_vN) -> PromptSeguro` en
`src/handlers/marketplace.rs`, por allowlist explícita
`["titulo","precio_publico","zona","m2","habitaciones","descripcion_corta"]`;
test que falla si una migración añade columna no mapeada; `strip-vN`
pineado junto a `firma-v1`.

## Dependencias y contrato
- `opencode-propio` (no git, otro frente): solo adaptador; nadie modifica
  `marketplace-*` sin avisar; tarball verificado por sha256 pineado.
- Backend MN local + `OPENCODE_GO_API_KEY`; generador pineado `chat-MN vX.Y`
  (assert CI, nunca `jev`).
- Remoto del repo (lo crea ella o local hasta su aviso); sin remoto no hay
  E1; M1 exige `git ls-remote origin` verde + `push --tags` verificado (si
  no, `git bundle` fechado fuera del repo en vez de tag).

## E0 — Checklist ejecutable (SOLO lectura, 3 días, owner: agente)
1. Acceso día 1 + reservar viva 30min con fecha (E0+2d). Sin fecha, E0 no
   cierra: se aparca ya con este paso como único pendiente.
2. Freeze cifrado directo (nunca PII en claro en `C:\tmp`):
   `7z a -p -mhe=on <destino>.7z <fork>\*` + `Get-FileHash SHA256` +
   `icacls <destino> /inheritance:r /grant:r "%USERNAME%:F"`. Al cerrar E0:
   `cipher /w` del temporal + verificación.
3. Inventario `path:línea` por `marketplace-*.ts` (ruta+hash del freeze solo
   en archivo privado, nunca en repo).
4. `excerpt` 5 campos (remitente, texto, hora, aviso enlazado si hay, estado
   leído); `firma-v1 = HMAC_SHA256(sal_en_keyring, NFC(trim(colapso(texto)))
   + "|" + (avisoId ?? ""))`; test `null→enlazado` (cambia la firma, no
   colisiona).
5. `scripts/anonimizar-mjs` + verificador (falla con `\d{7,}` o nombres
   fuera de allowlist). Fotos→HMAC (no sha256 reversible).
6. Store: `%MP_PRIVADO%` (excepción documentada en roadmap: fuera de
   `area-trabajo` y de `C:\tmp` por ser PII cifrada; con backup cifrado con
   rotación e `icacls` explícito). Clave en keyring del SO. En repo: solo
   hash del corpus + 1 fixture sintética. Borrado a petición <72h.
7. Consentimiento de la viva por escrito (antes de grabar).
8. Sintético = NO desbloquea E1 (por escrito). Recalibrar = checklist viva.
9. DoD: freeze (ruta+hash en privado) + inventario + corpus/sintético +
   fecha de viva reservada o declaración de aparcado.

## Fases (secuenciales)
- **E1 — Repo + núcleo + contrato congelado (bloqueado hasta E0 real):**
  Sentinel instalado por el agente con tooling del proyecto; `lector`+
  `firmas`; schema M3 v1 publicado aquí. DoD: tests verde, 0 Electron salvo
  bridge documentado (`madge`+`depcheck`), 5/5 campos en corpus.
- **E2 — Adaptador con flag (tras E1, contra stub M3 explícito):** flag
  `mp_nucleo=off`, lógica vieja intacta, sha256 verificado. Paridad 10 casos
  (3 hilos × variaciones), timeout 20s + estado de carga. DoD: paridad 10/10
  en corpus (la viva es de M1).
- **M3 — Endpoints:** `POST /api/admin/marketplace/borrador` (JWT
  `iss mn-backend`, `aud mp`, scope `mp:borrador`, `exp 15min`; `429 30/min`
  por `sub` + `Retry-After`, **exime `sub` de ella** vía `MP_SIN_LIMITE_SUB`;
  métrica separa `429-humano`/`429-bot`). Emisor
  `POST /api/admin/marketplace/token`: exige sesión admin vigente (no API
  key suelta), `429 5/min` propio, `jti` + lista de revocación,
  `audit(emision,sub,jti)`, TTL emisor (sesión) ≠ TTL token. Tokens: panel
  solo `mp:lectura`, CLI `mp:borrador` `exp 8h + binding máquina`
  (re-auth documentada; expiración probada en DoD E3); assert CI rechaza
  wildcard. Schema v1: `{threadId, firma, firma_version, lang,
  excerpt:{remitente_hash, texto, hora ISO8601 America/Caracas, leido},
  avisoId: string | null, extras?:{tono [corto,amable,formal], largo
  [s,m,l]}}`; `avisoId` fuera, prohibido dentro (mismatch=422); ruta
  `sin-ficha` (solo contexto del hilo) cuando es null. Allowlist regex
  normalizada + matriz negativa versionada. Frontera: fetch → hashFicha
  (definida byte-a-byte en M4) → `strip` → IA; fallback sin cifra. p95 <8s:
  `measure-mjs` con mock determinista por defecto; `--real` opt-in N=10 con
  tope de presupuesto + flag `no-audit` + purga post-test.
- **M2 — Panel (contrato E1):** `selectores.json {version, dom_pin,
  ultima_verificacion}` (owner nominal + runbook <24h + test semanal que
  corre el agente con fixture); selectores por rol/accesibilidad, no por
  clase hasheada; fixture = DOM real anonimizado (solo si E0 real).
  `registerMode()`: prioridad asistente>radar + mutex. Borrador en memoria
  + `MutationObserver` a cambio de ruta (SPA: sin recarga no hay
  sessionStorage fiable); limpieza al cambiar threadId; fallback
  `threadId=hash(excerpt)` sin URL. Modo degradado: `selectores stale` +
  botón deshabilitado. DoD: fixture con/sin composer + 1280/390 + teclado/
  contraste + test SPA.
- **E3 — CLI (tras M3):** token 8h del emisor. DoD: sin Electron, fuga 0,
  expiración probada.
- **M4 — Caché:** migración + DOWN (`DROP TABLE`);
  `UNIQUE(firma,precio_hash,catalog_hash)`; `hashFicha()` byte-a-byte
  (campos, moneda, redondeo) llamada tras fetch y antes de `strip`;
  Regenerar = `DELETE` + bypass (test doble simultáneo); `corregida` la
  marca el admin al corregir (`POST .../corregir`); singleflight en memoria
  con invariante `single-process` + assert al arrancar (si hay workers, lock
  DB); purga al arrancar + pg_cron diario solo si `DB_24H=true`
  (America/Caracas), si no aviso en logs; `valida_hasta` funcional ≠
  retención legal 90d (columnas separadas). Contador
  `GET /api/admin/marketplace/uso-hoy` (scope `mp:lectura`). DoD: hit-rate,
  precio 10/10 tras cambio, hit con ficha vieja = 0.
- **M1 — Strip (SOLO si E2+M2+M3 verdes en viva + firma del otro frente):**
  tag `pre-strip-vX` (o bundle) + backup zip <15min antes + `diff`
  pre-restore; restore solo por dueño de `opencode-propio` en ventana de
  congelación escrita; recién ahí se borra watch/strip UI + i18n. Sin firma,
  M1 prohibido. Rollback = runbook probado 1..N.

## Observabilidad
`POST /api/admin/marketplace/audit` + tabla
`(hilo_hmac, ts_trunc_hora, evento:hit/miss/copiar/regenerar/emision)`:
HMAC con sal en keyring (nunca sha256 de texto), ts truncado a hora,
amenaza de correlación declarada. Dashboard se construye en M2
(`marketplace/uso`). Alerta fuga: log + contador + umbral 1. Viva con
consentimiento, en store cifrado, nunca en repo.

## IA y coste
Genera el chat MN pineado (nunca `jev`, nunca en el plugin). Caché antes
que IA; Regenerar sin tope (decisión usuaria), contador de métrica.

## Estado
E0 en curso (checklist, 3 días). E1 bloqueado hasta E0 real + remoto.
E2+ espera tus puntos restantes.

## Gate y cierre
Plugin TS (`tsc` 0 + tests) y backend Rust (fmt/check/clippy/test) por
separado; Sentinel donde aplique. Cierre con viva grabada; evidencia en
`MN-Inmobiliaria/Agente/completados/` + nota en docs del plugin.
