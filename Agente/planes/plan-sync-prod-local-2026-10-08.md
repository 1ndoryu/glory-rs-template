# Plan sync prod→local + automatización — 2026-10-08 (08AA-2)

> Estado: EJECUTADO 2026-10-08 (espejo OK 13/259, re-run exit 0).
> Origen: pedido de ella 2026-10-08 ("sincroniza los inmuebles de prod con
> los locales; si se puede automatizar, hazlo, que no falle ni rompa nada").
> Hallazgo: `scripts/sync-pull.mjs` quedó planeado en Fase 5 del plan deploy
> (solo lectura contra prod, destructivo en local con `--si`) pero nunca se
> creó (glob vacío 2026-10-08). Este plan lo construye y lo usa.

## Objetivo

Traer snapshot prod→local (inmuebles + fotos) y dejarlo en un comando
repetible (`npm run sync:pull`), idempotente y que no falle a medias.

## No alcance

- Nada se escribe en prod (ni datos, ni fotos, ni config). Prod solo se lee.
- Recetas, `publicado`, precios y copy se editan donde se usan (manda prod;
  el pull sobrescribe local sin preguntar con `--si`).
- Solicitudes/suscriptores de visitantes solo existen en prod; el pull no los
  toca (solo escribe en local).
- Nada de fusión bidireccional de BD (UUID, fotos y estados chocarían).

## Reglas de seguridad (no negociables)

1. Todo contra prod pasa por la API pública/admin (`https://mn-inmobiliaria.com`);
   cero SSH/docker/scp directos (reglas 1 y 19; el manager solo se usa para
   `health`/lecturas si hace falta, nunca para mutar).
2. Backup local ANTES de escribir: `pg_dump` de `glory_backend_inmobiliaria`
   a `C:\tmp` (rollback = restore del dump).
3. Todo se resuelve por `slug` + `orden`, nunca por id hardcodeado (los UUID
   difieren entre entornos).
4. Secretos solo en `scripts/.env.prod.local` (gitignored, JWT en memoria,
   nunca en logs ni en git). Verificar `.gitignore` antes del primer uso.
5. Dry-run por defecto; lo destructivo en local exige `--si` explícito.
6. Exit codes accionables (2 preflight, 3 descarga/mejora, 4 verificación).

## Fases

### F0 — Preflight y foto del estado (solo lectura en ambos lados)

1. `health` prod + local; conteos prod vs local (inmuebles, fotos, users).
2. `pg_dump` local → `C:\tmp` (con fecha/hora en el nombre).
3. Verificar `.gitignore` cubre `scripts/.env.prod.local` y `logs/`.
4. Criterio de salida: números anotados en la completada; si prod no responde,
   se aborta aquí sin tocar nada.

### F1 — One-shot ahora: `scripts/sync-pull.mjs` (Node, sin deps nuevas)

1. Login prod (`POST /api/auth/login`, JWT en memoria) → lista admin +
   públicas → upsert en local por `slug` (inmueble nuevo se crea, existente
   se sobrescribe: manda prod).
2. Fotos: descarga `{PROD}/uploads/<storage_key>` (público, sin auth) al
   `UPLOAD_DIR` local; empareja por `slug` + `orden` + `origen`.
3. Verificación: conteos local == prod + 1 foto HD 200 por muestreo.
4. Criterio de salida: `local == prod` en inmuebles y fotos, o exit ≠ 0 con
   mensaje accionable y local intacta (restore del dump si escribió a medias).

### F2 — Humo local + gate

1. Front local (`:5199`): lista con el total de prod, detalle, 1 foto 200.
2. Gate del bloque: `npx tsc --noEmit` (si se tocó front; el script solo no
   lo exige) + conteos iguales. Sin cambios `.rs` → sin gate Rust.
3. Criterio de salida: lista visible con datos de prod.

### F3 — Automatización (requiere decisiones de ella, ver abajo)

1. `npm run sync:pull` en `package.json` raíz (tras leer sus scripts reales).
2. Opcional: tarea programada Windows (diaria, hora que ella diga) que corre
   el comando con `--si` + log rotado en `logs/`; si falla, avisa y no reintenta
   a ciegas.
3. Criterio de salida: segundo run idempotente (0 cambios, exit 0).

### F4 — Cierre

Completada en `Agente/completados/tareas-2026-10-08.md` (qué, archivos,
gotchas, Sentinel: no aplica, GLORY: no aplica) + roadmap actualizado +
commit `08AA-2: ...` + push. Releer roadmap al cerrar (regla 16).

## Verificación (DoD)

- [ ] Conteos inmuebles/fotos local == prod.
- [ ] Humo front local OK (lista + detalle + foto 200).
- [ ] Cero escrituras en prod (solo GETs + login; auditable en logs del script).
- [ ] Re-run idempotente exit 0.
- [ ] Commit + push del bloque.

## Decisiones que requiere de ella (preguntadas 2026-10-08)

1. Fotos también en el primer sync (recomendado: sí, ~28 MB) o solo datos.
2. Automatizar con tarea programada diaria o dejarlo en comando manual.
3. `--si` (sobrescribir local sin preguntar) como default del comando o
   pedir confirmación siempre.

## Respuestas de ella (2026-10-08)

1. **Datos + fotos.** 2. **Solo comando manual** (`npm run sync:pull`, sin
   tarea programada). 3. **Sobrescribir sin preguntar** (el script escribe en
   local por defecto; `--dry-run` para previsualizar; backup `pg_dump`
   previo obligatorio). Además: **que nada afecte prod** → el script solo
   hace login + GETs contra prod; ni deploy, ni restart, ni escrituras.
