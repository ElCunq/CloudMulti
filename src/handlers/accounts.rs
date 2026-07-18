use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::error::AppError;
use crate::models::{AccountResponse, CreateAccountRequest};
use crate::state::AppState;

pub async fn create_account(
    State(state): State<AppState>,
    Json(payload): Json<CreateAccountRequest>,
) -> Result<impl IntoResponse, AppError> {
    if payload.account_name.trim().is_empty() {
        return Err(AppError::BadRequest("Account name cannot be empty".to_string()));
    }
    if payload.api_token.trim().is_empty() {
        return Err(AppError::BadRequest("API token cannot be empty".to_string()));
    }

    // Probe request to GET https://api.cloudflare.com/client/v4/user/tokens/verify
    let _verify_result = state.cf.verify_token(&payload.api_token).await?;

    // If HTTP 200/Active, encrypt the token
    let encrypted_token = state.crypto.encrypt(&payload.api_token)?;
    let id = uuid::Uuid::new_v4().to_string();

    let record = state
        .db
        .insert_account(&id, &payload.account_name, &encrypted_token)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(AccountResponse {
            id: record.id,
            account_name: record.account_name,
            created_at: record.created_at,
        }),
    ))
}

pub async fn list_accounts(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let summaries = state.db.list_accounts().await?;

    let responses: Vec<AccountResponse> = summaries
        .into_iter()
        .map(|s| AccountResponse {
            id: s.id,
            account_name: s.account_name,
            created_at: s.created_at,
        })
        .collect();

    Ok(Json(responses))
}

pub async fn delete_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let rows_affected = state.db.delete_account(&id).await?;
    if rows_affected == 0 {
        return Err(AppError::AccountNotFound(id));
    }

    Ok(StatusCode::NO_CONTENT)
}
