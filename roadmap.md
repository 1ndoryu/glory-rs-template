Objetivo: crear un template de Rust + Ts React + OpenAPI + Codegen + Clippy nivel paranoia, para crear sitios web, un solo repositorio, con una pagina sencilla de ejemplo, con el gitignore configurado y todo listo para empezar a desarrollar, adelante a todo, comandos, documentación, he creado https://github.com/1ndoryu/glory-rs para lo subas alli, se tiene que pdoer crear varias ramas para varios sitios y que no haya problema de cambiar a un sitio a otro cambiando de rama, aplica todo lo que por defecto vendría siendo optimización, seguridad, rendimiento, experiencia de desarrolo, etc, de base de datos tengo corriendo postgres aca localmente.

## Estado: ✅ Template base completado (253A-1)

Ver `Agente/completados/tareas-2026-03-25.md` para detalles.

## Stack implementado

| Capa | Herramienta |
|------|-------------|
| Framework web | Axum 0.7 |
| OpenAPI | utoipa 4 + utoipa-swagger-ui 7 |
| Serialización | serde |
| Base de datos | SQLx 0.8 (PostgreSQL) |
| Migraciones | SQLx migrate |
| Validación | validator 0.18 |
| Variables de entorno | dotenvy |
| Logging | tracing + tracing-subscriber |
| Errores | thiserror 2 |
| Auth | jsonwebtoken + argon2 |
| CORS | tower-http |
| Linter | clippy (deny all + warn pedantic) |
| Frontend | React 18 + TypeScript + Vite |
| State | React Query + Zustand |
| Codegen | Orval 8 (reemplaza openapi-typescript-codegen) |

## Pendientes

(Sin pendientes.)

## Deploy mn-inmobiliaria.com (239A-1, en curso 2026-09-23)

- Plan: `Agente/planes/plan-deploy-produccion-2026-09-23.md`.
- Monorepo: `INMOBILIARIA/*` → `frontend/`; servir `STATIC_DIR` + fallback SPA;
  CORS prod solo dominio; `Dockerfile.rust` multi-stage (`VITE_API_URL` build-arg).
- Fase 0 cerrada 2026-09-23: commits receta+mejora (INMOBILIARIA `5a113f5`,
  `70a6361`) y suscriptor+plan (`6cd8a90e`, `3ac8ca46`); `.env` a
  `glory_backend_inmobiliaria`; gate local verde (tsc, vite build, fmt, check,
  clippy `-D warnings`, test 15/15).
- Mejora IA queda solo-local (prompt Ultra HD + descarga `=s0` validados);
  cookies fuera del VPS; sync prod→local solo lectura.
- Fase 1 cerrada 2026-09-23 (`39f383de`+`4c9f1d45`): humo local `/`+fallback SPA 200,
  `/api/health` 200, 11 publicados. En prod manda el `Dockerfile.rust` del template del
  manager (no el del repo): 239A-2 (`944bacf8`) cambió `glory-agent` a git-dep pineado
  `f2f19e7` tras el fallo `failed to read /glory-agent/Cargo.toml`.
- Fase 3+4 hechas 2026-09-23: manager recompilado (1.0.0), sitio `inmobiliaria`
  (`as0scgwg44wkkkccgwcwg8w0`) creado. `sync-env push` bloqueado (el manager exige claves
  Stripe que el proyecto no usa; mejora pendiente a la herramienta, sin dummies).
- Incidencia 2026-09-24 CERRADA: `Servidor no disponible` con API sana = `VITE_API_URL`
  sin hornear (bundle con `127.0.0.1:3000`). Fix herramienta (`d908b94` template
  `ARG VITE_API_URL`, `3008e65` push `--only` exime trio Stripe) + push var + rebuild
  6/6 OK; bundle `index-EZwYRSQX.js` con dominio; API `total=11` fotos `218`.
  Detalle en plan (seccion incidencia) y `Agente/completados/tareas-2026-09-24.md`.
  Pendiente del usuario: recargar https://mn-inmobiliaria.com y confirmar lista visible.
- Fase 5 COMPLETADA 2026-09-24: dump data-only + `run-sql --file` (el `import` del
  manager es solo-WP) → 11/218/2; 218 fotos vía `POST /api/admin/fotos/upload` (el
  manager no tiene push a volúmenes); admin rotado a password nueva generada,
  `import@example.com` eliminado.
- Fase 6.3 E2E COMPLETADO 2026-09-24: 11 publicados, fotos HD 200, login admin, receta
  PUT→pública→revertida, solicitud POST→admin→borrada, WhatsApp horneado.
  Estado prod: 11 inmuebles, 218 fotos, 0 solicitudes, 1 user.
- Requiere del usuario: nueva password admin para la rotación de Fase 5.4 (o confirmar
  mantener la actual / que la genere yo).

## Optimizacion PageSpeed movil + SEO profundo (249A-1, completada 2026-09-24)

- Cerrada y desplegada: ver `Agente/completados/tareas-2026-09-24.md` y plan en
  `Agente/planes/completados/plan-optimizacion-pagespeed-seo-2026-09-24.md`.
- Restos abiertos: F8 SEO profundo (h1/alt/canonical/JSON-LD `ItemList` si falta);
  rerun PageSpeed movil (la API devolvia 429 por cuota).
- 249A-3 (2026-09-24, desplegada): preload del heroe muerto eliminado
  (`PresentacionCaja` no se renderiza; 502 KB que competian con JS/CSS) + `cn`
  local (`clsx`+`tailwind-merge`) en vez del paquete `cn`; JS 192.73 KB gzip.
  Ver `Agente/completados/tareas-2026-09-24.md`.
- Requiere del usuario: confirmar que https://mn-inmobiliaria.com muestra la lista
  y correr PageSpeed movil para el puntaje final.
- 249A-4b+249A-4d (2026-09-24, desplegadas commit `c8ae5341`): thumbs de tabla
  320→160 (`min160-<uuid>.jpg`, legacy `thumb-` con backfill+borrado al servir)
  + `llms.txt` y `/.well-known/ai-catalog.json` dinámicos desde el backend
  (11 entries, URNs saneadas, validados con el tester oficial ARD: 0 errores);
  prebuild `generar-llms.mjs` y `public/llms.txt` eliminados; `lighthouse`,
  `@radix-ui/*` y `shadcn` desinstalados del front. Verify prod: `/llms.txt`
  200 6092 B, ai-catalog 200 11 entries, thumb `min160-` 200 jpeg, health 200.
  Ver `Agente/completados/tareas-2026-09-24.md`.
- 249A-4a+249A-4c (2026-09-24, desplegadas commit `80b6ba2e`): fallback
  metric-matched `Söhne-relevo`/`Kräftig→Arial Bold` en `src/index.css`
  (CLS footer 0.195), primera tarjeta `eager`+`fetchpriority=high` resto
  `lazy`, `Agentmap:` en `robots.txt` + `<link rel=ai-catalog>` en
  `index.html` (4/4 navegación agéntica), 5 overlays públicos tras
  `React.lazy` (`modal-filtros 5.16kB, detalle 6.45kB, publicar 7.67kB,
  chat 11.18kB, dialog 106.97kB gzip 35.70kB`). Verify prod: `robots.txt`
  con `Agentmap:`, `/` con `rel=ai-catalog`, bundle nuevo
  `index-D8vNccQi.js` + chunks `inmueble/plantilla-publicidad`.
  Ver `Agente/completados/tareas-2026-09-24.md`.
- 249A-5 (2026-09-24, desplegada commit `6cbe0134`): PSI escritorio
  79/100/100/92 + 3/4 agéntica → `Agentmap:` fuera de `robots.txt`
  (directiva desconocida para Google, SEO 92→100; descubrimiento solo
  vía `<link rel=ai-catalog>`), 5 overlays `lazy` con pestillo de montaje
  (`useMontarAlAbrir`: montan al primer uso, fuera de la ruta crítica,
  conservan animación de cierre), esqueleto de 10 filas `h-[100px]` en
  vez de `Cargando…` + conteos de píldoras con ancho fijo + reserva de
  alto de paginación (CLS 0.484→~0). Verify prod: `robots.txt` sin
  `Agentmap:`, bundle nuevo `index-Br2UKVtj.js` solo con preloads
  estáticos (`inmueble`, `plantilla-publicidad`). Ver
  `Agente/completados/tareas-2026-09-24.md`.
- Restos abiertos de 249A: F8 SEO profundo, rerun PageSpeed escritorio y
  móvil tras 249A-5 (cuota API 429 el 2026-09-24; último manual escritorio
  79/100/100/92 + 3/4, móvil 88/100/100/100 + 3/4), y el sub-chequeo
  agéntico que sigue en 3/4 (pendiente: nombre del chequeo fallido
  expandiendo la sección en pagespeed.web.dev).

## Receta publicitaria en el servidor (229A-2, en curso 2026-09-22)

- El modal público genera la publi con receta automática; lo configurado en
  admin vive solo en `localStorage` del admin y nunca llega al público.
- Backend: columna `inmuebles.receta JSONB` (migración `...12`), struct
  `RecetaPublicidad` validada (formato allowlist, índices >= 0), `PUT`
  la fija, el `GET` público la expone.
- Front (repo `INMOBILIARIA`): `guardar` hace PUT con la receta; el modal
  público usa `i.receta` (servidor manda, local queda de reserva).

## Notas

- Configurar coolify-manager-rs para desplegar proyectos Rust (repo separado)
- Prioridades: 1. Velocidad desarrollo, 2. Decisiones futuras, 3. Rendimiento, 4. Seguridad, 5. Popularidad, 6. Facilidad, 7. Docs, 8. Compatibilidad, 9. Flexibilidad, 10. Escalabilidad

## F5 chat visitante con IA (169A-1, en curso 2026-09-16)

- Plan: `Agente/planes/plan-chat-inmobiliaria-2026-09-16.md`.
- Backend: `glory-agent` por path, migracion `agent_*`, rutas
  `/api/agent/*` (WS + REST + historial), PromptConfig inmobiliario.
- Front (`INMOBILIARIA`, repo hermano): widget en `src/features/publica/`
  montado en `PaginaPublica`, tokens de `disenno.ts` (sin redondeados,
  sin sombras, Söhne).
- Alcance v1: sin tools (la ejecucion de tools es F6 en `glory-agent`).
- Requiere del usuario: fijar `OPENCODE_GO_API_KEY` y `AGENTE_CONTACTO`
  reales en `MN-Inmobiliaria/.env` (sin key el chat persiste y reenvia
  en realtime pero la IA no responde).

## Publicar mi inmueble: solicitudes pendientes (169A-2, en curso 2026-09-16)

- Plan: `Agente/planes/plan-solicitudes-2026-09-16.md`.
- Backend: tabla `solicitudes`, subida publica de fotos (reutiliza
  validacion magic-bytes + 10 MiB), `POST /api/public/solicitudes`,
  `POST /api/public/solicitudes/fotos`, `GET/PATCH /api/admin/solicitudes`
  (JWT). Validacion basica de email/telefono.
- Front (`INMOBILIARIA`, repo hermano): `ModalPublicar` cuadrado con
  borrador persistente; las fotos ya subidas son claves (no pesan).
- Sin UI admin de revision (pendiente explicito para despues).

## Chat con atencion humana + WhatsApp + config admin (169A-3..6, 2026-09-16)

- Plan: `Agente/planes/completados/plan-chat-atencion-2026-09-16.md`
  (cerrado 2026-09-16; diseno contra Nakomi como referencia
  solo-lectura).
- 169A-3 (repo `glory-agent`): loop de tools F6 + gate IA real
  (`ai_enabled` + ciclo `escalated` se hacen cumplir) + `0002_config.sql`
  + persistencia sesiones/config + tests. Nucleo agnostico.
- 169A-4 (este repo) [x] 2026-09-16: 5 tools (`buscar_inmuebles`,
  `detalle_inmueble`, `registrar_contacto`, `datos_contacto`,
  `escalar_a_humano`) + worker WhatsApp (outbox, poll 15 s) + rutas staff
  `/api/admin/agent/*` (JWT) + publicas `/api/agent/info` y
  `/sesiones/:id/contacto` + hub compartido. Sin seeds: claves opcionales
  con defaults en codigo. Gate verde (fmt/check/clippy/test 11 lib OK);
  humo BD rama OK (tools contra esquema real, staff toma hilo, config
  roundtrip, worker marca `failed` sin gateway, 2 tests HTTP OK).
- 169A-5 (repo `INMOBILIARIA`) [x] 2026-09-16, commit `fae846e`: admin
  Mensajes (lista + hilo + responder + tomar/soltar IA) y Config chat
  (prompt_extra, telefonos, kill-switch). `oxlint` 0 errores, `npm run
  build` verde, dev 200. Incluye frente visitante 169A-1 (cliente, canal
  WS+REST, hook, ventana).
- 169A-6 (repo `INMOBILIARIA`) [x] 2026-09-16, commit `fa745e0`: tarjeta
  contacto/escalado en el widget (`pedirInfo` + `enviarContacto`,
  auto-apertura si pide humano) + E2E en vivo contra este backend
  (health/messages/history/info/contacto OK, staff 401 sin token).
- Requiere del usuario: `GLORY_ALERT_GATEWAY_URL` + numero admin
  (`whatsapp_admin`); `OPENCODE_GO_API_KEY` + `AGENTE_CONTACTO` siguen
  pendientes de 169A-1.

## Centro de IA: GloryAPI + OpenCode Go via backend (199A-1/199A-2, en curso 2026-09-19)

- Plan: `Agente/planes/plan-ia-central-2026-09-19.md`.
- 199A-1 (este repo): `src/handlers/ia.rs` con `GET /api/admin/ia/estado`,
  `PUT /api/admin/ia/config`, `POST /api/admin/ia/probar`,
  `POST /api/admin/ia/completar` (JWT); config en `agent_config`
  (`ia_activo`, `ia_hab_*`, `ia_check_*`); claves solo en `.env`
  (`GLORY_API_URL`, `GLORY_API_KEY`, `OPENCODE_GO_API_KEY`).
  Codigo completo 2026-09-19: `fmt` + `check` + `clippy -D warnings` +
  `test --lib` 14/14 (3 nuevos `handlers::ia::pruebas`).
  Pendiente humo HTTP: el dev-server vivo (otra sesion) usa binario anterior;
  reiniciarlo y probar `estado` + `completar` antes de 199A-2.
- 199A-2 (repo `INMOBILIARIA`): redaccion y copy via `completar` (activo
  manual + fallback); pestana `ia` en Config con estado, habilitar/elegir
  activo y probar por proveedor.
- Requiere del usuario: `GLORY_API_KEY` real en `MN-Inmobiliaria/.env`
  (sin ella GloryAPI queda `sin-clave` y todo va por OpenCode Go).

## Sesion admin de 10 años (199A-3, en curso 2026-09-19)

- El JWT ya duraba 365 dias con `JWT_SECRET` estable en `.env` (gitignored):
  la sesion solo caia por secreto rotado o expiracion real.
- 199A-3: `generate_token` pasa a 10 años. Tras arrancar el servidor nuevo,
  salir y entrar una vez para que el token traiga la expiracion larga
  (los tokens viejos conservan la suya).

## Editar inmueble 405: front mandaba PATCH, contrato es PUT (199A-4, 2026-09-19)

- `actualizarRemoto` enviaba `PATCH /api/admin/inmuebles/:id` pero el router
  solo acepta `PUT` (asi desde 159A-2 + utoipa `put`): 405 preexistente, no
  causado por los reinicios de 199A-1/199A-3.
- Fix en repo `INMOBILIARIA`: `PUT` en `actualizarRemoto` (manda el objeto
  entero = reemplazo, semantica PUT correcta). `fijarPublicado` sigue PATCH
  (su ruta si es `patch`). Verificado con PUT+JWT a id inexistente → 404.

## Fotos 404 al editar: sincronizarFotos borraba antes de re-descargar (199A-5, 2026-09-19)

- Ante CUALQUIER cambio de fotos (anadir, quitar, reordenar, elegir
  principal), `sincronizarFotos` borraba TODOS los originales (fila + archivo
  en disco via `delete_foto`) y luego intentaba re-descargar las conservadas
  del servidor para re-subirlas: 404 inevitable ("No se pudo leer la foto
  para subirla") y el inmueble quedaba sin fotos (perdida real; los datos
  texto si se guardaban porque el PUT va antes).
- Fix en repo `INMOBILIARIA` (commit `c0037dc`): leer los bytes de TODAS las
  deseadas ANTES de borrar; si alguna falla no se ha borrado nada. Servidor
  verificado sano (fotos existentes responden 200, 92 archivos en
  `uploads/`); causa 100 % front, no backend.
- Pendiente usuario: revisar la propiedad editada y re-subirle las fotos
  perdidas; recargar `:5200` con `Ctrl+F5`.

## Principal + mejoradas en el frente público (199A-6, 2026-09-19)

- Dos causas front (repo `INMOBILIARIA`, commit `7e245b2`; backend intacto,
  la API pública ya devuelve originales + mejoradas emparejadas por `orden`):
  1. La galería del detalle público (`modal-detalle-publico.tsx`) mostraba
     `i.fotos` = solo originales; las mejoradas solo aparecían en la portada
     de la tarjeta. Ahora usa `fotosVisiblesDe` (mejorada por posición si
     existe, si no el original; `portadaDe` = primera visible).
  2. Al reordenar/elegir principal, `sincronizarFotos` re-creaba los
     originales con `orden` nuevo pero dejaba las mejoradas con el viejo: el
     pareo por `orden` quedaba roto y la portada mostraba la mejora de otra
     foto. Ahora las mejoradas se descargan antes de borrar y se re-suben
     siguiendo a su original (misma garantía sin-pérdida de 199A-5); la de un
     original eliminado se descarta.
- Pendiente usuario: `Ctrl+F5` en `:5200` y comprobar Townhouse/Orquídea
  (editadas hoy 17:07): si alguna portada sigue mostrando la mejora de otra
  foto (pareo roto anterior, irreparable por contenido), avisar cuál para
  limpiar sus mejoradas y regenerarlas pareadas.

## Pareo roto en Orquídea: reparado por contenido (199A-7, 2026-09-19)

- La usuaria elegía principal en admin (se guardaba bien) pero en público
  seguía viendo otra foto: el pareo `orden` original↔mejorada de Orquídea
  estaba roto por un reorden anterior (las mejoradas conservaban el `orden`
  viejo). Demostrado por hash perceptual (dHash, distancias 0-8 = misma
  escena): mejorada 0→original 8, 1→9, 2→10, 3→0, 4→11, 5→12. La portada
  (mejorada 0) mostraba la mejora del original 8, no de la principal.
- Townhouse comprobada igual: sus 6 parejas correctas, no se tocó.
- Reparación solo-datos vía API (sin código ni reinicio): subidas primero
  las 6 mejoradas con `orden` correcto (`origen=mejorada`, mismos bytes),
  luego borradas las 6 viejas por id, verificado por contenido (las 6
  parejas distancia ≤8). Total Orquídea: 15 originales + 6 mejoradas = 21.
- El fix 199A-6 impide futuros despareos al reordenar desde el front.
- Seguimiento propuesto (no urgente): endpoint `PATCH /api/admin/fotos/:id`
  con `{orden}` para reordenar sin borrar+re-subir (menos churn y sin
  cambiar urls/ids que cachea el store local de mejora).

## Siguiente bloque

(279A-1 cerrada 2026-09-27 ~17:10: ver `Agente/completados/tareas-2026-09-27.md`.)

## 279A-2 — Agente WhatsApp MN (pendiente: número + QR de usuaria)
- Plan: `Agente/planes/plan-whatsapp-2026-09-27.md`.
- Gateway Baileys en PC usuaria, cerebro `opencode serve` local, sin API de pago.
- Fases: F1 envío manual → F2 lectura+bandeja `/admin` → F3 borradores →
  F4 auto-respuesta + imágenes + audios (Whisper local). Mismo agente
  (glory-agent: canal en sesiones, outbox con destino/texto/media).
- Requiere de usuaria: número WhatsApp + 2 min para escanear QR. Sin código hasta entonces.

## 279A-3 — /ask cuestionario de ficha (F1 verificado local 2026-09-27; pendiente: respuestas usuaria)
- Plan: `Agente/planes/plan-ask-2026-09-27.md`.
- Página privada con login existente; preguntas por tipo (piso solo
  apartamento), condicionales, dinámicas IA opcional; `precio_minimo`
  privado con frontera en API; tabla % + semáforo en admin; alimenta
  chat web + WhatsApp.
- Dudas resueltas 2026-09-27 (commit `40244fb5`): mínimo insinuable sin
  cifras, piso = apartamento + townhouse, dinámicas desde F1, aviso sin
  bloquear, `/ask` con sesión admin.
- F1 implementado y verificado local (commit `57f8410f`): migración
  `extras` + `precio_minimo`, `GET/PUT /api/admin/.../ficha`, frontera
  pública (`extras={}` sin `precio_minimo`, 422 a array), login `/ask`
  centrado, lista + cuestionario (respuesta → 14% en DB y UI), `tsc` 0,
  `cargo test --lib` 18 passed. Dato de prueba limpiado.
- F2 mejora /ask verificada local (commit `e119b9c4`, `tsc` 0): sin
  `urbanizacion_zona` en `extras`; paso inteligente ubicación+residencia
  (rellena columnas vía PUT, confirma lo existente); numéricos
  condicionales por tipo (m² salvo terreno, parcela en casa/terreno,
  puestos salvo terreno, solo si faltan); orden aleatorio por sesión;
  botón Anterior (deshacer/corrige); No sé en Sí/No; numéricos con unidad,
  stepper y error visible; página centrada angosta con foto de portada +
  descripción; sección Ficha /ask editable en el modal admin (pisa
  `extras`/`precio_minimo`; ubicación/medidas ya se editaban). Flujo
  probado en navegador sin ensuciar datos (ubicación restaurada,
  `extras={}` intacto, guardado admin sin cambios = no-op).
- 279A-4 sin lista /ask verificado local (commit `ecb71411`, `tsc` 0): al
  entrar elige sola una propiedad con algo que preguntar (candidatas en
  aleatorio, ficha una a una, primera con pendientes); sin lista visible,
  solo preguntas; botón Otra propiedad (excluye la actual) + Otra pregunta
  aleatoria al terminar; panel ¡Todo al día! con Revisar de nuevo;
  `estado-ask.ts` separa estado/updaters (hook 117 líneas). Navegador:
  entrada directa a pregunta, Otra propiedad salta de Calas Suites a
  Arivana, recarga cae en Riberas del Caroní, solo lecturas.
- 279A-5 verificado local (`tsc` 0): numéricos nombran el tipo («¿Cuántos
  m² tiene el apartamento?»; terreno: «¿Cuántos m² tiene el terreno?»);
  precio arriba («35.000 € · venta», «Precio a consultar» si es 0);
  lenguaje neutral en UI/comentarios/plan (no es para el dueño): mínimo
  «¿Cuál es el precio mínimo aceptado? (privado, no se publica)».
  Navegador: precio + m² + mínimo confirmados, solo saltos sin guardar.
- Requiere de usuaria: responder el cuestionario en `/ask`.

