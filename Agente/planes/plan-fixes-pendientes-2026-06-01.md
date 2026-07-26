# Plan: Correcciones urgentes y refactor de emails

> **Fecha:** 2026-06-01
> **Rama:** glory-rust-nakomi
> **Estado:** ⚠️ Código parcialmente implementado; no cerrado hasta la aceptación
> funcional indicada por bloque abajo.
>
> **Actualización 2026-07-25:** La mayoría de los bloques fueron completados en la
> sesión 20CA (20 julio 2026). Los items restantes están subsumidos por el plan
> de estabilidad `plan-estabilidad-nakomi-2026-07-23.md` (Fase H).

---

## Resumen

7 problemas reportados por el usuario, agrupados en bloques coherentes. Cada bloque = un commit separado.

---

## Bloque A — Auto-asignación al administrador

**ID:** `016A-4`
**Estado:** ✅ Completado (20CA-2, 2026-07-20) — webhook ya asigna admin automáticamente
**Subsumido por:** plan de estabilidad Fase H (asignación transaccional completa)

---

## Bloque B — Correcciones de UI/UX en asignación

**ID:** `016A-3` (B1), `016A-2` (B3), `016A-5` (B2)
**Estado:**
- **B1 (Modal):** ✅ Completado (20CA-4, 2026-07-20) — búsqueda en ModalAsignar con filtrado
- **B2 (Reasignar/Cancelar):** ✅ Completado parcialmente (20CA-5, 2026-07-20) — admin puede cancelar y reasignar. Backend transaccional pendiente en Fase H del plan de estabilidad.
- **B3 (Freelancer):** ✅ Completado (20CA-3, 2026-07-20) — reemplazo global Empleado→Freelancer

---

## Bloque C — Chat en tiempo real + notificaciones

**ID:** `016A-6`
**Estado revisado (2026-07-25):** La implementación fuente de C1-C3 existe,
pero no hay aceptación funcional suficiente para marcarlas completas.
- **C1 (Notif realtime):** pendiente de prueba en producción para mensaje de
  cliente, panel/fuera de panel y reconexión.
- **C2 (Correo/alerta):** la lógica legacy y la outbox existen, pero las flags
  de delivery son fail-closed y falta evidencia de correo recibido. El fallback
  IA de 10 minutos también requiere prueba de carrera con respuesta humana.
- **C3 (Punto rojo):** el componente existe, pero fuera del panel el contador no
  se carga inicialmente; corregir y verificar en desktop/móvil.

**Subsumido por:** bloques A/B/C/D del plan de cierre (`plan-cierre-bloques-dificiles-nakomi-2026-07-23.md`)

---

## Bloque D — Bug visual en notificaciones

**ID:** `016A-1`
**Estado:** ✅ Completado (20CA-12, 2026-07-20) — CSS corregido, texto alineado a la izquierda

---

## Bloque E — Refactor de plantillas de email (centralizar)

**ID:** `016A-7`
**Estado:** ✅ Completado (20CA-11, 2026-07-20) — verificado que `email_templates.rs` tiene 21 funciones `render_*` con helpers compartidos. `email.rs` y `email_preview.rs` delegan en ellas. Zero HTML inline.

---

## Pendiente pre-existente: 5 nuevas plantillas admin

**Estado:** ✅ Completado — integradas en el sistema centralizado de `email_templates.rs`

---

## Orden sugerido

```
Bloque D (½h) → Bloque B3 (¼h) → Bloque B1 (1-2h) → Bloque A (1-2h)
→ Bloque B2 (1h) → Bloque C (5-8h) → Commit parcial de A+B+C+D
→ Bloque E (6-10h) → Commit final
```

O, alternativamente, si el usuario prefiere:

```
Primero Bloque E (refactor) → luego todo lo demás con las templates ya centralizadas
```

---

## Preguntas al usuario

1. **Orden:** ¿Prefieres que haga el refactor de plantillas (Bloque E) primero antes de las nuevas funcionalidades, o al final?
2. ✅ **Auto-asignación:** Todas las órdenes nuevas, con opción de delegar/cancelar.
3. ✅ **Chat 20 min:** Email a cliente + admin + empleado asignado.
4. ✅ **Modal referencia:** `ModalCrearUsuario` (usa `ModalBody`/`ModalField`/`ModalLabel`).
