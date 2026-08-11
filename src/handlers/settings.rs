use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZoneSettingsResponse {
    pub ssl_mode: String,
    pub security_level: String,
    pub development_mode: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateZoneSettingsRequest {
    pub ssl_mode: Option<String>,
    pub security_level: Option<String>,
    pub development_mode: Option<String>,
}

pub async fn get_zone_settings(
    State(state): State<AppState>,
    Path(zone_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let (_, token) = state.resolve_token_for_zone(&zone_id).await?;

    let cache_key = state.cache.make_key("settings", &token, Some(&zone_id));
    if let Some(cached) = state.cache.get::<ZoneSettingsResponse>(&cache_key).await {
        return Ok(Json(cached));
    }

    // Fetch settings in parallel
    let ssl_fut = state.cf.get_setting(&token, &zone_id, "ssl");
    let sec_fut = state.cf.get_setting(&token, &zone_id, "security_level");
    let dev_fut = state.cf.get_setting(&token, &zone_id, "development_mode");

    let (ssl_res, sec_res, dev_res) = futures::join!(ssl_fut, sec_fut, dev_fut);

    let ssl_mode = ssl_res?.value;
    let security_level = sec_res?.value;
    let development_mode = dev_res?.value;

    let response = ZoneSettingsResponse {
        ssl_mode,
        security_level,
        development_mode,
    };

    state.cache.set(&cache_key, &response, std::time::Duration::from_secs(60)).await;

    Ok(Json(response))
}

pub async fn update_zone_settings(
    State(state): State<AppState>,
    Path(zone_id): Path<String>,
    Json(payload): Json<UpdateZoneSettingsRequest>,
) -> Result<impl IntoResponse, AppError> {
    let (_, token) = state.resolve_token_for_zone(&zone_id).await?;

    let mut ssl_mode = None;
    let mut security_level = None;
    let mut development_mode = None;

    if let Some(val) = payload.ssl_mode {
        let res = state.cf.update_setting(&token, &zone_id, "ssl", &val).await?;
        ssl_mode = Some(res.value);
    }

    if let Some(val) = payload.security_level {
        let res = state.cf.update_setting(&token, &zone_id, "security_level", &val).await?;
        security_level = Some(res.value);
    }

    if let Some(val) = payload.development_mode {
        let res = state.cf.update_setting(&token, &zone_id, "development_mode", &val).await?;
        development_mode = Some(res.value);
    }

    // Return the current state (fetch any that weren't updated so we always return a full response)
    let final_ssl = match ssl_mode {
        Some(v) => v,
        None => state.cf.get_setting(&token, &zone_id, "ssl").await?.value,
    };

    let final_sec = match security_level {
        Some(v) => v,
        None => state.cf.get_setting(&token, &zone_id, "security_level").await?.value,
    };

    let final_dev = match development_mode {
        Some(v) => v,
        None => state.cf.get_setting(&token, &zone_id, "development_mode").await?.value,
    };

    // Invalidate cached settings
    state.cache.clear().await;

    Ok(Json(ZoneSettingsResponse {
        ssl_mode: final_ssl,
        security_level: final_sec,
        development_mode: final_dev,
    }))
}
