use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures::future::join_all;
use crate::error::AppError;
use crate::models::{CreateZoneRequest, ZoneSummaryResponse};
use crate::state::AppState;

pub async fn list_zones(
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

            match state.cf.list_zones(&token).await {
                Ok(zones) => {
                    let mut result = Vec::with_capacity(zones.len());
                    for z in zones {
                        let _ = state.db.upsert_zone_cache(&z.id, &acc.id).await;
                        result.push(ZoneSummaryResponse {
                            account_name: acc.account_name.clone(),
                            account_id: acc.id.clone(),
                            name: z.name,
                            status: z.status,
                            id: z.id,
                            name_servers: z.name_servers,
                        });
                    }
                    result
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch zones for account {}: {:?}", acc.id, e);
                    Vec::new()
                }
            }
        }
    });

    let results: Vec<Vec<ZoneSummaryResponse>> = join_all(tasks).await;
    let unified_zones: Vec<ZoneSummaryResponse> = results.into_iter().flatten().collect();

    Ok(Json(unified_zones))
}

pub async fn create_zone(
    State(state): State<AppState>,
    Json(payload): Json<CreateZoneRequest>,
) -> Result<impl IntoResponse, AppError> {
    if payload.domain_name.trim().is_empty() {
        return Err(AppError::BadRequest("Domain name cannot be empty".to_string()));
    }

    let account = state
        .db
        .get_account(&payload.account_id)
        .await?
        .ok_or_else(|| AppError::AccountNotFound(payload.account_id.clone()))?;

    let token = state.crypto.decrypt(&account.encrypted_token)?;

    let cf_account_id = match &payload.cf_account_id {
        Some(id) if !id.trim().is_empty() => id.clone(),
        _ => {
            // Automatically resolve Cloudflare Account ID using the token
            let cf_accounts = state.cf.get_cf_accounts(&token).await?;
            cf_accounts
                .first()
                .map(|a| a.id.clone())
                .ok_or_else(|| {
                    AppError::CloudflareApi(
                        "No Cloudflare account found associated with this token".to_string(),
                    )
                })?
        }
    };

    let zone = state
        .cf
        .create_zone(&token, &payload.domain_name, &cf_account_id)
        .await?;

    let _ = state.db.upsert_zone_cache(&zone.id, &account.id).await;

    Ok((
        StatusCode::CREATED,
        Json(ZoneSummaryResponse {
            account_name: account.account_name,
            account_id: account.id,
            name: zone.name,
            status: zone.status,
            id: zone.id,
            name_servers: zone.name_servers,
        }),
    ))
}
