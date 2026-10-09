# SKILL: publicar y gestionar inmuebles (MN-Inmobiliaria)

Receta del agente para alta, fotos, mejora, publicación y sync local↔prod.
Fuentes verificadas 2026-10-08 (08AA-27). Si el código cambió, manda el código.

## 0. Contrato de entrada (lo que pide ella)

Carpeta con fotos + texto libre con: título, precio, tipo, operación
(venta/alquiler), ubicación. Opcionales: residencia, habitaciones, baños,
metros, metros_terreno, puestos, descripción larga, estado.
Lo que falte → borrador con defaults y SE LE PREGUNTA JUNTO, no foto por foto.

Allowlist (`src/models/inmueble.rs:14-18`): tipo
`apartamento|casa|local|terreno|townhouse`, operación `venta|alquiler`, estado
`disponible|reservado|vendido|alquilado`. Defaults: apartamento/venta/disponible.
Límites: título/ubicación/residencia ≤500, descripción ≤20000, numéricos ≥0.
`precio=0` = "Precio a consultar". Extensiones: `.jpg .jpeg .png .webp`.

## 1. Atajos (CLI único)

```
node scripts/inmueble.mjs estado --slug <slug>            # reimprime local+prod
node scripts/inmueble.mjs publicar --fotos <carpeta> --datos <json> [--solo-local|--solo-prod] [--borrador] [--sobrescribir] [--dry-run]
node scripts/inmueble.mjs mejorar --slug <slug> [--limite N] [--repetir]   # solo local
node scripts/inmueble.mjs push --slug <slug> [--dry-run] [--sobrescribir]  # local → prod
node scripts/inmueble.mjs verificar [--slug <slug>] [--sin-bytes] [--par N]  # prod↔local, solo lectura (08AA-35 F1)
```

Sin CLI a mano (fallback): los endpoints de §2 con `scripts/.env.prod.local`
(`LOCAL_API`, `LOCAL_EMAIL/PASSWORD`, `PROD_BASE`, `PROD_EMAIL/PASSWORD`).
JWT solo en memoria. `--dry-run` primero, siempre, antes de un push real.

## 2. Endpoints (base local `http://127.0.0.1:3110`, prod `https://mn-inmobiliaria.com`)

Auth: `POST /api/auth/login {email,password} → {token}`. Todo `/api/admin/*`
lleva `Authorization: Bearer`. 401 = login de nuevo.

- `POST /api/admin/inmuebles` → 201 `Inmueble` (con `id`, `slug`). `{}` válido.
- `PUT /api/admin/inmuebles/:id` reemplazo completo. `PATCH` aquí = 405.
- `PATCH /api/admin/inmuebles/:id/publicacion {publicado:bool}`.
- `POST /api/admin/fotos/upload?inmueble_id=&filename=&origen=original|mejorada&orden=N`
  bytes crudos `application/octet-stream` (magia JPEG/PNG/WebP, tope 10 MiB/413).
- `DELETE /api/admin/fotos/:id` → 204.
- `GET/PUT /api/admin/inmuebles/:id/ficha` (ruta `ask.rs`; en prod vieja puede
  dar 404 → se conserva la local y se anota, como `sync-pull.mjs`).
- Público: `GET /api/public/inmuebles`, `GET /api/public/inmuebles/:slug`.

## 3. Orden de operaciones (publicar)

1. Preflight fotos: carpeta existe, magia por BYTES (no extensión), ≤10 MiB,
   HEIC → convertir o pedir reemplazo. Pelar EXIF (GPS) antes de subir.
2. `POST /inmuebles` con el núcleo (mismos campos que `nucleo()` en
   `sync-pull.mjs:85-92` + `publicado` NO va aquí) en LOCAL.
3. Subir originales con `orden` 0..N (`origen=original`).
4. `PATCH publicacion {publicado:true}` en local. Verificar: GET admin (conteo
   fotos) + GET público por slug.
5. `push --slug` a prod: si el slug NO existe → crear + fotos + publicar; si
   EXISTE y difiere → informar diff y pedir decisión (solo `--sobrescribir`
   reemplaza). Verificar igual que en local.
6. Mejora (§4) cuando esté lista → `origen=mejorada` con el MISMO `orden` que
   su original → push de mejoradas a prod. Portada = mejorada `orden=0`.

## 4. Mejora de fotos (solo local) + token autónomo

Cadena: `POST :3122/api/mejora {fotoId, original:dataURL, prompt}` + polling
`GET :3122/api/mejora/:jobId` (cola 1 en vuelo, 120 s/foto, 40/día).
Salud: `GET :3122/api/salud → {listo, detalle, ...}` (`mejora.mjs:569-582`).
Requiere `server:mejora` corriendo (`frontend/`, `MEJORA_PORT` default 3122).

Token `GEMINI_PSID` (`frontend/.env.local`, gitignored, recarga en caliente):
cada lote empieza con salud; si `listo:false` → si no hay puerto `:9223`, el
propio CLI abre el Chrome dedicado (`chrome.exe --remote-debugging-port=9223
--user-data-dir=%LOCALAPPDATA%\ChromeHoracioDebug`); luego corre
`node frontend/scripts/renovar-cookies.mjs`. Códigos: 0 seguir sin
molestar; 2/3/4 → decirle a ella qué hacer (abrir acceso / iniciar sesión /
revisar cuenta). OJO: `listo:true` no garantiza sesión viva — si el worker
falla con `UNAUTHENTICATED`, se renueva igual y se reintenta. Las ORIGINALES
siempre se publican primero; la mejora nunca bloquea la publicación.

Gotchas vencidos (08AA-27, verificados): lo que devuelve Gemini no siempre es
PNG decodificable aunque traiga esa magia — el CLI normaliza TODO a JPG con
PIL antes de subir (el frontend lo tapaba al re-codificar en canvas). El
servidor no duplica `lista` por foto (F21): el CLI usa `fotoId` único por
intento. `--repetir` borra la mejorada vieja del mismo orden antes de subir.

## 5. Gestionar / modificar

- Cambiar datos: `PUT` completo (leer primero, modificar, mandar entero).
- Publicar/ocultar: `PATCH publicacion`. Borrar: `DELETE /inmuebles/:id`.
- Cambiar fotos sin romper pareo: subir nuevas con `orden` correcto y borrar
  viejas por id (nunca borrar antes de tener los bytes, lección 199A-5).
  Reordenar = re-subir con `orden` nuevo + mejoradas seguidoras (lección 199A-6).
- Tras CUALQUIER cambio en local que deba verse fuera: `push --slug` (dry-run).

## 6. Verificación (DoD por inmueble + sync total)

`estado --slug`: mismo conteo de fotos en local+prod, `publicado:true` en ambos,
GET público 200 con portada = mejorada `orden=0` si existe. Ella recarga la
web y confirma visualmente.

`verificar` (sync total, 08AA-35 F1, permanente en el CLI): P1 metadatos
(presencia por slug, núcleo canónico campo a campo, ficha útil sin
`inmueble_id`, publicado, set `origen:orden`, duplicados) + P2 sha256 de
bytes por par con pool (`--par`, default 6; `--sin-bytes` lo salta;
`--slug` acota). Exit 0 limpio / 1 con diferencias / 2 preflight. Solo
login + GETs: cero escrituras. Testigo 2026-10-08: 13/259 vs 13/255 con el
único frente en `mejorada` de `casa-en-venta-en-altos-del-caron`;
2026-10-09 convergencia total por pull prod→local: 13/259 = 13/259 exit 0.

Pull quirúrgico (08AA-35 F2, permanente): `sync-pull.mjs --slug <slug>`
reemplaza núcleo + ficha + publicado + TODAS las fotos del slug con bytes
de prod (siempre dry-run primero). Solo escribe en local, jamás en prod.
Gotchas fijados en código: el parser tomaba la siguiente flag como valor
(`--dry-run --slug` ejecutaba de verdad) — el valor solo se consume si no
empieza por `--`; el borrado es drenaje con relectura (borrar una
`original` arrastra a su `mejorada` + renumera, la lista stale da 404);
respaldo file-level del slug en `C:\tmp` si no hay pg_dump.

## 7. Fallos típicos

405 editar → usaste PATCH, es PUT. 404 foto al re-subir → borraste antes de
descargar. Portada muestra otra foto → pareo `orden` roto (reparar por
contenido como 199A-7). Mejora parada → `GET /api/salud` dice por qué.
Prod sin `/ficha` (404) → backend anterior, se anota y se sigue.
Ruta pública real: `GET /api/public/inmuebles/:slug` (NO `/api/inmuebles/:slug`).
Tras reinicio del backend local, el primer login puede fallar transitorio:
reintentar antes de tocar credenciales.
