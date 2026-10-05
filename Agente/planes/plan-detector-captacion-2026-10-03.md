# Plan 03AA-5 — Detector de captación en Marketplace (plugin unificado)

## Objetivo
Detectar inmuebles de **contacto directo** (particulares) en Marketplace para
captarlos, con **riesgo 0 de baneo**: sistema semimanual donde la usuaria abre
lo que haya que abrir y el plugin guarda lo que ella ve. Cero automatización
contra Meta. Vive en el repo `plugins-opencode` junto al asistente (03AA-3),
orden interno por plugin, todo bajo Sentinel.

## Riesgo 0 (reglas duras)
Solo-lectura del DOM de páginas que **ella** abrió (aviso, perfil, búsqueda);
cero clics, cero auto-scroll masivo, cero navegación automática, cero login
automatizado, red solo hacia el backend local. Si Meta cambia el markup, el
lector lo registra y se recalibra (igual que el float).

## Pipeline por página vista
1. **Detectar qué es:** página de aviso vs perfil vs búsqueda (por URL+DOM).
2. **Extraer aviso:** id FB, url, título, precio, descripción, ubicación,
   publicador (nombre + url perfil), fecha, urls de fotos (no se descargan).
3. **¿Es inmueble?** (no todo lo publicado lo es): heurística
   (precio + m²/hab/ubicación/zona conocida) + `jev` como juez; lo dudoso se
   guarda como `revisar`, nunca se descarta solo.
4. **¿Particular o asesor?** patrones en descripción (asesor, inmobiliaria,
   agencia, nombre comercial, "contáctame al equipo") + si ella abre el perfil:
   nº de avisos publicados (varios = asesor). Sin evidencia = `desconocido`.
5. **Guardar todo en backend** (tablas `mp_avisos`, `mp_publicadores`,
   `mp_busquedas`): descripciones completas, fotos solo url; las fotos se
   descargan a disco **solo** si el candidato se aprueba.
6. **Teléfono:** regex venezolano en descripción; si FB lo oculta, campo
   manual en la ficha (truco incógnito lo hace ella y lo pega).
7. **Dedupe:** por id FB; re-visitas actualizan, no duplican.
8. **Búsquedas que ella ejecuta** se guardan (texto + filtros + N + urls) para
   aprender repertorio de cacería.

## Cola de revisión humana
Candidatos = particulares (+desconocidos con buena pinta). Nueva pestaña
"Captación" en admin MN: aprobar (descarga fotos, ficha completa) / descartar
(con motivo, alimenta patrones). Convertir a inmueble real queda fuera (fase
futura).

## Fases
C1 lector pasivo + extractor del aviso; C2 clasificador inmueble/no con jev;
C3 particular/asesor + perfil; C4 tablas backend + dedupe; C5 pestaña
Captación + aprobar/descartar; C6 búsquedas guardadas. Cada fase: tests +
viva riesgo-0 (ninguna acción ante Meta).

## Estado
Plan escrito 2026-10-03. Duda abierta: qué es `jev` exactamente y cómo se
invoca (ver nota a usuaria). Próximo: C1 al aclararse.

## Gate y DoD
Sentinel PASS en el plugin; backend MN con su gate; verificación viva con la
usuaria abriendo avisos reales; ni un clic automático en ninguna fase.
