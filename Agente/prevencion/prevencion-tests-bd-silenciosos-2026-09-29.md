# Prevención: tests con BD que se saltan en silencio (2026-09-29)

## Caso mínimo
`cargo test --lib` sin `DATABASE_URL` en el entorno: `pool_si_hay()` retorna
`None`, cada test `async` con BD hace `return` temprano y reporta **ok**.
41/41 en 0.01 s sin tocar la BD. Con `DATABASE_URL` real: 5 fallos de verdad
(renombre `sin_tilde`→`sencilla` a medias + CHECK `canal` en test nuevo) y
2.73 s. Tres tests nuevos se "verificaron" dos veces antes de la verificación
real (Fase3-v2).

## Capa responsable
Harness de tests (`mod pruebas` en `chat_tools.rs` y otros): `pool_si_hay()`.

## Detección esperada
- Si `test --lib` termina en <0.5 s con tests `async` de BD en el árbol,
  sospechar salto silencioso: re-correr con `DATABASE_URL` y comparar tiempos.
- Señal barata: `cargo test --lib buscar_ 2>&1 | ... finished in X` — X≈0.00 s
  por test con BD = no tocó BD.

## Regla
Correr tests con BD siempre vía `node scripts/run-with-db.mjs test --lib`
(wrapper del proyecto: fija `DATABASE_URL` + `CARGO_TARGET_DIR` por rama) o
con `DATABASE_URL` exportado en el shell. Nunca dar por verde una suite
`--lib` que corra en centésimas con tests de BD presentes.

## Referencia
Roadmap Fase 3 v2; `Agente/completados/fase3-bateria-resultados-2026-09-29.md`
(§Batería v2, H9).
