# Evidencia de la segunda release de Sentinel y VarSense

> Registro reproducible de la auditoría 2026-08-11. Los resultados se obtuvieron en worktrees aislados; los commits todavía no están publicados ni adoptados por el consumidor. Este documento no sustituye `doctor`, `quality:lock` ni el gate posterior a la adopción.

## Identidad de los artefactos

| Herramienta | Release preparada | Commit completo | Padre | Estado remoto |
| --- | --- | --- | --- | --- |
| Sentinel | 0.7.1, tag local `v0.7.1` | `b22c8484fd2334f19a88f930c494091d02942e39` | `0dd9c2130d5aebeff384d1448218cf660e4aef6c` | pendiente de publicación |
| VarSense | 2.2.1, tag local `v2.2.1` | `88f281f94e6febd02a386b7ed03d30d285eb82e1` | `998505c734c0cb040b2b7c53bfefadadb03b025f` | pendiente de publicación |

La identidad se comprobó con `git show refs/heads/release/<versión>` y
`git show refs/tags/v<versión>` en cada submódulo. No se usó una versión global para
sustituir estos commits.

## Sentinel 0.7.1

Worktree: `tools/sentinel/.sentinel/tmp/sentinel-release-071` (eliminado después de la comprobación).

Comandos ejecutados y resultado:

```text
npm install --ignore-scripts --no-audit --no-fund    PASS
npm run compile                                      PASS
npx mocha --no-config --ui tdd --timeout 10000 out/test/suite/projectInit.test.js
                                                     8 passing
npm run check:core                                   PASS
npm run smoke:lsp                                    PASS
```

La prueba focalizada cubre las cuatro variantes de `sentinel init --json`: escribe
configuración, lock y manifest, y no crea una carpeta privada ni scripts del consumidor.
El cambio corrige el defecto de `0dd9c21` y conserva la cobertura en la release `b22c848`.

El VSIX generado desde este commit fue `glory-sentinel-0.7.1.vsix`, 949876 bytes,
SHA-256 `4A7134E1FF1ABA295E270679FC9A66E96D20C0683A159D50A2346D1EDC2C10AF`.

## VarSense 2.2.1

Worktree: `tools/varsense/.sentinel/tmp/varsense-instrumented` (eliminado después de la comprobación).

```text
npm run compile                                      PASS
npm run compile:tests                                PASS
npm run test                                         61 passing
  pretest: lint, check:core y smoke:lsp              PASS
```

La optimización consolida los patrones de descubrimiento en una pasada del provider y
reutiliza el snapshot del workspace durante el proceso CLI. La prueba de contrato nueva
verifica dos llamadas del provider (CSS y consumidores) y todos los patrones esperados.

### Medición instrumentada

La medición se ejecutó sobre el workspace real con el CLI `all`, salida JSON y el índice
persistentemente reutilizable. Cada ejecución terminó con código 0 y 335 archivos
descubiertos; se conservaron las métricas emitidas por el CLI y el tiempo de proceso observado:

| Ejecución | Tiempo CLI | Tiempo observado | Resultado |
| --- | ---: | ---: | --- |
| 1 (cold) | 3184 ms | 3288 ms | PASS |
| 2 (warm) | 2729 ms | 2810 ms | PASS |
| 3 (warm) | 2667 ms | 2743 ms | PASS |

Una salida representativa del primer ciclo registró `durationMs=3014`,
`discoveryMs=351`, `classIndexMs=883`, `variableIndexMs=1650` y RSS pico aproximado
de 146 MB. La comparación de los reportes mantuvo los mismos hallazgos; el cambio solo
reduce el trabajo repetido de descubrimiento.

La línea base anterior del consumidor (`.quality-bench/baseline-small.json`) queda
conservada como evidencia histórica: VarSense cold 12.665 s e incremental 1 ms.
No se reemplaza esa medición; se añade esta muestra del commit correctivo para que el gate
posterior a la adopción vuelva a comprobar el SLO de 6 s.

## Qué falta para cerrar

- Publicar exactamente `b22c8484fd2334f19a88f930c494091d02942e39` como
  `release/0.7.1` y `v0.7.1` en `github.com/1ndoryu/glory-sentinel`.
- Publicar exactamente `88f281f94e6febd02a386b7ed03d30d285eb82e1` como
  `release/2.2.1` y `v2.2.1` en `github.com/1ndoryu/varsense`.
- Repinear los dos consumidores, regenerar lock, ejecutar `doctor`, gate y rollback, y
  solo después actualizar la documentación de estado y decidir la retirada de wrappers.
