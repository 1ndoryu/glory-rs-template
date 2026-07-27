# Plan de pruebas reales de lectura BDP — 2026-07-26

> **Objetivo:** Verificar que las 4 páginas de lectura BDP (Stock, Explorador, Historial, Compras) funcionan correctamente contra un BDP real conectado, no solo con datos demo.
> **Prerrequisito:** BDP conectado con credenciales válidas (`bdp_sync_enabled: true`, `bdp_base_url`, `bdp_login`, `bdp_password`, `bdp_integrator_code` configurados).
> **Modo de operación:** Solo lectura (`bdp_sync_mode: read_only`). Ninguna prueba modifica datos en BDP.

---

## 0. Checklist previo (ejecutar antes de las pruebas)

| # | Verificación | Cómo | Esperado |
|---|---|---|---|
| 0.1 | BDP conectado | Badge header ≠ "BDP: off" | Badge muestra "BDP: lectura" o "BDP: escritura" |
| 0.2 | Health check | Configuración → BDP → "Probar conexión" | `health_ok: true`, `login_ok: true` |
| 0.3 | Feature flags | Configuración → Funcionalidades BDP | Flags de lectura activados según necesidad |
| 0.4 | Modo operación | Badge header | "BDP: lectura" (no escritura) |

---

## 1. Stock BDP (`/bdp/stock`)

### 1.1 Carga de datos

| # | Paso | Acción | Resultado esperado |
|---|---|---|---|
| 1.1.1 | Carga inicial | Navegar a `/bdp/stock` | Tabla se llena con artículos del BDP real. Spinner desaparece en <10s. |
| 1.1.2 | Sin demo mode | Verificar que el toggle "Demo" NO está activo | Datos vienen de BDP, no de fixtures |
| 1.1.3 | Conteo razonable | Comparar número de artículos con los que hay en el TPV | Coinciden (± margen por filtros) |

### 1.2 Campos y formato

| # | Campo | Verificar | Ejemplo |
|---|---|---|---|
| 1.2.1 | Código | No vacío, coincide con código BDP del artículo | `1001`, `BEB003` |
| 1.2.2 | Nombre/Descripción | Texto legible, sin caracteres rotos | "Cerveza", "Café con leche" |
| 1.2.3 | Precio | Número positivo con 2 decimales | `2.50`, `12.00` |
| 1.2.4 | Stock/Cantidad | Número (puede ser 0 o negativo si hay consumos) | `150`, `0`, `-3` |
| 1.2.5 | Familia/Categoría | Texto o vacío | "Bebidas", "Entrantes" |
| 1.2.6 | IVA | Porcentaje válido | `10`, `21` |

### 1.3 Funcionalidad

| # | Acción | Resultado esperado |
|---|---|---|
| 1.3.1 | Filtro por nombre | Escribe texto → tabla filtra en tiempo real |
| 1.3.2 | Filtro por familia | Selecciona familia → solo artículos de esa familia |
| 1.3.3 | Ordenar por columna | Click en encabezado → ordena asc/desc |
| 1.3.4 | Exportar CSV | Click "Exportar" → descarga `.csv` con los datos visibles |
| 1.3.5 | Paginación | Si hay >50 artículos, muestra paginación funcional |
| 1.3.6 | Refresh | Botón actualizar → vuelve a consultar BDP |

### 1.4 Errores

| # | Escenario | Resultado esperado |
|---|---|---|
| 1.4.1 | BDP caído | Toast de error, no crash. Datos anteriores visibles si los había. |
| 1.4.2 | Sin artículos | Mensaje "No hay artículos" o tabla vacía con aviso |
| 1.4.3 | Timeout | Mensaje claro tras 30s, no spinner infinito |

---

## 2. Explorador BDP (`/bdp/explorador`)

### 2.1 Carga de catálogo

| # | Paso | Acción | Resultado esperado |
|---|---|---|---|
| 2.1.1 | Carga inicial | Navegar a `/bdp/explorador` | Se muestran familias/grupos del catálogo BDP |
| 2.1.2 | Estructura jerárquica | Expandir una familia | Se muestran artículos hijos |
| 2.1.3 | Navegación | Click en un artículo | Se muestran detalles (nombre, precio, código, mapeo Glory si existe) |

### 2.2 Mapeo Glory ↔ BDP

| # | Verificar | Esperado |
|---|---|---|
| 2.2.1 | Artículos mapeados | Muestran código Glory + código BDP |
| 2.2.2 | Artículos sin mapeo | Indican "Sin mapear" o permiten crear mapeo |
| 2.2.3 | Duplicados | Si hay 2 artículos Glory → mismo BDP, se indica |

### 2.3 Funcionalidad

| # | Acción | Resultado esperado |
|---|---|---|
| 2.3.1 | Búsqueda | Texto filtra artículos en tiempo real |
| 2.3.2 | Expandir/Colapsar | Toggle familias funciona sin recarga |
| 2.3.3 | Volver | Navegación atrás funciona |

---

## 3. Historial BDP (`/bdp/historial`)

### 3.1 Carga de comandas

| # | Paso | Acción | Resultado esperado |
|---|---|---|---|
| 3.1.1 | Carga inicial | Navegar a `/bdp/historial` | Lista de comandas recientes de BDP |
| 3.1.2 | Rango por defecto | Sin filtros | Muestra comandas del día actual o últimos 7 días |
| 3.1.3 | Conteo | Comparar con TPV | Número de comandas coincide (± margen por hora) |

### 3.2 Campos

| # | Campo | Verificar |
|---|---|---|
| 3.2.1 | OrderId | Número válido de BDP |
| 3.2.2 | Fecha/Hora | Timestamp correcto, formato legible |
| 3.2.3 | Estado | pending/accepted/invoiced/cancelled según BDP |
| 3.2.4 | Importe | Número positivo con 2 decimales |
| 3.2.5 | Mesa | Identificador de mesa si BDP lo devuelve |
| 3.2.6 | Empleado | Código/nombre de empleado |

### 3.3 Filtros

| # | Filtro | Acción | Resultado |
|---|---|---|---|
| 3.3.1 | Rango fechas | Cambiar fecha inicio/fin | Lista se actualiza |
| 3.3.2 | Estado | Filtrar por estado | Solo comandas de ese estado |
| 3.3.3 | Buscar por ID | Introducir OrderId | Encuentra comanda específica |

### 3.4 Detalle de comanda

| # | Acción | Resultado esperado |
|---|---|---|
| 3.4.1 | Click en comanda | Abre detalle con líneas de artículos |
| 3.4.2 | Líneas | Cada línea muestra artículo, cantidad, precio unitario |
| 3.4.3 | Pagos | Si tiene pagos, muestra método e importe |
| 3.4.4 | Factura | Si está facturada, muestra número de factura |

---

## 4. Compras BDP (`/bdp/compras`)

### 4.1 Feature flags

| # | Verificar | Esperado |
|---|---|---|
| 4.1.1 | `ff_bdp_purchase_notes_read` | Activado para poder ver albaranes |
| 4.1.2 | `ff_bdp_purchase_notes_draft` | Activado solo si se prueban borradores |
| 4.1.3 | `ff_bdp_purchase_notes_receive` | Activado solo si se prueban conciliaciones |

### 4.2 Carga de albaranes (Fase 1 — lectura)

| # | Paso | Acción | Resultado esperado |
|---|---|---|---|
| 4.2.1 | Carga inicial | Navegar a `/bdp/compras` | Lista de albaranes de compra recientes |
| 4.2.2 | Rango por defecto | Sin filtros | Muestra albaranes del último mes |
| 4.2.3 | Conteo | Comparar con albaranes conocidos del restaurante | Coinciden |

### 4.3 Campos de albarán

| # | Campo | Verificar |
|---|---|---|
| 4.3.1 | Número albarán | Identificador único del proveedor |
| 4.3.2 | Proveedor | Nombre/Razón social |
| 4.3.3 | Fecha | Fecha del albarán |
| 4.3.4 | Importe total | Suma de líneas |
| 4.3.5 | Estado | Recibido/Pendiente/Conciliado |

### 4.4 Detalle de albarán

| # | Acción | Resultado esperado |
|---|---|---|
| 4.4.1 | Click en albarán | Abre detalle con líneas de productos |
| 4.4.2 | Líneas | Cada línea muestra producto, cantidad, precio unitario, IVA |
| 4.4.3 | JSON crudo | Campo `datos_bdp` accesible para debug |

### 4.5 Filtros

| # | Filtro | Acción | Resultado |
|---|---|---|---|
| 4.5.1 | Rango fechas | Cambiar rango | Lista se actualiza |
| 4.5.2 | Proveedor | Filtrar por proveedor | Solo albaranes de ese proveedor |
| 4.5.3 | Buscar | Texto en número de albarán | Encuentra albarán específico |

### 4.6 Borradores y conciliación (Fases 2-3, si flags activados)

| # | Acción | Resultado esperado |
|---|---|---|
| 4.6.1 | Marcar como borrador | Albarán pasa a estado "borrador" localmente. No escribe en BDP. |
| 4.6.2 | Conciliar con gasto | Vincula albarán a un gasto existente o crea uno nuevo |
| 4.6.3 | Deshacer conciliación | Desvincula albarán del gasto |

---

## 5. Pruebas cross-cutting

### 5.1 Navegación entre páginas

| # | Acción | Resultado esperado |
|---|---|---|
| 5.1.1 | Stock → Explorador | Navega sin perder estado |
| 5.1.2 | Explorador → Historial | Navega sin recarga innecesaria |
| 5.1.3 | Historial → Compras | Navega correctamente |
| 5.1.4 | Volver atrás | Browser back funciona en todas |

### 5.2 Modo demo vs real

| # | Verificar | Esperado |
|---|---|---|
| 5.2.1 | Toggle demo | Al activar demo, datos cambian a fixtures. Al desactivar, vuelve a BDP real. |
| 5.2.2 | Persistencia | El modo demo no se persiste entre sesiones |
| 5.2.3 | Indicador visual | Badge "Demo" visible cuando está activo |

### 5.3 Rendimiento

| # | Métrica | Umbral aceptable |
|---|---|---|
| 5.3.1 | Carga Stock | <5s con conexión normal |
| 5.3.2 | Carga Historial (30 días) | <10s |
| 5.3.3 | Carga Compras (1 mes) | <10s |
| 5.3.4 | Filtro en tabla | <500ms (client-side) |

### 5.4 Responsividad

| # | Viewport | Verificar |
|---|---|---|
| 5.4.1 | Mobile (320px) | Tablas scroll horizontal, filtros accesibles |
| 5.4.2 | Tablet (768px) | Layout correcto, sin overflow |
| 5.4.3 | Desktop (1024+) | Layout completo con sidebar |

---

## 6. Registro de resultados

| Página | Estado | Notas |
|---|---|---|
| Stock | ⚠️ OK modo demo | Carga 6 artículos, filtros/paginación/CSV OK. Pendiente probar con BDP real (requiere restaurante en TailScale). |
| Explorador | ⚠️ OK modo demo | 4 definiciones, búsqueda OK. Pendiente probar consulta por código contra BDP real. |
| Historial | ⚠️ OK modo demo | 3 auditorías + 2 snapshots. Solo lee DB local — no contacta BDP. Pendiente probar con datos reales. |
| Compras | ⚠️ OK modo demo | 4 albaranes, filtros OK. Pendiente probar Sync albaranes contra BDP real. |

> **Nota:** Las pruebas contra BDP real se hacen **desde local** (no requiere deploy). La IP `100.83.196.35` es accesible vía TailScale cuando el restaurante está conectado. Ver `resultados-pruebas-lectura-bdp-2026-07-26.md`.

> **Formato:** Al ejecutar cada sección, marcar ✅ (pasó), ⚠️ (pasó con observaciones) o ❌ (falló) y documentar hallazgos.

---

## 7. Riesgos conocidos

| Riesgo | Mitigación |
|---|---|
| BDP no devuelve stock en tiempo real | Verificar con dato conocido reciente en el TPV |
| Albaranes de compras con datos BDP en formato distinto al esperado | Revisar campo `datos_bdp` raw si las líneas tipadas están vacías |
| Timeout en historial con mucho rango | Reducir rango de fechas si tarda >30s |
| Artículos con código especial (tildes, ñ) | Verificar encoding UTF-8 correcto |
