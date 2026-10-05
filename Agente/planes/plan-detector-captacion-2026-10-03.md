# Plan 03AA-5 — Detector de captación en Marketplace (plugin unificado)

> Revisado por reto hostil 2026-10-03: veredicto REPLANTEAR. Este documento
> ya incorpora las correcciones exigidas. Estado: **bloqueado hasta C0**.

## Objetivo
Detectar inmuebles de **contacto directo** (particulares) en Marketplace para
captarlos. Sistema semimanual: la usuaria abre lo que haya que abrir, el
plugin guarda lo que ella ve. Riesgo **mínimo** ante Meta (no existe riesgo 0
real; ver §Riesgo). Vive en el repo `plugins-opencode` junto al asistente
(03AA-3), todo bajo Sentinel.

## Alcance / no-alcance
Sí: leer avisos/perfiles/búsquedas que ella abre, extraer, clasificar
(inmueble/no, particular/asesor/desconocido), persistir, cola de revisión
humana con aprobar/descartar. No: clics, auto-scroll, navegación automática,
login automatizado, publicación, mensajes, convertir aprobado a inmueble del
sistema (fase futura), scrapear a escala.

## Riesgo (mínimo, no 0)
Abrir un aviso lo guardas automáticamente para gestionarlo después: ese es
el flujo, sin fricción. Corrección 2026-10-03 (usuaria): si ella abre cada
aviso a ritmo humano, ese ritmo YA es el límite — leer el DOM no genera ni
una petición extra hacia Meta (todo va al backend local, invisible para
ellos). Se elimina el throttle por hora/día en la vía pasiva; en su lugar el
admin muestra `avisos/día` como métrica visible, sin bloquear. Reglas que sí
quedan: solo-lectura del DOM de páginas abiertas por ella; red desde
content-script prohibida (todo vía `background`); cero clipboard automático;
cero clics/navegación automática/login automatizado, hoy y en toda fase
futura (cualquier propuesta que los pida se rechaza en diseño). Nota honesta:
el riesgo real no es leer, es la escala automatizada — y esa no existe aquí.
El teléfono oculto se obtiene solo por vía manual (ella lo ve y lo pega).

## Adenda conjunta 03AA-3 / 03AA-5 (repo `plugins-opencode`)
Un solo repo, un solo `watch`/lector en `nucleo/` (agnóstico: observa,
extrae texto, firma). `asistente-respuestas/` y `detector-captacion/` son
plugins que consumen el núcleo; cada uno su adaptador y tests. Una sola
caché de firmas en backend MN. Endpoints separados: `/mp/respuestas/*` vs
`/mp/captacion/*`. Orden: primero existe E1 (repo + Sentinel + núcleo),
después C1. Sin E1 no hay C1.

## C0 — Corpus + dataset (pre-requisito, sin código de producto)
1. Corpus DOM: 5 avisos + 2 perfiles + 1 búsqueda guardados como HTML
   anonimizados + URLs canónicas + lista de campos obligatorios/opcionales.
   Sin corpus no se escribe extractor.
2. Dataset semilla: 30–50 avisos etiquetados (inmueble/no ×
   particular/asesor/desconocido) con casos trampa (asesor con cuenta
   particular, curioso, no-inmueble con precio). Es la verdad contra la que
   miden C2/C3.
3. Salida C0: `corpus/` + `dataset.csv` + DDL borrador (§Datos). DoD: corpus
   versionado, dataset con ≥30 filas y ≥5 trampas.

## Datos (borrador DDL, a fijar en C0)
`mp_avisos(fb_id UNIQUE PK, url, titulo, precio_num, moneda, descripcion,
ubicacion_txt, publicador_id FK, estado, jev_cache_hash, visto_en,
actualizado_en)`; estados: `nuevo|revisar|candidato|aprobado|descartado`.
`mp_publicadores(fb_perfil UNIQUE, nombre, n_avisos_vistos, veredicto)`.
`mp_busquedas(id, texto, filtros, n_resultados, creada_en)`.
`mp_revisiones(aviso_id, decision, motivo, revisada_en)` (el descartar con
motivo alimenta patrones). Precio: numérico + moneda (no string). Re-visita =
UPDATE + fila de historial de precio (tabla `mp_precios`). Teléfono: columna
separada, normalizado E.164, **enmascarado en admin** (solo últimos 4
visibles; ver completo con clic registrado). Fotos: solo url + `url_caduca_en`
(las CDN de FB firman con expiración; al aprobar se re-intenta descarga y si
caducó se marca `foto_perdida`, no se finge).

## `jev` (verificado n=1 el 2026-10-03, pendiente validar en dataset)
TypeSafe AI en Zen: `POST https://opencode.ai/zen/v1/systemone` con el mismo
`OPENCODE_GO_API_KEY`, `model: jev-1.13-free` (gratis temporal;
`jev-1.13` $0.042/1M in de respaldo). Corre **solo en backend**, nunca en el
plugin. Analiza con la información comercial **completa**: título, precio,
descripción íntegra, ubicación, datos del publicador y nº de avisos en su
perfil; lo único que NO viaja es PII (teléfono, fotos, URLs de perfil) porque
para decidir no hace falta. Una llamada por aviso con 3 preguntas:
`es_inmueble` (noul), `origen` (choice particular/asesor/desconocido),
`viable_captacion` (noul → **% viabilidad = round(p*100)**). Regla de oro:
**viabilidad <60% → `revisar` humano siempre**, aunque todo lo demás cuadre;
`origen` con `confidence<0.7` → `desconocido`. Caché backend por
`fb_id+hash(descripción)` TTL 30 días: re-visitas = 0 tokens. Si jev falla:
heurística + `revisar`. Nada se descarta solo jamás. Umbrales se calibran
contra el dataset en C2/C3 (60% es el piso propuesto por la usuaria).

## Clasificador particular/asesor (anti-ruido)
`particular` exige 2+ señales independientes (p.ej. lenguaje en primera
persona + sin marca comercial + perfil con ≤2 avisos). Señales de asesor:
palabras (asesor/agencia/inmobiliaria/equipo), marca comercial, teléfono
corporativo repetido, perfil con 3+ avisos. Conteo de perfil: solo si ella
abre el perfil (sin auto-scroll; se guarda lo visible). `desconocido` con
tope: más de 50 sin resolver en cola → se pausan nuevos hasta revisar
(cola inundable = fallo). Descartar exige motivo (lista cerrada).

## Teléfonos (spec)
Acepta `+58`, `04xx/0212` con separadores, y ofuscados simples
(`0412 123 4567`, `0412-1234567`); NO adivina `cero cuatro doce` (queda a
campo manual). Normaliza a E.164 `58xxxxxxxxxx`. Quién ve: admin con
enmascarado por defecto.

## Panel en tiempo real ("Radar")
Mientras ella navega, un cuadro visible muestra: **aviso actual** (título,
precio, veredicto particular/asesor/desconocido, **viabilidad %** con color,
qué campos faltan: ej. "sin ubicación", "sin teléfono"); **hoy** (vistos,
candidatos, en revisión, descartados); **cola** (pendientes por revisar).
Fuente: backend (`GET /mp/captacion/hoy` + estado del aviso abierto, polling
5s desde el host; sin websockets nuevos). El % se muestra siempre con su
base ("viable 82% · particular, 2 señales") para que el número sea
auditable, nunca una caja negra. DoD en C4: lo guardado coincide con lo
visto, el % coincide con backend, y "qué falta" detecta ≥3 tipos de hueco.

## Fases reordenadas (cada una con DoD medible)
- C0 corpus + dataset + DDL (DoD arriba).
- C1 lector pasivo + persistencia + dedupe (DoD: 5/5 avisos del corpus
  extraídos campo a campo, re-visita = UPDATE sin duplicar, 0 llamadas de
  red desde content-script salvo a `background`).
- C2/C3 clasificadores contra dataset (DoD: precisión ≥90% inmueble/no,
  `particular` sin falsos-positivos en trampas-asesor, viabilidad <60% siempre
  a `revisar`, % `revisar` total <25%).
- C4 cola "Captación" en admin MN + panel Radar + endpoint stats (DoD:
  aprobar/descartar con motivo, teléfono enmascarado, fotos con estado
  `ok|perdida`, panel con % auditable y ≥3 huecos detectados).
- C5 búsquedas guardadas (DoD: re-visita 0 tokens).
- Cada fase: tests + Sentinel PASS; viva = usuaria abre avisos reales y lo
  guardado coincide con lo visto.

## Estado
Bloqueado hasta C0 (corpus + dataset 30–50 + DDL) y E1 de 03AA-3 (repo +
núcleo). Nada de código de producto antes.

## Gate y DoD global
Sentinel PASS en el plugin; backend MN con su gate; métricas C2/C3 sobre
dataset cumplidas; verificación viva final; ni una acción automática ante
Meta en ninguna fase.
