use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CfResponse<T> {
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<CfApiError>,
    pub result: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct CfApiError {
    #[serde(default)]
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct CfTokenVerifyResult {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CfZone {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub name_servers: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CfAccount {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CfCreateZonePayload {
    pub name: String,
    pub account: CfAccountRef,
}

#[derive(Debug, Serialize)]
pub struct CfAccountRef {
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CfDnsRecord {
    pub id: String,
    pub r#type: String,
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub proxied: bool,
    #[serde(default)]
    pub ttl: i32,
    pub created_on: Option<String>,
    pub modified_on: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CfSslSetting {
    pub id: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct CfPurgeCachePayload {
    pub purge_everything: bool,
}

#[derive(Debug, Deserialize)]
pub struct CfPurgeCacheResult {
    pub id: String,
}
