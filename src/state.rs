use crate::cloudflare::CloudflareClient;
use crate::crypto::Crypto;
use crate::db::{AccountRecord, Db};
use crate::error::AppError;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub crypto: Arc<Crypto>,
    pub cf: CloudflareClient,
}

impl AppState {
    pub async fn resolve_token_for_zone(&self, zone_id: &str) -> Result<(AccountRecord, String), AppError> {
        // 1. Check cache in database
        if let Some(record) = self.db.get_account_for_zone(zone_id).await? {
            match self.crypto.decrypt(&record.encrypted_token) {
                Ok(token) => return Ok((record, token)),
                Err(e) => {
                    tracing::warn!("Failed to decrypt cached token for account {}: {:?}", record.id, e);
                }
            }
        }

        // 2. If not found in cache or decryption failed, check all accounts
        let accounts = self.db.list_all_accounts_with_tokens().await?;
        if accounts.is_empty() {
            return Err(AppError::ZoneNotFound(format!(
                "No Cloudflare accounts configured to check for zone {}",
                zone_id
            )));
        }

        for record in accounts {
            if let Ok(token) = self.crypto.decrypt(&record.encrypted_token) {
                // Try fetching dns records or zone info to see if this token has access to zone_id
                if self.cf.list_dns_records(&token, zone_id).await.is_ok() {
                    let _ = self.db.upsert_zone_cache(zone_id, &record.id).await;
                    return Ok((record, token));
                }
            }
        }

        Err(AppError::ZoneNotFound(format!(
            "Zone {} not found or not accessible with any saved Cloudflare account",
            zone_id
        )))
    }
}
