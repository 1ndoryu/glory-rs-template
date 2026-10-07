# Plan 07AA-6 — Flujo controlado para modificar opencode-propio (lab + skill)

> Origen: 2026-10-07, pedido de ella. Su app corre en dev con recarga
> automática (`electron-vite dev`): guardar un archivo la rompe. Este plan
> deja el flujo completo y repetible para cualquier agente futuro.

## Objetivo

Modificar `opencode-propio` sin romper jamás la app viva, con mecanismo de
promoción explícito y una skill que lo enseñe a otros agentes.

## Hallazgos (evidencia, no suposiciones)

- Lanzador real: `Desktop/Opencode-Propio-Dev.cmd` → backend `:4096`
  (reutiliza si ya escucha) + `bun run dev` en `packages/desktop`
  (= `electron-vite dev`, HMR activo) + `OPENCODE_DB` identity compartida.
- Su checkout está SUCIO (decenas de `M` sin commit: i18n, settings,
  generados, `bun.lock`): jamás pisarlo; promoción siempre con diff primero.
- Lab creado [07AA-5]: `area-trabajo/opencode-propio-dev/` (copia sin
  `node_modules/.git/.turbo/out/dist/build`), `bun install` propio OK
  (523 paquetes), `marketplace-assistant.test.ts` 26/26, regla en
  `LEEME-LAB.md`.
- Puertos/app protegidos (§5 área): nunca matar procesos ni tocar
  5174/5175/8787/4096; el reinicio siempre lo hace ella.

## Decisión: ¿se usa git para mover el cambio? No

- Descartado worktree: parte de HEAD y pierde sus cambios sin commit.
- Descartado `git init` en el lab: repo fantasma que confunde y riesgo de
  push al remoto real (`push → 1ndoryu/opencode`).
- Elegido: **promoción por copia archivo-por-archivo con diff-first +
  manifest** (lista archivos + hash + origen) archivado en la ventana.
  Simple, auditable, sin repos falsos. Revisitar si el volumen crece.

## Fases

- **F1 — Skill `lab-opencode`**: `SKILL.md` (frontmatter `name` +
  `description` + cuerpo, como las de `.agents/skills/`) con rutas,
  reglas, puertos/db, comandos de verificación, promoción y rollback.
  DoD: la skill carga y se usa en el piloto F3.
- **F2 — Mecanismo de promoción (checklist de ventana)**: 1) diff de cada
  archivo lab vs suyo (respeta sus `M`); 2) manifest con hashes;
  3) aplicar; 4) ella reinicia con su acceso directo; 5) prueba viva;
  6) si falla, rollback restaurando su estado previo (sus `M` intactos).
- **F3 — Piloto real**: aplicar este flujo al C1 del plan 03AA-3
  (cableado Marketplace). DoD: C1 en ventana, app arranca, off = viejo
  intacto, viva verde.
- **F4 — Prohibiciones fijas**: matar/reiniciar su app o backend, tocar
  puertos, correr la app del lab por defecto, commitear en `src/`,
  compartir su `identity.db`.

## F3a — Puntos de cableado (estudiado 2026-10-07, solo lectura)

- Motor actual (lab `marketplace-service.ts`): `syncFloatGuest` —bucle auto
  5 s + poll manual— y el IPC `marketplace-drafts` usan el motor local
  `generarBorradores` de `./marketplace-assistant`. Punto de inserción: antes
  de `generarBorradores`, rama núcleo con fallback silencioso al local.
- Contrato backend (verificado `marketplace.rs` + `handlers/marketplace.rs`):
  `POST /api/admin/marketplace/borrador` con `BorradorRequest` (`threadId`,
  `firma` hex64 + `firma_version: "firma-v1"`, `lang` 2 letras,
  `excerpt{remitente_hash hex64, texto 1..2000, hora ISO8601 -04:00}`,
  `avisoId?`, `extras?`) + JWT mp. El handler valida formato y usa `firma`
  como clave de caché —no verifica HMAC—.
- Cripto ya existe en cliente (`plugins-opencode/src/nucleo/firmas.ts`,
  `excerpt.ts`, `lector.ts`): HMAC_SHA256 con sal del keyring del SO
  (hex ≥16 B); `remitenteHash` + `firmaAviso` + `construirExcerpt`.
- Flag del bridge (`bridge.ts`): `MP_NUCLEO=on` activa, default OFF.

## F3b — Diseño (mínimo, sin UX)

Portar al lab (main, `node:crypto` disponible): `firmaAviso` +
`remitenteHash` + construcción del excerpt desde `window.excerpt`/`key`.
Token mp por env `MP_MN_TOKEN` (pegado manual, sin cambios de settings en
el piloto); flag default OFF; ante cualquier fallo, fallback silencioso
al motor local. UX de obtención del token queda pendiente documentado,
no decidido en código.

## F3b — Implementado en lab (2026-10-07, verificado)

- `opencode-propio-dev/src/packages/desktop/src/main/marketplace-nucleo.ts`
  (nuevo): `nucleoActivo` (flag `MP_NUCLEO=on` + `MP_MN_TOKEN` + `MP_SAL`,
  default OFF), port fiel `firmaAviso`/`remitenteHash`/`colapso`,
  `horaCaracasISO` (-04:00), `buildBorradorRequest` (espejo de
  `validar_borrador`), `parseBorradorResponse`,
  `pedirBorradorNucleo` (timeout 20 s, nunca lanza) y
  `borradorNucleoParaVentana` (forma de panel o `undefined`).
- `marketplace-service.ts` (+import, 3 líneas en `syncFloatGuest`): núcleo
  primero, motor local intacto como fallback; caché/auto-copia/panel sin
  cambios. `marketplace-drafts` IPC queda local a propósito en el piloto.
- Tests nuevos `marketplace-nucleo.test.ts` (12) + existentes (26):
  **38/38 verde** (`bun test`, un fallo inicial corrigió el test, no el
  código: `""` normaliza a sin-aviso como el backend). `bun run typecheck`
  limpio. Viva intacta: cero ediciones fuera del lab.
- Lección test: `firmaAviso(x, "", sal) === firmaAviso(x, null, sal)` por
  diseño (`?? ""`); el backend rechaza `avisoId` vacío, el builder manda
  siempre `null` en el piloto (Aviso no trae id).

## Estado

F1+F2+F3a+F3b+F3c hechas. C1b CANCELADO 2026-10-07 (navegadores
separados, sin copia de sesión, sin dev del lab). Promoción 2026-10-07 en
ventana explícita (app cerrada, `:4096` libre, diff-first limpio, manifest
en lab `PROMOCION-VENTANA.md`): F3b a su app (3 archivos marketplace,
`MP_NUCLEO` default OFF) + OP-47 auto (3 archivos `session/`, guard
terminar-vs-pendientes). Tests en su árbol 33/33 + 38/38, sin commit.
Sigue: ella reinicia con su acceso directo + prueba viva en opencode-propio.

## C1-lab — Puente verificado 2026-10-07 (~17:00)
- Lab `opencode-propio-dev` con `MP_NUCLEO=on` + token CLI 8h (`mid
  d9f5aed6…`, header `X-MP-Maquina`) + `MP_SAL` 32hex, lanzado vía
  `C:\tmp\lab-dev.cmd` (las env por `Invoke-CimMethod` directo no llegaban;
  cero líneas `[mp-nucleo]` todo el día hasta el relanzamiento).
- Prueba viva de ella: Regenerar cambió el texto. Evidencia:
  `lab-dev.log` 4× `[mp-nucleo] borrador del núcleo` + `mn-server.log`
  `POST /borrador 200` (latencias 5-14s = IA, luego 4ms = caché).
- Retención vigente: `mp_respuestas_cache` 90d (`valida_hasta`, `usos`,
  `corregida`), solo se cachea `fuente=ia`; audit sin PII (tope 90d).

## C1b — Sesiones de navegador (CANCELADO por ella 2026-10-07)

- Las sesiones viven en `<userData>/Partitions/propio-browser-shared`;
  userData suya = `%APPDATA%/ai.opencode.desktop.propio`, userData del lab
  = `%APPDATA%/ai.opencode.desktop.dev` (separado por diseño, `index.ts`).
- Compartir en vivo es imposible y peligroso: mismo userData con las dos
  apps = pelea por single-instance lock + dos Chromium escribiendo el
  mismo LevelDB (riesgo real: corromper la sesión y sacarla de Facebook,
  justo lo que ella teme; además no recuerda la contraseña).
- Procedimiento C1b ~~(cancelado, ver abajo)~~: 1) ella cierra su app; 2) el agente copia
  `Partitions/propio-browser-shared` (app cerrada = copia consistente) al
  userData del lab; 3) el dev corre con esa copia (su app puede reabrirse
  después; el lab nunca escribe en su perfil). Sin ella cerrando, no hay
  prueba con sesión.
- **CANCELADO 2026-10-07 (decisión de ella)**: no se copia ninguna sesión al
  lab, no se corre el dev del lab, navegadores siempre separados. La prueba
  del piloto se hace en su opencode-propio tras la promoción.
- **Incidente 2026-10-07 (sesión Facebook cerrada)**: el agente copió por
  error jars obsoletos de `%APPDATA%/ai.opencode.desktop.propio`
  (`Cookies` 15/09, cero cookies facebook) sobre el perfil VIVO, que es
  `%APPDATA%/ai.opencode.desktop.dev` (`lockfile`+`DevToolsActivePort` de
  hoy; sin `OPENCODE_CHANNEL` el userData desempaquetado es `.dev`, el
  `.propio` está obsoleto). Forense readonly posterior: cero cookies
  facebook en las 10 particiones del perfil vivo. Causa probable del
  logout; sin contraseña no hay recuperación forense, solo vías oficiales
   de Facebook. Regla 6 agregada a la skill `lab-opencode`: prohibido
   escribir en perfiles de navegador.
- **Trasplante 2026-10-07 (excepción autorizada por ella, app cerrada)**:
  cookies de su Chrome wan trasplantadas al jar shared del perfil vivo
  (`...ai.opencode.desktop.dev\Partitions\propio-browser-shared\Network\Cookies`)
  con `C:\tmp\fb-insert.ts` (DELETE+INSERT, solo imprimió nombres).
  Insertadas las 7 (c_user,xs,datr,fr,sb,ps_l,ps_n) + 2 preexistentes (dpr,wd).
  Gotchas del script: (1) `new Database(path,{})`/`{create:false}` falla en
  este bun (`SQLITE_MISUSE`); sin opciones abre bien. (2) el jar usa schema
  Chromium viejo: sin `is_same_party`, con `encrypted_value`+`source_type`+
  `has_cross_site_ancestor` (INSERT ajustado). Valores nunca mostrados ni
  commiteados. Sigue: ella borra `C:\tmp\fb-wan.txt`, reabre su app, prueba
  facebook.com en el Navegador (si pide "¿fuiste tú?", aprueba en Chrome wan).
  Si no restaura: probable invalidación servidor → solo queda código de mañana.
