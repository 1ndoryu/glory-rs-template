use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use rand::Rng;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{HostingPlanConfig, UserRole};
use crate::repositories::HostingRepository;
use crate::services::{
    CoolifyConfig, HostingRuntimeKind, HostingRuntimeService, HostingRuntimeUpdate,
};
use crate::AppState;

/* [165A-21] Los dominios custom ya no se activan en Coolify en cuanto se guardan.
 * Ahora quedan en pending_verification hasta que el usuario publique el TXT de ownership,
 * y solo entonces propagamos la ruta/SSL al compose del hosting.
 * Gotcha: si el cliente cambia un dominio ya activo, primero retiramos la ruta anterior.
 * Pendiente: validar el flujo completo con un dominio real en staging/producción controlada. */
pub(super) const DOMAIN_STATUS_NONE: &str = "none";
pub(super) const DOMAIN_STATUS_PENDING: &str = "pending_verification";
pub(super) const DOMAIN_STATUS_VERIFIED: &str = "verified";
pub(super) const DOMAIN_STATUS_ACTIVE: &str = "active";

pub(super) fn normalize_domain(domain: Option<&str>) -> Option<String> {
    domain
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
}

fn generate_domain_verification_token() -> String {
    let token: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(24)
        .map(char::from)
        .collect();
    format!("nakomi-verification={}", token.to_ascii_lowercase())
}

pub(super) fn build_domain_verification_state(
    domain: Option<&str>,
) -> (
    String,
    Option<String>,
    Option<chrono::DateTime<chrono::Utc>>,
) {
    if domain.is_some() {
        (
            DOMAIN_STATUS_PENDING.to_string(),
            Some(generate_domain_verification_token()),
            None,
        )
    } else {
        (DOMAIN_STATUS_NONE.to_string(), None, None)
    }
}

pub(super) fn domain_verification_txt_host(domain: &str) -> String {
    format!("_nakomi-verify.{domain}")
}

pub(super) fn domain_ready_for_route(sub: &crate::models::HostingSubscription) -> Option<&str> {
    let routable = sub.domain_verification_status == DOMAIN_STATUS_VERIFIED
        || sub.domain_verification_status == DOMAIN_STATUS_ACTIVE;
    if !routable {
        return None;
    }
    sub.domain.as_deref().filter(|domain| !domain.is_empty())
}

pub(super) fn active_custom_domain(sub: &crate::models::HostingSubscription) -> Option<&str> {
    if sub.domain_verification_status != DOMAIN_STATUS_ACTIVE {
        return None;
    }
    sub.domain.as_deref().filter(|domain| !domain.is_empty())
}

async fn lookup_txt_records(record_name: &str) -> Result<Vec<String>, String> {
    let resolver = hickory_resolver::TokioAsyncResolver::tokio_from_system_conf()
        .map_err(|error| format!("No se pudo inicializar el resolver DNS: {error}"))?;
    let lookup = resolver
        .txt_lookup(record_name)
        .await
        .map_err(|error| format!("No se pudo consultar el registro TXT: {error}"))?;

    let mut values = lookup
        .iter()
        .map(|txt| {
            let value = txt
                .txt_data()
                .iter()
                .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
                .collect::<String>();
            value.trim().to_string()
        })
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    Ok(values)
}

pub(super) async fn sync_custom_domain_route(
    http_client: &reqwest::Client,
    config: Option<&CoolifyConfig>,
    runtime_kind: HostingRuntimeKind,
    update: HostingRuntimeUpdate<'_>,
) -> Result<(), AppError> {
    HostingRuntimeService::update_deployment(http_client, config, Some(runtime_kind), update).await
}

pub(super) fn compose_update_from_subscription<'a>(
    sub: &'a crate::models::HostingSubscription,
    custom_domain: Option<&'a str>,
    plan_config: &'a HostingPlanConfig,
) -> Option<HostingRuntimeUpdate<'a>> {
    Some(HostingRuntimeUpdate {
        deployment_id: sub.deployment_id_or_legacy()?,
        deployment_name: sub.coolify_site_name.as_deref()?,
        custom_domain,
        access_user: sub.sftp_user.as_deref()?,
        access_password: sub.sftp_password.as_deref()?,
        access_port: sub.sftp_port?,
        plan_config,
    })
}

struct DomainVerificationResponse<'a> {
    verified: bool,
    applied: bool,
    status: &'a str,
    domain: &'a str,
    txt_host: &'a str,
    txt_value: Option<&'a str>,
    txt_records: &'a [String],
    message: &'a str,
}

pub(super) struct DomainActivation<'a> {
    pub subscription_id: Uuid,
    pub runtime_kind: HostingRuntimeKind,
    pub token: Option<&'a str>,
    pub verified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub update: HostingRuntimeUpdate<'a>,
}

fn domain_verification_response(payload: &DomainVerificationResponse<'_>) -> serde_json::Value {
    serde_json::json!({
        "verified": payload.verified,
        "applied": payload.applied,
        "status": payload.status,
        "domain": payload.domain,
        "txt_host": payload.txt_host,
        "txt_value": payload.txt_value,
        "txt_records": payload.txt_records,
        "message": payload.message,
    })
}

fn active_domain_response(
    domain: &str,
    txt_host: &str,
    txt_value: Option<&str>,
) -> serde_json::Value {
    domain_verification_response(&DomainVerificationResponse {
        verified: true,
        applied: true,
        status: DOMAIN_STATUS_ACTIVE,
        domain,
        txt_host,
        txt_value,
        txt_records: &[],
        message: "El dominio ya está verificado y activo en el hosting.",
    })
}

async fn resolve_domain_activation(
    state: &AppState,
    subscription_id: Uuid,
    sub: &crate::models::HostingSubscription,
    domain: &str,
    expected_token: &str,
    verified_at: chrono::DateTime<chrono::Utc>,
) -> Result<(bool, &'static str, String), AppError> {
    let pending_message =
        "Dominio verificado. Se activará automáticamente en el siguiente provisioning o refresh.";
    let active_message =
        "Dominio verificado y activado en el hosting. El SSL podrá emitirse cuando los registros A apunten al servidor.";
    let fallback_message =
        "Dominio verificado, pero la activación en hosting quedó pendiente. Usa refresh cuando el servicio esté listo.";

    let Some(plan_config) = HostingRepository::get_plan_config(&state.pool, &sub.plan).await?
    else {
        return Ok((false, DOMAIN_STATUS_VERIFIED, pending_message.to_string()));
    };
    let Some(update) = compose_update_from_subscription(sub, Some(domain), &plan_config) else {
        return Ok((false, DOMAIN_STATUS_VERIFIED, pending_message.to_string()));
    };

    let applied = activate_domain_route(
        state,
        DomainActivation {
            subscription_id,
            runtime_kind: HostingRuntimeKind::from_persisted(&sub.runtime_kind),
            token: Some(expected_token),
            verified_at: Some(verified_at),
            update,
        },
    )
    .await;

    if applied {
        Ok((true, DOMAIN_STATUS_ACTIVE, active_message.to_string()))
    } else {
        Ok((false, DOMAIN_STATUS_VERIFIED, fallback_message.to_string()))
    }
}

async fn mark_domain_active(
    state: &AppState,
    subscription_id: Uuid,
    token: Option<&str>,
    verified_at: Option<chrono::DateTime<chrono::Utc>>,
) {
    if let Err(error) = HostingRepository::update_domain_verification(
        &state.pool,
        subscription_id,
        DOMAIN_STATUS_ACTIVE,
        token,
        Some(verified_at.unwrap_or_else(Utc::now)),
    )
    .await
    {
        tracing::warn!("No se pudo marcar dominio activo para {subscription_id}: {error}");
    }
}

pub(super) async fn activate_domain_route(
    state: &AppState,
    activation: DomainActivation<'_>,
) -> bool {
    let Some(custom_domain) = activation.update.custom_domain else {
        return false;
    };
    let Ok(config) = HostingRuntimeService::optional_target_config_for(
        activation.runtime_kind,
        state.coolify_config.as_ref(),
        "activar dominios custom",
    ) else {
        return false;
    };

    match sync_custom_domain_route(
        &state.http_client,
        config,
        activation.runtime_kind,
        activation.update,
    )
    .await
    {
        Ok(()) => {
            mark_domain_active(
                state,
                activation.subscription_id,
                activation.token,
                activation.verified_at,
            )
            .await;
            true
        }
        Err(error) => {
            tracing::warn!(
                "No se pudo activar el dominio '{}' para {}: {}",
                custom_domain,
                activation.subscription_id,
                error,
            );
            false
        }
    }
}

/* [154A-16] Verificación DNS: resuelve el dominio y compara con server_ip.
 * Retorna qué IPs apuntan al dominio y si coinciden con el servidor. */
#[utoipa::path(
    get,
    path = "/api/hosting/subscriptions/{id}/dns-check",
    params(("id" = Uuid, Path, description = "ID suscripción")),
    responses(
        (status = 200, description = "Resultado verificación DNS", body = serde_json::Value),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn dns_check(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;

    if auth.effective_role != UserRole::Admin && sub.user_id != Some(auth.user_id) {
        return Err(AppError::Forbidden("Sin permisos".into()));
    }

    let domain = match &sub.domain {
        Some(d) if !d.is_empty() => d.clone(),
        _ => {
            return Ok(Json(serde_json::json!({
                "configured": false,
                "message": "No hay dominio configurado"
            })));
        }
    };

    let server_ip = sub.server_ip.clone().unwrap_or_default();
    let lookup_target = format!("{domain}:80");
    let resolved_ips: Vec<String> = match tokio::net::lookup_host(&lookup_target).await {
        Ok(addrs) => addrs.map(|a| a.ip().to_string()).collect(),
        Err(e) => {
            return Ok(Json(serde_json::json!({
                "configured": true,
                "domain": domain,
                "resolved": false,
                "error": format!("No se pudo resolver el dominio: {e}"),
                "expected_ip": server_ip,
            })));
        }
    };

    let mut unique_ips: Vec<String> = resolved_ips.clone();
    unique_ips.sort();
    unique_ips.dedup();

    let points_to_server = !server_ip.is_empty() && unique_ips.contains(&server_ip);

    Ok(Json(serde_json::json!({
        "configured": true,
        "domain": domain,
        "resolved": true,
        "resolved_ips": unique_ips,
        "expected_ip": server_ip,
        "points_to_server": points_to_server,
        "ssl_provider": "Let's Encrypt (automático vía Coolify)",
    })))
}

#[utoipa::path(
    post,
    path = "/api/hosting/subscriptions/{id}/verify-domain",
    params(("id" = Uuid, Path, description = "ID suscripción")),
    responses(
        (status = 200, description = "Resultado verificación ownership", body = serde_json::Value),
    ),
    security(("bearer_auth" = [])),
    tag = "hosting"
)]
pub(super) async fn verify_domain(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sub = HostingRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound("Suscripción no encontrada".into()))?;

    if auth.effective_role != UserRole::Admin && sub.user_id != Some(auth.user_id) {
        return Err(AppError::Forbidden("Sin permisos".into()));
    }

    let domain = sub
        .domain
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Validation("No hay dominio configurado para verificar".into()))?;
    let txt_host = domain_verification_txt_host(domain);

    if sub.domain_verification_status == DOMAIN_STATUS_ACTIVE {
        return Ok(Json(active_domain_response(
            domain,
            &txt_host,
            sub.domain_verification_token.as_deref(),
        )));
    }

    let expected_token = sub.domain_verification_token.as_deref().ok_or_else(|| {
        AppError::Validation("No hay un token de verificación pendiente para este dominio".into())
    })?;
    let txt_records = match lookup_txt_records(&txt_host).await {
        Ok(records) => records,
        Err(error) => {
            return Ok(Json(domain_verification_response(
                &DomainVerificationResponse {
                    verified: false,
                    applied: false,
                    status: &sub.domain_verification_status,
                    domain,
                    txt_host: &txt_host,
                    txt_value: Some(expected_token),
                    txt_records: &[],
                    message: &error,
                },
            )));
        }
    };

    let verified = txt_records.iter().any(|value| value == expected_token);
    if !verified {
        return Ok(Json(domain_verification_response(
            &DomainVerificationResponse {
                verified: false,
                applied: false,
                status: &sub.domain_verification_status,
                domain,
                txt_host: &txt_host,
                txt_value: Some(expected_token),
                txt_records: &txt_records,
                message: "El TXT todavía no coincide con el token esperado. Revisa el registro y espera la propagación.",
            },
        )));
    }

    let verified_at = Utc::now();
    let (applied, next_status, message) =
        resolve_domain_activation(&state, id, &sub, domain, expected_token, verified_at).await?;

    let updated = HostingRepository::update_domain_verification(
        &state.pool,
        id,
        next_status,
        Some(expected_token),
        Some(verified_at),
    )
    .await?;

    if let Err(error) = HostingRepository::add_event(
        &state.pool,
        id,
        "domain_verified",
        Some(serde_json::json!({
            "domain": domain,
            "txt_host": txt_host,
            "applied": applied,
            "status": updated.domain_verification_status,
            "by": auth.user_id.to_string(),
        })),
    )
    .await
    {
        tracing::warn!("Error registrando evento domain_verified para {id}: {error}");
    }

    Ok(Json(domain_verification_response(
        &DomainVerificationResponse {
            verified: true,
            applied,
            status: &updated.domain_verification_status,
            domain,
            txt_host: &txt_host,
            txt_value: Some(expected_token),
            txt_records: &txt_records,
            message: &message,
        },
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_domain_verification_state_creates_pending_token_for_domain() {
        let (status, token, verified_at) = build_domain_verification_state(Some("example.com"));

        assert_eq!(status, DOMAIN_STATUS_PENDING);
        assert!(token
            .as_deref()
            .is_some_and(|value| value.starts_with("nakomi-verification=")));
        assert!(verified_at.is_none());
    }

    #[test]
    fn domain_verification_txt_host_uses_expected_prefix() {
        assert_eq!(
            domain_verification_txt_host("example.com"),
            "_nakomi-verify.example.com"
        );
    }
}
