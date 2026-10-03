/* [01AA-4-f3l] DTOs de Contabo Domains & DNS (extraido de contabo_domains.rs).
 * ListResponse/AuthCodeEntry/req_id son pub(crate): solo los usan los
 * submodulos hermanos (domains, handles, dns). */

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/* Tipos: Dominios. */

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContaboDomain {
    pub sld: Option<String>,
    pub tld: Option<String>,
    pub status: Option<String>,
    pub handles: Option<DomainHandles>,
    pub nameservers: Option<Vec<Nameserver>>,
    #[serde(rename = "createdDate")]
    pub created_date: Option<String>,
    #[serde(rename = "paidUntil")]
    pub paid_until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DomainHandles {
    pub owner: Option<String>,
    pub admin: Option<String>,
    pub tech: Option<String>,
    pub zone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Nameserver {
    pub hostname: Option<String>,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
}

/* Tipos: Handles (contactos WHOIS). */

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContaboHandle {
    pub handle_id: Option<String>,
    pub handle_type: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub organization: Option<String>,
    pub email: Option<String>,
    pub gender: Option<String>,
    pub address: Option<HandleAddress>,
    pub phone: Option<HandlePhone>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandleAddress {
    pub street: Option<String>,
    pub street_number: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub zip_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HandlePhone {
    pub prefix: Option<String>,
    pub number: Option<String>,
}

/* Tipos: DNS. */

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DnsZone {
    pub zone_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DnsRecord {
    pub record_id: Option<i64>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub record_type: Option<String>,
    pub ttl: Option<i64>,
    pub prio: Option<i64>,
    pub data: Option<String>,
    pub port: Option<i64>,
    pub weight: Option<i64>,
    pub flag: Option<i64>,
    pub tag: Option<String>,
}

/* Tipos: Requests. */

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderDomainRequest {
    pub domain: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_code: Option<String>,
    pub handles: DomainHandles,
    pub nameservers: Vec<Nameserver>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHandleRequest {
    pub handle_type: String,
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    pub email: String,
    pub gender: String,
    pub address: HandleAddress,
    pub phone: HandlePhone,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDnsRecordRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub record_type: String,
    pub ttl: i64,
    pub prio: i64,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDnsRecordRequest {
    #[serde(rename = "type")]
    pub record_type: String,
    pub ttl: i64,
    pub prio: i64,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/* Responses internas (wrapping Contabo). */

#[derive(Debug, Deserialize)]
pub(crate) struct ListResponse<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
pub(crate) struct AuthCodeEntry {
    #[serde(rename = "authCode")]
    pub(crate) auth_code: String,
}

/* Helper para request ID. */

pub(crate) fn req_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
