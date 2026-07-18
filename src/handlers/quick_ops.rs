use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::error::AppError;
use crate::models::{PurgeCacheRequest, UpdateSslRequest};
use crate::state::AppState;

pub async fn purge_cache(
    State(state): State<AppState>,
    Path(zone_id): Path<String>,
    _payload: Option<Json<PurgeCacheRequest>>,
) -> Result<impl IntoResponse, AppError> {
    let (_, token) = state.resolve_token_for_zone(&zone_id).await?;
    let result = state.cf.purge_cache(&token, &zone_id).await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "id": result.id
        })),
    ))
}

pub async fn update_ssl(
    State(state): State<AppState>,
    Path(zone_id): Path<String>,
    Json(payload): Json<UpdateSslRequest>,
) -> Result<impl IntoResponse, AppError> {
    let valid_modes = ["off", "flexible", "full", "strict"];
    let mode = payload.value.to_lowercase();

    if !valid_modes.contains(&mode.as_str()) {
        return Err(AppError::BadRequest(
            "Invalid SSL mode. Must be one of: off, flexible, full, strict".to_string(),
        ));
    }

    let (_, token) = state.resolve_token_for_zone(&zone_id).await?;
    let result = state.cf.update_ssl_settings(&token, &zone_id, &mode).await?;

    Ok((StatusCode::OK, Json(result)))
}
