use crate::cloudflare::models::*;
use crate::error::AppError;
use reqwest::Client;

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";

#[derive(Clone, Debug)]
pub struct CloudflareClient {
    http: Client,
}

impl CloudflareClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .user_agent("Self-Hosted-Cloudflare-Dashboard/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self { http }
    }

    fn check_response<T>(cf_resp: CfResponse<T>) -> Result<T, AppError> {
        if cf_resp.success {
            if let Some(result) = cf_resp.result {
                Ok(result)
            } else {
                Err(AppError::CloudflareApi(
                    "Cloudflare returned success but no result data".to_string(),
                ))
            }
        } else {
            let error_msg = cf_resp
                .errors
                .into_iter()
                .map(|e| format!("[{}] {}", e.code, e.message))
                .collect::<Vec<_>>()
                .join("; ");
            let msg = if error_msg.is_empty() {
                "Cloudflare API error".to_string()
            } else {
                error_msg
            };
            Err(AppError::CloudflareApi(msg))
        }
    }

    pub async fn verify_token(&self, token: &str) -> Result<CfTokenVerifyResult, AppError> {
        let url = format!("{}/user/tokens/verify", CF_API_BASE);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(AppError::InvalidToken);
        }

        let cf_resp = resp.json::<CfResponse<CfTokenVerifyResult>>().await?;
        let result = Self::check_response(cf_resp)?;

        if result.status.to_lowercase() != "active" {
            return Err(AppError::InvalidToken);
        }

        Ok(result)
    }

    pub async fn list_zones(&self, token: &str) -> Result<Vec<CfZone>, AppError> {
        let url = format!("{}/zones", CF_API_BASE);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?
            .json::<CfResponse<Vec<CfZone>>>()
            .await?;

        Self::check_response(resp)
    }

    pub async fn get_cf_accounts(&self, token: &str) -> Result<Vec<CfAccount>, AppError> {
        let url = format!("{}/accounts", CF_API_BASE);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?
            .json::<CfResponse<Vec<CfAccount>>>()
            .await?;

        Self::check_response(resp)
    }

    pub async fn create_zone(
        &self,
        token: &str,
        domain_name: &str,
        cf_account_id: &str,
    ) -> Result<CfZone, AppError> {
        let url = format!("{}/zones", CF_API_BASE);
        let payload = CfCreateZonePayload {
            name: domain_name.to_string(),
            account: CfAccountRef {
                id: cf_account_id.to_string(),
            },
        };

        let resp = self
            .http
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?
            .json::<CfResponse<CfZone>>()
            .await?;

        Self::check_response(resp)
    }

    pub async fn list_dns_records(&self, token: &str, zone_id: &str) -> Result<Vec<CfDnsRecord>, AppError> {
        let url = format!("{}/zones/{}/dns_records", CF_API_BASE, zone_id);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?
            .json::<CfResponse<Vec<CfDnsRecord>>>()
            .await?;

        Self::check_response(resp)
    }

    pub async fn create_dns_record(
        &self,
        token: &str,
        zone_id: &str,
        payload: &crate::models::CreateDnsRecordRequest,
    ) -> Result<CfDnsRecord, AppError> {
        let url = format!("{}/zones/{}/dns_records", CF_API_BASE, zone_id);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(token)
            .json(payload)
            .send()
            .await?
            .json::<CfResponse<CfDnsRecord>>()
            .await?;

        Self::check_response(resp)
    }

    pub async fn delete_dns_record(
        &self,
        token: &str,
        zone_id: &str,
        record_id: &str,
    ) -> Result<String, AppError> {
        let url = format!("{}/zones/{}/dns_records/{}", CF_API_BASE, zone_id, record_id);
        let resp = self
            .http
            .delete(&url)
            .bearer_auth(token)
            .send()
            .await?
            .json::<CfResponse<serde_json::Value>>()
            .await?;

        if resp.success {
            Ok(record_id.to_string())
        } else {
            Self::check_response(resp).map(|_| record_id.to_string())
        }
    }

    pub async fn purge_cache(&self, token: &str, zone_id: &str) -> Result<CfPurgeCacheResult, AppError> {
        let url = format!("{}/zones/{}/purge_cache", CF_API_BASE, zone_id);
        let payload = CfPurgeCachePayload {
            purge_everything: true,
        };

        let resp = self
            .http
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?
            .json::<CfResponse<CfPurgeCacheResult>>()
            .await?;

        Self::check_response(resp)
    }

    pub async fn update_ssl_settings(
        &self,
        token: &str,
        zone_id: &str,
        value: &str,
    ) -> Result<CfSslSetting, AppError> {
        let url = format!("{}/zones/{}/settings/ssl", CF_API_BASE, zone_id);
        let payload = serde_json::json!({ "value": value });

        let resp = self
            .http
            .patch(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?
            .json::<CfResponse<CfSslSetting>>()
            .await?;

        Self::check_response(resp)
    }
}
