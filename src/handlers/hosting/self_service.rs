use uuid::Uuid;

use crate::errors::AppError;
use crate::models::HostingSubscription;
use crate::repositories::{CreateHostingParams, HostingRepository, InfrastructureRepository};

/* [01AA-4-f3s] Borrador de suscripción del self-service: agrupa los campos
 * necesarios para persistirla sin pasar una docena de argumentos sueltos.
 * Vive en módulo propio para no engordar checkout.rs (god-object-rs >500). */
pub(super) struct SelfHostingDraft<'a> {
    pub(super) user_id: Uuid,
    pub(super) client_name: &'a str,
    pub(super) client_email: &'a str,
    pub(super) plan: &'a str,
    pub(super) requested_domain: &'a Option<String>,
    pub(super) domain_verification_status: &'a str,
    pub(super) domain_verification_token: &'a Option<String>,
    pub(super) domain_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub(super) runtime_kind: &'a str,
    pub(super) price: i32,
    pub(super) storage: i32,
    pub(super) bandwidth_limit_gb: i32,
}

/* [01AA-4-f3s] Persiste la suscripción del self-service + su límite de ancho
 * de banda (extraído de subscribe_self para el límite de 100 líneas). */
pub(super) async fn persist_self_hosting_subscription(
    pool: &sqlx::PgPool,
    draft: SelfHostingDraft<'_>,
) -> Result<HostingSubscription, AppError> {
    let sub = HostingRepository::create(
        pool,
        CreateHostingParams {
            user_id: Some(draft.user_id),
            client_name: draft.client_name,
            client_email: draft.client_email,
            plan: draft.plan,
            domain: draft.requested_domain.as_deref(),
            domain_verification_status: draft.domain_verification_status,
            domain_verification_token: draft.domain_verification_token.as_deref(),
            domain_verified_at: draft.domain_verified_at,
            runtime_kind: draft.runtime_kind,
            deployment_id: None,
            coolify_site_name: None,
            monthly_price_cents: draft.price,
            storage_limit_mb: draft.storage,
        },
    )
    .await?;
    InfrastructureRepository::set_subscription_bandwidth_limit(
        pool,
        sub.id,
        draft.bandwidth_limit_gb,
    )
    .await?;
    Ok(sub)
}
