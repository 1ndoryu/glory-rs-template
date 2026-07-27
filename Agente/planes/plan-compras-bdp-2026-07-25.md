# Plan detallado: Compras BDP — Fase 1 (lectura de albaranes)

> **Fecha:** 2026-07-25
> **Alcance:** Implementar funcionalidad read-only de albaranes de compra (ExportPurchaseNotes).
> **Restricción:** No se realiza ninguna llamada real al BDP ni se habilita escritura. El modo solo lectura es innegociable en esta fase.
> **Feature flag:** `ff_bdp_purchase_notes_read` (columna ya existe en `configuracion_restaurante`).

---

## Contexto

El cliente preguntó por la integración de compras. El plan original (`Agente/planes/plan-pendientes-bdp-2026-07-23.md`) dividió compras en 3 fases:

1. **Fase 1 — Lectura de albaranes** (~8h)
2. **Fase 2 — Crear borradores** (~10h)
3. **Fase 3 — Recepción y reconciliación** (~12h)

Este documento planifica y guía la implementación de la **Fase 1**.

---

## Endpoint BDP

`ExportPurchaseNotes`

- **Ruta:** `POST /API/ExportProfiles/PurchaseNotes`
- **Input:**
    - `ExportProfileCode` (Integer, obligatorio) — perfil de exportación en BDP
    - `InitialDate` / `FinalDate` (Date, opcional)
    - `InitialSupplier` / `FinalSupplier` (Long, opcional)
    - `InitialSerial` / `FinalSerial` (String, opcional)
- **Output:**
    - `DocumentsLists` — colección de albaranes de compra
    - `ErrorMessage` — vacío si éxito
    - Cada albarán incluye cabecera (`Serie_Albaran`, `Num_Albaran`, `Fecha_Albaran`, `Cod_Proveedor`, `Nom_Proveedor`, `Total_Albaran`) y líneas (`Lineas`).

---

## Arquitectura propuesta (Fase 1)

### 1. Backend — Servicios BDP WebLink

**Archivos a modificar:**

- `src/services/bdp_weblink_catalog.rs`
- `src/services/bdp_weblink.rs`

**Cambios:**

- Añadir constante `BDP_PATH_EXPORT_PURCHASE_NOTES = "/API/ExportProfiles/PurchaseNotes"`.
- Añadir endpoint a `BDP_ENDPOINTS` con área `Compras`.
- Crear `BdpExportPurchaseNotesRequest` con los campos del manual.
- Crear `BdpPurchaseNoteLine` y `BdpPurchaseNote` para parsear la respuesta defensivamente.
- Añadir `export_purchase_notes()` en `BdpWeblinkClient`.

### 2. Backend — Base de datos

**Archivos a crear:**

- `migrations/20260725170000_bdp_purchase_notes.up.sql`
- `migrations/20260725170000_bdp_purchase_notes.down.sql`

**Tabla `bdp_purchase_notes`:**

```sql
CREATE TABLE IF NOT EXISTS bdp_purchase_notes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    serie TEXT NOT NULL,
    numero TEXT NOT NULL,
    fecha DATE,
    codigo_proveedor TEXT,
    nombre_proveedor TEXT,
    total NUMERIC(14,4),
    datos_bdp JSONB NOT NULL DEFAULT '{}',
    ultima_sync_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, serie, numero)
);
```

### 3. Backend — Modelo

**Archivo a crear:**

- `src/models/bdp_purchase_note.rs`

**Structs:**

- `BdpPurchaseNote` — FromRow + Serialize + ToSchema
- `BdpPurchaseNoteListParams` — filtros de listado (query params)
- `BdpPurchaseNoteSyncRequest` — body de POST /sync (perfil, rango fechas, rango proveedores)
- `BdpPurchaseNoteSyncResult` — resumen de sync

### 4. Backend — Repositorio

**Archivo a crear:**

- `src/repositories/bdp_purchase_note.rs`

**Métodos:**

- `listar(pool, user_id, filtros) -> Vec<BdpPurchaseNote>`
- `upsert_from_bdp(pool, user_id, note) -> Result<bool>`

### 5. Backend — Handler

**Archivo a crear:**

- `src/handlers/bdp_purchase_note.rs`

**Endpoints:**

- `GET /api/bdp/purchase-notes` — listar albaranes locales con filtros
- `POST /api/bdp/purchase-notes/sync` — llamar a BDP y persistir resultados

**Seguridad:**

- Requiere autenticación Bearer.
- Verifica que `ff_bdp_purchase_notes_read == true` en `configuracion_restaurante`.
- En `sync`, exige rango de fechas para evitar import masivo sin límites.

### 6. Backend — Registro en router y OpenAPI

**Archivos a modificar:**

- `src/handlers/mod.rs`
- `src/models/mod.rs`
- `src/repositories/mod.rs`

### 7. Frontend — API client

**Archivo a modificar:**

- `frontend/src/api/bdp.ts`

**Funciones a añadir:**

- `fetchBdpPurchaseNotes(filters)`
- `syncBdpPurchaseNotes(params)`
- `useBdpPurchaseNotes()`
- `useSyncBdpPurchaseNotes()`

### 8. Frontend — Página de compras

**Archivo a crear:**

- `frontend/src/componentes/bdp/BdpCompras.tsx`

**UI:**

- Banner de solo lectura.
- Tabla de albaranes: Fecha, Serie, Número, Proveedor, Total.
- Filtros por proveedor y rango de fechas.
- Botón "Sync albaranes" con confirmación y feedback.

### 9. Frontend — Rutas y navegación

**Archivos a modificar:**

- `frontend/src/App.tsx` — añadir ruta `/bdp/compras`
- `frontend/src/components/app-sidebar.tsx` — añadir "BDP Compras" en `navBdp`

### 10. Validación

- `cargo check`
- `cargo clippy -- -D warnings`
- `npx tsc --noEmit` en `frontend/`
- Tests unitarios con Wiremock para `export_purchase_notes`

### 11. Documentación y roadmap

- Actualizar `roadmap.md`.
- Actualizar `Agente/planes/plan-pendientes-bdp-2026-07-23.md` para reflejar Fase 1 hecha.

---

## Mitigaciones de riesgo

| Riesgo                      | Mitigación                                                      |
| --------------------------- | --------------------------------------------------------------- |
| Escritura accidental        | Feature flag read-only; no endpoints POST que muten BDP.        |
| Import masivo descontrolado | Exigir rango de fechas en sync. Limitar a 31 días por defecto.  |
| Inyección SQL               | Query builder con bind parameters de sqlx.                      |
| Estructura JSON variable    | Guardar `datos_bdp` en JSONB para no perder campos no mapeados. |
| Timeout del TPV local       | Usar timeout de 20s y manejar throttling existente.             |
| Concurrencia                | `ON CONFLICT` en upsert por `(user_id, serie, numero)`.         |

---

## Estado

- [x] Plan creado (este documento)
- [x] Backend servicios BDP
- [x] Migración y modelo
- [x] Repositorio y handler
- [x] Router y OpenAPI
- [x] Frontend API + página + rutas
- [x] Validación y tests
- [x] Revisión de código
- [x] Roadmap actualizado

## Resultado de la implementación

Implementado en 25 julio 2026 (`247A-11`).

- Backend:
  - `src/services/bdp_weblink_catalog.rs`: endpoint `ExportPurchaseNotes` registrado con área `Compras`.
  - `src/services/bdp_weblink.rs`: método `export_purchase_notes()` para llamar a BDP.
  - `src/models/bdp_purchase_note.rs`: modelos `BdpPurchaseNote`, `BdpPurchaseNoteListParams`, `BdpPurchaseNoteSyncRequest`, `BdpPurchaseNoteSyncResult`.
  - `src/repositories/bdp_purchase_note.rs`: listado paginado/filtrado y upsert con `ON CONFLICT (user_id, serie, numero)`.
  - `src/handlers/bdp_purchase_note.rs`: endpoints `GET /api/bdp/purchase-notes` y `POST /api/bdp/purchase-notes/sync`.
  - `src/handlers/mod.rs`: rutas y esquemas OpenAPI registrados.
  - `src/models/mod.rs` y `src/repositories/mod.rs`: módulos exportados.
  - `migrations/20260725170000_bdp_purchase_notes.up.sql` / `.down.sql`.
- Frontend:
  - `frontend/src/api/bdp.ts`: hooks `useBdpPurchaseNotes` y `useSyncBdpPurchaseNotes`.
  - `frontend/src/componentes/bdp/BdpCompras.tsx`: página de compras con tabla, filtros y sync.
  - `frontend/src/App.tsx`: ruta `/bdp/compras`.
  - `frontend/src/components/app-sidebar.tsx`: ítem "BDP Compras".
- Validación:
  - `cargo check` ✅
  - `cargo test bdp_purchase_note` ✅ (3 tests)
  - `npx tsc --noEmit` en `frontend/` ✅

## Notas y próximos pasos

- Fase 1 es **solo lectura**. No se permite crear/modificar albaranes ni escribir en BDP.
- `BdpPurchaseNoteData.lineas` asume clave `Lineas` en la respuesta BDP; si BDP usa otra clave, las líneas tipadas quedarán vacías, pero el JSON completo se conserva en `datos_bdp`.
- **Actualización (verificación 2026-07-26):** Las Fases 2 y 3 están implementadas en código:
  - Fase 2 (borradores): endpoint `marcar_borrador_purchase_note` protegido por `ff_bdp_purchase_notes_draft`
  - Fase 3 (conciliación): endpoint `conciliar_purchase_note` protegido por `ff_bdp_purchase_notes_receive`
  - Modelo `BdpPurchaseNoteReconcileRequest` en `models/bdp_purchase_note.rs`
  - **Los 3 feature flags existen** en `configuracion_restaurante` pero están desactivados por defecto.
  - Pendiente: activar flags en producción y pruebas contra BDP real.
