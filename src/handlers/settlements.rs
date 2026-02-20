use axum::{
    extract::{Path, State, Query},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;
use crate::AppState;
use crate::db::queries;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct SettlementListResponse {
    pub settlements: Vec<crate::schemas::SettlementSchema>,
    pub total: i64,
}

/// List all settlements with pagination
/// 
/// Returns a paginated list of settlements
#[utoipa::path(
    get,
    path = "/settlements",
    params(Pagination),
    responses(
        (status = 200, description = "List of settlements retrieved successfully", body = SettlementListResponse),
        (status = 500, description = "Database error")
    ),
    tag = "Settlements"
)]
pub async fn list_settlements(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, AppError> {
    let limit = pagination.limit.unwrap_or(20);
    let offset = pagination.offset.unwrap_or(0);

    let settlements = queries::list_settlements(&state.db, limit, offset).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let settlement_schemas = settlements
        .into_iter()
        .map(|s| crate::schemas::SettlementSchema {
            id: s.id.to_string(),
            asset_code: s.asset_code,
            total_amount: s.total_amount.to_string(),
            tx_count: s.tx_count,
            period_start: s.period_start,
            period_end: s.period_end,
            status: s.status,
            created_at: s.created_at,
            updated_at: s.updated_at,
        })
        .collect();

    Ok(Json(SettlementListResponse {
        settlements: settlement_schemas,
        total: limit,
    }))
}

/// Get a settlement by ID
/// 
/// Returns details for a specific settlement
#[utoipa::path(
    get,
    path = "/settlements/{id}",
    params(
        ("id" = String, Path, description = "Settlement ID")
    ),
    responses(
        (status = 200, description = "Settlement found", body = crate::schemas::SettlementSchema),
        (status = 404, description = "Settlement not found"),
        (status = 500, description = "Database error")
    ),
    tag = "Settlements"
)]
pub async fn get_settlement(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let settlement = queries::get_settlement(&state.db, id).await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("Settlement {} not found", id)),
            _ => AppError::DatabaseError(e.to_string()),
        })?;

    Ok(Json(settlement))
}
