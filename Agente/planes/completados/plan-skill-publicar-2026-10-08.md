# 08AA-27 — Skill + comandos fáciles para publicar/gestionar inmuebles

Fecha: 2026-10-08. Estado: plan activo, decisiones de ella registradas (§decisiones).
Revisión supervisor-thinking 2026-10-08: VEREDICTO **VIABLE CON RESERVAS**
(reservas en §riesgos; F1+F2-local AUTORIZADOS PARA EJECUTAR, push real a prod
con dry-run previo obligatorio).

## Problema y no-goals

Ella quiere publicar diciendo solo **carpeta de fotos + descripción** y que el
resto pase solo: alta, fotos, mejora, publicación y sync local↔prod. Hoy no hay
guía (verificado 2026-10-08 con testigo: el método vive disperso en
`plan-deploy-produccion-2026-09-23.md`, `plan-sync-prod-local-2026-10-08.md`,
completadas 159A-1/199A-5/6/7/259A-1/279A-1/08AA-2/08AA-4 y cabeceras de scripts).

No-goals: no se toca el front admin (ya publica bien), no se genera mejora en
prod (allí solo se SUBEN mejoradas), no MCP (descartado por ella), no abstracción
a núcleo agnóstico (lógica 100 % MN, YAGNI), no deploy de código en este plan.

## Decisiones de ella (2026-10-08)

1. Formato → **skill + scripts** (receta + comandos npm).
2. Prod → **en los dos a la vez**: cada inmueble se publica en local y prod por
   API desde ya (autorización explícita suya para esta escritura externa; cada
   push real lleva dry-run previo + verificación).
3. Token → **autonomía total con pc-control**: yo abro el Chrome-Horacio-debug
   si está cerrado y corro `renovar-cookies.mjs`. Único límite: el **login de
   Google** (yo nunca escribo contraseñas).

## Método real (hechos confirmados)

- Alta: `POST /api/admin/inmuebles` (JWT; `{}` = borrador) → `PUT /:id`
  (reemplazo; `PATCH` da 405) → `PATCH /:id/publicacion {publicado}`.
- Fotos: `POST /api/admin/fotos/upload?inmueble_id=&filename=&origen=&orden=`
  (bytes crudos, magia JPEG/PNG/WebP, tope 10 MiB). Pareo original↔mejorada
  por `orden`. Portada pública = mejorada `orden=0` si existe.
- Mejora: SOLO local (`:3122` + `gemini_worker.py` + `GEMINI_PSID` en
  `frontend/.env.local`; throttle 120 s/foto, tope 40/día). Salud:
  `GET /api/salud`; recarga: `frontend/scripts/renovar-cookies.mjs`
  (códigos: 0 OK, 2 sin puerto debug, 3 login manual, 4 sin cookies).
- Sync hoy: solo `sync-pull.mjs` (prod→local por slug). Sin push.
- Supuestos a validar en F2: prod acepta el mismo `nucleo()` aunque su backend
  sea anterior (degrada `/ficha` 404 como `sync-pull`); `scripts/.env.prod.local`
  ya existe con ambas credenciales.

## Diseño (corregido tras revisión)

- **Un solo CLI** `scripts/inmueble.mjs <publicar|mejorar|push|estado>` en vez de
  3 scripts (evita triplicar auth/api/`nucleo()`; un `--help`). Comparte
  `api()`/`leerEnv()` con `sync-pull.mjs` vía `scripts/lib-api.mjs` nuevo
  (SRP: el CLI orquesta, el lib habla HTTP, cada subcomando es una función).
- **Skill = doc en repo** `Agente/skills/publicar-inmuebles-SKILL.md` (receta
  agente: endpoints, orden, pareo, verificación, fallos típicos). Sin instalación
  global: el protocolo ya me obliga a leer el `Agente/` del proyecto al arrancar.
- **Doc de ella** `Agente/documentacion/operacion/publicar-inmuebles-2026-10-08.md`
  (qué decirme + contrato de entrada + qué le pido y cuándo). Dos audiencias,
  cero duplicación de contenido operativo.
- **Contrato de entrada** (lo que ella me pasa): carpeta con fotos + texto libre
  con título, precio, tipo, operación (venta/alquiler), ubicación. Lo que falte
  va como borrador (`precio=0` = "Precio a consultar") y se lo pregunto junto,
  no foto por foto. Campos allowlist: `tipo` 5 valores, `operacion` 2,
  `estado` 4 (ver `models/inmueble.rs:13-22`).
- **Token autónomo**: cada lote empieza con `GET /api/salud`; si `listo:false`,
  lanzo Chrome-debug por pc-control si hace falta y corro `renovar-cookies.mjs`;
  solo molesto si código 3/4 (login o cuenta). Fotos originales siempre se
  publican primero; la mejora nunca bloquea la publicación.

## Eficiencia / rendimiento (números)

- Escala real: 1–3 inmuebles/semana, 5–15 fotos cada uno. Sin objetivo de carga
  mayor (riesgo abierto si algún día hay lote de 50+: el throttle 120 s/foto +
  tope 40/día manda; `mejorar` corre en background con readiness por polling,
  nunca bloqueando la sesión).
- Push: 1 `POST` + N uploads por destino (~2× fotos). Reutiliza reemplazo por
  `(orden, origen)` como `sync-pull` (idempotente, re-ejecutable).

## Seguridad

- Secretos: solo `scripts/.env.prod.local` y `frontend/.env.local` (gitignored);
  JWT solo en memoria; dry-run nunca imprime tokens. Verificación pre-commit:
  `git diff --stat` + grep de secretos.
- **EXIF/GPS**: las fotos pueden traer ubicación (riesgo privacidad al publicar).
  F2 pela EXIF al importar (rotación incluida) o documenta por qué no; nunca se
  publica una foto sin saber si lleva GPS.
- Validación de entrada: carpeta existente, magia de bytes (no extensión),
  ≤10 MiB por foto, campos contra allowlist; todo fallo = mensaje + qué hacer,
  nunca silencio.

## Riesgos y mitigación

| Riesgo | Mitigación |
|---|---|
| Push simultáneo con backends de distinta versión (prod anterior) | `push` tolera 404 en rutas nuevas (`/ficha`), anota divergencia; el deploy de código la cierra después |
| Slug existe en prod con otro contenido | `push` nunca sobrescribe sin `--sobrescribir`; por defecto crea o informa diff y pide decisión |
| Mejora tarda (120 s/foto) y la sesión expira el contexto | `mejorar` en background + estado reanudable (`estado` reimprime pendiente); publicar no espera mejora |
| Fotos HEIC/iPhone o >10 MiB | preflight las lista y las convierte/comprime o pide reemplazo antes de subir nada |
| Roadmap con cambios ajenos sin commit (marketplace, 2026-10-08) | no commitear `roadmap.md` mezclado; mi commit lleva solo plan+skill+scripts+doc; coordinar cierre con el otro frente |
| pc-control no puede lanzar el Chrome-debug | se corrige en F3 (parte del plan, no deuda silenciosa) |

## Fases (checklist ejecutable)

- [ ] F1 skill + doc de ella (sin código). Verificación: yo simulo la receta en
  seco contra local (solo GETs) + ella confirma que el contrato de entrada le sirve.
- [ ] F2 `lib-api.mjs` + `inmueble.mjs publicar/estado` + strip EXIF. Prueba en
  local con inmueble de prueba (se crea y se borra; limpieza verificada por GET).
- [ ] F3 `mejorar` + token autónomo (salud → Chrome-debug → renovar → reintento).
  Prueba con 1–2 fotos reales en local; si pc-control falla al lanzar Chrome, fix aquí.
- [ ] F4 `push` (dry-run obligatorio + push real a prod autorizado 2026-10-08 +
  verificación conteos + GET público + portada). Primer push = 1 inmueble piloto.
- [ ] F5 gate (`node --check`, `sync-pull --dry-run` intacto, `cargo test` si se
  tocó Rust —no previsto—), commit por fase (archivos explícitos, sin
  `roadmap.md` si sigue mezclado), archivado en `completados/` + lección EXIF.

**SIGUIENTE ACCIÓN**: F1 (AUTORIZADO PARA EJECUTAR: solo docs, cero riesgo).

## DoD

Ella dice carpeta+descripción → inmueble publicado en local Y prod con mejoradas
pareadas, doc escrita, cero secretos en git, gate verde, `estado` reimprime el
inmueble en ambos lados.

## Documentación / entropía

Toca: roadmap (línea 08AA-27, ya registrada), este plan, `Agente/skills/…SKILL.md`
(nuevo), `Agente/documentacion/operacion/…md` (nuevo), `scripts/inmueble.mjs` +
`lib-api.mjs` (nuevos), `completados/tareas-2026-10-08.md` al cerrar, lección
EXIF si aplica. Nada fuera de `MN-Inmobiliaria/`.
