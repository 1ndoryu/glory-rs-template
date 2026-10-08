# FP Sentinel: regex global `/g` reutilizada entre scopes oculta hallazgos (lastIndex)

- **Fecha:** 2026-10-08. **Origen:** 08AA-26 (fix `path-join-sin-canonicalize`).
- **Caso mínimo:** `PATRON_JOIN = /\b\w+\.join\(/g` se usa primero como pre-chequeo
  (`if (!PATRON_JOIN.test(texto))`) y luego en el bucle principal. Si un scope
  queda eximido (ej. contiene `Component::Normal`) después de que el pre-chequeo
  avanzó `lastIndex`, el siguiente scope del mismo fichero empieza a buscar a
  mitad de texto → un `join` real posterior no se reporta (falso negativo
  orden-dependiente: el mismo fichero da 1 o 0 hallazgos según haya o no un
  scope eximido antes).
- **Capa responsable:** `glory-sentinel/src/analyzers/rustReglasNuevas.ts`
  (`detectarPathJoinSinCanonicalize`). Aplica a cualquier detector que reutilice
  una regex con flag `/g` entre scopes o entre pre-chequeo y análisis.
- **Corrección:** `PATRON_JOIN.lastIndex = 0` antes de cada uso (pre-chequeo y
  bucle). Regla general: con `/g`, resetear `lastIndex` antes de cada `.test()`/
  `.exec()` sobre un texto distinto, o no usar `/g` si solo se necesita
  existencia.
- **Detección esperada:** test determinista `[08AA-26] determinista: un scope
  eximido antes no oculta un join posterior` en
  `src/test/suite/batch149A1.test.ts` (fixture `un hallazgo por linea` con scope
  eximido delante → debe dar 1, antes del fix daba 0).
- **Referencia:** `roadmap.md` 08AA-26; `Agente/completados/tareas-2026-10-08.md`
  entrada 08AA-26 (parcial).
