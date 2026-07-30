# Lecciones aprendidas

## 2026-05-08 — Core editor-agnostico en extensiones
- Para extraer un core real no basta cambiar tipos: hay que eliminar imports indirectos de servicios del editor, como `configService`, `vscode.workspace` o registries que lean settings globales.
- Si una regla aun necesita workspace/watchers, aislarla como callback/adaptador permite avanzar el core sin romper el provider existente.
- Los reportes y scanners deben recibir datos y providers como parametros; escribir archivos, abrir documentos y escuchar watchers pertenece al adaptador, no al core.
- Las pruebas unitarias con mocks de VS Code no garantizan que una CLI arranque en Node puro; despues de compilar hay que ejecutar el JS real y buscar imports indirectos de `vscode`.

## 2026-05-10 — LSP y lint como cierre de arquitectura
- Un LSP fino debe importar core y adaptadores de transporte, no la CLI; si CLI y LSP comparten defaults, moverlos a `core/config.ts` evita drift silencioso.
- Smoke stdio real debe buscar `textDocument/publishDiagnostics` y un `ruleId` esperado; compilar no prueba que el entrypoint LSP no este ejecutando codigo CLI.
- Activar lint tarde puede revelar errores de regex antiguos. Corregir escapes redundantes es bajo riesgo; patrones Unicode compuestos intencionales necesitan excepcion local documentada.
- Si se agregan fixtures `.tsx` fuera de `src`, `tsconfig.json` debe declarar `include` explicito; si no, `tsc` intenta compilar fixtures fuera de `rootDir` y crashea antes de ejecutar tests reales.

## 2026-07-29 — Documentación canónica y planes ejecutables
- Mezclar visión, arquitectura, identidad, auditoría y tareas en varios planes crea fuentes de verdad competidoras aunque cada documento sea correcto por separado.
- El roadmap debe responder únicamente qué sigue y qué lo bloquea; los contratos viven en manuales y la secuencia detallada en un solo plan maestro.
- Un plan ejecutable necesita checklists, dependencias, gates y criterio de salida por bloque; una lista narrativa de fases no basta para trabajar una por una.
- Los estados visuales, editoriales, comerciales y de papelera deben documentarse como ejes distintos antes de diseñar API o UI; un `status` único produce contradicciones y fugas.
- Archivar planes históricos evita que decisiones superadas —Admin monolítico, uploads públicos, fuentes pixel o productos ligados a artículos— reaparezcan durante la implementación.
