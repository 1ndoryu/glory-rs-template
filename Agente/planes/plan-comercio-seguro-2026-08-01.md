# Plan de comercio seguro — 2026-08-01

Estado: implementación técnica autónoma completada en el primer bloque; quedan integración de proveedor, UI y operación humana.

## Alcance

El navegador solo inicia un checkout. PostgreSQL y el webhook son la autoridad para pago, versión adquirida, entitlement y descarga. Los enlaces son opacos, temporales y nunca exponen rutas internas.

## Fase 1 — Idempotencia y modelo de entrega

- [x] Añadir `orders.idempotency_key`, usuario opcional y versión adquirida.
- [x] Crear índice único parcial por cliente + clave para reintentos seguros.
- [x] Aceptar la clave en body/header y enviarla también a Stripe.
- [x] Mantener compatibilidad con órdenes legacy sin clave.
- [x] Validar email, longitud de lote y tamaño de clave en el boundary.

Gate: `cargo check`, tests Rust y type-check frontend pasan; una misma clave no crea una segunda orden.

## Fase 2 — Webhook y grants

- [x] Registrar eventos Stripe por `provider_event_id` y hacer el procesamiento repetible.
- [x] Crear entitlement por orden, asociando la última versión inmutable del producto.
- [x] Generar token aleatorio; persistir solo su SHA-256.
- [x] Añadir endpoint `/api/downloads/:token` con expiración, revocación y path traversal fail-closed.
- [x] Emitir evento outbox deduplicado para la creación del grant.
- [x] Enviar al comprador el enlace privado y marcar la orden entregada solo después de éxito.

Gate: tests de firma/evento duplicado/grant y descarga con archivo válido, expirado, revocado y ruta inválida.

## Fase 3 — Pendiente con intervención humana

- [ ] Worker/outbox que, si el correo falla, emita un grant nuevo de forma explícita, reintente correo y marque `commerce_outbox.processed_at` con backoff observable.
- [ ] Integrar Resend/Stripe en entorno de staging con secretos reales y webhook firmado.
- [ ] Panel OS de Tienda, checkout, Pedidos y estado de entrega.
- [ ] Reembolso, chargeback, revocación manual y política de retención de grants.
- [ ] Migrar archivos legacy de `/uploads` a storage privado y retirar el servicio estático público.

## Definition of Done

- No se concede acceso desde el cliente.
- Reintentos del proveedor no duplican órdenes, grants ni correos exitosos.
- Descarga privada verifica estado, expiración y confinamiento al storage.
- Las tareas humanas permanecen explícitas hasta disponer de credenciales, UI y pruebas E2E.

## Evidencia

- `npm test`: 28 tests Rust PASS.
- `cargo check` y `npx tsc --noEmit`: PASS.
- `npm --prefix frontend run test:full`: 382 tests en 49 suites PASS.
