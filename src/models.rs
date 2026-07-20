use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub account_name: String,
    pub api_token: String,
}

#[derive(Debug, Serialize)]
pub struct AccountResponse {
    pub id: String,
    pub account_name: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateZoneRequest {
    pub account_id: String,
    pub domain_name: String,
    pub cf_account_id: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ZoneSummaryResponse {
    pub account_name: String,
    pub account_id: String,
    pub name: String,
    pub status: String,
    pub id: String,
    pub name_servers: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateDnsRecordRequest {
    pub r#type: String,
    pub name: String,
    #[serde(default = "default_content")]
    pub content: String,
    #[serde(default = "default_ttl")]
    pub ttl: i32,
    #[serde(default = "default_proxied")]
    pub proxied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

fn default_content() -> String {
    String::new()
}

fn default_ttl() -> i32 {
    1
}

fn default_proxied() -> bool {
    false
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateSslRequest {
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct PurgeCacheRequest {
    #[serde(default = "default_purge_everything")]
    pub purge_everything: bool,
}

fn default_purge_everything() -> bool {
    true
}
