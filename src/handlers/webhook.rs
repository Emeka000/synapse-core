use crate::db::models::Transaction;
use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::types::BigDecimal;
use std::str::FromStr;
use uuid::Uuid;

/// Payload received from Stellar Anchor Platform webhook
#[derive(Debug, Deserialize)]
pub struct CallbackPayload {
    pub id: String,
    pub amount_in: String,
    pub stellar_account: String,
    pub asset_code: String,
    #[serde(default)]
    pub callback_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CallbackResponse {
    pub transaction_id: Uuid,
    pub status: String,
}

/// Validate the callback payload according to business rules
fn validate_payload(payload: &CallbackPayload) -> Result<(), AppError> {
    // Validate amount > 0
    let amount = BigDecimal::from_str(&payload.amount_in)
        .map_err(|_| AppError::Validation("Invalid amount format".to_string()))?;
    
    if amount <= BigDecimal::from(0) {
        return Err(AppError::Validation("Amount must be greater than 0".to_string()));
    }

    // Validate Stellar account address length (should be 56 characters for a valid public key)
    if payload.stellar_account.len() != 56 {
        return Err(AppError::Validation(
            "Invalid Stellar account address length (must be 56 characters)".to_string(),
        ));
    }

    // Validate Stellar account starts with 'G' (public key prefix)
    if !payload.stellar_account.starts_with('G') {
        return Err(AppError::Validation(
            "Stellar account must start with 'G'".to_string(),
        ));
    }

    // Validate asset code length (max 12 characters per Stellar spec)
    if payload.asset_code.is_empty() || payload.asset_code.len() > 12 {
        return Err(AppError::Validation(
            "Asset code must be between 1 and 12 characters".to_string(),
        ));
    }

    Ok(())
}

/// Handle POST /callback/transaction endpoint
/// Receives fiat deposit events from Stellar Anchor Platform
pub async fn handle_callback(
    State(state): State<AppState>,
    Json(payload): Json<CallbackPayload>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(
        "Received callback for transaction {} with amount {} {}",
        payload.id,
        payload.amount_in,
        payload.asset_code
    );

    // Validate payload
    validate_payload(&payload)?;

    // Parse amount
    let amount = BigDecimal::from_str(&payload.amount_in)
        .map_err(|_| AppError::Validation("Invalid amount format".to_string()))?;

    // Create transaction model
    let transaction = Transaction::new(
        payload.stellar_account.clone(),
        amount,
        payload.asset_code.clone(),
        Some(payload.id.clone()),
        payload.callback_type.clone(),
        payload.status.clone(),
    );

    // Insert into database
    let result = sqlx::query!(
        r#"
        INSERT INTO transactions (
            id, stellar_account, amount, asset_code, status,
            created_at, updated_at, anchor_transaction_id, callback_type, callback_status
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id
        "#,
        transaction.id,
        transaction.stellar_account,
        transaction.amount,
        transaction.asset_code,
        transaction.status,
        transaction.created_at,
        transaction.updated_at,
        transaction.anchor_transaction_id,
        transaction.callback_type,
        transaction.callback_status,
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?;

    tracing::info!(
        "Transaction {} persisted with status: pending",
        result.id
    );

    let response = CallbackResponse {
        transaction_id: result.id,
        status: "pending".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Legacy webhook handler - kept for backward compatibility
/// The idempotency middleware should be applied to this handler
#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub id: String,
    pub anchor_transaction_id: String,
}

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub success: bool,
    pub message: String,
}

pub async fn handle_webhook(
    State(state): State<AppState>,
    Json(payload): Json<WebhookPayload>,
) -> impl IntoResponse {
    tracing::info!("Processing webhook with id: {}", payload.id);
    
    let response = WebhookResponse {
        success: true,
        message: format!("Webhook {} processed successfully", payload.id),
    };

    (StatusCode::OK, Json(response))
}
