use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::error::AppError;
use crate::models::CreateDnsRecordRequest;
use crate::state::AppState;

pub async fn list_dns_records(
    State(state): State<AppState>,
    Path(zone_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let (_, token) = state.resolve_token_for_zone(&zone_id).await?;
    
    let cache_key = state.cache.make_key("dns", &token, Some(&zone_id));
    if let Some(cached) = state.cache.get::<Vec<crate::cloudflare::models::CfDnsRecord>>(&cache_key).await {
        return Ok(Json(cached));
    }

    let records = state.cf.list_dns_records(&token, &zone_id).await?;
    state.cache.set(&cache_key, &records, std::time::Duration::from_secs(60)).await;
    
    Ok(Json(records))
}

pub async fn create_dns_record(
    State(state): State<AppState>,
    Path(zone_id): Path<String>,
    Json(payload): Json<CreateDnsRecordRequest>,
) -> Result<impl IntoResponse, AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest("DNS record name cannot be empty".to_string()));
    }
    if payload.data.is_none() && payload.content.trim().is_empty() {
        return Err(AppError::BadRequest("DNS record content cannot be empty".to_string()));
    }

    let (_, token) = state.resolve_token_for_zone(&zone_id).await?;
    let record = state.cf.create_dns_record(&token, &zone_id, &payload).await?;

    // Invalidate caches
    state.cache.clear().await;

    Ok((StatusCode::CREATED, Json(record)))
}

pub async fn delete_dns_record(
    State(state): State<AppState>,
    Path((zone_id, record_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let (_, token) = state.resolve_token_for_zone(&zone_id).await?;
    let _ = state.cf.delete_dns_record(&token, &zone_id, &record_id).await?;

    // Invalidate caches
    state.cache.clear().await;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_dns_record(
    State(state): State<AppState>,
    Path((zone_id, record_id)): Path<(String, String)>,
    Json(payload): Json<CreateDnsRecordRequest>,
) -> Result<impl IntoResponse, AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest("DNS record name cannot be empty".to_string()));
    }
    if payload.data.is_none() && payload.content.trim().is_empty() {
        return Err(AppError::BadRequest("DNS record content cannot be empty".to_string()));
    }

    let (_, token) = state.resolve_token_for_zone(&zone_id).await?;
    let record = state.cf.update_dns_record(&token, &zone_id, &record_id, &payload).await?;

    // Invalidate caches
    state.cache.clear().await;

    Ok(Json(record))
}
