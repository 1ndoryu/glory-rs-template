/* [154A-1] Contabo Domains & DNS API.
 * Extension del servicio Contabo para gestion de dominios, handles y zonas DNS.
 * Usa la misma autenticacion OAuth2 de ContaboService (get_token).
 * API docs: https://api.contabo.com/ — seccion Domains, Handles, DNS. */

/* [01AA-4-f3l] Partido por area: services/contabo_domains.rs 673L superaba
 * limite 500 (god-object-rs). domains = CRUD dominios + availability +
 * auth-code; handles = contactos WHOIS; dns = zonas y registros;
 * types = DTOs + ListResponse/AuthCodeEntry + req_id.
 * Gotcha: ContaboService vive en services::contabo; aqui solo bloques impl.
 * Pendiente: resto 500-650 (prerender, payments, deployments, hosting_domains). */

pub mod dns;
pub mod domains;
pub mod handles;
pub mod types;

pub use types::{
    ContaboDomain, ContaboHandle, CreateDnsRecordRequest, CreateHandleRequest, DnsRecord, DnsZone,
    DomainHandles, HandleAddress, HandlePhone, Nameserver, OrderDomainRequest,
    UpdateDnsRecordRequest,
};
