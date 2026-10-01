/* [084A-24] Servicio de suscripciones Stripe para hosting.
 * Crea Checkout Sessions en modo subscription usando price_data dinámico desde BD.
 * Maneja webhooks de invoice.paid y customer.subscription.* para sincronizar estado.
 * Gotcha: NO usa PaymentIntents directos — usa Stripe Subscriptions nativas.
 * [094A-9] Auditoría de seguridad: validación estricta de campos JSON en webhooks,
 * idempotency key en checkout, verificación de status antes de activar.
 * [01AA-4-F3i] Partido por etapa del flujo (era god-object 700+): types = structs
 * públicos; helpers = copy + config; provision = auto-provisioning runtime;
 * checkout = Stripe checkout + webhooks. */

mod checkout;
mod helpers;
mod provision;
mod types;

pub use types::{CheckoutParams, HostingStripeService};
