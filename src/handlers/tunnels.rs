use axum::{extract::State, response::IntoResponse, Json};
use futures::future::join_all;
use serde::Serialize;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct UnifiedTunnelResponse {
    pub account_name: String,
    pub account_id: String,
    pub tunnel_id: String,
    pub name: String,
    pub status: String,
    pub created_at: Option<String>,
}

pub async fn list_all_tunnels(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let accounts = state.db.list_all_accounts_with_tokens().await?;

    let tasks = accounts.into_iter().map(|acc| {
        let state = state.clone();
        async move {
            let token = match state.crypto.decrypt(&acc.encrypted_token) {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("Failed to decrypt token for account {}: {:?}", acc.id, e);
                    return Vec::new();
                }
            };

            // Get all Cloudflare account IDs associated with this token
            let cf_accounts = match state.cf.get_cf_accounts(&token).await {
                Ok(accounts) => accounts,
                Err(e) => {
                    tracing::warn!("Failed to fetch Cloudflare accounts for account {}: {:?}", acc.id, e);
                    return Vec::new();
                }
            };

            let mut tunnels_list = Vec::new();
            for cf_acc in cf_accounts {
                match state.cf.list_tunnels(&token, &cf_acc.id).await {
                    Ok(tunnels) => {
                        for t in tunnels {
                            tunnels_list.push(UnifiedTunnelResponse {
                                account_name: acc.account_name.clone(),
                                account_id: cf_acc.id.clone(),
                                tunnel_id: t.id,
                                name: t.name,
                                status: t.status,
                                created_at: t.created_at,
                            });
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to list tunnels for account_id {}: {:?}", cf_acc.id, e);
                    }
                }
            }

            tunnels_list
        }
    });

    let results = join_all(tasks).await;
    let unified_tunnels: Vec<UnifiedTunnelResponse> = results.into_iter().flatten().collect();

    Ok(Json(unified_tunnels))
}
