use axum::{extract::State, response::IntoResponse, Json};
use futures::future::join_all;
use serde::Serialize;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct UnifiedAnalyticsResponse {
    pub total_requests: i64,
    pub total_bandwidth_bytes: i64,
    pub unique_visitors: i64,
}

pub async fn get_unified_analytics(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let accounts = state.db.list_all_accounts_with_tokens().await?;
    // Get past 1 day (today & yesterday)
    let date_geq = (chrono::Utc::now() - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();

    let mut tasks = Vec::new();

    for acc in accounts {
        let state = state.clone();
        let token = match state.crypto.decrypt(&acc.encrypted_token) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("Failed to decrypt token for account {}: {:?}", acc.id, e);
                continue;
            }
        };

        // Fetch zones for this account
        let zones = match state.cf.list_zones(&token).await {
            Ok(z) => z,
            Err(e) => {
                tracing::warn!("Failed to fetch zones for account {}: {:?}", acc.id, e);
                continue;
            }
        };

        for zone in zones {
            let state = state.clone();
            let token = token.clone();
            let date_geq = date_geq.clone();
            tasks.push(async move {
                match state.cf.get_graphql_analytics(&token, &zone.id, &date_geq).await {
                    Ok(resp) => {
                        let mut requests = 0;
                        let mut bytes = 0;
                        let mut uniques = 0;

                        if let Some(data) = resp.data {
                            if let Some(viewer) = data.viewer {
                                for z in viewer.zones {
                                    for group in z.http_requests_1d_groups {
                                        if let Some(sum) = group.sum {
                                            requests += sum.requests;
                                            bytes += sum.bytes;
                                        }
                                        if let Some(uniq) = group.uniq {
                                            uniques = uniques.max(uniq.uniques);
                                        }
                                    }
                                }
                            }
                        }
                        (requests, bytes, uniques)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch analytics for zone {}: {:?}", zone.name, e);
                        (0, 0, 0)
                    }
                }
            });
        }
    }

    let results = join_all(tasks).await;

    let mut total_requests = 0;
    let mut total_bandwidth_bytes = 0;
    let mut unique_visitors = 0;

    for (req, b, uniq) in results {
        total_requests += req;
        total_bandwidth_bytes += b;
        unique_visitors += uniq;
    }

    Ok(Json(UnifiedAnalyticsResponse {
        total_requests,
        total_bandwidth_bytes,
        unique_visitors,
    }))
}
