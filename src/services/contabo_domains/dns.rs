/* [01AA-4-f3l] Zonas y registros DNS Contabo (extraido de contabo_domains.rs). */

use super::super::contabo::{ContaboService, API_BASE};
use super::types::{
    req_id, CreateDnsRecordRequest, DnsRecord, DnsZone, ListResponse, UpdateDnsRecordRequest,
};

impl ContaboService {
    /// Listar todas las zonas DNS.
    pub async fn list_dns_zones(&self) -> Result<Vec<DnsZone>, String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .get(format!("{API_BASE}/dns/zones"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .query(&[("size", "100")])
            .send()
            .await
            .map_err(|e| format!("List DNS zones failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("List DNS zones error: {body}"));
        }

        let data: ListResponse<DnsZone> = resp
            .json()
            .await
            .map_err(|e| format!("DNS zone parse error: {e}"))?;

        Ok(data.data)
    }

    /// Crear una zona DNS.
    pub async fn create_dns_zone(&self, zone_name: &str) -> Result<DnsZone, String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .post(format!("{API_BASE}/dns/zones"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .json(&serde_json::json!({"zoneName": zone_name}))
            .send()
            .await
            .map_err(|e| format!("Create DNS zone failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Create DNS zone error: {body}"));
        }

        let data: ListResponse<DnsZone> = resp
            .json()
            .await
            .map_err(|e| format!("DNS zone parse error: {e}"))?;

        data.data
            .into_iter()
            .next()
            .ok_or_else(|| "Create DNS zone: empty response".into())
    }

    /// Eliminar una zona DNS.
    pub async fn delete_dns_zone(&self, zone_name: &str) -> Result<(), String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .delete(format!("{API_BASE}/dns/zones/{zone_name}"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .send()
            .await
            .map_err(|e| format!("Delete DNS zone failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Delete DNS zone {zone_name} error: {body}"));
        }

        Ok(())
    }

    /// Listar registros de una zona DNS.
    pub async fn list_dns_records(&self, zone_name: &str) -> Result<Vec<DnsRecord>, String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .get(format!("{API_BASE}/dns/zones/{zone_name}/records"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .query(&[("size", "100")])
            .send()
            .await
            .map_err(|e| format!("List DNS records failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("List DNS records for {zone_name} error: {body}"));
        }

        let data: ListResponse<DnsRecord> = resp
            .json()
            .await
            .map_err(|e| format!("DNS record parse error: {e}"))?;

        Ok(data.data)
    }

    /// Crear un registro DNS en una zona.
    pub async fn create_dns_record(
        &self,
        zone_name: &str,
        req: &CreateDnsRecordRequest,
    ) -> Result<DnsRecord, String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .post(format!("{API_BASE}/dns/zones/{zone_name}/records"))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Create DNS record failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Create DNS record error: {status} — {body}"));
        }

        let data: ListResponse<DnsRecord> = resp
            .json()
            .await
            .map_err(|e| format!("DNS record parse error: {e}"))?;

        data.data
            .into_iter()
            .next()
            .ok_or_else(|| "Create DNS record: empty response".into())
    }

    /// Actualizar un registro DNS.
    pub async fn update_dns_record(
        &self,
        zone_name: &str,
        record_id: i64,
        req: &UpdateDnsRecordRequest,
    ) -> Result<DnsRecord, String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .patch(format!(
                "{API_BASE}/dns/zones/{zone_name}/records/{record_id}"
            ))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Update DNS record failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Update DNS record {record_id} error: {body}"));
        }

        let data: ListResponse<DnsRecord> = resp
            .json()
            .await
            .map_err(|e| format!("DNS record parse error: {e}"))?;

        data.data
            .into_iter()
            .next()
            .ok_or_else(|| format!("Update DNS record {record_id}: empty response"))
    }

    /// Eliminar un registro DNS.
    pub async fn delete_dns_record(&self, zone_name: &str, record_id: i64) -> Result<(), String> {
        let token = self.get_token().await?;

        let resp = self
            .client
            .delete(format!(
                "{API_BASE}/dns/zones/{zone_name}/records/{record_id}"
            ))
            .bearer_auth(&token)
            .header("x-request-id", req_id())
            .send()
            .await
            .map_err(|e| format!("Delete DNS record failed: {e}"))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Delete DNS record {record_id} error: {body}"));
        }

        Ok(())
    }
}
