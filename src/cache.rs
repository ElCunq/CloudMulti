use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use sha2::{Sha256, Digest};

#[derive(Clone)]
struct CacheEntry {
    data: serde_json::Value,
    expires_at: Instant,
}

#[derive(Clone)]
pub struct AppCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl AppCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn make_key(&self, prefix: &str, token: &str, subkey: Option<&str>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let token_hash = hex::encode(hasher.finalize());
        
        if let Some(sk) = subkey {
            format!("{}:{}:{}", prefix, token_hash, sk)
        } else {
            format!("{}:{}", prefix, token_hash)
        }
    }

    pub async fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            if entry.expires_at > Instant::now() {
                if let Ok(val) = serde_json::from_value(entry.data.clone()) {
                    return Some(val);
                }
            }
        }
        None
    }

    pub async fn set<T: serde::Serialize>(&self, key: &str, val: T, ttl: Duration) {
        if let Ok(json_val) = serde_json::to_value(val) {
            let mut entries = self.entries.write().await;
            entries.insert(
                key.to_string(),
                CacheEntry {
                    data: json_val,
                    expires_at: Instant::now() + ttl,
                },
            );
        }
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
        tracing::info!("In-memory Cloudflare API cache cleared.");
    }
}
