# Prevención: `path-join-sin-canonicalize` no reconoce guardia léxica (2026-10-08)

## Caso mínimo
`src/services/turno.rs:161` (`ruta_clave`, [08AA-3 B3]): `Path::new(dir).join(rel)`
donde `rel` solo sobrevive si TODOS sus componentes son `Component::Normal`
(rechaza `..`, rutas absolutas y prefijos). Sentinel sigue marcando
`path-join-sin-canonicalize` porque la regla exige textualmente
`canonicalize() + starts_with()` en el fichero.

## Capa responsable
Regla `path-join-sin-canonicalize` de Sentinel: heurística textual por fichero,
ciega a mitigaciones por allowlist de componentes.

## Detección esperada
- Un `join` precedido de `components().all(matches!(c, Component::Normal(_)))`
  es traversal-imposible por construcción: sin `..` no hay escape, sin
  absoluto no hay ancla fuera, sin prefijo no hay `C:\`/UNC.
- `canonicalize()` NO es superior aquí: falla si el fichero aún no existe en
  disco y añade IO + TOCTOU entre chequeo y lectura. La guardia léxica pura
  es la mejor opción arquitectónica (no el camino fácil para callar el linter).

## Regla
No reescribir una guardia léxica correcta a `canonicalize()` solo para
satisfacer la regla. Documentar el falso positivo aquí y referenciarlo desde
el roadmap; si la regla evoluciona, que detecte el patrón
`components().all(... Normal ...)` antes de un `join` como mitigación válida.

## Referencia
Roadmap 08AA-3 B3; `src/services/turno.rs` (`ruta_clave`,
`describir_y_anexar`, `transcribir_y_anexar`).
