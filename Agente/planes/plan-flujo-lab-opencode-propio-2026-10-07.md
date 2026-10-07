# Plan 07AA-6 — Flujo controlado para modificar opencode-propio (lab + skill)

> Origen: 2026-10-07, pedido de ella. Su app corre en dev con recarga
> automática (`electron-vite dev`): guardar un archivo la rompe. Este plan
> deja el flujo completo y repetible para cualquier agente futuro.

## Objetivo

Modificar `opencode-propio` sin romper jamás la app viva, con mecanismo de
promoción explícito y una skill que lo enseñe a otros agentes.

## Hallazgos (evidencia, no suposiciones)

- Lanzador real: `Desktop/Opencode-Propio-Dev.cmd` → backend `:4096`
  (reutiliza si ya escucha) + `bun run dev` en `packages/desktop`
  (= `electron-vite dev`, HMR activo) + `OPENCODE_DB` identity compartida.
- Su checkout está SUCIO (decenas de `M` sin commit: i18n, settings,
  generados, `bun.lock`): jamás pisarlo; promoción siempre con diff primero.
- Lab creado [07AA-5]: `area-trabajo/opencode-propio-dev/` (copia sin
  `node_modules/.git/.turbo/out/dist/build`), `bun install` propio OK
  (523 paquetes), `marketplace-assistant.test.ts` 26/26, regla en
  `LEEME-LAB.md`.
- Puertos/app protegidos (§5 área): nunca matar procesos ni tocar
  5174/5175/8787/4096; el reinicio siempre lo hace ella.

## Decisión: ¿se usa git para mover el cambio? No

- Descartado worktree: parte de HEAD y pierde sus cambios sin commit.
- Descartado `git init` en el lab: repo fantasma que confunde y riesgo de
  push al remoto real (`push → 1ndoryu/opencode`).
- Elegido: **promoción por copia archivo-por-archivo con diff-first +
  manifest** (lista archivos + hash + origen) archivado en la ventana.
  Simple, auditable, sin repos falsos. Revisitar si el volumen crece.

## Fases

- **F1 — Skill `lab-opencode`**: `SKILL.md` (frontmatter `name` +
  `description` + cuerpo, como las de `.agents/skills/`) con rutas,
  reglas, puertos/db, comandos de verificación, promoción y rollback.
  DoD: la skill carga y se usa en el piloto F3.
- **F2 — Mecanismo de promoción (checklist de ventana)**: 1) diff de cada
  archivo lab vs suyo (respeta sus `M`); 2) manifest con hashes;
  3) aplicar; 4) ella reinicia con su acceso directo; 5) prueba viva;
  6) si falla, rollback restaurando su estado previo (sus `M` intactos).
- **F3 — Piloto real**: aplicar este flujo al C1 del plan 03AA-3
  (cableado Marketplace). DoD: C1 en ventana, app arranca, off = viejo
  intacto, viva verde.
- **F4 — Prohibiciones fijas**: matar/reiniciar su app o backend, tocar
  puertos, correr la app del lab por defecto, commitear en `src/`,
  compartir su `identity.db`.

## Estado

Plan nuevo 2026-10-07, pendiente de ejecución (F1 → F3). C1 de 03AA-3
espera a F1+F2.
