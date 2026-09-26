use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct LightweightManagerInventoryReport {
    pub(crate) target: String,
    pub(crate) target_ip: String,
    pub(crate) sites: Vec<LightweightManagerSite>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LightweightManagerSite {
    pub(crate) deployment_id: String,
    pub(crate) name: String,
    pub(crate) status: String,
    pub(crate) fqdn: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LightweightManagerProvisionStaticReport {
    pub(crate) target_ip: String,
    pub(crate) deployment_id: String,
    pub(crate) public_url: String,
    pub(crate) access_user: String,
    pub(crate) access_password: String,
    pub(crate) access_port: u16,
}
