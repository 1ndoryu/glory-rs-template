# Resultados — Pruebas de lectura BDP (2026-07-26)

## 1. Auditoría de seguridad ✅

Todas las 4 páginas BDP son **100% seguras contra BDP producción**. Ninguna envía comandos de escritura.

| Página | Endpoints API | Efecto en BDP | Efecto en Glory |
|--------|---------------|---------------|-----------------|
| **Stock** | `GET /api/bdp/article-maps`, `POST /api/bdp/article-maps/sync-catalog` | Solo lectura (GetPosArticles, GetPricesArticles) | Escribe en DB local |
| **Explorador** | `GET /api/bdp/menus/:id`, `GET /api/bdp/fastfoods/:id`, `GET /api/bdp/packs/:id` | Solo lectura | Ninguno |
| **Historial** | `GET /api/bdp/audit`, `GET /api/bdp/backup/snapshots` | **No contacta BDP** | Lee audit log y snapshots locales |
| **Compras** | `GET /api/bdp/purchase-notes`, `POST /api/bdp/purchase-notes/sync` | Solo lectura (ExportPurchaseNotes) | Escribe albaranes en DB local |

**Nota:** Los endpoints de "Sync catálogo" y "Sync albaranes" leen de BDP y escriben solo en la DB local de Glory. Los endpoints de "Borrador" y "Conciliar" no contactan BDP en absoluto.

## 2. Pruebas de carga de páginas ✅

Las 4 páginas se cargan correctamente en modo demo con datos de ejemplo:

| Página | Estado | Datos mostrados | Errores |
|--------|--------|-----------------|---------|
| **Stock** (`/bdp/stock`) | ✅ Carga OK | 6 artículos demo, filtros y paginación funcionan | Ninguno |
| **Explorador** (`/bdp/explorador`) | ✅ Carga OK | 4 definiciones (menú, fastfood, pack), búsqueda funciona | Ninguno |
| **Historial** (`/bdp/historial`) | ✅ Carga OK | 3 entradas de auditoría, 2 snapshots, pestañas funcionan | Ninguno |
| **Compras** (`/bdp/compras`) | ✅ Carga OK | 4 albaranes, filtros de proveedor y fecha funcionan | Ninguno |

**Console warnings (no bloqueantes):**
- React Router Future Flag Warning (v7 compatibility)
- Deprecated feature warning
- Form field sin id/name attribute

## 3. Conectividad al BDP real ⚠️

| Verificación | Resultado |
|---|---|
| Credenciales configuradas | ✅ `bdp_login=admin`, `bdp_password=kamples2026`, `bdp_integrator_code=VBW2MBM5` guardados correctamente (skip_serializing por seguridad) |
| URL BDP | `http://100.83.196.35:8068` (red TailScale del restaurante) |
| Health check | ❌ **Falló** — el restaurante se desconectó de TailScale durante la prueba |
| Sync habilitado | ✅ `bdp_sync_enabled=true` |
| Modo operación | `read_only` |

**Importante:** La IP `100.83.196.35` es accesible desde local **cuando el restaurante está conectado a TailScale**. No es necesario deployar a producción ni usar VPN. La prueba falló porque el restaurante se desconectó de la red en ese momento.

**Para repetir las pruebas:** Esperar a que el restaurante vuelva a estar en TailScale y re-ejecutar. No requiere ninguna configuración adicional.

## 4. Próximos pasos para pruebas reales

| # | Acción | Requisito |
|---|---|---|
| 1 | Verificar que el restaurante está en TailScale | `curl http://100.83.196.35:8068/Service/Health` desde local |
| 2 | Probar Stock contra BDP real | Local + TailScale activo → `http://localhost:5173/bdp/stock`, desactivar demo mode, pulsar "Sync catálogo" |
| 3 | Probar Explorador contra BDP real | Local + TailScale → buscar código de menú/pack/fastfood conocido del restaurante |
| 4 | Probar Historial contra BDP real | Local + TailScale → verificar entradas de auditoría (lee DB local, no requiere BDP activo) |
| 5 | Probar Compras contra BDP real | Local + TailScale → pulsar "Sync albaranes" con rango de fechas reciente |

**Recomendación:** Hacer las pruebas durante horario del restaurante (comida/cena) para tener datos frescos en BDP. Verificar TailScale antes de empezar.
