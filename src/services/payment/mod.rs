/* [044A-38 Fase 3] Servicio de pagos: integración con Stripe REST API.
 * PaymentIntent con capture_method manual (escrow).
 * 3 modos: full (20% desc), half_half (10% desc), phased (sin desc).
 * Webhook verifica firma HMAC-SHA256 con constant-time comparison.
 *
 * [01AA-4-F3b] Partido por flujo (era god-object 754 + limite 754):
 * checkout.rs (intents + alta post-pago), webhook.rs (eventos + firma),
 * success.rs (máquina de estados post-pago), settlement.rs (captura,
 * listado y reembolsos). Un solo `PaymentService`; este mod lo declara. */

mod checkout;
mod settlement;
mod success;
mod webhook;

pub struct PaymentService;
