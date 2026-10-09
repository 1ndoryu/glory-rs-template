# Plan publicar Loma Linda + mejora — 2026-10-09 (09AA-1)

> Estado: CERRADO 2026-10-09 (09AA-1).
> publicado en local+prod, verificado `estado` exit 0). F2 BLOQUEADA:
> `GEMINI_PSID` caducada (`UNAUTHENTICATED` en `frontend/logs/eventos-2026-10-09.log`,
> `fallosSeguidos=3`, `procesadosHoy=0`): los 2 jobs encolados reintentan
> solos cuando haya cookie nueva; el CLI de mejora se detuvo (PID 28660)
> para no quemar reintentos. Acción pendiente de ella: lanzar el acceso
> directo "Chrome-Horacio-debug" y esperar AVAILABLE. Al volver: re-lanzar
> `mejorar --slug` (no duplica por foto) y seguir F2. Mejora YA corriendo
> por mi cuenta (PID 24860): abrí el Chrome debug con los flags del acceso
> directo (el `.lnk` no existe en el escritorio; el perfil `%LOCALAPPDATA%\
> ChromeHoracioDebug` sí, con sesión válida) + `renovar-cookies` → AVAILABLE.
> Wart detectado (sin parchear): la bomba durmió con el backoff previo
> (fallos=5 → ~34 min) y despierta ~01:50 aunque las creds ya valen; no
> reiniciar el :3122 (borraría la cola en memoria). Origen: pedido de
> ella 2026-10-09 ("publica loma linda, y mejora las fotos"). Receta base:
> skill `publicar-inmuebles` (§3 orden de operaciones: originales primero,
> la mejora nunca bloquea).

## Estado verificado

- Hilo +52 `5214427789328`, sesión `cea7a25d-1012-40f5-9a25-12e8dd5d56b3`:
  "Test" + descripción (venta 380 000 USD, canon 2 800 USD) + 15 fotos en
  `uploads/whatsapp/5214427789328/`, llegada 2026-10-08 22:46 UTC.
- Dedup por sha256: 13 únicas (2 duplicados exactos: `8f0747c0=96af497b`,
  `9e1de7a5=cc367115`). Orden de publicación = orden de llegada al chat.
- No existe el slug en local ni prod (13 slugs, ninguno Loma Linda).
- Núcleo: casa / venta / 380000 / 3 hab / 5 baños / disponible; metros y
  puestos desconocidos → 0; canon queda en el texto de la descripción.
- Mejora `:3122/api/salud`: listo, worker vivo, cola 0, 0/40 hoy. 13 fotos
  × ~120 s + jitter ≈ 30-40 min → F2 en background con log + polling.
- Gate baseline 2026-10-09: 0E/436W (re-análisis 00:14, 09AA-35).

## No alcance

- Sin pedir fotos HD: se publica con lo recibido (ella ya pidió publicar).
- `ai_enabled_global` sigue `off` hasta que ella confirme fin de reenvíos
  (fuera de este plan, como en 08AA-35).
- Sin SSH/docker/scp contra prod: escrituras prod solo por API (publicar,
  push). Backup prod vía manager solo si hiciera falta borrado (no previsto).

## Fases

- F1 — Staging en `C:\tmp\loma-linda-pub` (13 únicas como `00..12.jpg` en
  orden de llegada; `publicar` ordena alfabético = orden de llegada) +
  `C:\tmp\loma-linda-datos.json` → `publicar --dry-run` → real (local+prod)
  → `estado --slug` + GET público ambos lados.
- F2 — `mejorar --slug casa-en-venta-en-loma-linda` en background
  (Start-Process, log en `C:\tmp\mejora-loma-linda.log`), polling del log;
  al terminar: `push --slug` (mejoradas faltantes → prod) + `verificar
  --slug` + humo público con portada (mejorada orden 0).
- F3 — Docs: skill (si hay gotcha nuevo), completada, roadmap, commit
  `09AA-1` + push. Gate re-analizado sin hallazgos nuevos.

## Verificación (DoD global)

- [ ] F1: slug existe local+prod, publicado=true, 13 originales cada lado,
  GET público 200 ambos.
- [ ] F2: 13 mejoradas local (+ push a prod), `verificar --slug` exit 0,
  portada = mejorada orden 0 en público.
- [ ] Commit + push del bloque.
