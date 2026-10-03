/* [01AA-4-f3l] Handles Contabo / contactos WHOIS (extraido de contabo_domains.rs). */

use super::super::contabo::{API_BASE, ContaboService};
use super::types::{ContaboHandle, CreateHandleRequest, ListResponse, req_id};

impl ContaboService {
    /// Listar todos los handles (contactos registrados en Contabo).
    pub async fn list_handles(&self) -> Result<Vec<ContaboHandle>, String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .get(format!("{API_BASE}/domains/handles"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .query(&[("size", "100")])
            .send()
            .await
            .map_err(|e| format!("List handles failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("List handles error: {body}"));
        }

        let data: ListResponse<ContaboHandle> = resp
            .json()
            .await
            .map_err(|e| format!("Handle parse error: {e}"))?;

        Ok(data.data)
    }

    /// Crear un handle (contacto para dominios).
    pub async fn create_handle(&self, req: &CreateHandleRequest) -> Result<ContaboHandle, String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .post(format!("{API_BASE}/domains/handles"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Create handle failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Create handle error: {status} — {body}"));
        }

        let data: ListResponse<ContaboHandle> = resp
            .json()
            .await
            .map_err(|e| format!("Handle parse error: {e}"))?;

        data.data
            .into_iter()
            .next()
            .ok_or_else(|| "Create handle: empty response".into())
    }
}
