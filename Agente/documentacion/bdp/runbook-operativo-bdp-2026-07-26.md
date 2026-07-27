# Runbook Operativo BDP

> **Fecha:** 2026-07-26
> **Propósito:** Procedimientos paso a paso para cada tipo de incidente en la integración BDP.
> **Audiencia:** Operador del restaurante + soporte técnico.

---

## 1. BDP no responde (PC del restaurante apagado o sin red)

**Síntomas:**

- Botón "Probar conexión" → error de timeout o conexión rechazada
- Ventas nuevas se crean normalmente en Glory pero no se sincronizan
- Badge BDP muestra "❌ Error" o "⏳ Pendiente"

**Procedimiento:**

1. Verificar que el PC del restaurante con BDP-NET esté encendido.
2. Verificar que Tailscale esté conectado entre el servidor y el PC.
3. Ir a Configuración → BDP → "Probar conexión". Si falla → el PC no es alcanzable.
4. Una vez que BDP vuelva, las ventas pendientes se sincronizarán automáticamente si el polling está activo. Si no, usar el botón "Reintentar sync" por venta.
5. **No activar escritura manual** mientras BDP esté inestable.

**Impacto:** Las ventas se registran localmente en Glory sin pérdida de datos. Solo se retrasa la sincronización con BDP.

---

## 2. Comanda duplicada en BDP

**Síntomas:**

- La misma comanda aparece dos veces en el TPV de BDP
- Dos comandas con el mismo `MarketplaceOrderId` (ej: `G1234567890abcd`)

**Procedimiento:**

1. **No paniquear.** BDP deduplica por `MarketplaceOrderId` en la mayoría de versiones.
2. Verificar en Glory: la venta debe mostrar `bdp_synced=true` con un solo `bdp_order_id`.
3. Si Glory muestra `bdp_synced=false` con `bdp_sync_error=ambiguo`:
    - El sistema ya detectó la ambigüedad y bloqueó reintentos.
    - Ir a "Historial BDP" → buscar la venta → verificar el resultado.
    - Si `GetOrder` devuelve la comanda → marcar como sincronizada manualmente.
4. Si hay dos comandas reales en BDP:
    - Cancelar la duplicada **desde el TPV de BDP** (no desde Glory).
    - `CancelOrder` no está disponible vía API.
5. Reportar el incidente con: fecha, hora, venta_id, order_id, capturas de Glory y BDP.

**Prevención:** El sistema ya implementa advisory lock + MarketplaceOrderId estable + `bdp_synced` guard + reconciliación post-ambiguo.

---

## 3. Pago registrado en BDP pero no en Glory (o viceversa)

**Síntomas:**

- BDP muestra la comanda como pagada pero Glory muestra `pendiente`
- O Glory muestra pagada pero BDP no

**Procedimiento:**

1. Ir a "Historial BDP" → buscar la venta → verificar el estado de la auditoría.
2. Si el estado es `ambiguo`:
    - El sistema bloqueará nuevas escrituras sobre esa venta.
    - Usar "Consultar estado BDP" (botón 🔍) para verificar el estado real en BDP.
3. Si BDP confirma el pago:
    - El worker de reconciliación (`reconcile_ambiguous_pagos`) debería cerrarlo automáticamente.
    - Si no, contactar soporte técnico.
4. Si BDP no tiene el pago:
    - El estado `ambiguo` es correcto — no reintentar sin verificar.
5. **Nunca pulsar "Pagar" dos veces** si la primera no respondió.

**Prevención:** Idempotency key por pago + ledger local + advisory lock por venta + reconciliación automática.

---

## 4. Factura sin número o factura duplicada

**Síntomas:**

- Glory muestra `bdp_invoiced=true` pero sin `InvoiceNumber`
- BDP tiene dos facturas para la misma comanda

**Procedimiento:**

1. Si `InvoiceNumber` está vacío en Glory:
    - El sistema marcará la auditoría como `ambiguo`.
    - Usar "Consultar estado BDP" para verificar si BDP tiene la factura.
    - Si BDP tiene factura → reconciliar manualmente el número.
2. Si hay dos facturas en BDP:
    - Esto no debería pasar (el sistema verifica `status=3` antes de facturar).
    - Si pasó: cancelar la duplicada desde el TPV.
3. **Consecuencias fiscales:** Una factura emitida es irreversible por la API. Cualquier corrección requiere procedimiento manual en BDP-NET.

**Prevención:** Verificación de estado + saldo cero + reconciliación de facturas existentes + `InvoiceNumber` no vacío.

---

## 5. Error 300035 (serie de facturación no válida)

**Síntomas:**

- CreateOrder devuelve `[300035]-NO SE HA DEFINIDO UNA SERIE DE FACTURACION VALIDA`

**Procedimiento:**

1. Ir a BDP-NET → Configuración TPV → Terminales → Terminal 31 → Parámetros 6.
2. Verificar que "Comandas Facturadas Weblink → Serie Destino" tenga un valor (ej: `00031TI`).
3. Verificar en Facturas 1 que las series de Barra y Mesas estén asignadas.
4. Documentación completa: `Agente/documentacion/api/bdp-300035-resumen-completo-2026-06-01.md`.

**Historial:** Este error se resolvió el 2026-06-07 creando la serie `00031TI` (IVA Incluido).

---

## 6. Error 300005 (terminal no configurado para IVA incluido)

**Síntomas:**

- CreateOrder devuelve `[300005]-EL TERMINAL NO ESTÁ CONFIGURADO PARA TRABAJAR CON IVA INCLUIDO`

**Procedimiento:**

1. Verificar que la serie asignada al terminal tenga "IVA Incluido" activo.
2. Si no, crear una nueva serie con IVA Incluido y asignarla al terminal.
3. Documentación: `Agente/documentacion/api/bdp-300035-resumen-completo-2026-06-01.md` §3.6.

---

## 7. Throttling (demasiadas peticiones concurrentes)

**Síntomas:**

- Error "BDP throttled" en logs o en `bdp_sync_error` de la venta
- Estado de auditoría: `ambiguo`

**Procedimiento:**

1. **No reintentar inmediatamente.** El sistema marca throttling como `ambiguo` (no error permanente).
2. Esperar 30 segundos.
3. El worker de reconciliación verificará si la operación se aplicó.
4. Si no se aplicó, usar "Reintentar sync" o "Reintentar pago".
5. Si el throttling es recurrente:
    - Verificar que no haya múltiples usuarios sincronizando simultáneamente.
    - El `BdpThrottleManager` limita a 2 peticiones concurrentes por destino.

**Prevención:** Mapeo `Throttled→AmbiguousTransport` + reconciliación automática.

---

## 8. Cliente sin código BDP bloquea la venta

**Síntomas:**

- Error "Cliente sin código BDP confirmado" al intentar enviar venta a BDP
- La venta se crea en Glory pero no se sincroniza

**Procedimiento:**

1. Ir a Clientes → buscar el cliente → botón "BDP" → vincular con código BDP explícito.
2. O usar "Importar BDP" para importar el cliente desde BDP primero.
3. Una vez vinculado, usar "Reintentar sync" en la venta.
4. **Alternativa:** Configurar un `bdp_default_customer_code` en Configuración → BDP para que las ventas sin cliente usen uno genérico.

---

## 9. Restore de snapshot falla a mitad

**Síntomas:**

- Error durante la restauración de un snapshot de Glory
- Algunos registros restaurados, otros no

**Procedimiento:**

1. **Esto ya no debería pasar.** La restauración está envuelta en transacción atómica (`[207A-3] S14-H1`).
2. Si ocurre: la transacción se revierte completamente — ningún registro queda a mitad.
3. Verificar el error específico en el toast de la UI.
4. Si el snapshot no existe → 404.
5. Si el snapshot no es del usuario → "No autorizado".
6. Si el snapshot es de tipo BDP (no Glory) → "Solo se pueden restaurar snapshots de Glory".

---

## 10. Logs útiles para diagnóstico

| Qué buscar en logs | Significado                                |
| ------------------ | ------------------------------------------ |
| `[BDP-SAFE]`       | Mensaje de seguridad/mitigación BDP        |
| `[065A-5]`         | Mensaje del servicio de sync BDP           |
| `[F2]`             | Auto-backup o pre-write audit              |
| `[F3]`             | Gate de modo (read_only vs unidirectional) |
| `[F7.5]`           | Auto-sync de cliente                       |
| `[F8.1]`           | Pago registrado                            |
| `[F8.2]`           | Factura registrada                         |
| `[AUDIT-2.11]`     | Transacción post-HTTP en sync_venta        |
| `[207A-2]`         | Transacción post-HTTP en pago/factura      |
| `[207A-3]`         | Transacción en restore                     |
| `[R2]`             | Lock distribuido cerrado antes de HTTP     |
| `[R3]`             | Throttling mapeado a ambiguo               |
| `[R5]`             | Timeout global de 45s                      |
| `[R14]`            | SyncLockGuard RAII                         |

---

## 11. Contactos y escalamiento

| Nivel | Quién                      | Cuándo                                                  |
| ----- | -------------------------- | ------------------------------------------------------- |
| 1     | Operador del restaurante   | Errores de UI, ventas no sincronizadas, throttling      |
| 2     | Soporte técnico (nosotros) | Errores de código, reconciliación manual, configuración |
| 3     | Soporte BDP-NET            | Errores 300035, 300005, CancelOrder, módulos no activos |
