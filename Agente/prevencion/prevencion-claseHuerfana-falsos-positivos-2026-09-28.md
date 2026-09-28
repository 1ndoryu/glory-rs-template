# Prevención: falsos positivos de `claseHuerfana` VarSense

**Fecha:** 2026-09-28 · **Origen:** auditoría 279A-5 (NAKOMI, 129 → 27, los 27 son FP).

## Familias de FP verificadas contra código real

1. **Mapas de clases en `src/api/*.ts`** (el detector no indexa ese directorio como consumidores):
   `statusActivo/Baneado/Suspendido` en `src/api/admin-users.ts:80-82`,
   `pagoEstado--pendiente/retenido/liberado/reembolsado/fallido` en `src/api/payments.ts:134-138`.
2. **Props portadoras de clase** (`panelClassName=`, `triggerClassName=`):
   `chatBell__dropdown` (`ChatBell.tsx:90`), `notificationBell__dropdown`
   (`NotificationBell.tsx:71`), `usuariosCrearRolBtn` (`ModalCrearUsuario.tsx:66`).
3. **Clases dinámicas por template** (`` `boton${Variante}` ``, `` `input${...}` ``):
   `botonPrimario/Secundario/Outline/Texto/Mediano/Grande` (`Button.tsx:19`),
   `inputDefault/Outline` (`Input.tsx:11`), `textareaDefault/Outline` (`Textarea.tsx:10`),
   `infraFila--huerfana` (`DeploymentRow.tsx:171`).
4. **Clases runtime de terceros** (DOM inyectado por librerías):
   `uplot`/`u-legend` (uPlot + `legend:{show:true}`), `tiptap` + `is-editor-empty`
   (@tiptap/extension-placeholder, `content: attr(data-placeholder)`),
   `richTextContenido`/`modalBaseContenedor`/`hostingFeatureTitulo` (wrappers reales de un solo uso que el índice no siguió).

## Detección esperada

- Indexar `src/api/**/*.ts` como archivos consumidores (familia 1).
- Tratar `*ClassName=` como portador de clase literal (familia 2).
- Resolver templates `` `prefijo${Var}` `` contra literales del mismo archivo (familia 3).
- Allowlist de clases runtime de dependencias conocidas u `excludeClassPatterns` documentado (familia 4).

## Referencia

Roadmap 279A-5 → `Agente/completados/tareas-2026-09-28.md`. Sin automatizar: los FP
quedan como warnings aceptados (`severityCounts.error = 0`).
