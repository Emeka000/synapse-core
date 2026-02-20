use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::db::queries;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub id: String,
    pub anchor_transaction_id: String,
    // Add other webhook fields as needed
}

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub success: bool,
    pub message: String,
}

/// Handle incoming webhook callbacks
/// The idempotency middleware should be applied to this handler
pub async fn handle_webhook(
    State(_state): State<AppState>,
    Json(payload): Json<WebhookPayload>,
) -> impl IntoResponse {
    tracing::info!("Processing webhook with id: {}", payload.id);

    // Process the webhook (e.g., create transaction, update database)
    // This is where your business logic goes
    
    let response = WebhookResponse {
        success: true,
        message: format!("Webhook {} processed successfully", payload.id),
    };

    (StatusCode::OK, Json(response))
}

/// Callback endpoint for transactions (placeholder)
pub async fn callback(State(_state): State<AppState>) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

/// Get a specific transaction
pub async fn get_transaction(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let transaction = queries::get_transaction(&state.db, id).await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("Transaction {} not found", id)),
            _ => AppError::DatabaseError(e.to_string()),
        })?;

    Ok(Json(transaction))}