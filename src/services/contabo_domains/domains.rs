/* [01AA-4-f3l] Operaciones de dominios Contabo (extraido de contabo_domains.rs).
 * Disponibilidad, CRUD, cancelacion y auth-code de transferencia. */

use tracing::error;

use super::super::contabo::{API_BASE, ContaboService};
use super::types::{
    AuthCodeEntry, ContaboDomain, DomainHandles, ListResponse, Nameserver, OrderDomainRequest,
    req_id,
};

impl ContaboService {
    /// Verificar si un dominio está disponible para registro.
    /// Retorna true si disponible (204), false si no (404).
    pub async fn check_domain_availability(&self, domain: &str) -> Result<bool, String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .post(format!(
                "{API_BASE}/registries-domains/{domain}/check-availability"
            ))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .send()
            .await
            .map_err(|e| format!("Domain availability check failed: {e}"))?;

        match resp.status().as_u16() {
            204 => Ok(true),
            404 => Ok(false),
            status => {
                let body = resp.text().await.unwrap_or_default();
                error!("Domain availability check error {status}: {body}");
                Err(format!("Domain check error: {status}"))
            }
        }
    }

    /// Listar todos los dominios registrados en Contabo.
    pub async fn list_domains(&self) -> Result<Vec<ContaboDomain>, String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .get(format!("{API_BASE}/domains"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .query(&[("size", "100")])
            .send()
            .await
            .map_err(|e| format!("List domains failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("List domains error: {body}"));
        }

        let data: ListResponse<ContaboDomain> = resp
            .json()
            .await
            .map_err(|e| format!("Domain parse error: {e}"))?;

        Ok(data.data)
    }

    /// Obtener detalles de un dominio específico.
    pub async fn get_domain(&self, domain: &str) -> Result<ContaboDomain, String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .get(format!("{API_BASE}/domains/{domain}"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .send()
            .await
            .map_err(|e| format!("Get domain failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Get domain {domain} error: {body}"));
        }

        let data: ListResponse<ContaboDomain> = resp
            .json()
            .await
            .map_err(|e| format!("Domain parse error: {e}"))?;

        data.data
            .into_iter()
            .next()
            .ok_or_else(|| format!("Domain {domain} not found"))
    }

    /// Registrar o transferir un dominio.
    pub async fn order_domain(&self, req: &OrderDomainRequest) -> Result<ContaboDomain, String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .post(format!("{API_BASE}/domains"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Order domain failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!("Order domain error {status}: {body}");
            return Err(format!("Order domain error: {status} — {body}"));
        }

        let data: ListResponse<ContaboDomain> = resp
            .json()
            .await
            .map_err(|e| format!("Domain parse error: {e}"))?;

        data.data
            .into_iter()
            .next()
            .ok_or_else(|| "Order domain: empty response".into())
    }

    /// Actualizar nameservers y handles de un dominio.
    pub async fn update_domain(
        &self,
        domain: &str,
        nameservers: Option<Vec<Nameserver>>,
        handles: Option<DomainHandles>,
    ) -> Result<ContaboDomain, String> {
        let token = self.get_token().await?;

        let body = serde_json::json!({
            "nameservers": nameservers,
            "handles": handles,
        });

        let resp = self
            .client
            .patch(format!("{API_BASE}/domains/{domain}"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Update domain failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Update domain {domain} error: {body}"));
        }

        let data: ListResponse<ContaboDomain> = resp
            .json()
            .await
            .map_err(|e| format!("Domain parse error: {e}"))?;

        data.data
            .into_iter()
            .next()
            .ok_or_else(|| format!("Update domain {domain}: empty response"))
    }

    /// Cancelar un dominio.
    pub async fn cancel_domain(&self, domain: &str, reason: Option<&str>) -> Result<(), String> {
        let token = self.get_token().await?;

        let body = serde_json::json!({
            "reason": reason.unwrap_or("Product not needed anymore"),
        });

        let resp = self
            .client
            .post(format!("{API_BASE}/domains/{domain}/cancel"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Cancel domain failed: {e}"))?;

        if !resp.status().is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(format!("Cancel domain {domain} error: {body_text}"));
        }

        Ok(())
    }

    /// Obtener auth code para transferencia saliente.
    pub async fn get_domain_auth_code(&self, domain: &str) -> Result<String, String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .post(format!("{API_BASE}/domains/{domain}/generate-auth-code"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .send()
            .await
            .map_err(|e| format!("Get auth code failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Auth code error for {domain}: {body}"));
        }

        let data: ListResponse<AuthCodeEntry> = resp
            .json()
            .await
            .map_err(|e| format!("Auth code parse error: {e}"))?;

        data.data
            .into_iter()
            .next()
            .map(|e| e.auth_code)
            .ok_or_else(|| format!("No auth code for {domain}"))
    }
}
